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
use broadsword::runtime::get_module_pointer_belongs_to;
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
    init_reguard_table();
    init_breakpoint_table();

    let mut exception = *(*exception_info).ExceptionRecord;
    let mut context = *(*exception_info).ContextRecord;
    let instruction_ptr = exception.ExceptionAddress as usize;

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

            // Disassemble the touching instruction
            // Create slice of instruction bytes that RIP points to
            let instruction_slice = slice::from_raw_parts(
                exception.ExceptionAddress as *const u8,
                0x100
            );

            // Do the actual disassembling
            let mut decoder = Decoder::new(64, instruction_slice, DecoderOptions::NONE);
            let instruction = decoder.decode();

            // Store the original instructions context
            ORIGINAL_RSP.with_borrow_mut(|t| {
                let rsp = context.Rsp.clone();
                info!("Setting original RSP: {:x}", rsp);
                *t = rsp;
            });

            ORIGINAL_CONTEXT.with_borrow_mut(|t| {
                info!("Original context from page guard hook");
                format_context(&*(*exception_info).ContextRecord);
                *t = context;
            });

            // TODO: we can store this alloc in a TLS slot and clean up with nops after every run
            // Build instruction buffer
            let sandbox_fn_alloc  = VirtualAlloc(
                None,
                0x18,
                MEM_COMMIT | MEM_RESERVE,
                PAGE_EXECUTE_READWRITE
            ) as *mut u8;

            let sandbox_fn_slice = slice::from_raw_parts_mut(
                sandbox_fn_alloc,
                0x18
            );

            // 0:  54                      push   rsp
            // 1:  cc                      int3
            // 2:  90                      nop
            // 3:  90                      nop
            // 4:  90                      nop
            // 5:  90                      nop
            // 6:  90                      nop
            // 7:  90                      nop
            // 8:  90                      nop
            // 9:  90                      nop
            // a:  90                      nop
            // b:  90                      nop
            // c:  90                      nop
            // d:  90                      nop
            // e:  90                      nop
            // f:  90                      nop
            // 10: 90                      nop
            // 11: 48 8b 64 24 f8          mov    rsp,QWORD PTR [rsp-0x8]
            // 16: cc                      int3
            // 17: c3                      ret
            let template: [u8; 0x18] = [
                0x54,
                0xCC,
                0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90, 0x90,
                0x48, 0x8B, 0x64, 0x24, 0xF8,
                0xCC,
                0xC3
            ];
            sandbox_fn_slice.copy_from_slice(&template);

            for i in 0..instruction.len() {
                sandbox_fn_slice[i.clone() + 2] = instruction_slice[i.clone()].clone();
            }

            log_instruction_buffer(sandbox_fn_slice, 0x0);

            BP_TABLE.with_borrow_mut(|t| {
                let mut table = t.as_mut().unwrap();
                table.insert((sandbox_fn_alloc as usize) + 0x1, 0x0);
                table.insert((sandbox_fn_alloc as usize) + 0x16, 0x1);
            });

            mem::transmute::<*mut u8, unsafe extern "system" fn()>(sandbox_fn_alloc)();
            info!("Hit after");

            // VirtualFree(
            //     sandbox_fn_alloc as *mut ffi::c_void,
            //     0,
            //     MEM_RELEASE,
            // );


            RESULT_CONTEXT.with_borrow(|t| {
                info!("Overwriting original exception context");
                ptr::copy_nonoverlapping(
                    t as *const CONTEXT,
                    (*exception_info).ContextRecord,
                    1
                );
                info!("Overwritten original exception context");
            });

            info!("Restoring original RSP");
            let original_rsp = ORIGINAL_RSP.with_borrow(|t| {
                t.clone()
            });

            info!("Restoring original RSP: {:x}", original_rsp);

            (*(*exception_info).ContextRecord).Rip = (instruction_ptr + instruction.len()) as u64;
            (*(*exception_info).ContextRecord).Rsp = original_rsp as u64;

            info!("Yielding control back to game flow...");
            format_context(&*(*exception_info).ContextRecord);

            -1
        },
        0x80000003 => {
            let bp_entry = BP_TABLE.with_borrow(|f| {
                f.as_ref().unwrap().get(&instruction_ptr).map(|x| x.clone()).clone()
            });

            match bp_entry {
                None => 0,
                Some(e) => {
                    match e {
                        0x0 => {
                            let rsp = (*(*exception_info).ContextRecord).Rsp.clone();

                            ORIGINAL_RIP.with_borrow_mut(|t| {
                                let rip = *((context.Rsp + 0x8) as *const u64);
                                info!("Setting original RIP: {:x}", rip);
                                *t = rip;
                            });

                            EMULATION_CONTEXT.with_borrow_mut(|t| {
                                *t = context.clone();
                            });

                            ORIGINAL_CONTEXT.with_borrow(|t| {
                                ptr::copy_nonoverlapping(
                                    t as *const CONTEXT,
                                    (*exception_info).ContextRecord,
                                    1
                                );
                            });

                            (*(*exception_info).ContextRecord).Rsp = rsp;
                            (*(*exception_info).ContextRecord).Rip = (instruction_ptr + 1) as u64;

                            info!("First INT3 restored context");
                            format_context(&*(*exception_info).ContextRecord);
                        },

                        0x1 => {
                            info!("Hit second BP");
                            RESULT_CONTEXT.with_borrow_mut(|t| {
                                *t = context;
                            });

                            info!("Original context after sandboxed instruction");
                            format_context(&*(*exception_info).ContextRecord);

                            EMULATION_CONTEXT.with_borrow(|t| {
                                ptr::copy_nonoverlapping(
                                  t as *const CONTEXT,
                                  (*exception_info).ContextRecord,
                                  1
                                );
                            });

                            info!("Context after emulation context restore");
                            format_context(&*(*exception_info).ContextRecord);

                            let new_rip = ORIGINAL_RIP.with_borrow_mut(|t| {
                                t.clone()
                            });
                            let module = get_module_pointer_belongs_to(new_rip as usize);
                            info!("New RIP: {:x} {:?}", new_rip, module);

                            // (*(*exception_info).ContextRecord).Rsp += 0x8;
                            (*(*exception_info).ContextRecord).Rip = new_rip;

                            info!("About to release control from second BP");
                            format_context(&*(*exception_info).ContextRecord);
                        }
                        _ => {
                            error!("HIT UNKNOWN EMULATION HOOK PHASE");
                            todo!("HIT UNKNOWN EMULATION HOOK PHASE");
                        },
                    };

                    -1
                }
            }
        },
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
    static REGUARD_TABLE: RefCell<Option<collections::HashMap<usize, usize>>> = RefCell::default();
    static BP_TABLE: RefCell<Option<collections::HashMap<usize, u8>>> = RefCell::default();

    static ORIGINAL_RSP: RefCell<u64> = RefCell::default();
    static ORIGINAL_RIP: RefCell<u64> = RefCell::default();
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

