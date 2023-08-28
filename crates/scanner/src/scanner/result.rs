use std::ops::Range;
use crate::scanner::ScanResult;

#[derive(Debug, PartialEq)]
pub struct ScanResultCapture {
    pub location: usize,
    pub bytes: Vec<u8>,
}

pub fn grab_captures(bytes: &[u8], groups: &[Range<usize>]) -> Vec<ScanResultCapture> {
    groups.iter()
        .map(|g| ScanResultCapture {
            location: g.start,
            bytes: bytes[g.start..g.end].to_vec()
        })
        .collect()
}

pub fn rebase_result(input: ScanResult, offset: usize) -> ScanResult {
    ScanResult {
        location: input.location + offset,
        captures: input.captures.into_iter()
            .map(|c| rebase_capture(c, offset))
            .collect(),
    }
}

pub fn rebase_capture(input: ScanResultCapture, offset: usize) -> ScanResultCapture {
    ScanResultCapture {
        location: input.location + offset,
        bytes: input.bytes,
    }
}