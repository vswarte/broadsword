use std::ops;
use std::env;
use std::fs::File;
use std::io::Read;
use std::collections::HashMap;
use std::array::TryFromSliceError;

use broadsword::rtti;
use broadsword::address;
use pelite::pe64::{Pe, PeFile};
use broadsword::static_analysis::parse_pdata;
use broadsword::scanner::{Scanner, Pattern, ThreadedScanner};
use broadsword::static_analysis::locate_base_class_descriptors;

use iced_x86::{Decoder, DecoderOptions, Formatter, Instruction, NasmFormatter, FlowControl, OpKind, Mnemonic, Register};

fn main() {
    let args = env::args().collect::<Vec<String>>();

    if args.len() != 2 {
        println!("Your invocation of this utility was incorrect. Specify an file to analyze.");
        println!("$ ./virtual-destructor-finder <exe file path>");
        return;
    }

    let path = &args[1];
    let mut file_handle = File::open(path).expect("Could not open file handle");

    let mut file_buffer = Box::leak(Box::new(Vec::new()));
    file_handle.read_to_end(&mut file_buffer).expect("Could not read file into buffer");
    let file_slice = file_buffer.as_slice();

    let pe = PeFile::from_bytes(file_slice)
        .expect("Could not parse file as PE file");

    let entries = {
        let pdata = pe.section_headers()
            .by_name(".pdata")
            .expect("Could not find pdata section");

        let pdata_range = ops::Range {
            start: pdata.file_range().start as usize,
            end: pdata.file_range().end as usize,
        };

        let pdata_buffer = &file_slice[pdata_range];

        parse_pdata(pdata_buffer)
    };

    let mut fns_with_free = HashMap::<u64, usize>::new();
    let image_base = address::Base::from(pe.optional_header().ImageBase as usize);
    for entry in entries.iter() {
        let start = &image_base + &entry.begin;
        let end = &image_base + &entry.end;

        // Skip over arxan'd stuff
        if start.as_usize() % 8 != 0 {
            continue;
        }

        let file_start = pe.rva_to_file_offset(entry.begin.as_usize() as u32);
        let file_end = pe.rva_to_file_offset(entry.end.as_usize() as u32);
        if !file_start.is_ok() || !file_end.is_ok() {
            continue;
        }

        //println!("Pointers: {:#x} .. {:#x}", start.as_usize(), end.as_usize());

        let file_slice_start = file_start.unwrap();
        let file_slice_end = file_end.unwrap();
        //println!("File: {:#x} .. {:#x}", file_slice_start, file_slice_end);

        let function_buffer = &file_slice[file_slice_start..file_slice_end];
        let mut decoder = Decoder::with_ip(64, function_buffer, start.as_usize() as u64, DecoderOptions::NONE);
        let mut formatter = NasmFormatter::new();

        let mut last_rdx: usize = 0;
        let mut output = String::new();
        while decoder.can_decode() {
            let instruction = decoder.decode();

            // Track register usage to find the size
            if instruction.mnemonic() == Mnemonic::Mov &&
                instruction.op0_register() == Register::EDX {
                last_rdx = instruction.immediate32() as usize;
            }

            if instruction.is_call_near() || instruction.is_jmp_near() {
                let target = instruction.near_branch64();

                if target == 0x1424b40b8 {
                    output.clear();
                    formatter.format(&instruction, &mut output);
                    if last_rdx != 0 {
                        fns_with_free.insert(start.as_usize() as u64, last_rdx);
                    } else {
                        //println!("Found free without size at {:#x}", instruction.ip());
                    }
                }
            }
        }
    }

    let scanner = ThreadedScanner::default();
    let rdata = pe.section_headers()
        .by_name(".rdata")
        .expect("Could not find pdata section");

    let rdata_range = ops::Range {
        start: rdata.file_range().start as usize,
        end: rdata.file_range().end as usize,
    };

    let rdata_slice = &file_slice[rdata_range];
    let rdata_virtual_start = rdata.virtual_range().start;
    for (fn_start, size) in fns_with_free.iter() {
        let pattern = Pattern::from_byte_slice(&fn_start.to_le_bytes());
        let result = scanner.scan(rdata_slice, &pattern);

        if result.is_some() {
            let ptr = image_base.value as u64 + rdata_virtual_start as u64 + result.unwrap().location.as_u64();
            println!("vftable entry at {:#x} frees {:#x} bytes of memory", ptr, size);
        }
    }
}
