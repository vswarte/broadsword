use std::mem;
use std::ffi;
use log::info;

use broadsword_address::Address;
use windows::Win32::System::Memory::{MEMORY_BASIC_INFORMATION, VirtualQuery};

pub fn pageguard(address: Address) {
    // TODO: get page guard instead of assuming it
    let mut info = MEMORY_BASIC_INFORMATION::default();

    unsafe {
        let test = VirtualQuery(
            Some(address.as_usize() as *const ffi::c_void),
            &mut info,
            mem::size_of::<MEMORY_BASIC_INFORMATION>(),
        );
    }

    info!("{:?}", info);
}
