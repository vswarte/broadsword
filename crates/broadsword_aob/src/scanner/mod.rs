use crate::pattern::Pattern;

trait Scanner {
    fn scan(&self, scannable: &[u8], pattern: &Pattern) -> Option<usize>;
}

pub mod simple;
pub mod threaded;
