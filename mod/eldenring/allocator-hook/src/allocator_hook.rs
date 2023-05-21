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
                    let mut table = ALLOCATIONS.as_mut().unwrap().lock().unwrap();
                    table.insert(allocation, layout);

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
                    let mut table = ALLOCATIONS.as_mut().unwrap().lock().unwrap();

                    let deallocated_entry = table.remove(&ptr);
                    match deallocated_entry {
                        Some(e) => {
                            if let Some(classname) = runtime::get_rtti_classname(ptr.into()) {
                                let size = e.size();
                                match SIZES.as_mut().unwrap()
                                    .lock().unwrap()
                                    .entry(classname.clone()) {
                                    Entry::Occupied(e) => {
                                        if e.get() != &size {
                                            warn!("Got differing size for structure {}", classname);
                                        }
                                    },
                                    Entry::Vacant(e) => {
                                        e.insert(size);
                                        debug!("{} - size: {}", classname, size);
                                    },
                                }
                            }
                        },
                        None => debug!("Could not find an allocation table entry for {:#x}", ptr),
                    };

                    paste!{ [<$name:upper _DEALLOC>] }.call(allocator, ptr);
                }
            ).unwrap();
            paste!{ [<$name:upper _DEALLOC>] }.enable().unwrap();
        }
    };
}
