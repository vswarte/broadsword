use std::ops::Range;
use crate::pattern::Pattern;
use crate::pattern::tokenizer;

#[derive(Debug)]
pub enum ParserError {
    Tokenizer(tokenizer::TokenizationError),
    CaptureGroupAlreadyOpened,
    CaptureGroupNotOpened,
}

pub(crate) fn parse_pattern(input: &str) -> Result<Pattern, ParserError> {
    let mut bytes = Vec::new();
    let mut mask = Vec::new();
    let mut capture_groups = Vec::<Range<usize>>::new();
    let mut current_capture_group_start = None as Option<usize>;

    for token in tokenizer::tokenize_pattern(input).map_err(ParserError::Tokenizer)?.iter() {
        match token {
            tokenizer::Token::ByteValue(b) => {
                mask.push(true);
                bytes.push(*b);
            },
            tokenizer::Token::ByteWildcard => {
                mask.push(false);
                bytes.push(0);
            },
            tokenizer::Token::CaptureGroupOpen => {
                match current_capture_group_start {
                    None => current_capture_group_start = Some(bytes.len()),

                    // Ensure we're not already in a group
                    Some(_) => return Err(ParserError::CaptureGroupAlreadyOpened),
                }
            },
            tokenizer::Token::CaptureGroupClose => {
                match current_capture_group_start.take() {
                    Some(start) => capture_groups.push(Range {start, end: bytes.len()}),

                    // Bail if capture group was never opened
                    None => return Err(ParserError::CaptureGroupNotOpened),
                }
            },
        }
    }

    Ok(Pattern {
        length: bytes.len(),
        bytes,
        mask,
        capture_groups,
        offset: None,
    })
}
