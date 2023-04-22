use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, mpsc};
use std::sync::mpsc::{Receiver, Sender, SendError};
use std::thread;
use crate::pattern::Pattern;
use crate::scanner::simple::SimpleScanner;
use crate::scanner::{GroupScanner, Scanner};
use crate::util::split_scannable;

pub struct ThreadedScanner {
    pub thread_count: usize,
}

impl Scanner for ThreadedScanner {
    fn scan(&self, scannable: &'static [u8], pattern: &Pattern) -> Option<usize> {
        let chunks = split_scannable(scannable, self.thread_count, pattern.length - 1);

        let mut thread_handles = Vec::new();
        for (offset, chunk) in chunks.into_iter() {
            let pattern = pattern.clone();

            let handle = std::thread::spawn(move || SimpleScanner::default().scan(chunk, &pattern));

            thread_handles.push((offset, handle));
        }

        for handle in thread_handles {
            let result = handle.1.join().unwrap().map(|r| r + handle.0);

            if result.is_some() {
                return result;
            }
        }

        None
    }
}


impl ThreadedScanner {
    pub fn group_scan(&self, scannable: &'static [u8], patterns: Vec<Pattern>) -> Vec<Pattern> {
        let length = patterns.iter().max_by_key(|p| p.length).unwrap().length - 1;
        let chunks = split_scannable(scannable, self.thread_count, length);

        let mut thread_handles = Vec::new();
        let (sx, rx): (Sender<Option<Pattern>>, Receiver<Option<Pattern>>) = mpsc::channel();
        let stop = Arc::new(AtomicBool::new(false));

        for (offset, chunk) in chunks.into_iter() {
            let mut pattern = patterns.clone();
            let sender = sx.clone();
            let stop_thread = stop.clone();

            let handle = thread::spawn(move || SimpleScanner::default().multi_group_scan(chunk, offset, pattern, sender, stop_thread));

            thread_handles.push(handle);
        }

        let mut results = Vec::with_capacity(patterns.len());

        for found_item in rx {
            // Push to result vec
            if let Some(found) = found_item {
                results.push(found);
            }

            if results.len() == patterns.len() || thread_handles.iter().all(|t| t.is_finished())  {
                // Cancel threads by setting atomic flag
                stop.store(true, Ordering::Relaxed);
                break;
            }
        }

        // Wait for the threads to complete any remaining work
        for thread in thread_handles {
            match thread.join().expect("The child thread panicked") {
                _ => {}
            };
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

#[cfg(test)]
mod tests {
    use crate::pattern::Pattern;
    use crate::scanner::threaded::ThreadedScanner;
    use crate::scanner::{GroupScanner, Scanner};

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
        let pattern = Pattern::from_ida_pattern("AA AA AA AA AA").unwrap();
        let slice = Box::leak(Box::new([]));
        let result = ThreadedScanner::new_with_thread_count(4).scan(slice, &pattern);

        assert_eq!(result, None);
    }

    #[test]
    fn threaded_scanner_behaves_with_too_long_of_an_aob() {
        let pattern = Pattern::from_ida_pattern("AA AA AA AA AA").unwrap();
        let slice = Box::leak(Box::new([0x00, 0x00, 0x00, 0x00]));
        let result = ThreadedScanner::new_with_thread_count(4).scan(slice, &pattern);

        assert_eq!(result, None);
    }

    #[test]
    fn threaded_scanner_finds_the_pattern_1() {
        let pattern = Pattern::from_ida_pattern("75 84 4A EF 23 24 CA 35").unwrap();
        let randomness = include_bytes!("../../test/random.bin");
        let result = ThreadedScanner::new_with_thread_count(4)
            .scan(randomness, &pattern)
            .unwrap();

        assert_eq!(result, 1309924);
    }

    #[test]
    fn threaded_scanner_finds_the_pattern_2() {
        let pattern = Pattern::from_ida_pattern("B7 ?? CF D8 ?? 0A ?? 27").unwrap();
        let randomness = include_bytes!("../../test/random.bin");
        let result = ThreadedScanner::new_with_thread_count(4)
            .scan(randomness, &pattern)
            .unwrap();

        assert_eq!(result, 867776);
    }

    #[test]
    fn threaded_scanner_finds_the_patterns() {
        let mut patterns = Vec::with_capacity(5);
        patterns.push(Pattern::from_ida_pattern("75 84 4A EF 23 24 CA 35").unwrap());
        patterns.push(Pattern::from_ida_pattern("B7 ?? CF D8 ?? 0A ?? 27").unwrap());
        let randomness = include_bytes!("../../test/random.bin");
        let result = ThreadedScanner::new_with_thread_count(4)
            .group_scan(
                randomness,
                patterns.clone(),
            );

        let valid = vec![1309924, 867776];
        assert_eq!(result.len(), 2);
        assert!(valid.contains(&result[0].offset.unwrap()));
        assert!(valid.contains(&result[1].offset.unwrap()));
    }

    #[test]
    fn threaded_scanner_finds_the_patterns_except_one() {
        let mut patterns = Vec::with_capacity(5);
        patterns.push(Pattern::from_ida_pattern("75 84 4A EF 23 24 CA 35").unwrap());
        patterns.push(Pattern::from_ida_pattern("B7 ?? CF D8 ?? 0A ?? 27").unwrap());
        patterns.push(Pattern::from_ida_pattern("AA BB CC DD EE FF 00 11").unwrap());
        let randomness = include_bytes!("../../test/random.bin");
        let result = ThreadedScanner::new_with_thread_count(4)
            .group_scan(
                randomness,
                patterns.clone(),
            );

        let valid = vec![1309924, 867776];
        assert_eq!(result.len(), 2);
        assert!(valid.contains(&result[0].offset.unwrap()));
        assert!(valid.contains(&result[1].offset.unwrap()));
    }

    #[test]
    fn threaded_scanner_doesnt_find_the_pattern() {
        let pattern = Pattern::from_ida_pattern("FF FF FF FF FF FF FF FF").unwrap();
        let randomness = include_bytes!("../../test/random.bin");
        let result = ThreadedScanner::new_with_thread_count(4).scan(randomness, &pattern);

        assert_eq!(result, None);
    }
}
