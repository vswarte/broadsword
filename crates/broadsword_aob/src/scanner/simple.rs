use crate::pattern::Pattern;
use crate::scanner::{Scanner, ScanResult};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{SendError, Sender};
use std::sync::Arc;

#[derive(Default)]
pub struct SimpleScanner;

impl Scanner for SimpleScanner {
    fn scan(&self, scannable: &[u8], pattern: &Pattern) -> Option<ScanResult> {
        let mut position_in_pattern = 0;

        for (position, byte) in scannable.iter().enumerate() {
            if pattern.mask[position_in_pattern] && pattern.bytes[position_in_pattern] != *byte {
                position_in_pattern = 0;
                continue;
            }

            if position_in_pattern == pattern.length - 1 {
                let location = position - pattern.length + 1;

                // Grab the capture group results
                let mut captures = Vec::new();
                for group in pattern.capture_groups.iter() {
                    let group_start = location + group.start;
                    let group_end = location + group.end;

                    captures.push(scannable[group_start..group_end].to_vec());
                }

                return Some(ScanResult { location, captures });
            }

            position_in_pattern += 1;
        }

        None
    }

    fn scan_multiple(&self, scannable: &'static [u8], pattern: &Pattern) -> Vec<ScanResult> {
        let mut offset = 0;
        let mut results = vec![];

        loop {
            // Run normal scan from offset
            let result = self.scan(&scannable[offset..scannable.len()], pattern);


            // If no result were found `scan` went up to the end and we've reached the end.
            if result.is_none() {
                break;
            }

            let found = result.unwrap();
            // Create a new result object to rebase the location
            results.push(ScanResult {
                location: found.location + offset,
                captures: found.captures,
            });

            // Update the search offset if a result was found so we don't find the same entry again
            offset = found.location + 1;
        }

        results
    }
}

impl SimpleScanner {
    pub fn group_scan(&self, scannable: &[u8], patterns: &mut Vec<Pattern>) -> Vec<Pattern> {
        let mut context = Vec::with_capacity(patterns.len());
        let mut results = vec![];

        for position in 0..scannable.len() {
            let mut position_in_pattern = 0;
            context.clear();
            context.resize(patterns.len(), false);

            for byte in scannable[position..].iter() {
                for (i, pattern) in patterns.iter_mut().enumerate() {
                    if context[i] {
                        continue;
                    }

                    if pattern.offset.is_some()
                        || (pattern.mask[position_in_pattern]
                            && pattern.bytes[position_in_pattern] != *byte)
                    {
                        context[i] = true;
                        continue;
                    }

                    if position_in_pattern == pattern.length - 1 {
                        pattern.offset = Some(position);
                        results.push(pattern.clone());
                        context[i] = true;
                    }
                }

                if context.iter().all(|b| *b) {
                    break;
                }

                position_in_pattern += 1;
            }
            if patterns.len() == results.len() {
                break;
            }
        }
        results
    }

    pub fn threaded_group_scan(
        &self,
        scannable: &[u8],
        offset: usize,
        mut patterns: Vec<Pattern>,
        sender: Sender<Pattern>,
        stop_thread: Arc<AtomicBool>,
    ) -> Result<(), SendError<Pattern>> {
        let mut context = Vec::with_capacity(patterns.len());

        for position in 0..scannable.len() {
            let mut position_in_pattern = 0;
            context.clear();
            context.resize(patterns.len(), false);

            for byte in scannable[position..].iter() {
                for (i, pattern) in patterns.iter_mut().enumerate() {
                    if context[i] {
                        continue;
                    }

                    if pattern.offset.is_some()
                        || (pattern.mask[position_in_pattern]
                            && pattern.bytes[position_in_pattern] != *byte)
                    {
                        context[i] = true;
                        continue;
                    }

                    if position_in_pattern == pattern.length - 1 {
                        pattern.offset = Some(offset + position);
                        sender.send(pattern.clone())?;
                        context[i] = true;
                    }
                }

                if context.iter().all(|b| *b) {
                    break;
                }

                position_in_pattern += 1;
            }
            if stop_thread.load(Ordering::Relaxed) {
                return Ok(());
            }
        }
        Ok(())
    }
}

impl SimpleScanner {
    pub fn new() -> Self {
        Self {}
    }
}

#[cfg(test)]
mod tests {
    use crate::scanner::Scanner;
    use crate::pattern::Pattern;
    use crate::scanner::simple::SimpleScanner;

    #[test]
    fn simple_scanner_behaves_with_empty_slice() {
        let pattern = Pattern::from_byte_slice(&[0xAA, 0xAA, 0xAA, 0xAA, 0xAA]);
        let slice = Box::leak(Box::new([]));
        let result = SimpleScanner::default().scan(slice, &pattern);

        assert_eq!(result, None);
    }

    #[test]
    fn simple_scanner_behaves_with_too_long_of_an_aob() {
        let pattern = Pattern::from_byte_slice(&[0xAA, 0xAA, 0xAA, 0xAA, 0xAA]);
        let slice = Box::leak(Box::new([0x00, 0x00, 0x00, 0x00]));
        let result = SimpleScanner::default().scan(slice, &pattern);

        assert_eq!(result, None);
    }

    #[test]
    fn simple_scanner_finds_the_pattern_1() {
        let pattern = Pattern::from_byte_slice(&[0x75, 0x84, 0x4A, 0xEF, 0x23, 0x24, 0xCA, 0x35]);
        let randomness = include_bytes!("../../test/random.bin");
        let result = SimpleScanner::default().scan(randomness, &pattern).unwrap();

        assert_eq!(result.location, 1309924);
        assert_eq!(result.captures.len(), 0);
    }

    #[test]
    fn simple_scanner_finds_the_pattern_2() {
        let pattern = Pattern::from_pattern_str("B7 [?? CF D8 ??] 0A ?? 27").unwrap();
        let randomness = include_bytes!("../../test/random.bin");
        let result = SimpleScanner::default().scan(randomness, &pattern).unwrap();

        assert_eq!(result.location, 867776);
        assert_eq!(result.captures.len(), 1);

        assert_eq!(result.captures[0], vec![0xc6, 0xcf, 0xd8, 0x11]);
    }

    #[test]
    fn simple_scanner_group_finds_the_patterns() {
        let mut patterns = Vec::with_capacity(5);
        patterns.push(Pattern::from_ida_pattern("75 84 4A EF 23 24 CA 35").unwrap());
        patterns.push(Pattern::from_ida_pattern("B7 ?? CF D8 ?? 0A ?? 27").unwrap());
        let randomness = include_bytes!("../../test/random.bin");
        let result = SimpleScanner::default().group_scan(randomness, &mut patterns);

        let valid = vec![1309924, 867776];
        assert_eq!(result.len(), 2);
        assert!(valid.contains(&result[0].offset.unwrap()));
        assert!(valid.contains(&result[1].offset.unwrap()));
    }

    #[test]
    fn simple_scanner_group_finds_the_patterns_except_one() {
        let mut patterns = Vec::with_capacity(5);
        patterns.push(Pattern::from_ida_pattern("75 84 4A EF 23 24 CA 35").unwrap());
        patterns.push(Pattern::from_ida_pattern("B7 ?? CF D8 ?? 0A ?? 27").unwrap());
        patterns.push(Pattern::from_ida_pattern("AA BB CC DD EE FF 00 11").unwrap());
        let randomness = include_bytes!("../../test/random.bin");
        let result = SimpleScanner::default().group_scan(randomness, &mut patterns);

        let valid = vec![1309924, 867776];
        assert_eq!(result.len(), 2);
        assert!(valid.contains(&result[0].offset.unwrap()));
        assert!(valid.contains(&result[1].offset.unwrap()));
    }

    #[test]
    fn simple_scanner_doesnt_find_the_pattern() {
        let pattern = Pattern::from_byte_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        let randomness = include_bytes!("../../test/random.bin");
        let result = SimpleScanner::default().scan(randomness, &pattern);

        assert_eq!(result, None);
    }
}
