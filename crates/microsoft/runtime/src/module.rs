use std::ffi;
use std::mem;
use std::ops;
use std::ffi::{CString};
use windows::core::{HSTRING, PCSTR, PCWSTR};

use windows::Win32::Foundation::{HMODULE, MAX_PATH};
use windows::Win32::System::Threading::GetCurrentProcess;
use windows::Win32::System::LibraryLoader::{GetModuleHandleW, GetProcAddress};
use windows::Win32::System::SystemServices::IMAGE_DOS_HEADER;
use windows::Win32::System::Diagnostics::Debug::ImageNtHeader;
use windows::Win32::System::Diagnostics::Debug::IMAGE_NT_HEADERS64;
use windows::Win32::System::Diagnostics::Debug::IMAGE_SECTION_HEADER;
use windows::Win32::System::ProcessStatus::{EnumProcessModules, GetModuleBaseNameA};

pub enum ModuleNameLookupError {
    BaseNameNotFound,
    EncodingError,
}

/// Enumerates all the modules in the current process
pub fn get_modules() -> Vec<Module> {
    let mut bytes_used = 0_u32;
    let mut modules = [HMODULE::default(); 1024];

    let res = unsafe {
        EnumProcessModules(
            GetCurrentProcess(),
            modules.as_mut_ptr(),
            mem::size_of::<[HMODULE; 1024]>() as u32,
            &mut bytes_used as *mut u32,
        )
    };

    if !res.as_bool() {
        return vec![];
    }

    let mut result = vec![];
    for i in 0..bytes_used / mem::size_of::<HMODULE>() as u32 {
        let module_base = modules[i as usize];
        let module_name_result = get_module_name(module_base);

        if let Ok(module_name) = module_name_result {
            result.push(Module {
                name: module_name.to_string(),
                memory_range: get_module_range_by_base(module_base.0 as usize).unwrap(),
            });
        }
    }

    result
}

/// Gives you to the module that a particular pointer falls in range of.
pub fn get_module_pointer_belongs_to(pointer: usize) -> Option<Module> {
    get_modules()
        .into_iter()
        .find(|x| x.memory_range.contains(&pointer))
}

/// WARNING: this function does not perform any sanity-checking on the input.
/// Gets a module name from the current process by its module base.
fn get_module_name(module: HMODULE) -> Result<String, ModuleNameLookupError> {
    let module_name_length;
    let mut module_name = [0_u8; MAX_PATH as usize];

    unsafe {
        module_name_length = GetModuleBaseNameA(GetCurrentProcess(), module, &mut module_name);
    }

    if module_name_length == 0 {
        return Err(ModuleNameLookupError::BaseNameNotFound);
    }

    String::from_utf8(module_name[0..module_name_length as usize].to_vec())
        .map_err(|_| ModuleNameLookupError::EncodingError)
}

/// WARNING: this function does not perform any sanity-checking on the input.
/// Gives you the range that a module spans by the modules base.
fn get_module_range_by_base(base: usize) -> Option<ops::Range<usize>> {
    let image_header = unsafe { ImageNtHeader(base as *const ffi::c_void) };
    let image_size = unsafe { (*image_header).OptionalHeader.SizeOfImage };
    let end = base + image_size as usize;

    Some(ops::Range { start: base, end })
}

#[derive(Debug)]
pub enum LookupError {
    ModuleNotFound,
    SymbolNotFound,
    SectionNotFound,
}

/// Retrieves the handle of a module by its string.
pub fn get_module_handle(module: impl AsRef<str>) -> Result<usize, LookupError> {
    unsafe {
        GetModuleHandleW(crate::string::string_to_pcwstr(module))
            .map_err(|_| LookupError::ModuleNotFound)
            .map(|x| x.0 as usize)
    }
}

pub fn get_module_symbol(module: impl AsRef<str>, symbol: impl AsRef<str>) -> Result<usize, LookupError> {
    unsafe {
        let h_module = HSTRING::from(module.as_ref());
        let module_handle = GetModuleHandleW(PCWSTR::from_raw(h_module.as_ptr()))
            .map_err(|_| LookupError::ModuleNotFound)?;

        let symbol = CString::new(symbol.as_ref()).unwrap();
        GetProcAddress(module_handle, PCSTR::from_raw(symbol.as_ptr() as *const u8))
            .ok_or(LookupError::SymbolNotFound)
            .map(|x| x as usize)
    }
}

/// Retrieves the address range of a section in a module.
pub fn get_module_section_range(module: impl AsRef<str>, specified_section: impl AsRef<str>) -> Result<ops::Range<usize>, LookupError> {
    let module_base = get_module_handle(module)?;

    let image_nt_header = unsafe { ImageNtHeader(module_base as *const ffi::c_void) };
    let num_sections = unsafe { (*image_nt_header).FileHeader.NumberOfSections as u32 };
    let number_of_rva_and_sizes =
        unsafe { (*image_nt_header).OptionalHeader.NumberOfRvaAndSizes };

    // The sections should be right after the Image NT header.
    // That means we'll have to parse the DOS header to figure out how when the optional header ends.
    let dos_header = module_base as *const IMAGE_DOS_HEADER;
    let nt_header_base = module_base + unsafe { (*dos_header).e_lfanew as usize };

    // The IMAGE_NT_HEADERS64 structure assumes 16 data directory entries. This is not a given so we subtract the difference.
    let section_base = nt_header_base
        + mem::size_of::<IMAGE_NT_HEADERS64>()
        + ((number_of_rva_and_sizes - 16) * 8) as usize;

    let specified_section = specified_section.as_ref();
    unsafe {
        let mut current_section_header = section_base;
        let section_header_size = mem::size_of::<IMAGE_SECTION_HEADER>();

        for _ in 0..num_sections {
            let section_header = current_section_header as *const IMAGE_SECTION_HEADER;

            let section_name = PCSTR::from_raw((*section_header).Name.as_ptr())
                .to_string()
                .expect("Could not get name from section");

            if section_name == specified_section {
                let section_size = (*section_header).SizeOfRawData;
                let section_va = (*section_header).VirtualAddress;

                let start = module_base + section_va as usize;
                let end = module_base + section_va as usize + section_size as usize;
                return Ok(ops::Range { start, end });
            }

            current_section_header += section_header_size;
        }
    }

    Err(LookupError::SectionNotFound)
}

#[derive(Debug)]
pub struct Module {
    pub name: String,
    pub memory_range: ops::Range<usize>,
}