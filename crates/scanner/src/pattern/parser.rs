use std::ops::Range;
use crate::pattern::Pattern;
use crate::pattern::tokenizer;

use super::tokenizer::TokenizerFn;

#[derive(Debug)]
pub enum ParserError {
    Tokenizer(tokenizer::TokenizationError),
    CaptureGroupAlreadyOpened,
    CaptureGroupNotOpened,
    CaptureGroupNotClosed,
}

pub type ParserResult = Result<Pattern, ParserError>;

pub(crate) fn parse_byte_pattern(input: &str) -> ParserResult {
    parse_pattern(input, tokenizer::tokenize_byte_pattern)
}

pub(crate) fn parse_bit_pattern(input: &str) -> ParserResult {
    parse_pattern(input, tokenizer::tokenize_byte_pattern)
}

fn parse_pattern(
    input: &str,
    tokenizer: TokenizerFn,
) -> ParserResult {
    let mut bytes = Vec::new();
    let mut mask = Vec::new();
    let mut capture_groups = Vec::<Range<usize>>::new();
    let mut current_capture_group_start = None as Option<usize>;

    for token in tokenizer(input)
        .map_err(ParserError::Tokenizer)?
        .iter() {

        match token {
            tokenizer::Token::ByteValue(b, m) => {
                bytes.push(*b);
                mask.push(*m);
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

    // Guard against unclosed capture groups as otherwise it'll silently not capture the input.
    if current_capture_group_start.is_some() {
        return Err(ParserError::CaptureGroupNotClosed)
    }

    Ok(Pattern {
        length: bytes.len(),
        bytes,
        mask,
        capture_groups,
    })
}

// pub(crate) fn parse_pattern(input: &str) -> Result<Pattern, ParserError> {
//     let mut bytes = Vec::new();
//     let mut mask = Vec::new();
//     let mut capture_groups = Vec::<Range<usize>>::new();
//     let mut current_capture_group_start = None as Option<usize>;
//
//     for token in tokenizer::tokenize_pattern(input)
//         .map_err(ParserError::Tokenizer)?
//         .iter() {
//
//         match token {
//             tokenizer::Token::ByteValue(b, m) => {
//                 bytes.push(*b);
//                 mask.push(*m);
//             },
//             tokenizer::Token::CaptureGroupOpen => {
//                 match current_capture_group_start {
//                     None => current_capture_group_start = Some(bytes.len()),
//
//                     // Ensure we're not already in a group
//                     Some(_) => return Err(ParserError::CaptureGroupAlreadyOpened),
//                 }
//             },
//             tokenizer::Token::CaptureGroupClose => {
//                 match current_capture_group_start.take() {
//                     Some(start) => capture_groups.push(Range {start, end: bytes.len()}),
//
//                     // Bail if capture group was never opened
//                     None => return Err(ParserError::CaptureGroupNotOpened),
//                 }
//             },
//         }
//     }
//
//     // Guard against unclosed capture groups as otherwise it'll silently not capture the input.
//     if current_capture_group_start.is_some() {
//         return Err(ParserError::CaptureGroupNotClosed)
//     }
//
//     Ok(Pattern {
//         length: bytes.len(),
//         bytes,
//         mask,
//         capture_groups,
//     })
// }
