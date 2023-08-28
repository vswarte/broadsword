use windows::core::PCWSTR;

pub(crate) fn string_to_pcwstr(input: String) -> PCWSTR {
    PCWSTR::from_raw([
        input.encode_utf16()
            .collect::<Vec<u16>>(),
            vec![0x0_u16]
    ].concat().as_ptr())
}