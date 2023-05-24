use std::ffi;
use std::ptr;
use std::mem;
use std::sync;
use std::slice;
use std::alloc;
use std::collections;
use std::collections::hash_map::Entry;
use std::cell::{Cell, RefCell, UnsafeCell};
use std::sync::Mutex;

use log::*;
use paste::paste;
use broadsword::runtime;
use detour::static_detour;
use tracing::callsite::register;
use broadsword::address::Address;
use tracy::alloc::GlobalAllocator;
use broadsword::runtime::get_module_pointer_belongs_to;
use windows::Win32::Foundation::{EXCEPTION_GUARD_PAGE, EXCEPTION_SINGLE_STEP, HANDLE};
use windows::Win32::System::Diagnostics::Debug::{AddVectoredExceptionHandler, CONTEXT, EXCEPTION_POINTERS, EXCEPTION_RECORD, FlushInstructionCache};
use windows::Win32::System::Memory::{MEM_COMMIT, MEM_DECOMMIT, MEM_FREE, MEM_RELEASE, MEM_RESERVE, PAGE_EXECUTE_READWRITE, VIRTUAL_FREE_TYPE, VirtualAlloc, VirtualFree, VirtualProtect};

use iced_x86::{Decoder, DecoderOptions, Instruction, Mnemonic, OpKind, Register, Formatter, NasmFormatter};

use crate::create_allocator_hook;

create_allocator_hook!(heap, 0x142b821b0);

pub(crate) unsafe fn hook() {
    ALLOCATION_TABLE = Some(sync::RwLock::new(collections::HashMap::default()));
    BP_RESERVATION_TABLE = Some(sync::Mutex::new(collections::HashMap::default()));

    unsafe {
        // Place it first in the list so PAGE_GUARDs doesn't clutter more complex filters
        AddVectoredExceptionHandler(0x1, Some(exception_filter));
    }

    heap();
}

#[allow(overflowing_literals)]
unsafe extern "system" fn exception_filter(exception_info: *mut EXCEPTION_POINTERS) -> i32 {
    init_reguard_table();
    init_breakpoint_table();

    let mut exception = *(*exception_info).ExceptionRecord;
    let mut context = *(*exception_info).ContextRecord;
    let instruction_ptr = exception.ExceptionAddress as usize;

    match exception.ExceptionCode.0 {
        // STATUS_GUARD_PAGE_VIOLATION
        0x80000001 => {
            info!("HIT PAGE GUARD");
            let address = exception.ExceptionInformation[1];
            if !ALLOCATION_TABLE.as_ref().unwrap().read().unwrap().contains_key(&address) {
                if instruction_ptr < 0x1459c5bff || instruction_ptr > 0x140000000 {
                    return -1;
                } else {
                    return 0;
                }
            }

            format_context(&context);

            // Create slice of instruction bytes that RIP points to
            let instruction_slice = slice::from_raw_parts(
                exception.ExceptionAddress as *const u8,
                0x100
            );

            let mut decoder = Decoder::new(64, instruction_slice, DecoderOptions::NONE);
            let instruction = decoder.decode();

            ORIGINAL_CONTEXT.with_borrow_mut(|t| {
                *t = context;
            });

            let sandbox_fn_alloc  = VirtualAlloc(
                None,
                18,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_EXECUTE_READWRITE
            ) as *mut u8;

            let sandbox_fn_slice = slice::from_raw_parts_mut(
                sandbox_fn_alloc,
                18
            );

            for i in 0..18 {
                sandbox_fn_slice[i] = 0x90 as u8;
            }

            sandbox_fn_slice[0] = 0xCC as u8;
            sandbox_fn_slice[sandbox_fn_slice.len() - 2] = 0xCC as u8;
            sandbox_fn_slice[sandbox_fn_slice.len() - 1] = 0xC3 as u8;

            for i in 0..instruction.len() {
                sandbox_fn_slice[i.clone() + 1] = instruction_slice[i.clone()].clone();
            }

            log_instruction_buffer(sandbox_fn_slice, sandbox_fn_alloc as usize);

            BP_TABLE.with_borrow_mut(|t| {
                let mut table = t.as_mut().unwrap();

                let first = sandbox_fn_alloc as usize;
                table.insert(first.clone(), 0x0);

                let second = first + 16;
                table.insert(second, 0x1);
            });

            let sandbox_fn = mem::transmute::<*mut u8, fn()>(sandbox_fn_alloc);
            sandbox_fn();
            
            VirtualFree(
                sandbox_fn_alloc as *mut ffi::c_void,
                0,
                // TODO: MAKE FUCKING MEM_DECOMMIT COUNT
                MEM_RELEASE,
            );

            let result_context = RESULT_CONTEXT.with_borrow(|t| {
                t.clone()
            });

            ptr::copy_nonoverlapping(
                &result_context as *const CONTEXT,
                (*exception_info).ContextRecord,
                mem::size_of::<CONTEXT>()
            );

            (*(*exception_info).ContextRecord).Rip = (instruction_ptr + instruction.len()) as u64;

            -1
        },
        // STATUS_BREAKPOINT
        0x80000003 => {
            format_context(&context);
            return BP_TABLE.with_borrow(|f| {
                match f.as_ref().unwrap().get(&instruction_ptr) {
                    None => 0,
                    Some(e) => {
                        match e {
                            0x0 => {
                                info!("HIT FIRST");
                                let original_context = ORIGINAL_CONTEXT.with_borrow(|t| {
                                    t.clone()
                                });

                                format_context(&original_context);

                                *(*exception_info).ContextRecord = original_context;
                                (*(*exception_info).ContextRecord).Rip = (instruction_ptr + 1) as u64;

                                EMULATION_CONTEXT.with_borrow_mut(|t| {
                                    *t = context;
                                });
                            },
                            0x1 => {
                                RESULT_CONTEXT.with_borrow_mut(|t| {
                                    *t = context;
                                });

                                let emulation_context = EMULATION_CONTEXT.with_borrow(|t| {
                                    t.clone()
                                });

                                ptr::copy_nonoverlapping(
                                    &emulation_context as *const CONTEXT,
                                    (*exception_info).ContextRecord,
                                    mem::size_of::<CONTEXT>()
                                );

                                (*(*exception_info).ContextRecord).Rip = (instruction_ptr + 1) as u64;
                            }
                            _ => {
                                error!("HIT UNKNOWN EMULATION HOOK PHASE");
                                todo!("HIT UNKNOWN EMULATION HOOK PHASE");
                            },
                        };

                        -1
                    }
                }
            });
        }
        _ => 0,
    }
}

fn format_context(c: &CONTEXT) {
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
    info!("RDI: {:#x}", c.Rdi);
    info!("RSI: {:#x}", c.Rsi);
    info!("SEG GS: {:#x}", c.SegGs);
}

unsafe fn place_breakpoint(ptr: usize) {
    let mut byte_ptr = ptr.clone() as *mut u8;
    let original_byte = *byte_ptr.clone();

    if ptr > 0x1459c5bff || ptr < 0x140000000 {
        warn!("Tried placing BP in area outside of expected range {:#x}", ptr);
        match get_module_pointer_belongs_to(ptr) {
            Some(m) => warn!("Found module for {:#x}: {}", ptr, m.name),
            None => warn!("Could not find module for {:#x}", ptr),
        }

        return;
    }

    *byte_ptr = 0xCC as u8;

    FlushInstructionCache(
        HANDLE(-1),
        Some(ptr.clone() as *const ffi::c_void),
        15
    );

    {
        let mut table = BP_RESERVATION_TABLE.as_mut().unwrap().lock().unwrap();
        table.insert(ptr, original_byte);
    }
}

unsafe fn remove_breakpoint(ptr: usize) {
    let mut byte_ptr = ptr as *mut u8;
    *byte_ptr = {
        let mut table = BP_RESERVATION_TABLE.as_ref().unwrap().lock().unwrap();
        table.get(&ptr).unwrap().clone()
    };

    info!("Restored byte: {:#x} {:#x}", byte_ptr as usize, *byte_ptr);

    FlushInstructionCache(
        HANDLE(-1),
        Some(ptr.clone() as *const ffi::c_void),
        15
    );
}

// No I don't want to talk about the mutexes
static mut ALLOCATION_TABLE: Option<sync::RwLock<collections::HashMap<usize, AllocationTableEntry>>> = None;
static mut BP_RESERVATION_TABLE: Option<sync::Mutex<collections::HashMap<usize, u8>>> = None;

thread_local! {
    static REGUARD_TABLE: RefCell<Option<collections::HashMap<usize, usize>>> = RefCell::default();
    static BP_TABLE: RefCell<Option<collections::HashMap<usize, u8>>> = RefCell::default();

    static ORIGINAL_CONTEXT: RefCell<CONTEXT> = RefCell::default();
    static EMULATION_CONTEXT: RefCell<CONTEXT> = RefCell::default();
    static RESULT_CONTEXT: RefCell<CONTEXT> = RefCell::default();
}

fn init_reguard_table() {
    REGUARD_TABLE.with_borrow_mut(|f| {
        if f.is_none() {
            *f = Some(collections::HashMap::default());
        }
    });
}

fn init_breakpoint_table() {
    BP_TABLE.with_borrow_mut(|f| {
        if f.is_none() {
            *f = Some(collections::HashMap::default());
        }
    });
}

struct AllocationTableEntry {
    pub name: Option<String>,
    // pub invalidated: bool,
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

