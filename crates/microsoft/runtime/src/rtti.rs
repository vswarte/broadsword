use collections::HashMap;
use std::mem;
use std::sync;
use std::slice;
use std::collections;
use sync::{OnceLock, RwLock};

use broadsword_rtti::type_descriptor::TypeDescriptor;
use broadsword_rtti::complete_object_locator::CompleteObjectLocator;

use crate::pointer;

/// Attempts to recover the RTTI classname of the structure at `address`.
/// It does so by resolving the vftable and resolving the pointer directly above it then following
/// the complete object locator to its type descriptor.
/// Because RTTI uses IBO (image base offsets) we need to know the base to build proper pointers.
pub fn get_instance_classname(ptr: usize) -> Option<String> {
    let vftable_candidate = get_vftable_pointer(ptr);
    match vftable_candidate {
        None => None,
        Some(ptr) => get_classname(ptr),
    }
}

/// Gets a classname from the vftable pointer passed in. This function uses a cache to prevent
/// redudant lookups since vftable pointers are ordinarily in a static area of the exe.
pub fn get_classname(ptr: usize) -> Option<String> {
    if !pointer::is_valid_pointer(ptr) {
        return  None;
    }

    match get_cache_entry(ptr) {
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
    let module: usize = match crate::module::get_module_pointer_belongs_to(ptr) {
        Some(m) => m.memory_range.start,
        None => {
            mark_as_non_vftable(ptr);
            return None;
        }
    };

    let meta_ptr = ptr - mem::size_of::<usize>();
    let col_ptr = unsafe { *(meta_ptr as *const usize) };
    if !pointer::is_valid_pointer(col_ptr) {
        mark_as_non_vftable(ptr);
        return  None;
    }

    let col_slice = unsafe { slice::from_raw_parts(col_ptr as *const u8, 0x100) };
    let col = CompleteObjectLocator::from_bytes(col_slice);

    // Resolve type descriptor
    let type_descriptor_ptr = module + col.type_descriptor as usize;
    if !pointer::is_valid_pointer(type_descriptor_ptr) {
        mark_as_non_vftable(ptr);
        return  None;
    }

    let type_descriptor_slice = unsafe {
        slice::from_raw_parts(
            type_descriptor_ptr as *const u8,
            0x8000
        )
    };

    let name = TypeDescriptor::from_bytes(type_descriptor_slice).name;
    if name.is_empty() || !name.starts_with(".?") {
        mark_as_non_vftable(ptr);
        return None;
    }

    mark_as_vftable(ptr, &name);

    Some(name)
}

pub fn get_vftable_pointer(ptr: usize) -> Option<usize> {
    let result = unsafe { *(ptr as *const usize) };
    if !pointer::is_valid_pointer(result) {
        None
    } else {
        Some(result.into())
    }
}

static mut VFTABLE_CACHE: OnceLock<RwLock<HashMap<usize, CachedRTTILookupResult>>> = OnceLock::new();

fn setup_vftable_cache() -> RwLock<HashMap<usize, CachedRTTILookupResult>> {
    RwLock::new(HashMap::new())
}

fn get_cache_entry(ptr: usize) -> Option<CachedRTTILookupResult> {
    unsafe {
        VFTABLE_CACHE.get_or_init(|| setup_vftable_cache())
            .read()
            .unwrap()
            .get(&ptr)
            .cloned()
    }
}

fn mark_as_non_vftable(ptr: usize) {
    add_cache_entry(ptr, CachedRTTILookupResult {
        name: None,
        is_vftable: false,
    });
}

fn mark_as_vftable(ptr: usize, name: impl AsRef<str>) {
    add_cache_entry(ptr, CachedRTTILookupResult {
        name: Some(name.as_ref().to_string()),
        is_vftable: true,
    });
}

fn add_cache_entry(ptr: usize, entry: CachedRTTILookupResult) {
    unsafe {
        let _ = VFTABLE_CACHE.get_or_init(|| setup_vftable_cache());

        let mut cache = VFTABLE_CACHE.get_mut()
            .unwrap()
            .write()
            .unwrap();

        cache.insert(ptr, entry);
    }
}

/// This data structure represents a single lookup
#[derive(Clone, Debug)]
struct CachedRTTILookupResult {
    pub name: Option<String>,
    pub is_vftable: bool,
}
