use std::ops::RangeInclusive;

use nom8::bytes::any;
use nom8::bytes::take_while1;
use nom8::combinator::peek;
use nom8::multi::separated_list1;

use crate::key::Key;
use crate::parser::errors::CustomError;
use crate::parser::prelude::*;
use crate::parser::strings::{basic_string, literal_string};
use crate::parser::trivia::{from_utf8_unchecked, ws};
use crate::repr::{Decor, Repr};
use crate::InternalString;

// key = simple-key / dotted-key
// dotted-key = simple-key 1*( dot-sep simple-key )
pub(crate) fn key(input: Input<'_>) -> IResult<Input<'_>, Vec<Key>, ParserError<'_>> {
    separated_list1(
        DOT_SEP,
        (ws, simple_key, ws).map(|(pre, (raw, key), suffix)| {
            Key::new(key)
                .with_repr_unchecked(Repr::new_unchecked(raw))
                .with_decor(Decor::new(pre, suffix))
        }),
    )
    .context(Context::Expression("key"))
    .map_res(|k| {
        // Inserting the key will require recursion down the line
        RecursionCheck::check_depth(k.len())?;
        Ok::<_, CustomError>(k)
    })
    .parse(input)
}

// simple-key = quoted-key / unquoted-key
// quoted-key = basic-string / literal-string
pub(crate) fn simple_key(
    input: Input<'_>,
) -> IResult<Input<'_>, (&str, InternalString), ParserError<'_>> {
    dispatch! {peek(any);
        crate::parser::strings::QUOTATION_MARK => basic_string
            .map(|s: std::borrow::Cow<'_, str>| s.as_ref().into()),
        crate::parser::strings::APOSTROPHE => literal_string.map(|s: &str| s.into()),
        _ => unquoted_key.map(|s: &str| s.into()),
    }
        .with_recognized()
        .map(|(k, b)| {
            let s = unsafe { from_utf8_unchecked(b, "If `quoted_key` or `unquoted_key` are valid, then their `recognize`d value is valid") };
            (s, k)
        })
        .parse(input)
}

// unquoted-key = 1*( ALPHA / DIGIT / %x2D / %x5F ) ; A-Z / a-z / 0-9 / - / _
fn unquoted_key(input: Input<'_>) -> IResult<Input<'_>, &str, ParserError<'_>> {
    take_while1(UNQUOTED_CHAR)
        .map(|b| unsafe { from_utf8_unchecked(b, "`is_unquoted_char` filters out on-ASCII") })
        .parse(input)
}

pub(crate) fn is_unquoted_char(c: u8) -> bool {
    use nom8::input::FindToken;
    UNQUOTED_CHAR.find_token(c)
}

const UNQUOTED_CHAR: (
    RangeInclusive<u8>,
    RangeInclusive<u8>,
    RangeInclusive<u8>,
    u8,
    u8,
) = (b'A'..=b'Z', b'a'..=b'z', b'0'..=b'9', b'-', b'_');

// dot-sep   = ws %x2E ws  ; . Period
const DOT_SEP: u8 = b'.';

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn keys() {
        let cases = [
            ("a", "a"),
            (r#""hello\n ""#, "hello\n "),
            (r#"'hello\n '"#, "hello\\n "),
        ];

        for (input, expected) in cases {
            let parsed = simple_key.parse(new_input(input)).finish();
            assert_eq!(parsed, Ok((input, expected.into())), "Parsing {input:?}");
        }
    }
}
