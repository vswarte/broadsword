#![feature(map_entry_replace)]
#![feature(local_key_cell_methods)]
extern crate core;

use detour::static_detour;

use tracy::tracing::TracyLayer;
use tracing_subscriber::prelude::*;

use broadsword::dll;
use broadsword::logging;

mod debug;
mod event;
mod stepper;
mod allocator;
mod allocations;
mod stepper_hook;
mod allocator_hook;

static_detour! {
  static HOOK: fn(usize, usize, usize) -> usize;
}

#[dll::entrypoint]
pub fn entry(_: usize) -> bool {
    logging::init("log/allocator_hook.log");

    tracing::subscriber::set_global_default(
        tracing_subscriber::registry().with(TracyLayer)
    ).unwrap();

    unsafe {
        // HOOK.initialize(
        //     mem::transmute(0x141e74e30 as usize), // Pointer assumes 1.09.1
        //     |size: usize, alignment: usize, allocator: usize| {
        //         // let classname = get_classname(0x140000000.into(), allocator.into());
        //         // ALLOC.alloc(Layout::from_size_align(size, alignment));
        //         HOOK.call(size, alignment, allocator)
        //     }
        // ).unwrap();

        // HOOK.enable().unwrap();

        // stepper::hook();
        allocator::hook();
    }

    true
}