use std::mem;
use std::ffi;

use detour::static_detour;
use windows::Win32::System::Diagnostics::Debug::PVECTORED_EXCEPTION_HANDLER;

use broadsword_microsoft_runtime::module;

static_detour! {
    static ADD_VECTORED_EXCEPTION_HANDLER_HOOK: unsafe extern "system" fn(u32, PVECTORED_EXCEPTION_HANDLER) -> *mut ffi::c_void;
    static REMOVE_VECTORED_EXCEPTION_HANDLER_HOOK: unsafe extern "system" fn(*const ffi::c_void) -> u32;
}

pub fn enable_veh_hooks() {
    let add_vectored_exception_handler = module::get_module_symbol("kernel32", "AddVectoredExceptionHandler")
        .expect("Could not locate AddVectoredExceptionHandler from IAT");

    let remove_vectored_exception_handler = module::get_module_symbol("kernel32", "RemoveVectoredExceptionHandler")
        .expect("Could not locate RemoveVectoredExceptionHandler from IAT");

    unsafe {
        ADD_VECTORED_EXCEPTION_HANDLER_HOOK.initialize(
            mem::transmute(add_vectored_exception_handler),
            |first: u32, handler: PVECTORED_EXCEPTION_HANDLER| {
                let handle = ADD_VECTORED_EXCEPTION_HANDLER_HOOK.call(first, handler);

                if let Some(fn_ptr) = handler {
                    log::info!("Added VE handler: {:#x} -> {:#x}", fn_ptr as usize, handle as usize);
                }

                handle
            }
        ).unwrap();
        ADD_VECTORED_EXCEPTION_HANDLER_HOOK.enable().unwrap();

        REMOVE_VECTORED_EXCEPTION_HANDLER_HOOK.initialize(
            mem::transmute(add_vectored_exception_handler),
            |handle: *const ffi::c_void| {
                let success = REMOVE_VECTORED_EXCEPTION_HANDLER_HOOK.call(handle);

                log::info!("Removed VE handler: {:#x} -> {:#x}", handle as usize, success);

                success
            }
        ).unwrap();
        REMOVE_VECTORED_EXCEPTION_HANDLER_HOOK.enable().unwrap();
    }
}

pub fn disable_veh_hooks() {
    unsafe {
        ADD_VECTORED_EXCEPTION_HANDLER_HOOK.disable().unwrap();
        REMOVE_VECTORED_EXCEPTION_HANDLER_HOOK.disable().unwrap();
    }
}