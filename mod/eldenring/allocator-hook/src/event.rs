use std::fs;
use std::thread;
use std::sync::mpsc;
use std::cell::RefCell;
use std::io::Write;
use broadsword_memorylog::MemoryEvent;

pub fn init_event_thread() {
    if unsafe { EVENT_CHANNEL_TX.is_some() } {
        panic!("Event thread already running");
    }

    let (tx, rx): (mpsc::Sender<MemoryEvent>, mpsc::Receiver<MemoryEvent>) = mpsc::channel();
    unsafe {
        EVENT_CHANNEL_TX = Some(tx);
    }

    // TODO: clean up thread after done
    thread::spawn(move || {
        let mut f = fs::File::create("log.allocatorlog").unwrap();

        for event in rx {
            let encoded: Vec<u8> = bincode::serialize(&event).unwrap();
            let size = encoded.len();

            f.write(&size.to_le_bytes()).unwrap();
            f.write(encoded.as_slice()).unwrap();
        }
    });
}

static mut EVENT_CHANNEL_TX: Option<mpsc::Sender<MemoryEvent>> = None;

thread_local! {
    static THREAD_EVENT_CHANNEL_TX: RefCell<Option<mpsc::Sender<MemoryEvent>>> = RefCell::default();
}

pub fn init_for_thread() {
    if !has_thread_event_channel() {
        unsafe {
            set_thread_event_channel(EVENT_CHANNEL_TX.as_ref().unwrap().clone());
        }
    }
}

fn has_thread_event_channel() -> bool {
    THREAD_EVENT_CHANNEL_TX.with_borrow(|t| {
        t.as_ref().is_some()
    })
}

fn set_thread_event_channel(v: mpsc::Sender<MemoryEvent>) {
    THREAD_EVENT_CHANNEL_TX.with_borrow_mut(|t| {
        *t = Some(v);
    });
}

pub fn get_thread_event_channel() -> mpsc::Sender<MemoryEvent> {
    THREAD_EVENT_CHANNEL_TX.with_borrow(|t| {
        t.as_ref().unwrap().clone()
    })
}
