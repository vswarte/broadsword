use std::sync;
use std::collections;
use windows::Win32::System::Diagnostics::Debug::EXCEPTION_POINTERS;

pub trait ExceptionObserver: Sync + Send {
    fn on_enter(&self, exception: *mut EXCEPTION_POINTERS);
    fn on_exit(&self, exception: *mut EXCEPTION_POINTERS, result: i32);
}

static EXCEPTION_OBSERVER_LIST: sync::OnceLock<sync::RwLock<collections::HashMap<String, Box<dyn ExceptionObserver>>>> = sync::OnceLock::new();


pub(crate) unsafe fn get_exception_observer_list() -> &'static sync::RwLock<collections::HashMap<String, Box<dyn ExceptionObserver>>> {
    EXCEPTION_OBSERVER_LIST.get_or_init(|| sync::RwLock::new(collections::HashMap::new()))
}

pub fn add_exception_observer(key: impl AsRef<str>, processor: Box<dyn ExceptionObserver>) {
    let mut preprocessors = unsafe { get_exception_observer_list() }
        .write()
        .unwrap();

    log::debug!("Adding exception observer: {}", key.as_ref());

    preprocessors.insert(
        key.as_ref().to_string(),
        processor
    );
}

pub fn remove_exception_observer(key: impl AsRef<str>) {
    let mut preprocessors = unsafe { get_exception_observer_list() }
        .write()
        .unwrap();

    log::debug!("Removing exception observer: {}", key.as_ref());

    preprocessors.remove(key.as_ref());
}
