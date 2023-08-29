use crate::pattern::Pattern;
use crate::scanner::result::ScanResultCapture;

pub mod result;
pub mod simple;
pub mod threaded;

#[derive(Debug, PartialEq)]
pub struct ScanResult {
    pub location: usize,
    pub captures: Vec<ScanResultCapture>,
}
