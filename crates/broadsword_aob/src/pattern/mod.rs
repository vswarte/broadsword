use std::ops::Range;

pub(crate) mod parser;
pub(crate) mod tokenizer;

#[derive(Debug, Clone)]
pub struct Pattern {
    pub bytes: Vec<u8>,
    pub mask: Vec<bool>,
    pub length: usize,
    pub capture_groups: Vec<Range<usize>>,
}

impl Pattern {
    pub fn from(pattern: &str) -> Result<Self, parser::ParserError> {
        parser::parse_pattern(pattern)
    }
}

#[cfg(test)]
#[allow(clippy::bool_assert_comparison)]
mod tests {
    use crate::pattern::Pattern;
    use crate::pattern::parser::ParserError;

    #[test]
    fn from_works_1() {
        let pattern = Pattern::from("12 [34 ??] 78 [9A]").unwrap();

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
        assert_eq!(mask_iter.next(), Some(&true));
        assert_eq!(mask_iter.next(), Some(&true));
        assert_eq!(mask_iter.next(), Some(&false));
        assert_eq!(mask_iter.next(), Some(&true));
        assert_eq!(mask_iter.next(), Some(&true));
        assert_eq!(mask_iter.next(), None);

        assert_eq!(pattern.capture_groups.len(), 2);
        assert_eq!(pattern.capture_groups[0].start, 1);
        assert_eq!(pattern.capture_groups[0].end, 3);
        assert_eq!(pattern.capture_groups[1].start, 4);
        assert_eq!(pattern.capture_groups[1].end, 5);

    }

    #[test]
    fn from_returns_error_on_invalid_hex_value() {
        let result = Pattern::from("XX 34 ?? 78 9A");

        assert!(matches!(
            result.unwrap_err(),
            ParserError::Tokenizer { .. }
        ));
    }

    #[test]
    fn from_returns_error_on_already_opened_capture_group() {
        let result = Pattern::from("12[34[56]");

        assert!(matches!(
            result.unwrap_err(),
            ParserError::CaptureGroupAlreadyOpened
        ));
    }

    #[test]
    fn from_returns_error_on_not_yet_opened_capture_group() {
        let result = Pattern::from("12]34[56]");

        assert!(matches!(
            result.unwrap_err(),
            ParserError::CaptureGroupNotOpened
        ));
    }

}
