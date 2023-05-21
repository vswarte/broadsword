use std::mem;
use std::ffi;
use std::slice;

use log::info;
use broadsword_address::{Base, Address};
use broadsword_rtti::type_descriptor::TypeDescriptor;
use broadsword_rtti::complete_object_locator::CompleteObjectLocator;

use crate::pointer;

/// Attempts to recover the RTTI classname of the structure at `address`.
/// It does so by resolving the vftable and resolving the pointer directly above it then following
/// the complete object locator to its type descriptor.
/// Because RTTI uses IBO (image base offsets) we need to know the base to build proper pointers.
pub fn get_classname(ptr: Address) -> Option<String> {
    let address = ptr.as_usize();
    if !pointer::is_valid_pointer(address) {
        return  None;
    }

    let vftable_ptr = match get_vftable_pointer(address) {
        Some(p) => p,
        None => { return None; }
    };

    // If we can't correlate the vftable ptr to a base we can't do validation.
    let module: Base = match crate::module::get_module_pointer_belongs_to(vftable_ptr) {
        Some(m) => m.memory_range.start.into(),
        None => { return None; }
    };

    // let module = module.unwrap();
    // info!("{:?}", module);

    let meta_ptr = vftable_ptr - mem::size_of::<usize>();
    let col_ptr = unsafe { *(meta_ptr as *const usize) };
    if !pointer::is_valid_pointer(col_ptr) {
        return  None;
    }

    let col_slice = unsafe { slice::from_raw_parts(col_ptr as *const u8, 0x100) };
    let col = CompleteObjectLocator::from_slice(col_slice);

    // Resolve type descriptor
    let type_descriptor_ptr = &module + &col.type_descriptor;
    if !pointer::is_valid_pointer(type_descriptor_ptr.as_usize()) {
        return  None;
    }

    let type_descriptor_slice = unsafe {
        slice::from_raw_parts(
            type_descriptor_ptr.as_usize() as *const u8,
            0x8000
        )
    };

    let name = TypeDescriptor::from_slice(type_descriptor_slice).name;
    if name.is_empty() {
        return None;
    }

    Some(name)
}

pub fn get_vftable_pointer(ptr: usize) -> Option<usize> {
    let result = unsafe { *(ptr as *const usize) };
    if !pointer::is_valid_pointer(result) {
        None
    } else {
        Some(result)
    }
}
