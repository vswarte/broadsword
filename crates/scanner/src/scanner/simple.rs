
use crate::pattern::Pattern;
use crate::scanner::ScanResult;
use crate::scanner::result::{grab_captures, rebase_capture};

pub fn scan(bytes: &'static [u8], pattern: &Pattern) -> Option<ScanResult> {
    let mut position_in_pattern = 0;

    for (position, byte) in bytes.iter().enumerate() {
        // Reset position in pattern if current byte is not masked off and doesn't match
        // expected byte.
        if pattern.bytes[position_in_pattern] != *byte & pattern.mask[position_in_pattern] {
            position_in_pattern = 0;
            continue;
        }

        // Check if all bytes in the pattern have been matched.
        if position_in_pattern == pattern.length - 1 {
            // Offset in the bytes the match was found
            let match_offset = position - pattern.length + 1;

            // Grab data for any capture groups
            let captures = grab_captures(
                    &bytes[match_offset..match_offset + pattern.length],
                    pattern.capture_groups.as_slice()
                )
                .into_iter()
                .map(|c| rebase_capture(c, match_offset))
                .collect();

            return Some(ScanResult {
                location: match_offset,
                captures,
            });
        }

        position_in_pattern += 1;
    }

    None
}

pub fn scan_all(bytes: &'static [u8], pattern: &Pattern) -> Vec<ScanResult> {
    let mut results = Vec::new();

    let mut current_offset = 0;
    loop {
        let search_area = &bytes[current_offset..];
        match scan(search_area, pattern) {
            Some(occurence) => {
                // Move cursor to the end of the match
                current_offset = current_offset + occurence.location + pattern.length;

                results.push(occurence);
            }
            None => break
        }
    }

    results
}

#[cfg(test)]
mod tests {
    use crate::scanner;
    use crate::pattern::Pattern;

    #[test]
    fn simple_scanner_behaves_with_empty_slice() {
        let pattern = Pattern::from_byte_slice(&[0xAA, 0xAA, 0xAA, 0xAA, 0xAA]);
        let slice = Box::leak(Box::new([]));
        let result = scanner::simple::scan(slice, &pattern);

        assert_eq!(result, None);
    }

    #[test]
    fn simple_scanner_behaves_with_too_long_of_a_pattern() {
        let pattern = Pattern::from_byte_slice(&[0xAA, 0xAA, 0xAA, 0xAA, 0xAA]);
        let slice = Box::leak(Box::new([0x00, 0x00, 0x00, 0x00]));
        let result = scanner::simple::scan(slice, &pattern);

        assert_eq!(result, None);
    }

    #[test]
    fn simple_scanner_finds_the_pattern_1() {
        let pattern = Pattern::from_byte_slice(&[0x75, 0x84, 0x4A, 0xEF, 0x23, 0x24, 0xCA, 0x35]);
        let randomness = include_bytes!("../../test/random.bin");
        let result = scanner::simple::scan(randomness, &pattern).unwrap();

        assert_eq!(result.location, 1309924);
        assert_eq!(result.captures.len(), 0);
    }

    #[test]
    fn simple_scanner_finds_the_pattern_2() {
        let pattern = Pattern::from_pattern_str("B7 [?? CF D8 ??] 0A ?? 27").unwrap();
        let randomness = include_bytes!("../../test/random.bin");
        let result = scanner::simple::scan(randomness, &pattern).unwrap();

        assert_eq!(result.location, 867776);
        assert_eq!(result.captures.len(), 1);

        assert_eq!(result.captures[0].location, 867777);
        assert_eq!(result.captures[0].bytes, vec![0xc6, 0xcf, 0xd8, 0x11]);
    }

    #[test]
    fn simple_scanner_doesnt_find_the_pattern() {
        let pattern = Pattern::from_byte_slice(&[0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF]);
        let randomness = include_bytes!("../../test/random.bin");
        let result = scanner::simple::scan(randomness, &pattern);

        assert_eq!(result, None);
    }

    #[test]
    fn simple_scanner_can_scan_all() {
        let pattern = Pattern::from_byte_slice(&[0x09, 0x02]);
        let randomness = include_bytes!("../../test/random.bin");
        let result = scanner::simple::scan_all(randomness, &pattern);

        assert_eq!(result.len(), 35);
    }
}
