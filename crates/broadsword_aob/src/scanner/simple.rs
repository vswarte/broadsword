use crate::pattern::Pattern;
use crate::scanner::{Scanner, ScanResult};

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
    fn simple_scanner_doesnt_find_the_pattern() {
        let pattern = Pattern::from_byte_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        let randomness = include_bytes!("../../test/random.bin");
        let result = SimpleScanner::default().scan(randomness, &pattern);

        assert_eq!(result, None);
    }
}
