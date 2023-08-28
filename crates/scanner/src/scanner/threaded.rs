use std::thread;

use crate::pattern::Pattern;
use crate::scanner::{Scanner, ScanResult};
use crate::scanner::result::rebase_result;
use crate::scanner::simple::SimpleScanner;

/// This scanner works by taking the search range, splitting it up in chunks and feeding every
/// chunk into its own SimpleScanner, then taking the results and rebasing the found offsets.
/// Because I'm lazy this does not yet halt when a single result is found *but* will only return
/// the first result that was found.
pub struct ThreadedScanner {
    pub thread_count: usize,
}

impl ThreadedScanner {
    fn split_into_chunks(&self, bytes: &'static [u8], overlap: usize) -> Vec<(usize, &'static [u8])> {
        let chunks = self.thread_count;
        let bytes_per_chunk = bytes.len() / chunks;

        let mut offset: usize = 0;
        let mut results = Vec::new();

        for _ in 0..chunks {
            let start = offset;
            // Clamp the end to the range so we don't go out-of-bounds
            let end = clamp(
                start + (bytes_per_chunk + overlap),
                0,
                bytes.len()
            );

            results.push((offset, &bytes[start..end]));
            offset += bytes_per_chunk;
        }

        results
    }
}

impl Scanner for ThreadedScanner {
    fn scan(&self, bytes: &'static [u8], pattern: &Pattern) -> Option<ScanResult> {
        let chunks = self.split_into_chunks(bytes, pattern.length - 1);

        let mut handles = Vec::new();
        for (offset, chunk) in chunks.into_iter() {
            let pattern = pattern.clone();
            let handle = thread::spawn(move || {
                SimpleScanner.scan(chunk, &pattern)
            });

            handles.push((offset, handle));
        }

        for handle in handles {
            // Rebase the scan result to its respective chunk
            let result = handle.1.join()
                .unwrap()
                .map(|r| rebase_result(r, handle.0));

            if result.is_some() {
                return result;
            }
        }

        None
    }

    fn scan_all(&self, bytes: &'static [u8], pattern: &Pattern) -> Vec<ScanResult> {
        let chunks = self.split_into_chunks(bytes, pattern.length - 1);

        let mut handles = Vec::new();
        for (offset, chunk) in chunks.into_iter() {
            let pattern = pattern.clone();
            let handle = thread::spawn(move || {
                SimpleScanner.scan_all(chunk, &pattern)
            });

            handles.push((offset, handle));
        }

        let mut results = Vec::new();
        for handle in handles {
            // Rebase the scan result to its respective chunk
            results.append(
                &mut handle.1.join()
                    .unwrap()
                    .into_iter()
                    .map(|r| rebase_result(r, handle.0))
                    .collect::<Vec<ScanResult>>()
            );
        }

        results
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

fn clamp(input: usize, min: usize, max: usize) -> usize {
    if input < min {
        return min;
    } else if input > max {
        return max;
    } else {
        input
    }
}

#[cfg(test)]
mod tests {
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

        assert_eq!(result.location, 1309924);
        assert_eq!(result.captures.len(), 0);
    }

    #[test]
    fn threaded_scanner_finds_the_pattern_2() {
        let pattern = Pattern::from_pattern_str("B7 [?? CF D8 ??] 0A ?? 27").unwrap();
        let randomness = include_bytes!("../../test/random.bin");
        let result = ThreadedScanner::new_with_thread_count(4)
            .scan(randomness, &pattern)
            .unwrap();

        assert_eq!(result.location, 867776);
        assert_eq!(result.captures.len(), 1);
        assert_eq!(result.captures[0].location, 867777);
        assert_eq!(result.captures[0].bytes, vec![0xc6, 0xcf, 0xd8, 0x11]);
    }

    #[test]
    fn threaded_scanner_doesnt_find_the_pattern() {
        let pattern = Pattern::from_byte_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        let randomness = include_bytes!("../../test/random.bin");
        let result = ThreadedScanner::new_with_thread_count(4).scan(randomness, &pattern);

        assert_eq!(result, None);
    }

    #[test]
    fn threaded_scanner_can_scan_all() {
        let pattern = Pattern::from_byte_slice(&[0x09, 0x02]);
        let randomness = include_bytes!("../../test/random.bin");
        let result = ThreadedScanner::new_with_thread_count(4).scan_all(randomness, &pattern);

        assert_eq!(result.len(), 35);
    }
}
