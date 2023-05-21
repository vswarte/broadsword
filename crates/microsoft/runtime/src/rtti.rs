use std::mem;
use std::slice;

use broadsword_address::{Base, Address};
use broadsword_rtti::complete_object_locator::CompleteObjectLocator;
use broadsword_rtti::type_descriptor::TypeDescriptor;

/// Attempts to recover the RTTI classname of the structure at `address`.
/// It does so by resolving the vftable and resolving the pointer directly above it then following
/// the complete object locator to its type descriptor.
/// Because RTTI uses IBO (image base offsets) we need to know the base to build proper pointers.
pub fn get_classname(image_base: Base, address: Address) -> Option<String> {
    // Locate the vftable metapointer
    let address = address.as_usize();
    let vftable = unsafe { *(address as *const usize) };
    let meta = vftable - mem::size_of::<usize>();

    // Get to the COL to find the type descriptor
    let col_ptr = unsafe { *(meta as *const usize) };
    let col_slice = unsafe { slice::from_raw_parts(col_ptr as *const u8, 0x100) };
    let col = CompleteObjectLocator::from_slice(col_slice);

    // Resolve type descriptor
    let type_descriptor_ptr = &image_base + &col.type_descriptor;
    let type_descriptor_slice = unsafe {
        slice::from_raw_parts(
            type_descriptor_ptr.as_usize() as *const u8,
            0x500
        )
    };
    let type_descriptor = TypeDescriptor::from_slice(type_descriptor_slice).name;

    Some(type_descriptor)
}
