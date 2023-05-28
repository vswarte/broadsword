#[macro_export]
macro_rules! create_allocator_hook {
    ($name: ident, $vftable: literal) => {
        paste! {
            static_detour! { static [<$name:upper _ALLOC>]: fn(usize, usize, usize) -> usize; }
            static_detour! { static [<$name:upper _DEALLOC>]: fn(usize, usize); }
        }

        unsafe fn $name() {
            let alloc_fn_ptr = {
                let vftable_entry: usize = $vftable + 0x50;
                (vftable_entry as *const usize)
            };

            paste!{ [<$name:upper _ALLOC>] }.initialize(
                mem::transmute(*alloc_fn_ptr),
                move |allocator: usize, size: usize, alignment: usize| {
                    let allocation = paste!{ [<$name:upper _ALLOC>] }.call(allocator, size, alignment);

                    let table_entry = AllocationTableEntry {
                        name: None,
                        size,
                        alignment,
                        range: ops::Range {
                            start: allocation,
                            end: allocation + size,
                        }
                    };

                    // info!("ADD: {:#?}", table_entry);
                    {
                        let mut table = ALLOCATION_TABLE.as_mut().unwrap().write().unwrap();
                        table.insert(allocation, table_entry);
                    }

                    runtime::set_pageguard(allocation.into());

                    allocation
                }
            ).unwrap();
            paste!{ [<$name:upper _ALLOC>] }.enable().unwrap();

            let dealloc_fn_ptr = {
                let vftable_entry: usize = $vftable + 0x68;
                (vftable_entry as *const usize)
            };

            paste!{ [<$name:upper _DEALLOC>] }.initialize(
                mem::transmute(*dealloc_fn_ptr),
                move |allocator: usize, ptr: usize| {
                    paste!{ [<$name:upper _DEALLOC>] }.call(allocator, ptr);

                    let deallocated_entry = {
                        let mut table = ALLOCATION_TABLE.as_mut().unwrap().write().unwrap();
                        table.remove(&ptr)
                    };

                    // match deallocated_entry {
                    //     Some(e) => debug!("Removed allocation table entry {:?}", ptr),
                    //     None => debug!("Could not find an allocation table entry for {:#x}", ptr),
                    // };
                }
            ).unwrap();
            paste!{ [<$name:upper _DEALLOC>] }.enable().unwrap();
        }
    };
}
