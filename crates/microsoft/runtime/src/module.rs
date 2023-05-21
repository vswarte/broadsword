use std::ffi;
use std::mem;
use std::ops;

use windows::Win32::System::Threading::GetCurrentProcess;
use windows::Win32::Foundation::{HMODULE, HANDLE, MAX_PATH};
use windows::Win32::System::Diagnostics::Debug::ImageNtHeader;
use windows::Win32::System::ProcessStatus::{EnumProcessModules, GetModuleBaseNameA};

pub enum ModuleNameLookupError {
    BaseNameNotFound,
    EncodingError,
}

pub fn get_modules() -> Vec<Module> {
    let mut bytes_used = 0 as u32;
    let mut modules = [HMODULE::default(); 1024];
    let process = unsafe { GetCurrentProcess() };

    let res = unsafe {
        EnumProcessModules(
            process,
            modules.as_mut_ptr(),
            mem::size_of::<[HMODULE; 1024]>() as u32,
            &mut bytes_used as *mut u32,
        )
    };

    if res.as_bool() == false {
        return vec![];
    }

    let mut result = vec![];
    for i in 0..bytes_used / mem::size_of::<HMODULE>() as u32 {
        let module_base = modules[i as usize];
        let module_name_result = get_module_name(process, module_base);

        if let Some(module_name) = module_name_result.ok() {
            result.push(Module {
                name: module_name.to_string(),
                memory_range: get_module_range_by_base(module_base.0 as usize).unwrap(),
            });
        }
    }

    result
}

pub fn get_module_pointer_belongs_to(pointer: usize) -> Option<Module> {
    get_modules()
        .into_iter()
        .find(|x| x.memory_range.contains(&pointer))
}

fn get_module_name(process: HANDLE, module: HMODULE) -> Result<String, ModuleNameLookupError> {
    let module_name_length;
    let mut module_name = [0 as u8; MAX_PATH as usize];

    unsafe {
        module_name_length = GetModuleBaseNameA(process, module, &mut module_name);
    }

    if module_name_length == 0 {
        return Err(ModuleNameLookupError::BaseNameNotFound);
    }

    String::from_utf8(module_name[0..module_name_length as usize].to_vec())
        .map_err(|_| ModuleNameLookupError::EncodingError)
}

fn get_module_range_by_base(base: usize) -> Option<ops::Range<usize>> {
    let image_header = unsafe { ImageNtHeader(base as *const ffi::c_void) };
    let image_size = unsafe { (*image_header).OptionalHeader.SizeOfImage as u32 };
    let end = base + image_size as usize;

    return Some(ops::Range { start: base, end });
}

#[derive(Debug)]
pub struct Module {
    pub name: String,
    pub memory_range: ops::Range<usize>,
}
