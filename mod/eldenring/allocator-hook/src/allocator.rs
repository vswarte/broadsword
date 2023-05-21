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

use crate::create_allocator_hook;

#[global_allocator]
static ALLOC: GlobalAllocator = GlobalAllocator::new();

// No I don't want to talk about the mutexes
static mut ALLOCATIONS: Option<sync::Mutex<HashMap<usize, alloc::Layout>>> = None;
static mut SIZES: Option<sync::Mutex<HashMap<String, usize>>> = None;

create_allocator_hook!(heap, 0x142b821b0);

pub(crate) unsafe fn hook() {
    ALLOCATIONS = Some(sync::Mutex::new(HashMap::default()));
    SIZES = Some(sync::Mutex::new(HashMap::default()));

    heap();
}
