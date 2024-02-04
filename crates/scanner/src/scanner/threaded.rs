use std::thread;

use crate::pattern::Pattern;
use crate::scanner::simple;
use crate::scanner::ScanResult;
use crate::scanner::result::rebase_result;

fn split_into_chunks(chunks: usize, bytes: &'static [u8], overlap: usize) -> Vec<(usize, &'static [u8])> {
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

pub fn scan(bytes: &'static [u8], pattern: &Pattern, parallelism: Option<usize>) -> Option<ScanResult> {
    let parallelism = parallelism.unwrap_or(default_parallelism());
    let chunks = split_into_chunks(parallelism, bytes, pattern.length - 1);

    let mut handles = Vec::new();
    for (offset, chunk) in chunks.into_iter() {
        let pattern = pattern.clone();
        let handle = thread::spawn(move || {
            simple::scan(chunk, &pattern)
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

pub fn scan_all(bytes: &'static [u8], pattern: &Pattern, parallelism: Option<usize>) -> Vec<ScanResult> {
    let parallelism = parallelism.unwrap_or(default_parallelism());
    let chunks = split_into_chunks(parallelism, bytes, pattern.length - 1);

    let mut handles = Vec::new();
    for (offset, chunk) in chunks.into_iter() {
        let pattern = pattern.clone();
        let handle = thread::spawn(move || {
            simple::scan_all(chunk, &pattern)
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

fn default_parallelism() -> usize {
    thread::available_parallelism().unwrap().get()
}

fn clamp(input: usize, min: usize, max: usize) -> usize {
    if input < min {
        min
    } else if input > max {
        max
    } else {
        input
    }
}

#[cfg(test)]
mod tests {
    use crate::scanner;
    use crate::pattern::Pattern;

    #[test]
    fn threaded_scanner_behaves_with_empty_slice() {
        let pattern = Pattern::from_byte_slice(&[0xAA, 0xAA, 0xAA, 0xAA, 0xAA]);
        let slice = Box::leak(Box::new([]));
        let result = scanner::threaded::scan(slice, &pattern, Some(4));

        assert_eq!(result, None);
    }

    #[test]
    fn threaded_scanner_behaves_with_too_long_of_a_pattern() {
        let pattern = Pattern::from_byte_slice(&[0xAA, 0xAA, 0xAA, 0xAA, 0xAA]);
        let slice = Box::leak(Box::new([0x00, 0x00, 0x00, 0x00]));
        let result = scanner::threaded::scan(slice, &pattern, Some(4));

        assert_eq!(result, None);
    }

    #[test]
    fn threaded_scanner_finds_the_pattern_1() {
        let pattern = Pattern::from_byte_slice(&[0x75, 0x84, 0x4A, 0xEF, 0x23, 0x24, 0xCA, 0x35]);
        let randomness = include_bytes!("../../test/random.bin");
        let result = scanner::threaded::scan(randomness, &pattern, Some(4))
            .unwrap();

        assert_eq!(result.location, 1309924);
        assert_eq!(result.captures.len(), 0);
    }

    #[test]
    fn threaded_scanner_finds_the_pattern_2() {
        let pattern = Pattern::from_byte_pattern("B7 [?? CF D8 ??] 0A ?? 27").unwrap();
        let randomness = include_bytes!("../../test/random.bin");
        let result = scanner::threaded::scan(randomness, &pattern, Some(4))
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
        let result = scanner::threaded::scan(randomness, &pattern, Some(4));

        assert_eq!(result, None);
    }

    #[test]
    fn threaded_scanner_can_scan_all() {
        let pattern = Pattern::from_byte_slice(&[0x09, 0x02]);
        let randomness = include_bytes!("../../test/random.bin");
        let result = scanner::threaded::scan_all(randomness, &pattern, Some(4));

        assert_eq!(result.len(), 35);
    }
}
