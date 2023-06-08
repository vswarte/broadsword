use std::fs;
use std::env;
use std::time;
use std::path;
use std::thread;
use clap::Parser;
use pelite::PeFile;
use rayon::prelude::*;
use broadsword::address;
use broadsword::scanner;
use std::io::{Write, Read};
use pelite::Wrap::{T32, T64};
use broadsword::scanner::Scanner;
use iced_x86::{Decoder, DecoderOptions, OpKind};
use indicatif::{ProgressStyle, ParallelProgressIterator};

use crate::input::{SignatureEntryMethod, parse_profile_file};

mod input;

#[derive(Parser, Debug)]
#[command(version, long_about = None)]
struct Args {
    /// Profile file that contains the signatures to look for
    #[arg(short, long)]
    profile: path::PathBuf,

    /// Input executable to match with.
    #[arg(short, long)]
    executable: path::PathBuf,

    /// Output header file. if not specified it'll write matches to stdout instead of generating a
    /// header file.
    #[arg(short, long)]
    outputHeader: Option<path::PathBuf>,
}

fn main() {
    let args = Args::parse();

    let profile = {
        let profile_file = fs::File::open(args.profile)
            .expect("Could not get file handle for config file");

        parse_profile_file(profile_file).expect("Could not parse config file")
    };

    // Leak file into mem for sync memes
    let mut executable = fs::File::open(args.executable)
        .expect("Could not get file handle for executable");

    let mut executable_bytes = Vec::new();
    executable.read_to_end(&mut executable_bytes)
        .expect("Could not read executable into buffer");

    let pe = PeFile::from_bytes(executable_bytes.as_slice())
        .expect("Could not parse file as PE file");

    let image_base_correction = match pe.optional_header() {
        T32(h) => h.BaseOfCode - h.SizeOfHeaders,
        T64(h) => h.BaseOfCode - h.SizeOfHeaders,
    };

    let leaked_executable = Box::leak(Box::new(executable_bytes));
    let search_bytes = leaked_executable.as_slice();

    let style = ProgressStyle::default_bar();
    let results = profile.entries.par_iter().progress_with_style(style)
        .map(|e| {
            let pattern = scanner::Pattern::from_pattern_str(e.signature.as_str()).unwrap();
            let result = scanner::SimpleScanner::default().scan(search_bytes, &pattern)
                .map(|x| get_result_for_method(
                    image_base_correction as u64,
                    &x,
                    &e.method,
                ));

            (e.key.clone(), result)
        })
        .collect::<Vec<(String, Option<u64>)>>();

    if let Some(output_path) = args.outputHeader {
        let mut output_file = fs::File::create(output_path)
            .expect("Could not open file handle for output header file");

        for (key, result) in results.into_iter() {
            match result {
                Some(ibo) => {
                    let line = format!("#define IBO_{} {:#x}\n", key, ibo);
                    output_file.write_all(line.as_bytes())
                        .expect("Could add define line to output header file");
                },
                None => println!("Could not match signature {}", key),
            }
        }
    } else {
        for (key, result) in results.into_iter() {
            match result {
                Some(ibo) => println!("Found {} at {:#x}", key, ibo),
                None => println!("Could not match signature {}", key),
            }
        }
    }
}

fn get_result_for_method(
    base_offset: u64,
    scan_result: &scanner::ScanResult,
    method: &SignatureEntryMethod,
) -> u64 {
    match method {
        SignatureEntryMethod::Offset => base_offset + scan_result.location.as_u64(),
        SignatureEntryMethod::CaptureTarget => {
            let capture = scan_result.captures.first()
                .expect("Tried getting capture for capture target method signature but had none");

            let mut decoder = Decoder::with_ip(
                64,
                capture.bytes.as_slice(),
                capture.location.as_u64() + base_offset,
                DecoderOptions::NONE
            );

            let instruction = decoder.decode();
            if instruction.is_ip_rel_memory_operand() {
                return instruction.ip_rel_memory_address();
            }

            // TODO: do we have any situations where this'll be any other register?
            match instruction.op0_kind() {
                OpKind::NearBranch64 => {
                    return instruction.near_branch_target();
                },
                _ =>  todo!("Could not acquire memory target from instruction {:?}", instruction)
            }
        },
    }
}
