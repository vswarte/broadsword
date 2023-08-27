use std::thread;

use crate::pattern::Pattern;
use crate::util::split_scannable;
use crate::scanner::{Scanner, ScanResult};
use crate::scanner::simple::SimpleScanner;

/// This scanner works by taking the search range, splitting it up in chunks and feeding every
/// chunk into its own SimpleScanner, then taking the results and rebasing the found offsets.
/// Because I'm lazy this does not yet halt when a single result is found *but* will only return
/// the first result that was found.
pub struct ThreadedScanner {
    pub thread_count: usize,
}

impl Scanner for ThreadedScanner {
    fn scan(&self, scannable: &'static [u8], pattern: &Pattern) -> Option<ScanResult> {
        let chunks = split_scannable(
            scannable,
            self.thread_count,
            pattern.length - 1
        );

        let mut thread_handles = Vec::new();
        for (offset, chunk) in chunks.into_iter() {
            let pattern = pattern.clone();

            let handle = thread::spawn(move || {
                SimpleScanner.scan(chunk, &pattern)
            });

            thread_handles.push((offset, handle));
        }

        for handle in thread_handles {
            // Rebase the scan result to its respective chunk
            let result = handle.1.join()
                .unwrap()
                .map(|mut r|{
                    let offset = handle.0.as_usize();

                    r.location.move_by(offset);
                    r.captures.iter_mut().for_each(|c| c.location.move_by(offset));

                    r
                });

            if result.is_some() {
                return result;
            }
        }

        None
    }
}

impl ThreadedScanner {
    pub fn new_with_thread_count(thread_count: usize) -> Self {
        Self { thread_count }
    }
}

impl Default for ThreadedScanner {
    fn default() -> Self {
        Self::new_with_thread_count(thread::available_parallelism().unwrap().get())
    }
}

#[cfg(test)]
mod tests {
    use broadsword_address::Offset;

    use crate::scanner::Scanner;
    use crate::pattern::Pattern;
    use crate::scanner::threaded::ThreadedScanner;

    #[test]
    fn thread_scanner_defaults_to_available_parallelism() {
        let scanner = ThreadedScanner::default();

        assert!(
            scanner.thread_count > 0,
            "Thread count was not a positive number"
        );
    }

    #[test]
    fn threaded_scanner_behaves_with_empty_slice() {
        let pattern = Pattern::from_byte_slice(&[0xAA, 0xAA, 0xAA, 0xAA, 0xAA]);
        let slice = Box::leak(Box::new([]));
        let result = ThreadedScanner::new_with_thread_count(4).scan(slice, &pattern);

        assert_eq!(result, None);
    }

    #[test]
    fn threaded_scanner_behaves_with_too_long_of_a_pattern() {
        let pattern = Pattern::from_byte_slice(&[0xAA, 0xAA, 0xAA, 0xAA, 0xAA]);
        let slice = Box::leak(Box::new([0x00, 0x00, 0x00, 0x00]));
        let result = ThreadedScanner::new_with_thread_count(4).scan(slice, &pattern);

        assert_eq!(result, None);
    }

    #[test]
    fn threaded_scanner_finds_the_pattern_1() {
        let pattern = Pattern::from_byte_slice(&[0x75, 0x84, 0x4A, 0xEF, 0x23, 0x24, 0xCA, 0x35]);
        let randomness = include_bytes!("../../test/random.bin");
        let result = ThreadedScanner::new_with_thread_count(4)
            .scan(randomness, &pattern)
            .unwrap();

        assert_eq!(result.location, Offset::from(1309924));
        assert_eq!(result.captures.len(), 0);
    }

    #[test]
    fn threaded_scanner_finds_the_pattern_2() {
        let pattern = Pattern::from_pattern_str("B7 [?? CF D8 ??] 0A ?? 27").unwrap();
        let randomness = include_bytes!("../../test/random.bin");
        let result = ThreadedScanner::new_with_thread_count(4)
            .scan(randomness, &pattern)
            .unwrap();

        assert_eq!(result.location, Offset::from(867776));
        assert_eq!(result.captures.len(), 1);
        assert_eq!(result.captures[0].location, Offset::from(867777));
        assert_eq!(result.captures[0].bytes, vec![0xc6, 0xcf, 0xd8, 0x11]);
    }

    #[test]
    fn threaded_scanner_doesnt_find_the_pattern() {
        let pattern = Pattern::from_byte_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        let randomness = include_bytes!("../../test/random.bin");
        let result = ThreadedScanner::new_with_thread_count(4).scan(randomness, &pattern);

        assert_eq!(result, None);
    }
}
