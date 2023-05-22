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

                    let layout = alloc::Layout::from_size_align(size, alignment).unwrap();
                    let mut table = ALLOCATIONS.as_mut().unwrap().write().unwrap();
                    table.insert(allocation, layout);

                    runtime::pageguard(allocation.into());

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
                    let deallocated_entry = {
                        let mut table = ALLOCATIONS.as_mut().unwrap().write().unwrap();
                        table.remove(&ptr)
                    };

                    match deallocated_entry {
                        Some(e) => {
                            let size = e.size();
                            match SIZES.as_mut().unwrap()
                                .write()
                                .unwrap()
                                .entry(ptr) {
                                Entry::Occupied(mut e) => {
                                    let entry = e.get();
                                    if entry.size != size && !entry.warned {
                                        warn!("Differing size for structure {}", entry.name);
                                        let mut entry = e.get().clone();
                                        entry.warned = true;
                                        e.replace_entry(entry);
                                    }
                                },
                                Entry::Vacant(e) => {
                                    if let Some(classname) = runtime::get_rtti_classname(ptr.into()) {
                                        e.insert(SizeEntry {
                                            name: classname.clone(),
                                            size: size,
                                            warned: false,
                                        });

                                        debug!("{} - size: {:x?}", classname, size);
                                    }
                                },
                            }
                        },
                        // None => warn!("Could not find an allocation table entry for {:#x}", ptr),
                        None => {},
                    };

                    paste!{ [<$name:upper _DEALLOC>] }.call(allocator, ptr);
                }
            ).unwrap();
            paste!{ [<$name:upper _DEALLOC>] }.enable().unwrap();
        }
    };
}
