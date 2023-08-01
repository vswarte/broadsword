use std::fmt;

use broadsword_address::Offset;


/// Parses pdata section and yields a map of all found function offsets, the functions end
/// and unwinds.
pub fn parse_pdata(buffer: &[u8]) -> Vec<PDataEntry> {
    (0..buffer.len() - 0xC)
        .step_by(0xC)
        .map(|s| {
            PDataEntry::from_slice(&buffer[s..s+0xC])
        })
        .collect()
}

pub struct PDataEntry {
    pub begin: Offset,
    pub end: Offset,
    pub unwind_info: Offset,
}

impl PDataEntry {
    pub fn from_slice(input: &[u8]) -> Self {
        let begin = Offset::from(u32::from_le_bytes(input[0..4].try_into().unwrap()));
        let end = Offset::from(u32::from_le_bytes(input[4..8].try_into().unwrap()));
        let unwind_info = Offset::from(u32::from_le_bytes(input[8..12].try_into().unwrap()));

        Self {
            begin,
            end,
            unwind_info,
        }
    }
}

impl fmt::Debug for PDataEntry {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("PDataEntry")
            .field("begin", &format_args!("{:?}", self.begin))
            .field("end", &format_args!("{:?}", self.end))
            .field("unwind_info", &format_args!("{:?}", self.unwind_info))
            .finish()
    }
}
