use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use crate::pattern::Pattern;
use crate::scanner::{GroupScanner, Scanner};

#[derive(Default)]
pub struct SimpleScanner;

impl Scanner for SimpleScanner {
    fn scan(&self, scannable: &[u8], pattern: &Pattern) -> Option<usize> {
        let mut position_in_pattern = 0;

        for (position, byte) in scannable.iter().enumerate() {
            if pattern.mask[position_in_pattern] && pattern.bytes[position_in_pattern] != *byte {
                position_in_pattern = 0;
                continue;
            }

            if position_in_pattern == pattern.length - 1 {
                return Some(position - pattern.length + 1);
            }

            position_in_pattern += 1;
        }

        None
    }
}

impl SimpleScanner {
    pub fn group_scan(&self, scannable: &[u8], mut patterns: Vec<Pattern>) -> Vec<Pattern> {
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
                        pattern.offset = Some(position);
                        context[i] = true;
                    }
                }

                if context.iter().all(|b| *b) {
                    break;
                }

                position_in_pattern += 1;
            }
            if patterns.iter().all(|p| p.offset.is_some()) {
                break;
            }
        }
        patterns
    }

    pub fn multi_group_scan(&self, scannable: &[u8], offset: usize, mut patterns: Vec<Pattern>, sender: Sender<Pattern>, finished: &AtomicBool) -> Vec<Pattern> {
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
                        sender.send(pattern.clone());
                        context[i] = true;
                    }
                }

                if context.iter().all(|b| *b) {
                    break;
                }

                position_in_pattern += 1;
            }
            if finished.load(Ordering::Relaxed) {
                break;
            }
        }
        patterns
    }
}

impl SimpleScanner {
    pub fn new() -> Self {
        Self {}
    }
}

#[cfg(test)]
mod tests {
    use crate::pattern::Pattern;
    use crate::scanner::simple::SimpleScanner;
    use crate::scanner::{GroupScanner, Scanner};

    #[test]
    fn simple_scanner_behaves_with_empty_slice() {
        let pattern = Pattern::from_ida_pattern("AA AA AA AA AA").unwrap();
        let slice = Box::leak(Box::new([]));
        let result = SimpleScanner::default().scan(slice, &pattern);

        assert_eq!(result, None);
    }

    #[test]
    fn simple_scanner_behaves_with_too_long_of_an_aob() {
        let pattern = Pattern::from_ida_pattern("AA AA AA AA AA").unwrap();
        let slice = Box::leak(Box::new([0x00, 0x00, 0x00, 0x00]));
        let result = SimpleScanner::default().scan(slice, &pattern);

        assert_eq!(result, None);
    }

    #[test]
    fn simple_scanner_finds_the_pattern_1() {
        let pattern = Pattern::from_ida_pattern("75 84 4A EF 23 24 CA 35").unwrap();
        let randomness = include_bytes!("../../test/random.bin");
        let result = SimpleScanner::default().scan(randomness, &pattern).unwrap();

        assert_eq!(result, 1309924);
    }

    #[test]
    fn simple_scanner_finds_the_pattern_2() {
        let pattern = Pattern::from_ida_pattern("B7 ?? CF D8 ?? 0A ?? 27").unwrap();
        let randomness = include_bytes!("../../test/random.bin");
        let result = SimpleScanner::default().scan(randomness, &pattern).unwrap();

        assert_eq!(result, 867776);
    }

    #[test]
    fn simple_scanner_finds_the_patterns() {
        let mut patterns = Vec::with_capacity(5);
        patterns.push(Pattern::from_ida_pattern("75 84 4A EF 23 24 CA 35").unwrap());
        patterns.push(Pattern::from_ida_pattern("B7 ?? CF D8 ?? 0A ?? 27").unwrap());
        let randomness = include_bytes!("../../test/random.bin");
        let result = SimpleScanner::default().group_scan(randomness, patterns);

        assert_eq!(result[0].offset.unwrap_or_default(), 1309924);
        assert_eq!(result[1].offset.unwrap_or_default(), 867776);
    }

    #[test]
    fn simple_scanner_doesnt_find_the_pattern() {
        let pattern = Pattern::from_ida_pattern("FF FF FF FF FF FF FF FF").unwrap();
        let randomness = include_bytes!("../../test/random.bin");
        let result = SimpleScanner::default().scan(randomness, &pattern);

        assert_eq!(result, None);
    }
}

