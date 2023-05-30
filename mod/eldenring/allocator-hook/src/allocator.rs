use std::mem;
use std::slice;
use std::fmt::write;
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
use windows::Win32::System::Diagnostics::Debug::{AddVectoredExceptionHandler, CONTEXT, EXCEPTION_POINTERS, EXCEPTION_RECORD};

use iced_x86::{Decoder, DecoderOptions, Instruction};

use crate::create_allocator_hook;
use broadsword_memorylog::{MemoryEvent, ReservationEvent, AccessEvent};
use crate::event::{init_event_thread, init_for_thread, get_thread_event_channel};
use crate::allocations::{init_allocation_table, register_allocation, remove_allocation};

create_allocator_hook!(heap, 0x142b821b0);
create_allocator_hook!(network, 0x142b84cb0);

pub(crate) unsafe fn hook() {
    init_allocation_table();
    init_event_thread();

    unsafe {
        AddVectoredExceptionHandler(0x1, Some(exception_filter));
    }

    // heap();
    network();
}

#[allow(overflowing_literals)]
unsafe extern "system" fn exception_filter(exception_info: *mut EXCEPTION_POINTERS) -> i32 {
    init_for_thread();

    let exception = *(*exception_info).ExceptionRecord;
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

    let access_size = instruction.memory_size().size();
    set_access_size(access_size);

    set_reguard_address(access_address as u64);
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
        get_thread_event_channel()
            .send(MemoryEvent::Access(AccessEvent {
                instruction_address: get_instruction_address(),
                instruction_bytes: sample_memory(get_instruction_address() as u64, 15),
                access_address: get_reguard_address(),
                data_after: sample_memory(get_reguard_address(), get_access_size()),
                is_write: get_is_write(),
            }))
            .unwrap();
    }

    set_pageguard((get_reguard_address() as usize).into());
}

unsafe fn sample_memory(address: u64, size: usize) -> Vec<u8> {
    slice::from_raw_parts(
        address as *const u8,
        size
    ).to_vec()
}


thread_local! {
    static ACCESS_SIZE: RefCell<usize> = RefCell::default();
    static IS_WRITE: RefCell<bool> = RefCell::default();
    static INSTRUCTION_ADDRESS: RefCell<u64> = RefCell::default();
    static RELEASE_ADDRESS: RefCell<u64> = RefCell::default();
    static REGUARD_ADDRESS: RefCell<u64> = RefCell::default();
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

fn set_access_size(size: usize) {
    ACCESS_SIZE.with_borrow_mut(|t| {
        *t = size;
    });
}

fn get_access_size() -> usize {
    ACCESS_SIZE.with_borrow(|t| {
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

fn set_reguard_address(base: u64) {
    REGUARD_ADDRESS.with_borrow_mut(|t| {
        *t = base;
    });
}

fn get_reguard_address() -> u64 {
    REGUARD_ADDRESS.with_borrow(|t| {
        t.clone()
    })
}

