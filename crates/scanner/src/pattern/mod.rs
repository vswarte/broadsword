use std::ops::Range;

pub(crate) mod parser;
pub(crate) mod tokenizer;

#[derive(Debug, Clone)]
pub struct Pattern {
    pub bytes: Vec<u8>,
    pub mask: Vec<u8>,
    pub length: usize,
    pub capture_groups: Vec<Range<usize>>,
}

impl Pattern {
    /// Parses a pattern string to a pattern used for searching.
    pub fn from_pattern_str(pattern: &str) -> Result<Self, parser::ParserError> {
        parser::parse_pattern(pattern)
    }

    /// Wraps `from_pattern_str`. Drops the input `String` after creation of pattern.
    pub fn from_pattern_string(pattern: String) -> Result<Self, parser::ParserError> {
        parser::parse_pattern(pattern.as_str())
    }

    /// Constructs a pattern from a byte slice. Assumes a mask where all bytes are matched.
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
    fn from_works_1() {
        let pattern = Pattern::from_pattern_str("12 [34 ??] 78 [9A]").unwrap();

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
    fn from_returns_error_on_invalid_hex_value() {
        let result = Pattern::from_pattern_str("XX 34 ?? 78 9A");

        assert!(matches!(
            result.unwrap_err(),
            ParserError::Tokenizer { .. }
        ));
    }

    #[test]
    fn from_returns_error_on_already_opened_capture_group() {
        let result = Pattern::from_pattern_str("12[34[56]");

        assert!(matches!(
            result.unwrap_err(),
            ParserError::CaptureGroupAlreadyOpened
        ));
    }

    #[test]
    fn from_returns_error_on_not_yet_opened_capture_group() {
        let result = Pattern::from_pattern_str("12]34[56]");

        assert!(matches!(
            result.unwrap_err(),
            ParserError::CaptureGroupNotOpened
        ));
    }
}
