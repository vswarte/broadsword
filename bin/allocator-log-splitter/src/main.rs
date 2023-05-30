use bincode;
use std::fs;
use std::env;
use std::ops;
use std::io::{Read, Write};
use indicatif::ProgressBar;
use serde::{Serialize, Deserialize};
use broadsword_memorylog::{MemoryEvent, Session};

fn main() {
    let args = env::args().collect::<Vec<String>>();

    if args.len() != 2 {
        println!("Your invocation of this utility was incorrect.");
        println!("$ ./allocator-log-splitter <exe file path>");
        return;
    }

    let path = &args[1];
    let mut file_handle = fs::File::open(path).expect("Could not open file handle");
    let mut sessions = vec![];

    let bar = ProgressBar::new(file_handle.metadata().unwrap().len());
    let mut size_buffer = [0u8; 8];
    while file_handle.read_exact(&mut size_buffer).is_ok() {
        let size = usize::from_le_bytes(size_buffer);
        let mut buffer = vec![0u8; size];
        file_handle.read_exact(&mut buffer).unwrap();

        let decoded: MemoryEvent = bincode::deserialize(&buffer[..]).unwrap();

        match decoded {
            MemoryEvent::Reserve(e) => {
                let padded_size = ((e.size / e.alignment) + 1) * e.alignment;
                sessions.push(Session {
                    index: sessions.len() as u64 + 1,
                    range: ops::Range {
                        start: e.ptr,
                        end: e.ptr + padded_size,
                    },
                    events: vec![],
                });
            },
            MemoryEvent::Access(e) => {
                match sessions.iter_mut().position(|s| s.range.contains(&e.access_address)) {
                    Some(s) => {
                        sessions[s].events.push(e)
                    },
                    None => {
                        // println!("Warning: Could not find session");
                    }
                }
            },
        };

        bar.inc((8 + size) as u64);
    }
    bar.finish();

    // Write the accumulated output
    let _ = fs::create_dir("./output");
    let bar = ProgressBar::new(sessions.len() as u64);
    for session in sessions.iter() {
        let bytes = bincode::serialize(&session).unwrap();
        let mut file = fs::File::create(format!("./output/{}.accum", session.index)).unwrap();
        file.write_all(&bytes).unwrap();
        bar.inc(1);
    }
    bar.finish();
}