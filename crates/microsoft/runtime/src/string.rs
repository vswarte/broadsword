use windows::core::{PCWSTR, PCSTR};

pub(crate) fn string_to_pcwstr(input: String) -> PCWSTR {
    PCWSTR::from_raw([
        input.encode_utf16().collect::<Vec<u16>>(),
        vec![0x0 as u16]
    ].concat().as_ptr())
}

pub(crate) fn string_to_pcstr(input: String) -> PCSTR {
    PCSTR::from_raw([
        input.as_bytes().to_vec(),
        vec![0x0 as u8]
    ].concat().as_ptr())
}
