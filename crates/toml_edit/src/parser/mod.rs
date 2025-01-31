#![allow(clippy::type_complexity)]

#[macro_use]
pub(crate) mod macros;

pub(crate) mod array;
pub(crate) mod datetime;
pub(crate) mod document;
pub(crate) mod errors;
pub(crate) mod inline_table;
pub(crate) mod key;
pub(crate) mod numbers;
pub(crate) mod state;
pub(crate) mod strings;
pub(crate) mod table;
pub(crate) mod trivia;
pub(crate) mod value;

pub use errors::TomlError;

pub(crate) fn parse_document(raw: &str) -> Result<crate::Document, TomlError> {
    use prelude::*;

    let b = new_input(raw);
    document::document
        .parse(b)
        .finish()
        .map_err(|e| TomlError::new(e, b))
}

pub(crate) fn parse_key(raw: &str) -> Result<crate::Key, TomlError> {
    use prelude::*;

    let b = new_input(raw);
    let result = key::simple_key.parse(b).finish();
    match result {
        Ok((raw, key)) => {
            Ok(crate::Key::new(key).with_repr_unchecked(crate::Repr::new_unchecked(raw)))
        }
        Err(e) => Err(TomlError::new(e, b)),
    }
}

pub(crate) fn parse_key_path(raw: &str) -> Result<Vec<crate::Key>, TomlError> {
    use prelude::*;

    let b = new_input(raw);
    let result = key::key.parse(b).finish();
    match result {
        Ok(keys) => Ok(keys),
        Err(e) => Err(TomlError::new(e, b)),
    }
}

pub(crate) fn parse_value(raw: &str) -> Result<crate::Value, TomlError> {
    use prelude::*;

    let b = new_input(raw);
    let parsed = value::value(RecursionCheck::default()).parse(b).finish();
    match parsed {
        Ok(mut value) => {
            // Only take the repr and not decor, as its probably not intended
            value.decor_mut().clear();
            Ok(value)
        }
        Err(e) => Err(TomlError::new(e, b)),
    }
}

pub(crate) mod prelude {
    pub(crate) use super::errors::Context;
    pub(crate) use super::errors::ParserError;
    pub(crate) use super::errors::ParserValue;
    pub(crate) use nom8::IResult;
    pub(crate) use nom8::Parser as _;

    pub(crate) use nom8::FinishIResult as _;

    pub(crate) type Input<'b> = &'b [u8];

    pub(crate) fn new_input(s: &str) -> Input<'_> {
        s.as_bytes()
    }

    pub(crate) fn ok_error<I, O, E>(res: IResult<I, O, E>) -> Result<Option<(I, O)>, nom8::Err<E>> {
        match res {
            Ok(ok) => Ok(Some(ok)),
            Err(nom8::Err::Error(_)) => Ok(None),
            Err(err) => Err(err),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn trace<I: std::fmt::Debug, O: std::fmt::Debug, E: std::fmt::Debug>(
        context: impl std::fmt::Display,
        mut parser: impl nom8::Parser<I, O, E>,
    ) -> impl FnMut(I) -> IResult<I, O, E> {
        static DEPTH: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);
        move |input: I| {
            let depth = DEPTH.fetch_add(1, std::sync::atomic::Ordering::SeqCst) * 2;
            eprintln!("{:depth$}--> {} {:?}", "", context, input);
            match parser.parse(input) {
                Ok((i, o)) => {
                    DEPTH.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                    eprintln!("{:depth$}<-- {} {:?}", "", context, i);
                    Ok((i, o))
                }
                Err(err) => {
                    DEPTH.fetch_sub(1, std::sync::atomic::Ordering::SeqCst);
                    eprintln!("{:depth$}<-- {} {:?}", "", context, err);
                    Err(err)
                }
            }
        }
    }

    #[cfg(not(feature = "unbounded"))]
    #[derive(Copy, Clone, Debug, Default)]
    pub(crate) struct RecursionCheck {
        current: usize,
    }

    #[cfg(not(feature = "unbounded"))]
    impl RecursionCheck {
        pub(crate) fn check_depth(depth: usize) -> Result<(), super::errors::CustomError> {
            if depth < 128 {
                Ok(())
            } else {
                Err(super::errors::CustomError::RecursionLimitExceeded)
            }
        }

        pub(crate) fn recursing(
            mut self,
            input: Input<'_>,
        ) -> Result<Self, nom8::Err<ParserError<'_>>> {
            self.current += 1;
            if self.current < 128 {
                Ok(self)
            } else {
                Err(nom8::Err::Error(
                    nom8::error::FromExternalError::from_external_error(
                        input,
                        nom8::error::ErrorKind::Eof,
                        super::errors::CustomError::RecursionLimitExceeded,
                    ),
                ))
            }
        }
    }

    #[cfg(feature = "unbounded")]
    #[derive(Copy, Clone, Debug, Default)]
    pub(crate) struct RecursionCheck {}

    #[cfg(feature = "unbounded")]
    impl RecursionCheck {
        pub(crate) fn check_depth(_depth: usize) -> Result<(), super::errors::CustomError> {
            Ok(())
        }

        pub(crate) fn recursing(
            self,
            _input: Input<'_>,
        ) -> Result<Self, nom8::Err<ParserError<'_>>> {
            Ok(self)
        }
    }
}
