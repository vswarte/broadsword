use broadsword_address::Offset;

use crate::pattern::Pattern;

pub mod simple;
pub mod threaded;

pub trait Scanner {
    fn scan(&self, scannable: &'static [u8], pattern: &Pattern) -> Option<ScanResult>;
}

#[derive(Debug, PartialEq)]
pub struct ScanResult {
    pub location: Offset,
    pub captures: Vec<ScanResultCapture>,
}

#[derive(Debug, PartialEq)]
pub struct ScanResultCapture {
    pub location: Offset,
    pub bytes: Vec<u8>,
}
