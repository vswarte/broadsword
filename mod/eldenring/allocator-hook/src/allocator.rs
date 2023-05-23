use std::mem;
use std::sync;
use std::alloc;
use std::collections::HashMap;
use std::collections::hash_map::Entry;

use log::*;
use paste::paste;
use broadsword::runtime;
use detour::static_detour;
use tracy::alloc::GlobalAllocator;
use windows::Win32::Foundation::EXCEPTION_GUARD_PAGE;
use windows::Win32::System::Kernel::{ExceptionContinueExecution, ExceptionContinueSearch};
use windows::Win32::System::Diagnostics::Debug::{AddVectoredExceptionHandler, EXCEPTION_POINTERS};


use crate::create_allocator_hook;

#[global_allocator]
static ALLOC: GlobalAllocator = GlobalAllocator::new();

// No I don't want to talk about the mutexes
static mut ALLOCATIONS: Option<sync::RwLock<HashMap<usize, alloc::Layout>>> = None;
static mut SIZES: Option<sync::RwLock<HashMap<usize, SizeEntry>>> = None;

create_allocator_hook!(heap, 0x142b821b0);

pub(crate) unsafe fn hook() {
    ALLOCATIONS = Some(sync::RwLock::new(HashMap::default()));
    SIZES = Some(sync::RwLock::new(HashMap::default()));

    unsafe {
        AddVectoredExceptionHandler(0x1, Some(exception_filter));
    }

    heap();
}

#[derive(Debug, Clone)]
struct SizeEntry {
    pub name: String,
    pub size: usize,
    pub warned: bool,
}

unsafe extern "system" fn exception_filter(exception_info: *mut EXCEPTION_POINTERS) -> i32 {
    let exception_record = *(*exception_info).ExceptionRecord;

    match exception_record.ExceptionCode {
        EXCEPTION_GUARD_PAGE => {
            info!("Guard page!");
            ExceptionContinueExecution.0
        },
        _ => ExceptionContinueSearch.0,
    }
}