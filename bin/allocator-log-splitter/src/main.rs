use std::fs;
use std::env;
use std::ops;
use std::io::{Read, Write};
use indicatif::ProgressBar;
use bincode;
use serde::{Serialize, Deserialize};

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
                sessions.push(Session {
                    index: sessions.len() + 1,
                    range: ops::Range {
                        start: e.ptr,
                        end: e.ptr + e.size,
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
    fs::create_dir("./output").unwrap();
    let bar = ProgressBar::new(sessions.len() as u64);
    for session in sessions.iter() {
        let bytes = bincode::serialize(&session).unwrap();
        let mut file = fs::File::create(format!("./output/{}.accum", session.index)).unwrap();
        file.write_all(&bytes).unwrap();
        bar.inc(1);
    }
    bar.finish();
}

#[derive(Debug, Serialize)]
struct Session {
    pub index: usize,
    pub range: ops::Range<usize>,
    pub events: Vec<AccessEvent>,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum MemoryEvent {
    Reserve(ReservationEvent),
    Access(AccessEvent),
}

#[derive(Debug, Deserialize, Serialize)]
pub struct ReservationEvent {
    pub ptr: usize,
    pub size: usize,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct AccessEvent {
    pub instruction_address: u64,
    // pub instruction: Instruction,
    pub access_address: usize,
    pub data_before: Vec<u8>,
    pub data_after: Vec<u8>,
    pub is_write: bool,
    // pub context: CONTEXT,
}
