use std::mem;
use std::sync;
use std::alloc;
use std::collections;
use std::collections::hash_map::Entry;

use log::*;
use paste::paste;
use broadsword::runtime;
use detour::static_detour;
use tracy::alloc::GlobalAllocator;
use windows::Win32::Foundation::EXCEPTION_GUARD_PAGE;
use windows::Win32::System::Diagnostics::Debug::{AddVectoredExceptionHandler, EXCEPTION_POINTERS};


use crate::create_allocator_hook;

// No I don't want to talk about the mutexes
static mut ALLOCATIONS: Option<sync::RwLock<collections::HashMap<usize, alloc::Layout>>> = None;
static mut SIZES: Option<sync::RwLock<collections::HashMap<usize, SizeEntry>>> = None;

create_allocator_hook!(heap, 0x142b821b0);

pub(crate) unsafe fn hook() {
    ALLOCATIONS = Some(sync::RwLock::new(collections::HashMap::default()));
    SIZES = Some(sync::RwLock::new(collections::HashMap::default()));

    unsafe {
        // Place it first in the list so PAGE_GUARDs doesn't clutter more complex filters
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
            info!("VEH RIP: {:#x}", (*(*exception_info).ContextRecord).Rip);

            return -1;
        },
        _ => 0,
    }
}