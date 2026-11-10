//! I-JSON parser (RFC 8259 grammar, RFC 7493 restrictions; §3.6.1 rule 1).

use crate::{JcsError, MAX_DEPTH, MAX_SAFE_INTEGER};
use serde_json::{Map, Number, Value};

/// Parses `text` as one I-JSON value.
pub fn parse(text: &str) -> Result<Value, JcsError> {
    let mut p = Parser {
        src: text.as_bytes(),
        text,
        pos: 0,
    };
    p.ws();
    let v = p.value(0)?;
    p.ws();
    if p.pos != p.src.len() {
        return Err(p.err("trailing characters"));
    }
    Ok(v)
}

struct Parser<'a> {
    src: &'a [u8],
    text: &'a str,
    pos: usize,
}

impl<'a> Parser<'a> {
    fn err(&self, reason: &'static str) -> JcsError {
        JcsError::Syntax {
            offset: self.pos,
            reason,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn ws(&mut self) {
        while let Some(b' ' | b'\t' | b'\n' | b'\r') = self.peek() {
            self.pos += 1;
        }
    }

    fn eat(&mut self, b: u8, reason: &'static str) -> Result<(), JcsError> {
        if self.peek() == Some(b) {
            self.pos += 1;
            Ok(())
        } else {
            Err(self.err(reason))
        }
    }

    fn literal(&mut self, word: &'static str, v: Value) -> Result<Value, JcsError> {
        if self.src[self.pos..].starts_with(word.as_bytes()) {
            self.pos += word.len();
            Ok(v)
        } else {
            Err(self.err("invalid literal"))
        }
    }

    fn value(&mut self, depth: usize) -> Result<Value, JcsError> {
        match self.peek() {
            Some(b'{') => self.object(depth + 1),
            Some(b'[') => self.array(depth + 1),
            Some(b'"') => self.string().map(Value::String),
            Some(b't') => self.literal("true", Value::Bool(true)),
            Some(b'f') => self.literal("false", Value::Bool(false)),
            Some(b'n') => self.literal("null", Value::Null),
            Some(b'-' | b'0'..=b'9') => self.number(),
            Some(_) => Err(self.err("expected a value")),
            None => Err(self.err("unexpected end of input")),
        }
    }

    fn object(&mut self, depth: usize) -> Result<Value, JcsError> {
        if depth > MAX_DEPTH {
            return Err(JcsError::TooDeep);
        }
        self.pos += 1;
        let mut map = Map::new();
        self.ws();
        if self.peek() == Some(b'}') {
            self.pos += 1;
            return Ok(Value::Object(map));
        }
        loop {
            self.ws();
            if self.peek() != Some(b'"') {
                return Err(self.err("expected a member name"));
            }
            let at = self.pos;
            let name = self.string()?;
            self.ws();
            self.eat(b':', "expected ':'")?;
            self.ws();
            let v = self.value(depth)?;
            // Names are compared after escape decoding (RFC 7493 §2.3).
            if map.contains_key(&name) {
                return Err(JcsError::DuplicateMember { name, offset: at });
            }
            map.insert(name, v);
            self.ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b'}') => {
                    self.pos += 1;
                    return Ok(Value::Object(map));
                }
                _ => return Err(self.err("expected ',' or '}'")),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<Value, JcsError> {
        if depth > MAX_DEPTH {
            return Err(JcsError::TooDeep);
        }
        self.pos += 1;
        let mut items = Vec::new();
        self.ws();
        if self.peek() == Some(b']') {
            self.pos += 1;
            return Ok(Value::Array(items));
        }
        loop {
            self.ws();
            items.push(self.value(depth)?);
            self.ws();
            match self.peek() {
                Some(b',') => self.pos += 1,
                Some(b']') => {
                    self.pos += 1;
                    return Ok(Value::Array(items));
                }
                _ => return Err(self.err("expected ',' or ']'")),
            }
        }
    }

    fn hex4(&mut self) -> Result<u16, JcsError> {
        let mut v: u16 = 0;
        for _ in 0..4 {
            let d = match self.peek() {
                Some(c @ b'0'..=b'9') => c - b'0',
                Some(c @ b'a'..=b'f') => c - b'a' + 10,
                Some(c @ b'A'..=b'F') => c - b'A' + 10,
                _ => return Err(self.err("invalid \\u escape")),
            };
            v = (v << 4) | u16::from(d);
            self.pos += 1;
        }
        Ok(v)
    }

    fn string(&mut self) -> Result<String, JcsError> {
        self.pos += 1; // opening quote
        let mut out = String::new();
        loop {
            let start = self.pos;
            while let Some(b) = self.peek() {
                if b == b'"' || b == b'\\' || b < 0x20 {
                    break;
                }
                self.pos += 1;
            }
            // `text` is valid UTF-8 and the scan stops only at ASCII bytes, so both ends
            // fall on character boundaries.
            out.push_str(self.text.get(start..self.pos).unwrap_or_default());
            match self.peek() {
                Some(b'"') => {
                    self.pos += 1;
                    return Ok(out);
                }
                Some(b'\\') => {
                    let esc_at = self.pos;
                    self.pos += 1;
                    let c = match self.peek() {
                        Some(b'"') => '"',
                        Some(b'\\') => '\\',
                        Some(b'/') => '/',
                        Some(b'b') => '\u{8}',
                        Some(b'f') => '\u{c}',
                        Some(b'n') => '\n',
                        Some(b'r') => '\r',
                        Some(b't') => '\t',
                        Some(b'u') => {
                            self.pos += 1;
                            out.push(self.unicode_escape(esc_at)?);
                            continue;
                        }
                        _ => return Err(self.err("invalid escape")),
                    };
                    self.pos += 1;
                    out.push(c);
                }
                Some(_) => return Err(self.err("unescaped control character in string")),
                None => return Err(self.err("unterminated string")),
            }
        }
    }

    /// Decodes the code point of a `\u` escape whose `u` has been consumed.
    fn unicode_escape(&mut self, esc_at: usize) -> Result<char, JcsError> {
        let hi = self.hex4()?;
        let lone = |code| JcsError::LoneSurrogate {
            code,
            offset: esc_at,
        };
        match hi {
            0xD800..=0xDBFF => {
                if !self.src[self.pos..].starts_with(b"\\u") {
                    return Err(lone(hi));
                }
                self.pos += 2;
                let lo = self.hex4()?;
                if !(0xDC00..=0xDFFF).contains(&lo) {
                    return Err(lone(hi));
                }
                let cp = 0x10000 + ((u32::from(hi) - 0xD800) << 10) + (u32::from(lo) - 0xDC00);
                char::from_u32(cp).ok_or_else(|| lone(hi))
            }
            0xDC00..=0xDFFF => Err(lone(hi)),
            _ => char::from_u32(u32::from(hi)).ok_or_else(|| lone(hi)),
        }
    }

    fn digits(&mut self) -> usize {
        let start = self.pos;
        while let Some(b'0'..=b'9') = self.peek() {
            self.pos += 1;
        }
        self.pos - start
    }

    fn number(&mut self) -> Result<Value, JcsError> {
        let start = self.pos;
        if self.peek() == Some(b'-') {
            self.pos += 1;
        }
        match self.peek() {
            Some(b'0') => self.pos += 1,
            Some(b'1'..=b'9') => {
                self.digits();
            }
            _ => return Err(self.err("invalid number")),
        }
        let mut integer = true;
        if self.peek() == Some(b'.') {
            integer = false;
            self.pos += 1;
            if self.digits() == 0 {
                return Err(self.err("expected a digit after '.'"));
            }
        }
        if let Some(b'e' | b'E') = self.peek() {
            integer = false;
            self.pos += 1;
            if let Some(b'+' | b'-') = self.peek() {
                self.pos += 1;
            }
            if self.digits() == 0 {
                return Err(self.err("expected a digit in the exponent"));
            }
        }
        let literal = self.text.get(start..self.pos).unwrap_or_default();
        if integer {
            let out_of_range = || JcsError::IntegerOutOfRange {
                literal: literal.to_string(),
            };
            let n: i64 = literal.parse().map_err(|_| out_of_range())?;
            if n.unsigned_abs() > MAX_SAFE_INTEGER {
                return Err(out_of_range());
            }
            return Ok(Value::Number(Number::from(n)));
        }
        let non_finite = || JcsError::NonFinite {
            literal: literal.to_string(),
        };
        let f: f64 = literal.parse().map_err(|_| non_finite())?;
        Number::from_f64(f)
            .filter(|_| f.is_finite())
            .map(Value::Number)
            .ok_or_else(non_finite)
    }
}
