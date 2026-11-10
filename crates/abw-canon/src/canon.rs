//! RFC 8785 serialisation (§3.6.1 rules 2–7).

use crate::{JcsError, MAX_DEPTH, MAX_SAFE_INTEGER};
use serde_json::{Number, Value};

/// The JCS canonical bytes of `v`.
pub fn jcs(v: &Value) -> Result<Vec<u8>, JcsError> {
    let mut out = String::new();
    write_value(&mut out, v, 0)?;
    Ok(out.into_bytes())
}

fn write_value(out: &mut String, v: &Value, depth: usize) -> Result<(), JcsError> {
    match v {
        Value::Null => out.push_str("null"),
        Value::Bool(true) => out.push_str("true"),
        Value::Bool(false) => out.push_str("false"),
        Value::Number(n) => write_number(out, n)?,
        Value::String(s) => write_string(out, s),
        Value::Array(items) => {
            if depth >= MAX_DEPTH {
                return Err(JcsError::TooDeep);
            }
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_value(out, item, depth + 1)?;
            }
            out.push(']');
        }
        Value::Object(map) => {
            if depth >= MAX_DEPTH {
                return Err(JcsError::TooDeep);
            }
            // Rule 6: sort by UTF-16 code units, independent of the map's own order.
            let mut members: Vec<(&String, &Value)> = map.iter().collect();
            members.sort_by(|a, b| a.0.encode_utf16().cmp(b.0.encode_utf16()));
            out.push('{');
            for (i, (k, val)) in members.into_iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_string(out, k);
                out.push(':');
                write_value(out, val, depth + 1)?;
            }
            out.push('}');
        }
    }
    Ok(())
}

fn write_string(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\u{c}' => out.push_str("\\f"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if (c as u32) < 0x20 => {
                const HEX: &[u8; 16] = b"0123456789abcdef";
                let b = c as u8;
                out.push_str("\\u00");
                out.push(char::from(HEX[usize::from(b >> 4)]));
                out.push(char::from(HEX[usize::from(b & 0xf)]));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

fn write_number(out: &mut String, n: &Number) -> Result<(), JcsError> {
    if let Some(i) = n.as_i64() {
        if i.unsigned_abs() > MAX_SAFE_INTEGER {
            return Err(JcsError::IntegerOutOfRange {
                literal: i.to_string(),
            });
        }
        out.push_str(&i.to_string());
        return Ok(());
    }
    if let Some(u) = n.as_u64() {
        // as_i64 failed, so u > i64::MAX, far beyond 2^53 − 1.
        return Err(JcsError::IntegerOutOfRange {
            literal: u.to_string(),
        });
    }
    match n.as_f64() {
        Some(f) if f.is_finite() => {
            out.push_str(&es_number(f));
            Ok(())
        }
        _ => Err(JcsError::NonFinite {
            literal: n.to_string(),
        }),
    }
}

/// ECMAScript Number.prototype.toString for a finite binary64 (ECMA-262 §7.1.12.1, as
/// restated in §3.6.1 rule 5).
pub(crate) fn es_number(f: f64) -> String {
    if f == 0.0 {
        return "0".to_string(); // also -0
    }
    // Rust's `{:e}` emits the shortest digit string that round-trips, as
    // `d[0](.d[1..])?e<exp>`. Only the digits and exponent are taken from it.
    let sci = format!("{:e}", f.abs());
    let (mantissa, exp) = sci.split_once('e').unwrap_or((sci.as_str(), "0"));
    let digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    let exp: i32 = exp.parse().unwrap_or(0);
    let k = digits.len() as i32;
    let n = exp + 1; // value = 0.d × 10^n

    let mut s = String::new();
    if f < 0.0 {
        s.push('-');
    }
    if k <= n && n <= 21 {
        s.push_str(&digits);
        s.extend(std::iter::repeat_n('0', (n - k) as usize));
    } else if 0 < n && n <= 21 {
        let (int, frac) = digits.split_at(n as usize);
        s.push_str(int);
        s.push('.');
        s.push_str(frac);
    } else if -6 < n && n <= 0 {
        s.push_str("0.");
        s.extend(std::iter::repeat_n('0', (-n) as usize));
        s.push_str(&digits);
    } else {
        let (first, rest) = digits.split_at(1);
        s.push_str(first);
        if !rest.is_empty() {
            s.push('.');
            s.push_str(rest);
        }
        s.push('e');
        s.push(if n > 0 { '+' } else { '-' });
        s.push_str(&(n - 1).abs().to_string());
    }
    s
}

#[cfg(test)]
mod tests {
    use super::es_number;

    #[test]
    fn ecmascript_examples() {
        for (f, s) in [
            (100.0, "100"),
            (1e20, "100000000000000000000"),
            (1e21, "1e+21"),
            (4.5, "4.5"),
            (0.000001, "0.000001"),
            (5e-7, "5e-7"),
            (1e-27, "1e-27"),
            (-0.0, "0"),
            (333333333.3333333, "333333333.3333333"),
            (-1.5, "-1.5"),
            (123.456e-10, "1.23456e-8"),
        ] {
            assert_eq!(es_number(f), s, "{f}");
        }
    }
}
