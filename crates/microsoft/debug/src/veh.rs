use std::mem;
use std::ffi;
use std::sync;

use detour::static_detour;
use windows::Win32::System::Diagnostics::Debug::{
    AddVectoredExceptionHandler,
    PVECTORED_EXCEPTION_HANDLER,
    EXCEPTION_POINTERS,
};

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
        AddVectoredExceptionHandler(0x1, Some(exception_handler));

        ADD_VECTORED_EXCEPTION_HANDLER_HOOK.initialize(
            mem::transmute(add_vectored_exception_handler),
            |first: u32, handler: PVECTORED_EXCEPTION_HANDLER| add_vectored_exception_handler_detour(first, handler)
        ).unwrap();
        ADD_VECTORED_EXCEPTION_HANDLER_HOOK.enable().unwrap();

        REMOVE_VECTORED_EXCEPTION_HANDLER_HOOK.initialize(
            mem::transmute(remove_vectored_exception_handler),
            |handle: *const ffi::c_void| remove_vectored_exception_handler_detour(handle)
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

unsafe extern "system" fn exception_handler(exception_info: *mut EXCEPTION_POINTERS) -> i32 {
    let handlers = get_veh_handlers()
        .read()
        .unwrap();

    for entry in handlers.iter() {
        if let Some(handler) = entry.handler {
            let result = handler(exception_info);
            log::info!("Called {:#x} and received {:#x}", entry.handle, result);

            if result == -1 {
                return -1;
            }
        }
    }

    0
}

unsafe extern "system" fn remove_vectored_exception_handler_detour(handle: *const ffi::c_void) -> u32 {
    let handle = handle as usize;

    log::info!("Removing VE handler: {:#x}", handle as usize);

    let mut handlers = get_veh_handlers()
        .write()
        .unwrap();

    match handlers.iter().position(|e| e.handle == handle) {
        Some(position) => {
            handlers.remove(position);
            0x1
        },

        // Call the original to prevent messing with stuff that was registered before we hooked
        None => REMOVE_VECTORED_EXCEPTION_HANDLER_HOOK.call(handle as *const ffi::c_void),
    }
}

unsafe extern "system" fn add_vectored_exception_handler_detour(
    first: u32,
    handler: PVECTORED_EXCEPTION_HANDLER
) -> *mut ffi::c_void {
    let handle = HANDLE_COUNTER.fetch_add(1, sync::atomic::Ordering::Relaxed);

    if let Some(fn_ptr) = handler {
        log::info!("Adding VE handler: {:#x} -> {:#x}", fn_ptr as usize, handle as usize);
    }

    let entry = VEHChainEntry {
        handle,
        handler,
    };

    let mut handlers = get_veh_handlers()
        .write()
        .unwrap();

    if first == 0x0 {
        handlers.push(entry);
    } else {
        handlers.insert(0, entry);
    }

    handle as *mut ffi::c_void
}

static HANDLE_COUNTER: sync::atomic::AtomicUsize = sync::atomic::AtomicUsize::new(1);
static VEH_LIST: sync::OnceLock<sync::RwLock<Vec<VEHChainEntry>>> = sync::OnceLock::new();

unsafe fn get_veh_handlers() -> &'static sync::RwLock<Vec<VEHChainEntry>> {
    VEH_LIST.get_or_init(|| sync::RwLock::new(Vec::new()))
}

struct VEHChainEntry {
    pub handle: usize,
    pub handler: PVECTORED_EXCEPTION_HANDLER,
}