#[macro_export]
macro_rules! create_allocator_hook {
    ($name: ident, $vftable: literal) => {
        paste! {
            static_detour! { static [<$name:upper _ALLOC>]: fn(usize, usize, usize) -> usize; }
            static_detour! { static [<$name:upper _DEALLOC>]: fn(usize, usize); }
        }

        paste! { static mut [<$name:upper _ALLOCATIONS>] : Option<HashMap<usize, alloc::Layout>> = None; }

        unsafe fn $name() {
            let alloc_fn_ptr = {
                let vftable_entry: usize = $vftable + 0x50;
                (vftable_entry as *const usize)
            };

            paste! { [<$name:upper _ALLOCATIONS>] = Some(HashMap::default()) };

            paste!{ [<$name:upper _ALLOC>] }.initialize(
                mem::transmute(*alloc_fn_ptr),
                move |allocator: usize, size: usize, alignment: usize| {
                    let table = paste! { [<$name:upper _ALLOCATIONS>] }.as_mut().unwrap();
                    let allocation = paste!{ [<$name:upper _ALLOC>] }.call(allocator, size, alignment);

                    let layout = alloc::Layout::from_size_align(size, alignment).unwrap();
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
                    let table = paste! { [<$name:upper _ALLOCATIONS>] }.as_mut().unwrap();

                    let deallocated_entry = table.remove(&ptr);
                    match deallocated_entry {
                        Some(e) => {
                            debug!("Attempting to get classname");
                            if let Some(classname) = runtime::get_rtti_classname(0x140000000.into(), ptr.into()) {
                                debug!("{} - size: {}", classname, e.size());
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
