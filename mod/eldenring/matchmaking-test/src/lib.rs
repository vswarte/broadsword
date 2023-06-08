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
  static BOX_DECRYPT: fn(usize, usize, usize, usize, usize, usize) -> usize;
  static BOX_ENCRYPT: fn(usize, usize, usize, usize, usize, usize) -> usize;
}

#[dll::entrypoint]
pub unsafe fn entry(_: usize) -> bool {
    logging::init("log/matchmaking_test.log");
    fs::create_dir("./matchmaking");

    SECRETBOX_DECRYPT.initialize(
        mem::transmute(0x141e24310 as usize), // Pointer assumes 1.09.1
        |output: usize, message: usize, mac: usize, size: usize, nonce: usize, key: usize| {
            debug!("secretbox_decrypt: {}", size);

            let buffer = slice::from_raw_parts(message as *const u8, size);
            dump_buffer("secretbox_recv_encrypted", buffer);

            let res = SECRETBOX_DECRYPT.call(output, message, mac, size, nonce, key);

            let buffer = slice::from_raw_parts(output as *const u8, size);
            dump_buffer("secretbox_recv_decrypted", buffer);

            res
        }
    ).unwrap();
    SECRETBOX_DECRYPT.enable().unwrap();

    SECRETBOX_ENCRYPT.initialize(
        mem::transmute(0x141e24140 as usize), // Pointer assumes 1.09.1
        |output: usize, mac: usize, message: usize, size: usize, nonce: usize, key: usize| {
            debug!("secretbox_encrypt: {}", size);

            let buffer = slice::from_raw_parts(message as *const u8, size);
            dump_buffer("secretbox_send_decrypted", buffer);

            let res = SECRETBOX_ENCRYPT.call(output, mac, message, size, nonce, key);

            let buffer = slice::from_raw_parts(output as *const u8, size);
            dump_buffer("secretbox_send_encrypted", buffer);

            res
        }
    ).unwrap();
    SECRETBOX_ENCRYPT.enable().unwrap();

    BOX_DECRYPT.initialize(
        mem::transmute(0x141e23da0 as usize), // Pointer assumes 1.09.1
        |output: usize, message: usize, mac: usize, size: usize, nonce: usize, key: usize| {
            debug!("box_decrypt: {}", size);

            let buffer = slice::from_raw_parts(message as *const u8, size);
            dump_buffer("box_recv_encrypted", buffer);

            debug!("Received message of size: {}", size);
            let res = BOX_DECRYPT.call(output, message, mac, size, nonce, key);

            let buffer = slice::from_raw_parts(output as *const u8, size);
            dump_buffer("box_recv_decrypted", buffer);

            res
        }
    ).unwrap();
    // BOX_DECRYPT.enable().unwrap();

    BOX_ENCRYPT.initialize(
        mem::transmute(0x141e23d00 as usize), // Pointer assumes 1.09.1
        |output: usize, mac: usize, message: usize, size: usize, nonce: usize, key: usize| {
            debug!("box_encrypt: {}", size);

            let buffer = slice::from_raw_parts(message as *const u8, size);
            dump_buffer("box_send_decrypted", buffer);

            let res = BOX_ENCRYPT.call(output, mac, message, size, nonce, key);

            let buffer = slice::from_raw_parts(output as *const u8, size);
            dump_buffer("box_send_encrypted", buffer);

            res
        }
    ).unwrap();
    BOX_ENCRYPT.enable().unwrap();

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
