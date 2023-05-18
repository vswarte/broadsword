use crate::pattern::Pattern;

pub mod simple;
pub mod threaded;

pub trait Scanner {
    fn scan(&self, scannable: &'static [u8], pattern: &Pattern) -> Option<ScanResult>;
    fn scan_multiple(&self, scannable: &'static [u8], pattern: &Pattern) -> Vec<ScanResult>;
}

#[derive(Debug, PartialEq)]
pub struct ScanResult {
    pub location: usize,
    pub captures: Vec<Vec<u8>>,
}