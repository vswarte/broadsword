use std::ptr;

use log::{trace, error, info};
use iced_x86::{Decoder, DecoderOptions, Formatter, NasmFormatter};
use windows::Win32::System::Diagnostics::Debug::EXCEPTION_POINTERS;

use broadsword::dll;
use broadsword::logging;
use broadsword::runtime;
use broadsword::debug;

#[dll::entrypoint]
pub fn entry(_: usize) -> bool {
    logging::init("log/crash_logger.log");

    info!("Test lmao");

    debug::enable_veh_hooks();

    let observer = Box::new(CrashLoggerExceptionObserver::default());
    debug::add_exception_observer("crash_logger", observer);

    true
}

#[derive(Default)]
struct CrashLoggerExceptionObserver { }

impl debug::ExceptionObserver for CrashLoggerExceptionObserver {
    fn on_enter(&self, _: *mut EXCEPTION_POINTERS) { }

    fn on_exit(&self, exception: *mut EXCEPTION_POINTERS, result: i32) {
        // Don't log exception if any of the handlers handled the exception
        if result == -1 {
            return
        }

        let exception_record = unsafe { *(*exception).ExceptionRecord };

        let exception_address = exception_record.ExceptionAddress as usize;
        let exception_module = runtime::get_module_pointer_belongs_to(exception_address);
        let exception_module_string = exception_module
            .map_or_else(|| String::from("Unknown"), |x| format_exception_module(x, exception_address));

        error!(
            "Got exception code {:#08x} at {:#08x} - {}",
            exception_record.ExceptionCode.0, exception_address, exception_module_string
        );

        error!("EXCEPTION: {:#?}", exception_record);
    }
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
            offset += 1;
            break;
        }

        if offset == length_guard {
            trace!("Hit length guard of {} bytes", length_guard);
            break;
        }

        offset += 1;
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
    instruction_buffer
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

