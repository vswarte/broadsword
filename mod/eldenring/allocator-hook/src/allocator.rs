use std::{ffi, ops};
use std::ptr;
use std::mem;
use std::sync;
use std::slice;
use std::alloc;
use std::thread;
use std::sync::mpsc;
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

use crate::{create_allocator_hook, entry};

create_allocator_hook!(heap, 0x142b821b0);

pub(crate) unsafe fn hook() {
    ALLOCATION_TABLE = Some(sync::RwLock::new(collections::BTreeMap::default()));

    let (tx, rx): (mpsc::Sender<AccessEvent>, mpsc::Receiver<AccessEvent>) = mpsc::channel();
    ACCESS_CHANNEL_TX = Some(tx);

    // TODO: clean up thread after done
    thread::spawn(move || {
        for event in rx {
            let allocation = {
                // Make range for memory page so we can query the allocation table
                let range = {
                    let nth_page = event.access_address / 4096;
                    let lower = nth_page.clone() * 4096;
                    let upper = (nth_page.clone() + 1) * 4096;

                    (Included(lower), Excluded(upper))
                };

                let table = ALLOCATION_TABLE.as_ref().unwrap()
                    .read().unwrap();

                let mut result = None;
                for (key, entry) in table.range(range) {
                    if entry.range.contains(&event.access_address) {
                        result = Some((key.clone(), entry.size.clone()));
                    }
                }

                result
            };

            // Ensure the instruction was touching one of our allocations
            if allocation.is_none() {
                continue;
            }

            let (ptr, size) = allocation.unwrap();
            if event.is_write {
                let data = get_written_data(
                    sample_instruction(event.instruction_address),
                    &event.context
                );
                info!("Data: {:#?}", data);
            }

            // if event.access_address == ptr {
                // match runtime::get_rtti_classname(ptr.into()) {
                //     None => {}
                //     Some(c) => {
                //         info!("From sidekick thread: {:#x} (size: {:#x} got reassigned to class {}", ptr, size, c);
                //         let instruction = sample_instruction(event.instruction_address);
                //         log_instruction(instruction);
                //         log_exception_context(&event.context);
                //     }
                // }
            // }
        }
    });

    unsafe {
        AddVectoredExceptionHandler(0x1, Some(exception_filter));
    }

    heap();
}

#[derive(Debug)]
enum WrittenData {
    Byte1(u8),
    Byte2(u16),
    Byte4(u32),
    Byte8(u64),
    Unknown,
}

fn get_written_data(instruction: Instruction, context: &CONTEXT) -> WrittenData {
    log_instruction(instruction);
    log_exception_context(context);
    match instruction.mnemonic() {
        Mnemonic::Mov => {
            match instruction.op1_register() {
                Register::RDI => WrittenData::Byte8(context.Rdi),
                Register::RSI => WrittenData::Byte8(context.Rsi),
                Register::RBP => WrittenData::Byte8(context.Rbp),
                Register::RAX => WrittenData::Byte8(context.Rax),
                Register::RBX => WrittenData::Byte8(context.Rbx),
                Register::RCX => WrittenData::Byte8(context.Rcx),
                Register::RDX => WrittenData::Byte8(context.Rdx),
                Register::R8 => WrittenData::Byte8(context.R8),
                Register::R9 => WrittenData::Byte8(context.R9),
                Register::R14 => WrittenData::Byte8(context.R14),
                Register::R15 => WrittenData::Byte8(context.R15),
                _ => {
                    warn!("Got unknown op1 register! {:#?}", instruction.op1_register());
                    WrittenData::Unknown
                }
            }
        },
        _ => {
            warn!("Got unknown mnemonic! {:#?}", instruction.mnemonic());
            WrittenData::Unknown
        }
    }
}

#[allow(overflowing_literals)]
unsafe extern "system" fn exception_filter(exception_info: *mut EXCEPTION_POINTERS) -> i32 {
    if !has_thread_access_channel() {
        set_thread_access_channel(ACCESS_CHANNEL_TX.as_ref().unwrap().clone());
    }

    let mut exception = *(*exception_info).ExceptionRecord;
    match exception.ExceptionCode.0 {
        // STATUS_GUARD_PAGE_VIOLATION
        0x80000001 => {
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

unsafe fn handle_pageguard_breakpoint(exception_info: *mut EXCEPTION_POINTERS) {
    let instruction_ptr = (*(*exception_info).ExceptionRecord).ExceptionAddress as u64;

    let instruction = sample_instruction(instruction_ptr);
    let next_instruction_ptr = instruction_ptr + instruction.next_ip();

    // Set the release address to after the access instruction
    (*(*exception_info).ContextRecord).EFlags = (*(*exception_info).ContextRecord).EFlags | 0x100;

    set_release_address(next_instruction_ptr);
    set_pageguard_context(*(*exception_info).ContextRecord);

    let access_type = (*(*exception_info).ExceptionRecord).ExceptionInformation[0];
    let access_address = (*(*exception_info).ExceptionRecord).ExceptionInformation[1];

    set_instruction_address(instruction_ptr.clone());
    set_reguard_address(access_address);
    set_is_write(access_type == 0x1);
}

unsafe fn sample_instruction(address: u64) -> Instruction {
    let instruction_slice = slice::from_raw_parts(
        address as *const u8,
        0x20
    );

    let mut decoder = Decoder::new(
        64,
        instruction_slice,
        DecoderOptions::NONE
    );

    decoder.decode()
}

unsafe fn handle_step_breakpoint(exception_info: *mut EXCEPTION_POINTERS) {
    let instruction_ptr = (*(*exception_info).ExceptionRecord).ExceptionAddress as u64;

    // Trap execution until we've found our release address
    if instruction_ptr != get_release_address() {
        (*(*exception_info).ContextRecord).EFlags = (*(*exception_info).ContextRecord).EFlags | 0x100;
        return;
    }

    get_thread_access_channel()
        .send(AccessEvent {
            instruction_address: get_instruction_address(),
            access_address: get_reguard_address(),
            is_write: get_is_write(),
            context: get_pageguard_context(),
        })
        .unwrap();

    // info!("Reapplying page guard at {:#x}", instruction_ptr);
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

// No I don't want to talk about the mutex
static mut ALLOCATION_TABLE: Option<sync::RwLock<collections::BTreeMap<usize, AllocationTableEntry>>> = None;
static mut ACCESS_CHANNEL_TX: Option<mpsc::Sender<AccessEvent>> = None;

struct AccessEvent {
    pub instruction_address: u64,
    pub access_address: usize,
    pub is_write: bool,
    pub context: CONTEXT,
}

thread_local! {
    static THREAD_ACCESS_CHANNEL_TX: RefCell<Option<mpsc::Sender<AccessEvent>>> = RefCell::default();
    static IS_WRITE: RefCell<bool> = RefCell::default();
    static PAGEGUARD_CONTEXT: RefCell<CONTEXT> = RefCell::default();
    static INSTRUCTION_ADDRESS: RefCell<u64> = RefCell::default();
    static RELEASE_ADDRESS: RefCell<u64> = RefCell::default();
    static REGUARD_ADDRESS: RefCell<usize> = RefCell::default();
}

fn has_thread_access_channel() -> bool {
    THREAD_ACCESS_CHANNEL_TX.with_borrow(|t| {
        t.as_ref().is_some()
    })
}

fn set_thread_access_channel(v: mpsc::Sender<AccessEvent>) {
    THREAD_ACCESS_CHANNEL_TX.with_borrow_mut(|t| {
        *t = Some(v);
    });
}

fn get_thread_access_channel() -> mpsc::Sender<AccessEvent> {
    THREAD_ACCESS_CHANNEL_TX.with_borrow(|t| {
        t.as_ref().unwrap().clone()
    })
}

fn set_is_write(v: bool) {
    IS_WRITE.with_borrow_mut(|t| {
        *t = v;
    });
}

fn get_is_write() -> bool {
    IS_WRITE.with_borrow(|t| {
        t.clone()
    })
}

fn set_pageguard_context(v: CONTEXT) {
    PAGEGUARD_CONTEXT.with_borrow_mut(|t| {
        *t = v;
    });
}

fn get_pageguard_context() -> CONTEXT {
    PAGEGUARD_CONTEXT.with_borrow(|t| {
        t.clone()
    })
}

fn set_instruction_address(base: u64) {
    INSTRUCTION_ADDRESS.with_borrow_mut(|t| {
        *t = base;
    });
}

fn get_instruction_address() -> u64 {
    INSTRUCTION_ADDRESS.with_borrow(|t| {
        t.clone()
    })
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

#[derive(Debug, Clone)]
struct AllocationTableEntry {
    pub name: Option<String>,
    pub size: usize,
    pub alignment: usize,
    pub range: ops::Range<usize>,
}

pub fn log_instruction(instruction: Instruction) {
    let mut formatter = NasmFormatter::new();
    let mut output = String::new();
    if instruction.is_invalid() {
        warn!("Tried logging invalid instruction");
        return;
    }

    output.clear();
    formatter.format(&instruction, &mut output);
    info!("{:016X} {}", instruction.ip(), output);
}