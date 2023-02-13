use crate::pattern::MemoryPattern;

trait Scanner {
    fn scan(&self, scannable: &[u8], pattern: &MemoryPattern) -> Option<usize>;
}

#[derive(Default)]
struct SimpleScanner;

impl Scanner for SimpleScanner {
    fn scan(&self, scannable: &[u8], pattern: &MemoryPattern) -> Option<usize> {
        let mut position_in_pattern = 0;

        for (position, byte) in scannable.iter().enumerate() {
            if position + pattern.length > scannable.len() {
                break;
            }

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

#[cfg(test)]
mod tests {
    use crate::scanner::Scanner;
    use crate::pattern::MemoryPattern;
    use crate::scanner::SimpleScanner;

    #[test]
    fn standard_scanner_behaves_with_empty_slice() {
        let pattern = MemoryPattern::from_ida_pattern("AA AA AA AA AA").unwrap();
        let slice = [];
        let result = SimpleScanner::default().scan(&slice, &pattern);

        assert_eq!(result, None);
    }

    #[test]
    fn standard_scanner_behaves_with_too_long_of_an_aob() {
        let pattern = MemoryPattern::from_ida_pattern("AA AA AA AA AA").unwrap();
        let slice = [0x00, 0x00, 0x00, 0x00];
        let result = SimpleScanner::default().scan(&slice, &pattern);

        assert_eq!(result, None);
    }

    #[test]
    fn standard_scanner_finds_the_pattern_1() {
        let pattern = MemoryPattern::from_ida_pattern("75 84 4A EF 23 24 CA 35").unwrap();
        let randomness = include_bytes!("../../../tests/random.bin");
        let result = SimpleScanner::default().scan(randomness, &pattern).unwrap();

        assert_eq!(result, 1309924);
    }

    #[test]
    fn standard_scanner_finds_the_pattern_2() {
        let pattern = MemoryPattern::from_ida_pattern("B7 ?? CF D8 ?? 0A ?? 27").unwrap();
        let randomness = include_bytes!("../../../tests/random.bin");
        let result = SimpleScanner::default().scan(randomness, &pattern).unwrap();

        assert_eq!(result, 867776);
    }

    #[test]
    fn standard_scanner_doesnt_find_the_pattern() {
        let pattern = MemoryPattern::from_ida_pattern("FF FF FF FF FF FF FF FF").unwrap();
        let randomness = include_bytes!("../../../tests/random.bin");
        let result = SimpleScanner::default().scan(randomness, &pattern);

        assert_eq!(result, None);
    }
}
