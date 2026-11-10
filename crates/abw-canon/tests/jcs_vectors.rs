//! T4: one canonical form (RFC 8785 JCS) and one hash format (`sha256:<64 hex>`).
//!
//! Vectors are from RFC 8785 and target-system-design §3.6.2. Expected hashes were computed
//! independently of this crate (`sha256sum` over the expected canonical bytes; V4 also
//! matches Appendix D case 15).

use abw_canon::{Sha256Ref, jcs, parse, sha256_jcs, sha256_raw};
use serde_json::json;

fn canon(text: &str) -> String {
    let value = match parse(text) {
        Ok(v) => v,
        Err(e) => panic!("parse({text:?}) failed: {e:?}"),
    };
    match jcs(&value) {
        Ok(bytes) => match String::from_utf8(bytes) {
            Ok(s) => s,
            Err(e) => panic!("JCS output is not UTF-8: {e}"),
        },
        Err(e) => panic!("jcs({text:?}) failed: {e:?}"),
    }
}

fn sha_of(text: &str) -> String {
    match parse(text).map(|v| sha256_jcs(&v)) {
        Ok(Ok(s)) => s.as_str().to_string(),
        other => panic!("hashing {text:?} failed: {other:?}"),
    }
}

const V1_IN: &str = r#"{
  "numbers": [333333333.33333329, 1E30, 4.50, 2e-3, 0.000000000000000000000000001],
  "string": "\u20ac$\u000F\u000aA'\u0042\u0022\u005c\\\"\/",
  "literals": [null, true, false]
}"#;
const V1_OUT: &str = "{\"literals\":[null,true,false],\"numbers\":[333333333.3333333,1e+30,4.5,0.002,1e-27],\"string\":\"\u{20ac}$\\u000f\\nA'B\\\"\\\\\\\\\\\"/\"}";

// @req T4
#[test]
fn jcs_v1_rfc8785_sample() {
    assert_eq!(canon(V1_IN), V1_OUT);
    assert_eq!(
        sha_of(V1_IN),
        "sha256:2d5e01a318d0f0879ab568c4be289c8b1f64ef8921a53c6277d5e069978baacb"
    );
}

// @req T4
#[test]
fn jcs_v2_rfc8785_key_sorting_by_utf16_code_units() {
    let input = r#"{
      "\u20ac": "Euro Sign",
      "\r": "Carriage Return",
      "\ufb33": "Hebrew Letter Dalet With Dagesh",
      "1": "One",
      "\ud83d\ude00": "Emoji: Grinning Face",
      "\u0080": "Control",
      "\u00f6": "Latin Small Letter O With Diaeresis"
    }"#;
    let expected = "{\"\\r\":\"Carriage Return\",\"1\":\"One\",\"\u{80}\":\"Control\",\"\u{f6}\":\"Latin Small Letter O With Diaeresis\",\"\u{20ac}\":\"Euro Sign\",\"\u{1F600}\":\"Emoji: Grinning Face\",\"\u{fb33}\":\"Hebrew Letter Dalet With Dagesh\"}";
    assert_eq!(canon(input), expected);
    assert_eq!(
        sha_of(input),
        "sha256:5e321556d22018a9656991a9e94f77ec175fa193e52a2429d312f8419ec8b08c"
    );
}

// @req T4
#[test]
fn jcs_utf16_order_differs_from_utf8_order() {
    // UTF-16: U+10000 (D800 DC00) sorts before U+FFFF. UTF-8 bytewise order is the reverse.
    let v = json!({"\u{FFFF}": 1, "\u{10000}": 2});
    let out = match jcs(&v) {
        Ok(b) => b,
        Err(e) => panic!("{e:?}"),
    };
    assert_eq!(out, "{\"\u{10000}\":2,\"\u{FFFF}\":1}".as_bytes());
}

// @req T4
#[test]
fn jcs_v3_empty_object() {
    assert_eq!(canon("{}"), "{}");
    assert_eq!(
        sha_of("{}"),
        "sha256:44136fa355b3678a1146ad16f7e8649e94fb4fc21fe77e8310c060f61caaff8a"
    );
}

// @req T4
#[test]
fn jcs_v4_raw_utf8_artifact_hash() {
    assert_eq!(
        sha256_raw(b"hello, ABW\n").as_str(),
        "sha256:0d4251ddf30651714bd5d954aac1e8545862c1311d4b2858aa8bb49c469c712a"
    );
}

// @req T4
#[test]
fn jcs_v6_numbers_use_the_ecmascript_algorithm() {
    assert_eq!(
        canon("[0.0, -0.0, 1.0, 1e21, 1e20, 5e-7, 1e-6, 0.1, 9007199254740991]"),
        "[0,0,1,1e+21,100000000000000000000,5e-7,0.000001,0.1,9007199254740991]"
    );
    assert_eq!(
        canon("[1e-7, -1.5, 0.5, 1.5e300, 5e-324, 1.7976931348623157e308, 1.2345678901234568e20]"),
        "[1e-7,-1.5,0.5,1.5e+300,5e-324,1.7976931348623157e+308,123456789012345680000]"
    );
    assert_eq!(
        canon("[-9007199254740991, 100, 4.50, 2e-3]"),
        "[-9007199254740991,100,4.5,0.002]"
    );
}

// @req T4
#[test]
fn jcs_v7_integers_beyond_2_pow_53_are_rejected() {
    assert!(parse("9007199254740992").is_err());
    assert!(parse("-9007199254740992").is_err());
    assert!(parse("[1, 18446744073709551615]").is_err());
    assert!(jcs(&json!(9_007_199_254_740_992_u64)).is_err());
    assert!(jcs(&json!({"a": [-9_007_199_254_740_993_i64]})).is_err());
}

// @req T4
#[test]
fn jcs_v8_duplicate_member_names_are_rejected() {
    assert!(parse(r#"{"a":1,"a":2}"#).is_err());
    assert!(parse(r#"{"x":{"b":1,"b":1}}"#).is_err());
    assert!(
        parse(r#"{"\u0061":1,"a":2}"#).is_err(),
        "escapes are decoded before comparing names"
    );
}

// @req T4
#[test]
fn jcs_v9_and_string_escapes() {
    assert_eq!(canon(r#""a\u0000b""#), "\"a\\u0000b\"");
    assert_eq!(
        canon(r#""\u001f\b\f\n\r\t\"\\\/\u007f\u00e9""#),
        "\"\\u001f\\b\\f\\n\\r\\t\\\"\\\\/\u{7f}\u{e9}\""
    );
}

// @req T4
#[test]
fn jcs_rejects_lone_surrogates() {
    assert!(parse(r#""\ud800""#).is_err());
    assert!(parse(r#""\udc00x""#).is_err());
}

// @req T4
#[test]
fn jcs_is_invariant_under_key_order_and_number_spelling() {
    // Appendix D case 29: jsonb reorders keys and respells numbers; JCS must not care.
    let a = canon(r#"{"b":1,"a":[1.0,"x"]}"#);
    let b = canon(r#"{ "a" : [ 1 , "x" ] , "b" : 1.000 }"#);
    assert_eq!(a, b);
    assert_eq!(a, r#"{"a":[1,"x"],"b":1}"#);
}

// @req T4
#[test]
fn jcs_is_idempotent() {
    for text in [
        V1_IN,
        r#"{"z":[{"y":0.1,"x":-0}],"a":"\u00e9"}"#,
        "[]",
        "null",
        "\"\"",
    ] {
        let once = canon(text);
        assert_eq!(canon(&once), once, "{text}");
    }
}

// @req T4
#[test]
fn hash_format_is_sha256_prefix_and_64_lowercase_hex() {
    let h = sha256_raw(b"");
    assert_eq!(
        h.as_str(),
        "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
    );
    assert_eq!(h.as_str().len(), 71);
    assert_eq!(h.to_string(), h.as_str());

    assert!(Sha256Ref::parse(h.as_str()).is_ok());
    let bad = [
        "SHA256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "sha256:E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855",
        "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b85",
        "sha256:e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855 ",
        "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
        "sha256:g3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
    ];
    for b in bad {
        assert!(Sha256Ref::parse(b).is_err(), "{b:?} should be rejected");
    }
}

// @req T4
#[test]
fn sha256_jcs_hashes_the_canonical_bytes() {
    let v = json!({"b": [1.0, 2.5], "a": "x"});
    let bytes = match jcs(&v) {
        Ok(b) => b,
        Err(e) => panic!("{e:?}"),
    };
    match sha256_jcs(&v) {
        Ok(h) => assert_eq!(h, sha256_raw(&bytes)),
        Err(e) => panic!("{e:?}"),
    }
}
