use snapbox::assert_eq;
use toml_edit::{Document, Key, Value};

macro_rules! parse {
    ($s:expr, $ty:ty) => {{
        let v = $s.parse::<$ty>();
        assert!(v.is_ok(), "Failed with {}", v.unwrap_err());
        v.unwrap()
    }};
}

macro_rules! parse_value {
    ($s:expr) => {
        parse!($s, Value)
    };
}

macro_rules! test_key {
    ($s:expr, $expected:expr) => {{
        let key = parse!($s, Key);
        assert_eq!(key.get(), $expected, "");
    }};
}

#[test]
fn test_key_from_str() {
    test_key!("a", "a");
    test_key!(r#"'hello key'"#, "hello key");
    test_key!(
        r#""Jos\u00E9\U000A0000\n\t\r\f\b\"""#,
        "Jos\u{00E9}\u{A0000}\n\t\r\u{c}\u{8}\""
    );
    test_key!("\"\"", "");
    test_key!("\"'hello key'bla\"", "'hello key'bla");
    test_key!(
        "'C:\\Users\\appveyor\\AppData\\Local\\Temp\\1\\cargo-edit-test.YizxPxxElXn9'",
        "C:\\Users\\appveyor\\AppData\\Local\\Temp\\1\\cargo-edit-test.YizxPxxElXn9"
    );
}

#[test]
fn test_value_from_str() {
    assert!(parse_value!("1979-05-27T00:32:00.999999-07:00").is_datetime());
    assert!(parse_value!("1979-05-27T00:32:00.999999Z").is_datetime());
    assert!(parse_value!("1979-05-27T00:32:00.999999").is_datetime());
    assert!(parse_value!("1979-05-27T00:32:00").is_datetime());
    assert!(parse_value!("1979-05-27").is_datetime());
    assert!(parse_value!("00:32:00").is_datetime());
    assert!(parse_value!("-239").is_integer());
    assert!(parse_value!("1e200").is_float());
    assert!(parse_value!("9_224_617.445_991_228_313").is_float());
    assert!(parse_value!(r#""basic string\nJos\u00E9\n""#).is_str());
    assert!(parse_value!(
        r#""""
multiline basic string
""""#
    )
    .is_str());
    assert!(parse_value!(r#"'literal string\ \'"#).is_str());
    assert!(parse_value!(
        r#"'''multiline
literal \ \
string'''"#
    )
    .is_str());
    assert!(parse_value!(r#"{ hello = "world", a = 1}"#).is_inline_table());
    assert!(
        parse_value!(r#"[ { x = 1, a = "2" }, {a = "a",b = "b",     c =    "c"} ]"#).is_array()
    );
    let wp = "C:\\Users\\appveyor\\AppData\\Local\\Temp\\1\\cargo-edit-test.YizxPxxElXn9";
    let lwp = "'C:\\Users\\appveyor\\AppData\\Local\\Temp\\1\\cargo-edit-test.YizxPxxElXn9'";
    assert_eq!(Value::from(wp).as_str(), parse_value!(lwp).as_str());
    assert!(parse_value!(r#""\\\"\b\f\n\r\t\u00E9\U000A0000""#).is_str());
}

#[test]
fn test_key_unification() {
    let toml = r#"
[a]
[a.'b'.c]
[a."b".c.e]
[a.b.c.d]
"#;
    let expected = r#"
[a]
[a.'b'.c]
[a.'b'.c.e]
[a.'b'.c.d]
"#;
    let doc = toml.parse::<Document>();
    assert!(doc.is_ok());
    let doc = doc.unwrap();

    assert_eq(doc.to_string(), expected);
}
