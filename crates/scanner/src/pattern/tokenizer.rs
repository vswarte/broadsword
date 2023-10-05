#[derive(Debug, PartialEq)]
pub(crate) enum Token {
    CaptureGroupOpen,
    CaptureGroupClose,
    ByteValue(u8, u8),
}

#[derive(Debug)]
pub enum TokenizationError {
    UnknownInput,
    IncompleteByte,
}

pub(crate) fn tokenize_pattern(input: &str) -> Result<Vec<Token>, TokenizationError> {
    // Can probably just shift the ASCII values as a speed up
    let input_lower = input.to_lowercase();
    let mut input_iter = input_lower.chars().peekable();

    let mut tokens = Vec::new();
    while let Some(current_character) = input_iter.next() {
        match current_character {
            ' ' | '\n' | '\r' => { },
            '[' => tokens.push(Token::CaptureGroupOpen),
            ']' => tokens.push(Token::CaptureGroupClose),
            '?' => {
                // Collapse double question marks
                if input_iter.peek() == Some(&'?') {
                    input_iter.next();
                }

                tokens.push(Token::ByteValue(0x00, 0x00))
            }
            'm' => {
                let mut result_byte = 0u8;
                let mut result_mask = 0u8;

                // Consume next 8 characters
                for i in 0..8 {
                    match input_iter.next() {
                        None => return Err(TokenizationError::IncompleteByte),
                        Some(c) => {
                            if !is_bit_char(&c) {
                                return Err(TokenizationError::UnknownInput);
                            }

                            let shift = 7 - i;
                            result_byte |= bool_to_u8(c == '1') << shift;
                            result_mask |= bool_to_u8(c != '?') << shift;
                        }
                    }
                }

                tokens.push(Token::ByteValue(result_byte, result_mask))
            },
            _ => {
                // Ensure current character is a 0-9a-f.
                if !is_radix_16_char(&current_character) {
                    return Err(TokenizationError::UnknownInput);
                }

                let next_character = input_iter.next();
                // Ensure next character is available and it's 0-9a-f.
                if next_character.is_none() || !is_radix_16_char(next_character.as_ref().unwrap()) {
                    return Err(TokenizationError::IncompleteByte);
                }

                // Parse both characters together as a single byte
                // TODO: can probably parse both independently and shift them into place?
                let mut byte_string = String::new();
                byte_string.push(current_character);
                byte_string.push(next_character.unwrap());
                let parsed_byte = u8::from_str_radix(byte_string.as_str(), 16).unwrap();

                tokens.push(Token::ByteValue(parsed_byte, 0xFF))
            }
        };
    }

    Ok(tokens)
}

fn bool_to_u8(input: bool) -> u8 {
    match input {
        true => 0x1,
        false => 0x0,
    }
}

const RADIX_16_CHARS: [char; 16] = [
    '0', '1', '2', '3', '4', '5', '6', '7', '8', '9', 'a', 'b', 'c', 'd', 'e', 'f'
];

fn is_radix_16_char(input: &char) -> bool {
    RADIX_16_CHARS.contains(input)
}

const BIT_CHARS: [char; 3] = [
    '0', '1', '?',
];

fn is_bit_char(input: &char) -> bool {
    BIT_CHARS.contains(input)
}

#[cfg(test)]
#[allow(clippy::bool_assert_comparison)]
mod tests {
    use crate::pattern::tokenizer::{Token, tokenize_pattern};

    #[test]
    fn tokenize_works() {
        let mut tokens = tokenize_pattern("00 [11 ?? ??] m10101?1? EF").unwrap().into_iter();

        assert_eq!(tokens.next(), Some(Token::ByteValue(0x00, 0xFF)));
        assert_eq!(tokens.next(), Some(Token::CaptureGroupOpen));
        assert_eq!(tokens.next(), Some(Token::ByteValue(0x11, 0xFF)));
        assert_eq!(tokens.next(), Some(Token::ByteValue(0x00, 0x00)));
        assert_eq!(tokens.next(), Some(Token::ByteValue(0x00, 0x00)));
        assert_eq!(tokens.next(), Some(Token::CaptureGroupClose));
        assert_eq!(tokens.next(), Some(Token::ByteValue(0xAA, 0xFA)));
        assert_eq!(tokens.next(), Some(Token::ByteValue(0xEF, 0xFF)));
        assert_eq!(tokens.next(), None);
    }
}
