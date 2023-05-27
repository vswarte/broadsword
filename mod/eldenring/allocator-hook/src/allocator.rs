use std::ffi;
use std::ptr;
use std::mem;
use std::sync;
use std::slice;
use std::alloc;
use std::collections;
use std::collections::hash_map::Entry;
use std::ops::Bound::{Included, Excluded};
use std::cell::{Cell, RefCell, UnsafeCell};

use log::*;
use paste::paste;
use broadsword::runtime;
use detour::static_detour;
use tracing::callsite::register;
use broadsword::address::Address;
use tracy::alloc::GlobalAllocator;
use broadsword::runtime::{get_module_pointer_belongs_to, set_pageguard};
use windows::Win32::Foundation::{EXCEPTION_GUARD_PAGE, EXCEPTION_SINGLE_STEP, HANDLE};
use windows::Win32::System::Diagnostics::Debug::{AddVectoredExceptionHandler, CONTEXT, EXCEPTION_POINTERS, EXCEPTION_RECORD, FlushInstructionCache};
use windows::Win32::System::Memory::{MEM_COMMIT, MEM_DECOMMIT, MEM_FREE, MEM_RELEASE, MEM_RESERVE, PAGE_EXECUTE_READWRITE, VIRTUAL_FREE_TYPE, VirtualAlloc, VirtualFree, VirtualProtect};

use iced_x86::{Decoder, DecoderOptions, Instruction, Mnemonic, OpKind, Register, Formatter, NasmFormatter};

use crate::create_allocator_hook;

create_allocator_hook!(heap, 0x142b821b0);

pub(crate) unsafe fn hook() {
    ALLOCATION_TABLE = Some(sync::RwLock::new(collections::BTreeMap::default()));

    unsafe {
        AddVectoredExceptionHandler(0x1, Some(exception_filter));
    }

    heap();
}

#[allow(overflowing_literals)]
unsafe extern "system" fn exception_filter(exception_info: *mut EXCEPTION_POINTERS) -> i32 {
    let mut exception = *(*exception_info).ExceptionRecord;

    match exception.ExceptionCode.0 {
        // STATUS_GUARD_PAGE_VIOLATION
        0x80000001 => {
            let address = exception.ExceptionInformation[1];

            // Make range for memory page so we can query the allocation table
            let range = {
                let nth_page = address / 4096;
                let lower = nth_page.clone() * 4096;
                let upper = (nth_page.clone() + 1) * 4096;

                (Included(lower), Excluded(upper))
            };

            // Ensure the instruction is touching something we page guarded
            if !ALLOCATION_TABLE.as_ref().unwrap().read().unwrap().range(range).count() == 0 {
                return 0;
            }

            handle_pageguard_breakpoint(exception_info);

            -1
        },
        0x80000004 => {
            handle_step_breakpoint(exception_info);
            -1
        },
        _ => 0,
    }
}

#[derive(Clone, Debug)]
enum SandboxPhase {
    Before,
    After,
    None,
}

unsafe fn handle_pageguard_breakpoint(exception_info: *mut EXCEPTION_POINTERS) {
    let instruction_ptr = (*(*exception_info).ExceptionRecord).ExceptionAddress as u64;

    // Disassemble the accessing instruction
    // Create slice of instruction bytes that RIP points to
    let instruction_slice = slice::from_raw_parts(
        instruction_ptr as *const u8,
        0x100
    );

    // Do the actual disassembling
    let mut decoder = Decoder::new(64, instruction_slice, DecoderOptions::NONE);
    let instruction = decoder.decode();

    // Set the release address to after the access instruction
    (*(*exception_info).ContextRecord).EFlags = (*(*exception_info).ContextRecord).EFlags | 0x100;

    let next_instruction_ptr = instruction_ptr + instruction.next_ip();
    // info!("Current IP: {:#x}", instruction_ptr);
    info!("Should trap until IP: {:#x}", next_instruction_ptr);

    set_release_address(next_instruction_ptr);

    let access_address = (*(*exception_info).ExceptionRecord).ExceptionInformation[1];
    info!("Tried accessing {:x}", access_address);
    set_reguard_address(access_address);

    // info!("Yielding control back to game flow...");
    // log_exception_context(&*(*exception_info).ContextRecord);
}

unsafe fn handle_step_breakpoint(exception_info: *mut EXCEPTION_POINTERS) {
    let instruction_ptr = (*(*exception_info).ExceptionRecord).ExceptionAddress as u64;

    // Trap execution until we've found our release address
    if instruction_ptr != get_release_address() {
        (*(*exception_info).ContextRecord).EFlags = (*(*exception_info).ContextRecord).EFlags | 0x100;
        return;
    }

    info!("Reapplying page guard at {:#x}", instruction_ptr);
    // let mut exception = *(*exception_info).ExceptionRecord;
    set_pageguard(get_reguard_address().into());
}

fn log_exception_context(c: &CONTEXT) {
    info!("CONTEXT");
    info!("RIP: {:#x}", c.Rip);
    info!("RAX: {:#x}", c.Rax);
    info!("RBX: {:#x}", c.Rbx);
    info!("RCX: {:#x}", c.Rcx);
    info!("RDX: {:#x}", c.Rdx);
    info!("R8: {:#x}", c.R8);
    info!("R9: {:#x}", c.R9);
    info!("R10: {:#x}", c.R10);
    info!("R11: {:#x}", c.R11);
    info!("R12: {:#x}", c.R12);
    info!("R13: {:#x}", c.R13);
    info!("R14: {:#x}", c.R14);
    info!("R15: {:#x}", c.R15);
    info!("RBP: {:#x}", c.Rbp);
    info!("RSP: {:#x}", c.Rsp);
    info!("RSP[0]: {:#x}", unsafe { *(c.Rsp as *const usize) });
    info!("RSP[1]: {:#x}", unsafe { *((c.Rsp + 0x8) as *const usize) });
    info!("RSP[2]: {:#x}", unsafe { *((c.Rsp + 0x16) as *const usize) });
    info!("RDI: {:#x}", c.Rdi);
    info!("RSI: {:#x}", c.Rsi);
    info!("SEG GS: {:#x}", c.SegGs);
}

// No I don't want to talk about the mutexes
static mut ALLOCATION_TABLE: Option<sync::RwLock<collections::BTreeMap<usize, AllocationTableEntry>>> = None;

thread_local! {
    static RELEASE_ADDRESS: RefCell<u64> = RefCell::default();
    static REGUARD_ADDRESS: RefCell<usize> = RefCell::default();
}

fn set_release_address(base: u64) {
    RELEASE_ADDRESS.with_borrow_mut(|t| {
        *t = base;
    });
}

fn get_release_address() -> u64 {
    RELEASE_ADDRESS.with_borrow(|t| {
        t.clone()
    })
}

fn set_reguard_address(base: usize) {
    REGUARD_ADDRESS.with_borrow_mut(|t| {
        *t = base;
    });
}

fn get_reguard_address() -> usize {
    REGUARD_ADDRESS.with_borrow(|t| {
        t.clone()
    })
}

struct AllocationTableEntry {
    pub name: Option<String>,
    pub layout: alloc::Layout,
}

pub fn log_instruction_buffer(instructions: &[u8], base_address: usize) {
    let mut formatter = NasmFormatter::new();
    let mut output = String::new();
    let decoder = Decoder::with_ip(64, &instructions, base_address as u64, DecoderOptions::NONE);
    for instruction in decoder {
        if instruction.is_invalid() {
            continue;
        }

        output.clear();
        formatter.format(&instruction, &mut output);
        info!("{:016X} {}", instruction.ip(), output);
    }
}

