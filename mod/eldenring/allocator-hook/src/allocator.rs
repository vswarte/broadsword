use std::mem;
use std::sync;
use std::slice;
use std::alloc;
use std::collections;
use std::collections::hash_map::Entry;

use log::*;
use paste::paste;
use broadsword::runtime;
use detour::static_detour;
use tracy::alloc::GlobalAllocator;
use windows::Win32::Foundation::EXCEPTION_GUARD_PAGE;
use broadsword::runtime::{get_rtti_classname, set_pageguard};
use windows::Win32::System::Diagnostics::Debug::{AddVectoredExceptionHandler, EXCEPTION_POINTERS};

use iced_x86::{
    Decoder, DecoderOptions, Instruction, Mnemonic, OpKind, Register,
};


use crate::create_allocator_hook;

create_allocator_hook!(heap, 0x142b821b0);

pub(crate) unsafe fn hook() {
    ALLOCATION_TABLE = Some(sync::RwLock::new(collections::HashMap::default()));

    unsafe {
        // Place it first in the list so PAGE_GUARDs doesn't clutter more complex filters
        AddVectoredExceptionHandler(0x1, Some(exception_filter));
    }

    heap();
}

unsafe extern "system" fn exception_filter(exception_info: *mut EXCEPTION_POINTERS) -> i32 {
    let exception_record = *(*exception_info).ExceptionRecord;

    match exception_record.ExceptionCode {
        EXCEPTION_GUARD_PAGE => {
            let access_instruction = (*(*exception_info).ContextRecord).Rip;
            let access_type = (*(*exception_info).ExceptionRecord).ExceptionInformation[0];
            let access_address = (*(*exception_info).ExceptionRecord).ExceptionInformation[1];

            // Break early if the access type wasn't a write
            if access_type != 0x1 {
                return -1;
            }

            // Check if write was to start of structure by checking the access address against the
            // allocation table.
            let is_top_write = {
                let table = ALLOCATION_TABLE.as_ref().unwrap().read().unwrap();
                table.contains_key(&access_address)
            };

            // Break early if it's not writing to the first 8 bytes where the vftable ptr lives.
            if !is_top_write {
                return -1;
            }

            // Create slice of instruction bytes that RIP points to
            let instruction_slice = slice::from_raw_parts(
                 access_instruction as *const u8,
                0x3
            );

            let mut decoder = Decoder::new(64, instruction_slice, DecoderOptions::NONE);

            // We just need a single instruction
            let mut instruction = Instruction::default();
            decoder.decode_out(&mut instruction);

            // In case the instruction did not fit within the 3 bytes we're probably not staring at
            // a MOV.
            if instruction.is_invalid() ||
                instruction.mnemonic() != Mnemonic::Mov ||
                instruction.op_kind(0) != OpKind::Memory {
                return -1;
            }

            let copy_from_register = instruction.op1_register();
            let context = *(*exception_info).ContextRecord;
            let moved_data= match copy_from_register {
                Register::RDI => context.Rdi,
                Register::RSI => context.Rsi,
                Register::RBP => context.Rbp,
                Register::RAX => context.Rax,
                Register::RBX => context.Rbx,
                Register::RCX => context.Rcx,
                Register::RDX => context.Rdx,
                Register::R8 => context.R8,
                Register::R9 => context.R9,
                Register::R14 => context.R14,
                Register::R15 => context.R15,
                _ => {
                    warn!("Unmapped or non-8-byte register: {:#?} ", copy_from_register);
                    return -1;
                }
            } as usize;

            // Perform the write ourselves...
            *(access_address as *mut usize) = moved_data;

            // Move the RIP forward as we did the move ourselves
            (*(*exception_info).ContextRecord).Rip = (*(*exception_info).ContextRecord).Rip + 3;

            match get_rtti_classname(moved_data.into()) {
                None => {}
                Some(name) => {
                    info!("Recovered RTTI classname: {:#?} -> {}", access_instruction, name);
                    set_pageguard(access_address.into());
                }
            }

            return -1;
        },
        _ => 0,
    }
}

// No I don't want to talk about the mutexes
static mut ALLOCATION_TABLE: Option<sync::RwLock<collections::HashMap<usize, AllocationTableEntry>>> = None;

struct AllocationTableEntry {
    pub name: Option<String>,
    // pub invalidated: bool,
    pub layout: alloc::Layout,
}
