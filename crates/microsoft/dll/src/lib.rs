#[macro_export]
macro_rules! make_entrypoint {
    ($fn: expr) => {
        #[no_mangle]
        pub extern "stdcall" fn DllMain(dll_base: usize, reason: u32) -> bool {
            match reason {
                1 => $fn(dll_base, reason),
                _ => {},
            }

            true
        }
    }
}

#[macro_export]
macro_rules! make_entrypoint_threaded {
    ($fn: expr) => {
        entrypoint!(thread::spawn($fn))
    }
}