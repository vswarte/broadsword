use std::mem;
use std::sync;
use std::slice;
use std::collections;

use broadsword_address::{Base, Address};
use broadsword_rtti::type_descriptor::TypeDescriptor;
use broadsword_rtti::complete_object_locator::CompleteObjectLocator;

use crate::pointer;

/// Attempts to recover the RTTI classname of the structure at `address`.
/// It does so by resolving the vftable and resolving the pointer directly above it then following
/// the complete object locator to its type descriptor.
/// Because RTTI uses IBO (image base offsets) we need to know the base to build proper pointers.
pub fn get_instance_classname(ptr: Address) -> Option<String> {
    let vftable_candidate = get_vftable_pointer(ptr);
    match vftable_candidate {
        None => None,
        Some(ptr) => get_classname(ptr),
    }
}

pub fn get_classname(ptr: Address) -> Option<String> {
    unsafe { init_cache() };

    if !pointer::is_valid_pointer(ptr.as_usize()) {
        return  None;
    }

    match get_cache_entry(&ptr.as_usize()) {
        Some(e) => {
            return if e.is_vftable {
                e.name.clone()
            } else {
                None
            }
        },
        None => {},
    }

    // If we can't correlate the vftable ptr to a base we can't do validation.
    let module: Base = match crate::module::get_module_pointer_belongs_to(ptr.as_usize()) {
        Some(m) => m.memory_range.start.into(),
        None => {
            mark_as_non_vftable(ptr);
            return None;
        }
    };

    let meta_ptr = ptr.as_usize() - mem::size_of::<usize>();
    let col_ptr = unsafe { *(meta_ptr as *const usize) };
    if !pointer::is_valid_pointer(col_ptr) {
        mark_as_non_vftable(ptr);
        return  None;
    }

    let col_slice = unsafe { slice::from_raw_parts(col_ptr as *const u8, 0x100) };
    let col = CompleteObjectLocator::from_slice(col_slice);

    // Resolve type descriptor
    let type_descriptor_ptr = &module + &col.type_descriptor;
    if !pointer::is_valid_pointer(type_descriptor_ptr.as_usize()) {
        mark_as_non_vftable(ptr);
        return  None;
    }

    let type_descriptor_slice = unsafe {
        slice::from_raw_parts(
            type_descriptor_ptr.as_usize() as *const u8,
            0x8000
        )
    };

    let name = TypeDescriptor::from_slice(type_descriptor_slice).name;
    if name.is_empty() || !name.starts_with(".?") {
        mark_as_non_vftable(ptr);
        return None;
    }

    mark_as_vftable(ptr, &name);

    Some(name)
}

pub fn get_vftable_pointer(ptr: Address) -> Option<Address> {
    let result = unsafe { *(ptr.as_usize() as *const usize) };
    if !pointer::is_valid_pointer(result) {
        None
    } else {
        Some(result.into())
    }
}

static mut VFTABLE_CACHE: Option<sync::RwLock<collections::HashMap<usize, CachedRTTILookupResult>>> = None;

fn get_cache_entry(ptr: &usize) -> Option<CachedRTTILookupResult> {
    unsafe {
        let cache = VFTABLE_CACHE.as_ref().unwrap()
            .read()
            .unwrap();

        match cache.get(ptr) {
            Some(e) => Some(e.clone()),
            None => None,
        }
    }
}

fn mark_as_non_vftable(ptr: Address) {
    add_cache_entry(&ptr.as_usize(), false, None);
}

fn mark_as_vftable(ptr: Address, name: &String) {
    add_cache_entry(&ptr.as_usize(), true, Some(name.clone()));
}

fn add_cache_entry(ptr: &usize, is_vftable: bool, name: Option<String>) {
    unsafe {
        let entry = CachedRTTILookupResult {
            name,
            is_vftable,
        };

        let mut cache = VFTABLE_CACHE.as_mut().unwrap()
            .write()
            .unwrap();

        cache.insert(ptr.clone(), entry);
    }
}

// Has a race when called from multiple threads at once
unsafe fn init_cache() {
    if VFTABLE_CACHE.is_none() {
        VFTABLE_CACHE = Some(sync::RwLock::new(collections::HashMap::default()));
    }
}

/// This data structure represents a single lookup
#[derive(Clone, Debug)]
struct CachedRTTILookupResult {
    pub name: Option<String>,
    pub is_vftable: bool,
}
