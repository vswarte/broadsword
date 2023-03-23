use crate::pattern::Pattern;
use crate::scanner::Scanner;

pub struct SimpleScanner;

impl Scanner for SimpleScanner {
    fn scan(&self, scannable: &[u8], pattern: &Pattern) -> Option<usize> {
        let mut position_in_pattern = 0;

        for (position, byte) in scannable.iter().enumerate() {
            if pattern.mask[position_in_pattern] &&
                pattern.bytes[position_in_pattern] != *byte {
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
        let pattern = Pattern::from_ida_pattern("AA AA AA AA AA").unwrap();
        let slice = [];
        let result = SimpleScanner::new().scan(&slice, &pattern);

        assert_eq!(result, None);
    }

    #[test]
    fn simple_scanner_behaves_with_too_long_of_an_aob() {
        let pattern = Pattern::from_ida_pattern("AA AA AA AA AA").unwrap();
        let slice = [0x00, 0x00, 0x00, 0x00];
        let result = SimpleScanner::new().scan(&slice, &pattern);

        assert_eq!(result, None);
    }

    #[test]
    fn simple_scanner_finds_the_pattern_1() {
        let pattern = Pattern::from_ida_pattern("75 84 4A EF 23 24 CA 35").unwrap();
        let randomness = include_bytes!("../../test/random.bin");
        let result = SimpleScanner::new().scan(randomness, &pattern).unwrap();

        assert_eq!(result, 1309924);
    }

    #[test]
    fn simple_scanner_finds_the_pattern_2() {
        let pattern = Pattern::from_ida_pattern("B7 ?? CF D8 ?? 0A ?? 27").unwrap();
        let randomness = include_bytes!("../../test/random.bin");
        let result = SimpleScanner::new().scan(randomness, &pattern).unwrap();

        assert_eq!(result, 867776);
    }

    #[test]
    fn simple_scanner_doesnt_find_the_pattern() {
        let pattern = Pattern::from_ida_pattern("FF FF FF FF FF FF FF FF").unwrap();
        let randomness = include_bytes!("../../test/random.bin");
        let result = SimpleScanner::new().scan(randomness, &pattern);

        assert_eq!(result, None);
    }
}
