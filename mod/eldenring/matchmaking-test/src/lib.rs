use std::fs;
use std::mem;
use std::ptr;
use std::slice;
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};
use detour::static_detour;
use log::debug;

use broadsword::dll;
use broadsword::logging;

static_detour! {
  static SECRETBOX_DECRYPT: fn(usize, usize, usize, usize, usize, usize) -> usize;
  static SECRETBOX_ENCRYPT: fn(usize, usize, usize, usize, usize, usize) -> usize;
}

#[dll::entrypoint]
pub unsafe fn entry(_: usize) -> bool {
    logging::init("log/matchmaking_test.log");
    fs::create_dir("./matchmaking");

    SECRETBOX_DECRYPT.initialize(
        mem::transmute(0x141e24310 as usize), // Pointer assumes 1.09.1
        |output: usize, message: usize, mac: usize, size: usize, nonce: usize, key: usize| {
            debug!("secretbox_decrypt: {}", size);

            let res = SECRETBOX_DECRYPT.call(output, message, mac, size, nonce, key);

            let buffer = slice::from_raw_parts(output as *const u8, size);
            dump_buffer("recv_decrypted", buffer);

            // 0x0
            res
        }
    ).unwrap();
    SECRETBOX_DECRYPT.enable().unwrap();

    SECRETBOX_ENCRYPT.initialize(
        mem::transmute(0x141e24140 as usize), // Pointer assumes 1.09.1
        |output: usize, mac: usize, message: usize, size: usize, nonce: usize, key: usize| {
            debug!("secretbox_encrypt: {}", size);

            let buffer = slice::from_raw_parts(message as *const u8, size);
            dump_buffer("send_decrypted", buffer);

            let res = SECRETBOX_ENCRYPT.call(output, mac, message, size, nonce, key);
            res
        }
    ).unwrap();
    SECRETBOX_ENCRYPT.enable().unwrap();

    true
}

fn dump_buffer(context: &str, buffer: &[u8]) {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis();

    let path = format!("./matchmaking/{}-{}.bin", ts, context);
    let mut f = fs::File::create(path).unwrap();
    f.write_all(buffer).unwrap();
}
