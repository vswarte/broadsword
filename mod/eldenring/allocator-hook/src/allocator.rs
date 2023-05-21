use std::mem;
use std::alloc;
use std::alloc::GlobalAlloc;
use std::collections::HashMap;

use log::*;
use paste::paste;
use repeated::repeated;
use broadsword::runtime;
use detour::static_detour;
use tracing::{span, Level};
use tracy::alloc::GlobalAllocator;

use crate::create_allocator_hook;

#[global_allocator]
static ALLOC: GlobalAllocator = GlobalAllocator::new();

create_allocator_hook!(heap, 0x142b821b0);

pub(crate) unsafe fn hook() {
    heap();
}
