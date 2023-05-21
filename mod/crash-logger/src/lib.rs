use std::ptr;

use log::{trace, error};
use windows::Win32::System::Kernel::ExceptionContinueSearch;
use iced_x86::{Decoder, DecoderOptions, Formatter, NasmFormatter};
use windows::Win32::System::Diagnostics::Debug::{AddVectoredExceptionHandler, EXCEPTION_POINTERS};

use broadsword::dll;
use broadsword::logging;
use broadsword::runtime;

dll::make_entrypoint!(entry);

pub fn entry(_: usize, _: u32) {
    logging::init("log/crash_logger.log");

    unsafe {
        AddVectoredExceptionHandler(0x0, Some(exception_filter));
    }
}

unsafe extern "system" fn exception_filter(exception_info: *mut EXCEPTION_POINTERS) -> i32 {
    let exception_record = *(*exception_info).ExceptionRecord;
    let exception_address = exception_record.ExceptionAddress as usize;
    let exception_module = runtime::get_module_pointer_belongs_to(exception_address);
    let exception_module_string = exception_module
        .map_or_else(|| String::from("Unknown"), |x| format_exception_module(x, exception_address));

    error!(
        "Got exception code {:#08x} at {:#08x} - {}",
        exception_record.ExceptionCode.0, exception_address, exception_module_string
    );

    let instructions = read_instructions_to_end_of_function(exception_address as *const u8, None);
    if instructions.len() != 0 {
        log_instruction_buffer(instructions, exception_address);
    }

    ExceptionContinueSearch.0
}

fn format_exception_module(module: runtime::Module, exception_address: usize) -> String {
    format!("{}+{:#08x}", module.name, exception_address - module.memory_range.start)
}

pub unsafe fn read_instructions_to_end_of_function(
    start_address: *const u8,
    max_length: Option<u64>,
) -> Vec<u8> {
    let length_guard = max_length.unwrap_or(0x50);
    let mut offset = 0;

    loop {
        let address = (start_address as u64 + offset) as *const u8;
        let current_byte = &*(address as *const u8);
        if current_byte == &0xc3 || current_byte == &0xcc {
            trace!("Found end of function after {} bytes", offset);
            // Include the RET or INT3
            offset = offset + 1;
            break;
        }

        if offset == length_guard {
            trace!("Hit length guard of {} bytes", length_guard);
            break;
        }

        offset = offset + 1;
    }

    let mut instruction_buffer = vec![];
    let length = offset as usize;
    unsafe {
        instruction_buffer.reserve(length);
        ptr::copy_nonoverlapping(
            start_address as *const u8,
            instruction_buffer.as_mut_ptr(),
            length,
        );
        instruction_buffer.set_len(length);
    }
    return instruction_buffer;
}

pub fn log_instruction_buffer(instructions: Vec<u8>, base_address: usize) {
    let mut formatter = NasmFormatter::new();
    let mut output = String::new();
    let decoder = Decoder::with_ip(64, &instructions, base_address as u64, DecoderOptions::NONE);
    for instruction in decoder {
        if instruction.is_invalid() {
            continue;
        }

        output.clear();
        formatter.format(&instruction, &mut output);
        error!("{:016X} {}", instruction.ip(), output);
    }
}

