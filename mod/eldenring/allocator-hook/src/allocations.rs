use std::ops;
use std::sync;
use std::collections;

static mut ALLOCATION_TABLE: Option<sync::RwLock<collections::BTreeMap<u64, u64>>> = None;

pub fn init_allocation_table() {
    unsafe {
        ALLOCATION_TABLE = Some(sync::RwLock::new(collections::BTreeMap::default()));
    }
}

pub fn register_allocation(ptr: u64, size: u64) {
    let mut table = unsafe {
        ALLOCATION_TABLE.as_mut().unwrap().write().unwrap()
    };

    table.insert(ptr, size);
}

pub fn remove_allocation(ptr: u64) {
    let mut table = unsafe {
        ALLOCATION_TABLE.as_mut().unwrap().write().unwrap()
    };

    table.remove(&ptr);
}

pub fn get_memory_page_range(ptr: u64) -> ops::Range<u64> {
    let nth_page = ptr / 4096;
    let lower = nth_page.clone() * 4096;
    let upper = (nth_page.clone() + 1) * 4096;

    ops::Range {
        start: lower,
        end: upper,
    }
}

pub fn page_contains_allocation(ptr: u64) -> bool {
    let page = get_memory_page_range(ptr);
    let table = unsafe {
        ALLOCATION_TABLE.as_ref().unwrap().read().unwrap()
    };

    table.range(page).count() != 0
}