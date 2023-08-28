use crate::pattern::Pattern;
use crate::scanner::result::ScanResultCapture;

pub mod result;
pub mod simple;
pub mod threaded;

pub trait Scanner {
    // Performs a scan over the given bytes
    fn scan(&self, bytes: &'static [u8], pattern: &Pattern) -> Option<ScanResult>;

    fn scan_all(&self, bytes: &'static [u8], pattern: &Pattern) -> Vec<ScanResult>;
}

#[derive(Debug, PartialEq)]
pub struct ScanResult {
    pub location: usize,
    pub captures: Vec<ScanResultCapture>,
}
