#[macro_export]
macro_rules! create_allocator_hook {
    ($name: ident, $vftable: literal) => {
        paste! {
            static_detour! { static [<$name:upper _ALLOC>]: fn(u64, u64, u64) -> u64; }
            static_detour! { static [<$name:upper _DEALLOC>]: fn(u64, u64); }
        }

        unsafe fn $name() {
            let alloc_fn_ptr = {
                let vftable_entry: u64 = $vftable + 0x50;
                (vftable_entry as *const u64)
            };

            paste!{ [<$name:upper _ALLOC>] }.initialize(
                mem::transmute(*alloc_fn_ptr),
                move |allocator: u64, size: u64, alignment: u64| {
                    let ptr = paste!{ [<$name:upper _ALLOC>] }.call(allocator, size, alignment);

                    register_allocation(ptr, size);

                    // TODO: page guard the entire reservation
                    get_thread_event_channel()
                        .send(MemoryEvent::Reserve(ReservationEvent {
                            ptr,
                            size,
                            alignment,
                        }))
                        .unwrap();

                    runtime::set_pageguard((ptr as usize).into());

                    ptr
                }
            ).unwrap();
            paste!{ [<$name:upper _ALLOC>] }.enable().unwrap();

            let dealloc_fn_ptr = {
                let vftable_entry: u64 = $vftable + 0x68;
                (vftable_entry as *const u64)
            };

            paste!{ [<$name:upper _DEALLOC>] }.initialize(
                mem::transmute(*dealloc_fn_ptr),
                move |allocator: u64, ptr: u64| {
                    remove_allocation(ptr);
                    paste!{ [<$name:upper _DEALLOC>] }.call(allocator, ptr);
                }
            ).unwrap();
            paste!{ [<$name:upper _DEALLOC>] }.enable().unwrap();
        }
    };
}
