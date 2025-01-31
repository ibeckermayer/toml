use std::ops::RangeInclusive;

use crate::parser::errors::CustomError;
use crate::parser::prelude::*;
use crate::parser::trivia::from_utf8_unchecked;

use nom8::branch::alt;
use nom8::bytes::one_of;
use nom8::bytes::take_while_m_n;
use nom8::combinator::cut;
use nom8::combinator::opt;
use nom8::sequence::preceded;
use toml_datetime::*;

// ;; Date and Time (as defined in RFC 3339)

// date-time = offset-date-time / local-date-time / local-date / local-time
// offset-date-time = full-date time-delim full-time
// local-date-time = full-date time-delim partial-time
// local-date = full-date
// local-time = partial-time
// full-time = partial-time time-offset
pub(crate) fn date_time(input: Input<'_>) -> IResult<Input<'_>, Datetime, ParserError<'_>> {
    alt((
        (full_date, opt((time_delim, partial_time, opt(time_offset))))
            .map(|(date, opt)| {
                match opt {
                    // Offset Date-Time
                    Some((_, time, offset)) => Datetime {
                        date: Some(date),
                        time: Some(time),
                        offset,
                    },
                    // Local Date
                    None => Datetime {
                        date: Some(date),
                        time: None,
                        offset: None,
                    },
                }
            })
            .context(Context::Expression("date-time")),
        partial_time
            .map(|t| t.into())
            .context(Context::Expression("time")),
    ))
    .parse(input)
}

// full-date      = date-fullyear "-" date-month "-" date-mday
pub(crate) fn full_date(input: Input<'_>) -> IResult<Input<'_>, Date, ParserError<'_>> {
    (date_fullyear, b'-', cut((date_month, b'-', date_mday)))
        .map(|(year, _, (month, _, day))| Date { year, month, day })
        .parse(input)
}

// partial-time   = time-hour ":" time-minute ":" time-second [time-secfrac]
pub(crate) fn partial_time(input: Input<'_>) -> IResult<Input<'_>, Time, ParserError<'_>> {
    (
        time_hour,
        b':',
        cut((time_minute, b':', time_second, opt(time_secfrac))),
    )
        .map(|(hour, _, (minute, _, second, nanosecond))| Time {
            hour,
            minute,
            second,
            nanosecond: nanosecond.unwrap_or_default(),
        })
        .parse(input)
}

// time-offset    = "Z" / time-numoffset
// time-numoffset = ( "+" / "-" ) time-hour ":" time-minute
pub(crate) fn time_offset(input: Input<'_>) -> IResult<Input<'_>, Offset, ParserError<'_>> {
    alt((
        one_of((b'Z', b'z')).value(Offset::Z),
        (one_of((b'+', b'-')), cut((time_hour, b':', time_minute))).map(
            |(sign, (hours, _, minutes))| {
                let hours = hours as i8;
                let hours = match sign {
                    b'+' => hours,
                    b'-' => -hours,
                    _ => unreachable!("Parser prevents this"),
                };
                Offset::Custom { hours, minutes }
            },
        ),
    ))
    .context(Context::Expression("time offset"))
    .parse(input)
}

// date-fullyear  = 4DIGIT
pub(crate) fn date_fullyear(input: Input<'_>) -> IResult<Input<'_>, u16, ParserError<'_>> {
    unsigned_digits::<4, 4>
        .map(|s: &str| s.parse::<u16>().expect("4DIGIT should match u8"))
        .parse(input)
}

// date-month     = 2DIGIT  ; 01-12
pub(crate) fn date_month(input: Input<'_>) -> IResult<Input<'_>, u8, ParserError<'_>> {
    unsigned_digits::<2, 2>
        .map_res(|s: &str| {
            let d = s.parse::<u8>().expect("2DIGIT should match u8");
            if (1..=12).contains(&d) {
                Ok(d)
            } else {
                Err(CustomError::OutOfRange)
            }
        })
        .parse(input)
}

// date-mday      = 2DIGIT  ; 01-28, 01-29, 01-30, 01-31 based on month/year
pub(crate) fn date_mday(input: Input<'_>) -> IResult<Input<'_>, u8, ParserError<'_>> {
    unsigned_digits::<2, 2>
        .map_res(|s: &str| {
            let d = s.parse::<u8>().expect("2DIGIT should match u8");
            if (1..=31).contains(&d) {
                Ok(d)
            } else {
                Err(CustomError::OutOfRange)
            }
        })
        .parse(input)
}

// time-delim     = "T" / %x20 ; T, t, or space
pub(crate) fn time_delim(input: Input<'_>) -> IResult<Input<'_>, u8, ParserError<'_>> {
    one_of(TIME_DELIM).parse(input)
}

const TIME_DELIM: (u8, u8, u8) = (b'T', b't', b' ');

// time-hour      = 2DIGIT  ; 00-23
pub(crate) fn time_hour(input: Input<'_>) -> IResult<Input<'_>, u8, ParserError<'_>> {
    unsigned_digits::<2, 2>
        .map_res(|s: &str| {
            let d = s.parse::<u8>().expect("2DIGIT should match u8");
            if (0..=23).contains(&d) {
                Ok(d)
            } else {
                Err(CustomError::OutOfRange)
            }
        })
        .parse(input)
}

// time-minute    = 2DIGIT  ; 00-59
pub(crate) fn time_minute(input: Input<'_>) -> IResult<Input<'_>, u8, ParserError<'_>> {
    unsigned_digits::<2, 2>
        .map_res(|s: &str| {
            let d = s.parse::<u8>().expect("2DIGIT should match u8");
            if (0..=59).contains(&d) {
                Ok(d)
            } else {
                Err(CustomError::OutOfRange)
            }
        })
        .parse(input)
}

// time-second    = 2DIGIT  ; 00-58, 00-59, 00-60 based on leap second rules
pub(crate) fn time_second(input: Input<'_>) -> IResult<Input<'_>, u8, ParserError<'_>> {
    unsigned_digits::<2, 2>
        .map_res(|s: &str| {
            let d = s.parse::<u8>().expect("2DIGIT should match u8");
            if (0..=60).contains(&d) {
                Ok(d)
            } else {
                Err(CustomError::OutOfRange)
            }
        })
        .parse(input)
}

// time-secfrac   = "." 1*DIGIT
pub(crate) fn time_secfrac(input: Input<'_>) -> IResult<Input<'_>, u32, ParserError<'_>> {
    static SCALE: [u32; 10] = [
        0,
        100_000_000,
        10_000_000,
        1_000_000,
        100_000,
        10_000,
        1_000,
        100,
        10,
        1,
    ];
    const INF: usize = usize::MAX;
    preceded(b'.', unsigned_digits::<1, INF>)
        .map_res(|mut repr: &str| -> Result<u32, CustomError> {
            let max_digits = SCALE.len() - 1;
            if max_digits < repr.len() {
                // Millisecond precision is required. Further precision of fractional seconds is
                // implementation-specific. If the value contains greater precision than the
                // implementation can support, the additional precision must be truncated, not rounded.
                repr = &repr[0..max_digits];
            }

            let v = repr.parse::<u32>().map_err(|_| CustomError::OutOfRange)?;
            let num_digits = repr.len();

            // scale the number accordingly.
            let scale = SCALE.get(num_digits).ok_or(CustomError::OutOfRange)?;
            let v = v.checked_mul(*scale).ok_or(CustomError::OutOfRange)?;
            Ok(v)
        })
        .parse(input)
}

pub(crate) fn unsigned_digits<const MIN: usize, const MAX: usize>(
    input: Input<'_>,
) -> IResult<Input<'_>, &str, ParserError<'_>> {
    take_while_m_n(MIN, MAX, DIGIT)
        .map(|b: &[u8]| unsafe { from_utf8_unchecked(b, "`is_ascii_digit` filters out on-ASCII") })
        .parse(input)
}

// DIGIT = %x30-39 ; 0-9
const DIGIT: RangeInclusive<u8> = b'0'..=b'9';

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn offset_date_time() {
        let inputs = [
            "1979-05-27T07:32:00Z",
            "1979-05-27T00:32:00-07:00",
            "1979-05-27T00:32:00.999999-07:00",
        ];
        for input in inputs {
            date_time.parse(new_input(input)).finish().unwrap();
        }
    }

    #[test]
    fn local_date_time() {
        let inputs = ["1979-05-27T07:32:00", "1979-05-27T00:32:00.999999"];
        for input in inputs {
            date_time.parse(new_input(input)).finish().unwrap();
        }
    }

    #[test]
    fn local_date() {
        let inputs = ["1979-05-27", "2017-07-20"];
        for input in inputs {
            date_time.parse(new_input(input)).finish().unwrap();
        }
    }

    #[test]
    fn local_time() {
        let inputs = ["07:32:00", "00:32:00.999999"];
        for input in inputs {
            date_time.parse(new_input(input)).finish().unwrap();
        }
    }

    #[test]
    fn time_fraction_truncated() {
        let input = "1987-07-05T17:45:00.123456789012345Z";
        date_time.parse(new_input(input)).finish().unwrap();
    }
}
