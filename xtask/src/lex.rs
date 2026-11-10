//! A small Rust lexer for static checks: separates code, string literals and comments.
//!
//! It is deliberately approximate (no macro expansion, no type information) but exact about
//! the three things the checks need: what is a comment, what is a string literal, and on
//! which line each starts.

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Code,
    Str,
    Comment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Segment {
    pub kind: Kind,
    /// 1-based line on which the segment starts.
    pub line: usize,
    pub text: String,
}

pub fn segments(src: &str) -> Vec<Segment> {
    let chars: Vec<char> = src.chars().collect();
    let mut out = Vec::new();
    let mut i = 0;
    let mut line = 1;
    let mut code = String::new();
    let mut code_line = 1;

    fn flush(out: &mut Vec<Segment>, code: &mut String, code_line: usize) {
        if !code.is_empty() {
            out.push(Segment {
                kind: Kind::Code,
                line: code_line,
                text: std::mem::take(code),
            });
        }
    }

    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        // Line comment.
        if c == '/' && next == Some('/') {
            flush(&mut out, &mut code, code_line);
            let start = line;
            let mut text = String::new();
            while i < chars.len() && chars[i] != '\n' {
                text.push(chars[i]);
                i += 1;
            }
            out.push(Segment {
                kind: Kind::Comment,
                line: start,
                text,
            });
            code_line = line;
            continue;
        }
        // Block comment (nested).
        if c == '/' && next == Some('*') {
            flush(&mut out, &mut code, code_line);
            let start = line;
            let mut depth = 0;
            let mut text = String::new();
            while i < chars.len() {
                if chars[i] == '/' && chars.get(i + 1) == Some(&'*') {
                    depth += 1;
                    text.push_str("/*");
                    i += 2;
                    continue;
                }
                if chars[i] == '*' && chars.get(i + 1) == Some(&'/') {
                    depth -= 1;
                    text.push_str("*/");
                    i += 2;
                    if depth == 0 {
                        break;
                    }
                    continue;
                }
                if chars[i] == '\n' {
                    line += 1;
                }
                text.push(chars[i]);
                i += 1;
            }
            out.push(Segment {
                kind: Kind::Comment,
                line: start,
                text,
            });
            code_line = line;
            continue;
        }
        // Raw string: r"..." or r#"..."# (also br"...").
        if (c == 'r' || (c == 'b' && next == Some('r'))) && !prev_is_ident(&chars, i) {
            let mut j = if c == 'b' { i + 2 } else { i + 1 };
            let mut hashes = 0;
            while chars.get(j) == Some(&'#') {
                hashes += 1;
                j += 1;
            }
            if chars.get(j) == Some(&'"') {
                flush(&mut out, &mut code, code_line);
                let start = line;
                j += 1;
                let mut text = String::new();
                while let Some(&ch) = chars.get(j) {
                    if ch == '"' && (0..hashes).all(|k| chars.get(j + 1 + k) == Some(&'#')) {
                        j += 1 + hashes;
                        break;
                    }
                    if ch == '\n' {
                        line += 1;
                    }
                    text.push(ch);
                    j += 1;
                }
                out.push(Segment {
                    kind: Kind::Str,
                    line: start,
                    text,
                });
                i = j;
                code_line = line;
                continue;
            }
        }
        // Ordinary string (also b"...").
        if c == '"' {
            flush(&mut out, &mut code, code_line);
            let start = line;
            let mut text = String::new();
            i += 1;
            while i < chars.len() {
                let ch = chars[i];
                if ch == '\\'
                    && let Some(&esc) = chars.get(i + 1)
                {
                    text.push(ch);
                    text.push(esc);
                    if esc == '\n' {
                        line += 1;
                    }
                    i += 2;
                    continue;
                }
                if ch == '"' {
                    i += 1;
                    break;
                }
                if ch == '\n' {
                    line += 1;
                }
                text.push(ch);
                i += 1;
            }
            out.push(Segment {
                kind: Kind::Str,
                line: start,
                text,
            });
            code_line = line;
            continue;
        }
        // Char literal or lifetime: skip a char literal so '"' does not open a string.
        if c == '\''
            && let (Some(a), Some(b)) = (chars.get(i + 1), chars.get(i + 2))
        {
            if *a == '\\' {
                // '\n', '\'', '\u{..}': the closing quote comes after the escaped char.
                let mut j = i + 3;
                while j < chars.len() && chars[j] != '\'' && j < i + 12 {
                    j += 1;
                }
                let end = j.min(chars.len() - 1);
                code.extend(&chars[i..=end]);
                i = end + 1;
                continue;
            }
            if *b == '\'' {
                code.extend(&chars[i..i + 3]);
                i += 3;
                continue;
            }
        }
        if code.is_empty() {
            code_line = line;
        }
        if c == '\n' {
            line += 1;
        }
        code.push(c);
        i += 1;
    }
    flush(&mut out, &mut code, code_line);
    out
}

fn prev_is_ident(chars: &[char], i: usize) -> bool {
    i > 0 && (chars[i - 1].is_alphanumeric() || chars[i - 1] == '_')
}

/// Splits text into lowercase words: identifiers are split on `_` and lower-to-upper case
/// transitions, everything else on non-alphanumeric characters.
pub fn words(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut cur = String::new();
    let mut prev_lower = false;
    for ch in text.chars() {
        if ch.is_alphanumeric() {
            if ch.is_uppercase() && prev_lower && !cur.is_empty() {
                out.push(std::mem::take(&mut cur).to_lowercase());
            }
            prev_lower = ch.is_lowercase() || ch.is_ascii_digit();
            cur.push(ch);
        } else {
            if !cur.is_empty() {
                out.push(std::mem::take(&mut cur).to_lowercase());
            }
            prev_lower = false;
        }
    }
    if !cur.is_empty() {
        out.push(cur.to_lowercase());
    }
    out
}

/// Line number of `offset` (a char index into `text`) given the segment's start line.
pub fn line_at(seg: &Segment, offset: usize) -> usize {
    seg.line + seg.text.chars().take(offset).filter(|c| *c == '\n').count()
}
