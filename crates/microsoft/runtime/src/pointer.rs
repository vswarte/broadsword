use std::ffi;

use windows::Win32::System::Memory::IsBadReadPtr;

pub fn is_valid_pointer(ptr: usize) -> bool {
    unsafe { IsBadReadPtr(Some(ptr as *const ffi::c_void), 0x8).0 == 0x0 }
}