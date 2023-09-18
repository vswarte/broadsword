use windows::core::PCWSTR;

pub(crate) fn string_to_pcwstr(input: impl AsRef<str>) -> PCWSTR {
    PCWSTR::from_raw([
        input.as_ref()
            .encode_utf16()
            .collect::<Vec<u16>>(),
            vec![0x0_u16]
    ].concat().as_ptr())
}