use crate::pattern::Pattern;

pub mod simple;
pub mod threaded;

// Because of the shortcuts I made further down the line to prevent copying this scannable has to
// be static. This is fine for my needs as I'll be dealing with memory that isn't managed by rust.
trait Scanner {
    fn scan(&self, scannable: &'static [u8], pattern: &Pattern) -> Option<ScanResult>;
}

#[derive(Debug, PartialEq)]
pub struct ScanResult {
    pub location: usize,
    pub captures: Vec<Vec<u8>>,
}
