use crate::pattern::Pattern;

// Because of the shortcuts I made further down the line to prevent copying this scannable has to
// be static. This is fine for my needs as I'll be dealing with memory that isn't managed by rust.
trait Scanner {
    fn scan(&self, scannable: &'static [u8], pattern: &Pattern) -> Option<usize>;
}

trait GroupScanner {
    fn group_scan(&self, scannable: &'static [u8], patterns: Vec<Pattern>) -> Vec<Pattern>;
}

pub mod simple;
pub mod threaded;
