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

create_allocator_hook!(heap, 0x142b821b0);

#[no_mangle]
pub(crate) unsafe fn EnableAllocatorHooks() {
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

            if !event.is_write {
                continue;
            }

            let (allocation_ptr, size) = allocation.unwrap();
            if allocation_ptr != event.access_address {
                continue;
            }

            // let mutation = get_mutation(
            //     event.instruction,
            //     &event.context
            // );
            //
            // match mutation {
            //     Mutation::Set(value) => {
            //         match value {
            //             RegisterValue::Byte8(v) => {
            //                 match runtime::get_rtti_classname((v as usize).into()) {
            //                     None => {}
            //                     Some(c) => {
            //                         info!("{:#x} (size: {:#x} got reassigned to class {:#x} {})", allocation_ptr, size, v, c);
            //                     }
            //                 }
            //             }
            //             _ => {}
            //         }
            //     }
            //     _ => {}
            // }
        }
    });

    unsafe {
        AddVectoredExceptionHandler(0x1, Some(exception_filter));
    }

    heap();
}

enum RegisterValue {
    Byte1(u8),
    Byte2(u16),
    Byte4(u32),
    Byte8(u64),
    Unknown,
}

impl fmt::Debug for RegisterValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RegisterValue::Byte1(v) => write!(f, "BYTE({:#x})", v),
            RegisterValue::Byte2(v) => write!(f, "SHORT({:#x})", v),
            RegisterValue::Byte4(v) => write!(f, "DWORD({:#x})", v),
            RegisterValue::Byte8(v) => write!(f, "QWORD({:#x})", v),
            RegisterValue::Unknown => write!(f, "UNKNOWN"),
        }
    }
}


enum RegisterSize {
    Byte1,
    Byte2,
    Byte4,
    Byte8,
}

#[derive(Debug)]
enum Mutation {
    Set(RegisterValue),
    Add(RegisterValue),
    Subtract(RegisterValue),
    Increment(usize),
    Decrement(usize),
    Unknown,
}

fn get_mutation(instruction: Instruction, context: &CONTEXT) -> Mutation {
    match instruction.mnemonic() {
        // Mnemonic::Inc => Mutation::Increment(instruction.op0_register().size()),
        // Mnemonic::Dec => Mutation::Decrement(instruction.op0_register().size()),
        //
        // Mnemonic::Xadd => {
        //     let value = match instruction.op1_register() {
        //         Register::None => {
        //             log_instruction(instruction);
        //             RegisterValue::Unknown
        //         },
        //         _ => sample_register(instruction.op1_register(), context),
        //     };
        //
        //     Mutation::Add(value)
        // },

        Mnemonic::Mov => {
            let value = match instruction.op1_register() {
                Register::None => {
                    log_instruction(instruction);
                    RegisterValue::Unknown
                },
                _ => sample_register(instruction.op1_register(), context),
            };

            Mutation::Set(value)
        },

        _ => {
            // warn!("Got unknown mnemonic! {:#?}", instruction.mnemonic());
            // log_instruction(instruction);
            // log_exception_context(context);
            Mutation::Unknown
        }
    }
}

fn sample_register(register: Register, context: &CONTEXT) -> RegisterValue {
    match register {
        Register::DIL => RegisterValue::Byte1(context.Rdi as u8),
        Register::DI => RegisterValue::Byte2(context.Rdi as u16),
        Register::EDI => RegisterValue::Byte4(context.Rdi as u32),
        Register::RDI => RegisterValue::Byte8(context.Rdi),

        Register::SIL => RegisterValue::Byte1(context.Rsi as u8),
        Register::SI => RegisterValue::Byte2(context.Rsi as u16),
        Register::ESI => RegisterValue::Byte4(context.Rsi as u32),
        Register::RSI => RegisterValue::Byte8(context.Rsi),

        Register::BPL => RegisterValue::Byte1(context.Rbp as u8),
        Register::BP => RegisterValue::Byte2(context.Rbp as u16),
        Register::EBP => RegisterValue::Byte4(context.Rbp as u32),
        Register::RBP => RegisterValue::Byte8(context.Rbp),

        Register::AL => RegisterValue::Byte1(context.Rax as u8),
        Register::AX => RegisterValue::Byte2(context.Rax as u16),
        Register::EAX => RegisterValue::Byte4(context.Rax as u32),
        Register::RAX => RegisterValue::Byte8(context.Rax as u64),

        Register::BL => RegisterValue::Byte1(context.Rbx as u8),
        Register::BX => RegisterValue::Byte2(context.Rbx as u16),
        Register::EBX => RegisterValue::Byte4(context.Rbx as u32),
        Register::RBX => RegisterValue::Byte8(context.Rbx),

        Register::CL => RegisterValue::Byte1(context.Rcx as u8),
        Register::CX => RegisterValue::Byte2(context.Rcx as u16),
        Register::ECX => RegisterValue::Byte4(context.Rcx as u32),
        Register::RCX => RegisterValue::Byte8(context.Rcx),

        Register::DL => RegisterValue::Byte1(context.Rdx as u8),
        Register::DX => RegisterValue::Byte2(context.Rdx as u16),
        Register::EDX => RegisterValue::Byte4(context.Rdx as u32),
        Register::RDX => RegisterValue::Byte8(context.Rdx),

        Register::R8L => RegisterValue::Byte1(context.R8 as u8),
        Register::R8W => RegisterValue::Byte2(context.R8 as u16),
        Register::R8D => RegisterValue::Byte4(context.R8 as u32),
        Register::R8 => RegisterValue::Byte8(context.R8),

        Register::R9L => RegisterValue::Byte1(context.R9 as u8),
        Register::R9W => RegisterValue::Byte2(context.R9 as u16),
        Register::R9D => RegisterValue::Byte4(context.R9 as u32),
        Register::R9 => RegisterValue::Byte8(context.R9),

        Register::R10L => RegisterValue::Byte1(context.R10 as u8),
        Register::R10W => RegisterValue::Byte2(context.R10 as u16),
        Register::R10D => RegisterValue::Byte4(context.R10 as u32),
        Register::R10 => RegisterValue::Byte8(context.R10),

        Register::R11L => RegisterValue::Byte1(context.R11 as u8),
        Register::R11W => RegisterValue::Byte2(context.R11 as u16),
        Register::R11D => RegisterValue::Byte4(context.R11 as u32),
        Register::R11 => RegisterValue::Byte8(context.R11),

        Register::R12L => RegisterValue::Byte1(context.R12 as u8),
        Register::R12W => RegisterValue::Byte2(context.R12 as u16),
        Register::R12D => RegisterValue::Byte4(context.R12 as u32),
        Register::R12 => RegisterValue::Byte8(context.R12),

        Register::R13L => RegisterValue::Byte1(context.R13 as u8),
        Register::R13W => RegisterValue::Byte2(context.R13 as u16),
        Register::R13D => RegisterValue::Byte4(context.R13 as u32),
        Register::R13 => RegisterValue::Byte8(context.R13),

        Register::R14L => RegisterValue::Byte1(context.R14 as u8),
        Register::R14W => RegisterValue::Byte2(context.R14 as u16),
        Register::R14D => RegisterValue::Byte4(context.R14 as u32),
        Register::R14 => RegisterValue::Byte8(context.R14),

        Register::R15L => RegisterValue::Byte1(context.R15 as u8),
        Register::R15W => RegisterValue::Byte2(context.R15 as u16),
        Register::R15D => RegisterValue::Byte4(context.R15 as u32),
        Register::R15 => RegisterValue::Byte8(context.R15),
        _ => {
            warn!("Got unknown register! {:#?}", register);
            RegisterValue::Unknown
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
    set_instruction_address(instruction_ptr);

    let instruction = sample_instruction(instruction_ptr);
    let next_instruction_ptr = instruction_ptr + instruction.next_ip();

    // Set the release address to after the access instruction
    (*(*exception_info).ContextRecord).EFlags = (*(*exception_info).ContextRecord).EFlags | 0x100;

    set_release_address(next_instruction_ptr);
    set_pageguard_context(*(*exception_info).ContextRecord);

    let access_type = (*(*exception_info).ExceptionRecord).ExceptionInformation[0];
    let access_address = (*(*exception_info).ExceptionRecord).ExceptionInformation[1];

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
            instruction: sample_instruction(get_instruction_address()),
            instruction_address: get_instruction_address(),
            access_address: get_reguard_address(),
            is_write: get_is_write(),
            context: get_pageguard_context(),
        })
        .unwrap();

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
    pub instruction: Instruction,
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
    // info!("{:016X} {}", instruction.ip(), output);
}