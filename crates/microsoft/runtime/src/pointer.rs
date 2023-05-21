use std::ffi;

use log::info;
use windows::Win32::System::Memory::IsBadReadPtr;

pub fn is_valid_pointer(ptr: usize) -> bool {
    unsafe { IsBadReadPtr(Some(ptr as *const ffi::c_void), 0x8).0 as i32 == 0x0 }
}