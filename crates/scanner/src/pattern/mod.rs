use std::ops::Range;

pub mod parser;
pub mod tokenizer;

#[derive(Debug, Clone)]
pub struct Pattern {
    pub bytes: Vec<u8>,
    pub mask: Vec<u8>,
    pub length: usize,
    pub capture_groups: Vec<Range<usize>>,
}

impl Pattern {
    /// Parses a pattern string to a pattern used for searching.
    pub fn from_byte_pattern(pattern: &str) -> parser::ParserResult {
        parser::parse_pattern(pattern, tokenizer::tokenize_byte_pattern)
    }

    pub fn from_bit_pattern(pattern: &str) -> parser::ParserResult {
        parser::parse_pattern(pattern, tokenizer::tokenize_bit_pattern)
    }

    /// Constructs a pattern from a byte slice. Assumes a mask where all bytes
    /// are matched.
    pub fn from_byte_vec(bytes: Vec<u8>) -> Self {
        let length = bytes.len();
        let mask = vec![0xFFu8; length];
        let capture_groups = vec![];

        Self { bytes, mask, length, capture_groups }
    }

    /// Wraps `from_byte_vec` and copies the `bytes` slice.
    pub fn from_byte_slice(bytes: &[u8]) -> Self {
        Self::from_byte_vec(bytes.to_vec())
    }
}

#[cfg(test)]
#[allow(clippy::bool_assert_comparison)]
mod tests {
    use crate::pattern::Pattern;
    use crate::pattern::parser::ParserError;

    #[test]
    fn byte_pattern_works() {
        let pattern = Pattern::from_byte_pattern(
            "12 [34 ??] 78 [9A]"
        ).unwrap();

        assert_eq!(
            pattern.length, 5,
            "Indicated pattern length did not match up with input"
        );
        assert_eq!(
            pattern.mask.len(),
            5,
            "Length on the mask did not match up with input"
        );
        assert_eq!(
            pattern.bytes.len(),
            5,
            "Length on the matching bytes did not match up with input"
        );

        let mut byte_iter = pattern.bytes.iter();
        assert_eq!(byte_iter.next(), Some(&0x12));
        assert_eq!(byte_iter.next(), Some(&0x34));
        assert_eq!(byte_iter.next(), Some(&0x00));
        assert_eq!(byte_iter.next(), Some(&0x78));
        assert_eq!(byte_iter.next(), Some(&0x9A));
        assert_eq!(byte_iter.next(), None);

        let mut mask_iter = pattern.mask.iter();
        assert_eq!(mask_iter.next(), Some(&0xFF));
        assert_eq!(mask_iter.next(), Some(&0xFF));
        assert_eq!(mask_iter.next(), Some(&0x00));
        assert_eq!(mask_iter.next(), Some(&0xFF));
        assert_eq!(mask_iter.next(), Some(&0xFF));
        assert_eq!(mask_iter.next(), None);

        assert_eq!(pattern.capture_groups.len(), 2);
        assert_eq!(pattern.capture_groups[0].start, 1);
        assert_eq!(pattern.capture_groups[0].end, 3);
        assert_eq!(pattern.capture_groups[1].start, 4);
        assert_eq!(pattern.capture_groups[1].end, 5);
    }

    #[test]
    fn bit_pattern_works() {
        let pattern = Pattern::from_bit_pattern(
            "00010010 [00110100 ........] 01111000 [10011010]"
        ).unwrap();

        assert_eq!(
            pattern.length, 5,
            "Indicated pattern length did not match up with input"
        );
        assert_eq!(
            pattern.mask.len(),
            5,
            "Length on the mask did not match up with input"
        );
        assert_eq!(
            pattern.bytes.len(),
            5,
            "Length on the matching bytes did not match up with input"
        );

        let mut byte_iter = pattern.bytes.iter();
        assert_eq!(byte_iter.next(), Some(&0x12));
        assert_eq!(byte_iter.next(), Some(&0x34));
        assert_eq!(byte_iter.next(), Some(&0x00));
        assert_eq!(byte_iter.next(), Some(&0x78));
        assert_eq!(byte_iter.next(), Some(&0x9A));
        assert_eq!(byte_iter.next(), None);

        let mut mask_iter = pattern.mask.iter();
        assert_eq!(mask_iter.next(), Some(&0xFF));
        assert_eq!(mask_iter.next(), Some(&0xFF));
        assert_eq!(mask_iter.next(), Some(&0x00));
        assert_eq!(mask_iter.next(), Some(&0xFF));
        assert_eq!(mask_iter.next(), Some(&0xFF));
        assert_eq!(mask_iter.next(), None);

        assert_eq!(pattern.capture_groups.len(), 2);
        assert_eq!(pattern.capture_groups[0].start, 1);
        assert_eq!(pattern.capture_groups[0].end, 3);
        assert_eq!(pattern.capture_groups[1].start, 4);
        assert_eq!(pattern.capture_groups[1].end, 5);

    }

    #[test]
    fn byte_pattern_returns_error_on_invalid_hex_value() {
        let result = Pattern::from_byte_pattern("XX 34 ?? 78 9A");

        assert!(matches!(
            result.unwrap_err(),
            ParserError::Tokenizer { .. }
        ));
    }

    #[test]
    fn byte_pattern_returns_error_on_already_opened_capture_group() {
        let result = Pattern::from_byte_pattern("12[34[56]");

        assert!(matches!(
            result.unwrap_err(),
            ParserError::CaptureGroupAlreadyOpened
        ));
    }

    #[test]
    fn byte_pattern_returns_error_on_not_yet_opened_capture_group() {
        let result = Pattern::from_byte_pattern("12]34[56]");

        assert!(matches!(
            result.unwrap_err(),
            ParserError::CaptureGroupNotOpened
        ));
    }

    #[test]
    fn bit_pattern_returns_error_on_invalid_bit_value() {
        let result = Pattern::from_bit_pattern("00000002 00000000");

        assert!(matches!(
            result.unwrap_err(),
            ParserError::Tokenizer { .. }
        ));
    }

    #[test]
    fn bit_pattern_returns_error_on_already_opened_capture_group() {
        let result = Pattern::from_bit_pattern("00000000 [[00000000]");

        assert!(matches!(
            result.unwrap_err(),
            ParserError::CaptureGroupAlreadyOpened
        ));
    }

    #[test]
    fn bit_pattern_returns_error_on_not_yet_opened_capture_group() {
        let result = Pattern::from_bit_pattern("00000000] [00000000]");

        assert!(matches!(
            result.unwrap_err(),
            ParserError::CaptureGroupNotOpened
        ));
    }
}
