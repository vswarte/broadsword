use std::mem;
use std::ffi;

use broadsword_address::Address;
use windows::Win32::System::Memory::{MEMORY_BASIC_INFORMATION, PAGE_GUARD, PAGE_PROTECTION_FLAGS, VirtualProtect, VirtualQuery};

pub fn set_pageguard(address: Address) -> bool {
    // TODO: get page guard instead of assuming it
    let address = address.as_usize() as *const ffi::c_void;
    let mut info = MEMORY_BASIC_INFORMATION::default();

    unsafe {
        if VirtualQuery(
            Some(address),
            &mut info,
            mem::size_of::<MEMORY_BASIC_INFORMATION>(),
        ) == 0x0 {
            return false;
        }

        let new_protection = info.Protect | PAGE_GUARD;
        let mut old_protection = PAGE_PROTECTION_FLAGS::default();
        VirtualProtect(
            address,
            0x8,
            new_protection,
            &mut old_protection
        );
    }

    true
}

pub fn remove_pageguard(address: Address) -> bool {
    // TODO: get page guard instead of assuming it
    let address = address.as_usize() as *const ffi::c_void;
    let mut info = MEMORY_BASIC_INFORMATION::default();

    unsafe {
        if VirtualQuery(
            Some(address),
            &mut info,
            mem::size_of::<MEMORY_BASIC_INFORMATION>(),
        ) == 0x0 {
            return false;
        }

        let new_protection = info.Protect & PAGE_GUARD;
        let mut old_protection = PAGE_PROTECTION_FLAGS::default();
        VirtualProtect(
            address,
            0x8,
            new_protection,
            &mut old_protection
        );
    }

    true
}
