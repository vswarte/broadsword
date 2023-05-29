use std::{ffi, fmt, ops};
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
use std::fmt::write;
use std::slice::SliceIndex;

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
use crate::event::{AccessEvent, init_event_thread, init_for_thread, get_thread_event_channel, MemoryEvent, ReservationEvent};
use crate::allocations::{init_allocation_table, register_allocation, remove_allocation};

create_allocator_hook!(heap, 0x142b821b0);

pub(crate) unsafe fn hook() {
    init_allocation_table();
    init_event_thread();

    unsafe {
        AddVectoredExceptionHandler(0x1, Some(exception_filter));
    }

    heap();
}

#[allow(overflowing_literals)]
unsafe extern "system" fn exception_filter(exception_info: *mut EXCEPTION_POINTERS) -> i32 {
    init_for_thread();

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
    set_instruction_address(instruction_ptr);

    let instruction = sample_instruction(instruction_ptr);
    let next_instruction_ptr = instruction_ptr + instruction.next_ip();

    // Set the release address to after the access instruction
    (*(*exception_info).ContextRecord).EFlags = (*(*exception_info).ContextRecord).EFlags | 0x100;

    set_release_address(next_instruction_ptr);

    let access_type = (*(*exception_info).ExceptionRecord).ExceptionInformation[0].clone();
    let access_address = (*(*exception_info).ExceptionRecord).ExceptionInformation[1].clone();

    set_data_sample_before(sample_memory(access_address));

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
        // (*(*exception_info).ContextRecord).EFlags = (*(*exception_info).ContextRecord).EFlags | 0x100;
        return;
    }

    if get_is_write() {
        let after_sample = sample_memory(get_reguard_address());
        get_thread_event_channel()
            .send(MemoryEvent::Access(AccessEvent {
                // instruction: sample_instruction(get_instruction_address()),
                instruction_address: get_instruction_address(),
                access_address: get_reguard_address(),
                data_before: get_data_sample_before(),
                data_after: after_sample,
                is_write: get_is_write(),
            }))
            .unwrap();
    }

    set_pageguard(get_reguard_address().into());
}

unsafe fn sample_memory(address: usize) -> Vec<u8> {
    slice::from_raw_parts(
        address as *const u8,
        0x8
    ).to_vec()
}


thread_local! {
    static DATA_SAMPLE_BEFORE: RefCell<Vec<u8>> = RefCell::default();
    static IS_WRITE: RefCell<bool> = RefCell::default();
    static INSTRUCTION_ADDRESS: RefCell<u64> = RefCell::default();
    static RELEASE_ADDRESS: RefCell<u64> = RefCell::default();
    static REGUARD_ADDRESS: RefCell<usize> = RefCell::default();
}

fn set_data_sample_before(v: Vec<u8>) {
    DATA_SAMPLE_BEFORE.with_borrow_mut(|t| {
        *t = v;
    });
}

fn get_data_sample_before() -> Vec<u8> {
    DATA_SAMPLE_BEFORE.with_borrow(|t| {
        t.clone()
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

