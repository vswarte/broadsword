use std::num;

#[derive(Debug, Clone)]
pub struct Pattern {
    pub bytes: Vec<u8>,
    pub mask: Vec<bool>,
    pub length: usize,
}

#[derive(Debug)]
pub enum PatternError {
    ParseError(num::ParseIntError),
    NoMatchableBytesError,
}

impl Pattern {
    pub fn from_ida_pattern(pattern: &str) -> Result<Self, PatternError> {
        let mut bytes = Vec::new();
        let mut mask = Vec::new();

        // TODO: this parsing relies on whitespaces to be properly placed
        for byte in pattern.split_whitespace() {
            if byte == "?" || byte == "??" {
                mask.push(false);
                bytes.push(0);
            } else {
                mask.push(true);
                bytes.push(
                    u8::from_str_radix(byte, 16)
                        .map_err(PatternError::ParseError)?
                );
            }
        }

       if !mask.iter().any(|x| *x) {
           return Err(PatternError::NoMatchableBytesError);
       }

        let length = bytes.len();
        Ok(Self { bytes, mask, length })
    }
}

#[cfg(test)]
#[allow(clippy::bool_assert_comparison)]
mod tests {
    use crate::pattern::Pattern;
    use crate::pattern::PatternError;

    #[test]
    fn from_ida_pattern_works() {
        let pattern = Pattern::from_ida_pattern("12 34 ?? 78 9A").unwrap();

        assert_eq!(pattern.length, 5, "Indicated pattern length did not match up with input");
        assert_eq!(pattern.mask.len(), 5, "Length on the mask did not match up with input");
        assert_eq!(pattern.bytes.len(), 5, "Length on the matching bytes did not match up with input");

        assert_eq!(pattern.bytes[0], 0x12);
        assert_eq!(pattern.bytes[1], 0x34);
        assert_eq!(pattern.bytes[2], 0x00);
        assert_eq!(pattern.bytes[3], 0x78);
        assert_eq!(pattern.bytes[4], 0x9A);

        assert_eq!(pattern.mask[0], true);
        assert_eq!(pattern.mask[1], true);
        assert_eq!(pattern.mask[2], false);
        assert_eq!(pattern.mask[3], true);
        assert_eq!(pattern.mask[4], true);
    }

    #[test]
    fn from_ida_pattern_returns_error_on_invalid_hex_value() {
        let result = Pattern::from_ida_pattern("XX 34 ?? 78 9A");

        assert!(matches!(result.unwrap_err(), PatternError::ParseError {..} ));
    }

    #[test]
    fn from_ida_pattern_returns_error_on_empty_pattern() {
        let result = Pattern::from_ida_pattern("");

        assert!(matches!(result.unwrap_err(), PatternError::NoMatchableBytesError));
    }

    #[test]
    fn from_ida_pattern_returns_error_on_all_wildcard_pattern() {
        let result = Pattern::from_ida_pattern("?? ?? ?? ??");

        assert!(matches!(result.unwrap_err(), PatternError::NoMatchableBytesError));
    }
}
