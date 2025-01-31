use nom8::bytes::one_of;
use nom8::combinator::cut;
use nom8::multi::separated_list0;
use nom8::sequence::delimited;

use crate::key::Key;
use crate::parser::errors::CustomError;
use crate::parser::key::key;
use crate::parser::prelude::*;
use crate::parser::trivia::ws;
use crate::parser::value::value;
use crate::table::TableKeyValue;
use crate::{InlineTable, InternalString, Item, Value};

use indexmap::map::Entry;

// ;; Inline Table

// inline-table = inline-table-open inline-table-keyvals inline-table-close
pub(crate) fn inline_table(
    check: RecursionCheck,
) -> impl FnMut(Input<'_>) -> IResult<Input<'_>, InlineTable, ParserError<'_>> {
    move |input| {
        delimited(
            INLINE_TABLE_OPEN,
            cut(inline_table_keyvals(check).map_res(|(kv, p)| table_from_pairs(kv, p))),
            cut(INLINE_TABLE_CLOSE)
                .context(Context::Expression("inline table"))
                .context(Context::Expected(ParserValue::CharLiteral('}'))),
        )
        .parse(input)
    }
}

fn table_from_pairs(
    v: Vec<(Vec<Key>, TableKeyValue)>,
    preamble: &str,
) -> Result<InlineTable, CustomError> {
    let mut root = InlineTable::new();
    root.preamble = preamble.into();
    // Assuming almost all pairs will be directly in `root`
    root.items.reserve(v.len());

    for (path, kv) in v {
        let table = descend_path(&mut root, &path)?;
        let key: InternalString = kv.key.get_internal().into();
        match table.items.entry(key) {
            Entry::Vacant(o) => {
                o.insert(kv);
            }
            Entry::Occupied(o) => {
                return Err(CustomError::DuplicateKey {
                    key: o.key().as_str().into(),
                    table: None,
                });
            }
        }
    }
    Ok(root)
}

fn descend_path<'a>(
    mut table: &'a mut InlineTable,
    path: &'a [Key],
) -> Result<&'a mut InlineTable, CustomError> {
    for (i, key) in path.iter().enumerate() {
        let entry = table.entry_format(key).or_insert_with(|| {
            let mut new_table = InlineTable::new();
            new_table.set_dotted(true);

            Value::InlineTable(new_table)
        });
        match *entry {
            Value::InlineTable(ref mut sweet_child_of_mine) => {
                table = sweet_child_of_mine;
            }
            ref v => {
                return Err(CustomError::extend_wrong_type(path, i, v.type_name()));
            }
        }
    }
    Ok(table)
}

// inline-table-open  = %x7B ws     ; {
pub(crate) const INLINE_TABLE_OPEN: u8 = b'{';
// inline-table-close = ws %x7D     ; }
const INLINE_TABLE_CLOSE: u8 = b'}';
// inline-table-sep   = ws %x2C ws  ; , Comma
const INLINE_TABLE_SEP: u8 = b',';
// keyval-sep = ws %x3D ws ; =
pub(crate) const KEYVAL_SEP: u8 = b'=';

// inline-table-keyvals = [ inline-table-keyvals-non-empty ]
// inline-table-keyvals-non-empty =
// ( key keyval-sep val inline-table-sep inline-table-keyvals-non-empty ) /
// ( key keyval-sep val )

fn inline_table_keyvals(
    check: RecursionCheck,
) -> impl FnMut(Input<'_>) -> IResult<Input<'_>, (Vec<(Vec<Key>, TableKeyValue)>, &str), ParserError<'_>>
{
    move |input| {
        let check = check.recursing(input)?;
        (separated_list0(INLINE_TABLE_SEP, keyval(check)), ws).parse(input)
    }
}

fn keyval(
    check: RecursionCheck,
) -> impl FnMut(Input<'_>) -> IResult<Input<'_>, (Vec<Key>, TableKeyValue), ParserError<'_>> {
    move |input| {
        (
            key,
            cut((
                one_of(KEYVAL_SEP)
                    .context(Context::Expected(ParserValue::CharLiteral('.')))
                    .context(Context::Expected(ParserValue::CharLiteral('='))),
                (ws, value(check), ws),
            )),
        )
            .map(|(key, (_, v))| {
                let mut path = key;
                let key = path.pop().expect("grammar ensures at least 1");

                let (pre, v, suf) = v;
                let v = v.decorated(pre, suf);
                (
                    path,
                    TableKeyValue {
                        key,
                        value: Item::Value(v),
                    },
                )
            })
            .parse(input)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn inline_tables() {
        let inputs = [
            r#"{}"#,
            r#"{   }"#,
            r#"{a = 1e165}"#,
            r#"{ hello = "world", a = 1}"#,
            r#"{ hello.world = "a" }"#,
        ];
        for input in inputs {
            let parsed = inline_table(Default::default())
                .parse(new_input(input))
                .finish();
            assert_eq!(parsed.map(|a| a.to_string()), Ok(input.to_owned()));
        }
        let invalid_inputs = [r#"{a = 1e165"#, r#"{ hello = "world", a = 2, hello = 1}"#];
        for input in invalid_inputs {
            let parsed = inline_table(Default::default())
                .parse(new_input(input))
                .finish();
            assert!(parsed.is_err());
        }
    }
}
