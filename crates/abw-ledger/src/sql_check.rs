//! The SELECT-only rule for `ReadQuery` (target-system-design §3.9 layer 2, OD-09).
//!
//! This file is compiled twice: by `abw-ledger-macros` (through `#[path]`), where a
//! violation becomes `compile_error!` at the macro call, and by `abw-ledger`, where
//! `ReadDb` re-checks every query before it is sent. One rule, one implementation.
//!
//! A query is accepted only if
//! - its first keyword (after any opening parentheses) is `SELECT` or `WITH`;
//! - no `INSERT`, `UPDATE`, `DELETE` or `MERGE` keyword appears anywhere, so a
//!   data-modifying CTE cannot hide behind `WITH`;
//! - no row-locking clause (`FOR UPDATE`, `FOR NO KEY UPDATE`, `FOR SHARE`,
//!   `FOR KEY SHARE`) appears;
//! - no `INTO` appears (`SELECT … INTO t` creates a table);
//! - it is one statement (a `;` may only end it).
//!
//! String literals, quoted identifiers, dollar-quoted bodies and comments are skipped, so
//! a keyword inside them does not count. A bare column named like one of the keywords is
//! rejected; quote it.

#[derive(Debug, Clone, PartialEq, Eq)]
enum Tok {
    Word(String),
    Semicolon,
    Other,
}

const FORBIDDEN: &[&str] = &["insert", "update", "delete", "merge", "into"];

/// `Ok(())` if `sql` is a read-only query by the rules above, else the reason.
pub fn check(sql: &str) -> Result<(), String> {
    let toks = tokenize(sql)?;
    let first = toks.iter().find(|t| !matches!(t, Tok::Other));
    match first {
        Some(Tok::Word(w)) if w == "select" || w == "with" => {}
        Some(Tok::Word(w)) => {
            return Err(format!(
                "read_query! accepts only SELECT or WITH queries; this one starts with `{}`",
                w.to_uppercase()
            ));
        }
        _ => return Err("read_query! needs a SELECT or WITH query".to_string()),
    }
    for (i, t) in toks.iter().enumerate() {
        match t {
            Tok::Word(w) if w == "for" => {
                let next = toks.get(i + 1);
                if let Some(Tok::Word(n)) = next
                    && matches!(n.as_str(), "update" | "share" | "key" | "no")
                {
                    return Err(
                        "read_query! is read-only; row-locking clauses (FOR UPDATE, FOR NO KEY UPDATE, FOR SHARE, FOR KEY SHARE) are not allowed"
                            .to_string(),
                    );
                }
            }
            Tok::Word(w) if FORBIDDEN.contains(&w.as_str()) => {
                return Err(format!(
                    "read_query! is read-only; `{}` is not allowed (data-modifying statement or CTE)",
                    w.to_uppercase()
                ));
            }
            Tok::Semicolon if toks[i + 1..].iter().any(|t| *t != Tok::Semicolon) => {
                return Err("read_query! takes exactly one statement".to_string());
            }
            _ => {}
        }
    }
    Ok(())
}

fn tokenize(sql: &str) -> Result<Vec<Tok>, String> {
    let chars: Vec<char> = sql.chars().collect();
    let mut toks = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        let next = chars.get(i + 1).copied();
        if c.is_whitespace() {
            i += 1;
        } else if c == '-' && next == Some('-') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
        } else if c == '/' && next == Some('*') {
            let mut depth = 1;
            i += 2;
            while depth > 0 {
                match (chars.get(i), chars.get(i + 1)) {
                    (Some('*'), Some('/')) => {
                        depth -= 1;
                        i += 2;
                    }
                    (Some('/'), Some('*')) => {
                        depth += 1;
                        i += 2;
                    }
                    (Some(_), _) => i += 1,
                    (None, _) => return Err("unterminated /* comment".to_string()),
                }
            }
        } else if c == '\'' {
            // An E'…' string allows backslash escapes; the `E` was lexed as a word.
            let escapes = matches!(toks.last(), Some(Tok::Word(w)) if w == "e")
                && i > 0
                && matches!(chars[i - 1], 'e' | 'E');
            if escapes {
                toks.pop();
            }
            i += 1;
            loop {
                match chars.get(i) {
                    Some('\\') if escapes => i += 2,
                    Some('\'') if chars.get(i + 1) == Some(&'\'') => i += 2,
                    Some('\'') => {
                        i += 1;
                        break;
                    }
                    Some(_) => i += 1,
                    None => return Err("unterminated string literal".to_string()),
                }
            }
            toks.push(Tok::Other);
        } else if c == '"' {
            i += 1;
            loop {
                match chars.get(i) {
                    Some('"') if chars.get(i + 1) == Some(&'"') => i += 2,
                    Some('"') => {
                        i += 1;
                        break;
                    }
                    Some(_) => i += 1,
                    None => return Err("unterminated quoted identifier".to_string()),
                }
            }
            toks.push(Tok::Other);
        } else if c == '$' && next.is_some_and(|n| n.is_ascii_digit()) {
            i += 1;
            while chars.get(i).is_some_and(char::is_ascii_digit) {
                i += 1;
            }
            toks.push(Tok::Other);
        } else if c == '$' {
            // Dollar quote: $tag$ … $tag$
            let mut j = i + 1;
            while chars
                .get(j)
                .is_some_and(|ch| ch.is_alphanumeric() || *ch == '_')
            {
                j += 1;
            }
            if chars.get(j) != Some(&'$') {
                return Err("unexpected `$`".to_string());
            }
            let tag: Vec<char> = chars[i..=j].to_vec();
            i = j + 1;
            loop {
                if i >= chars.len() {
                    return Err("unterminated dollar-quoted string".to_string());
                }
                if chars[i..].starts_with(&tag) {
                    i += tag.len();
                    break;
                }
                i += 1;
            }
            toks.push(Tok::Other);
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while chars
                .get(i)
                .is_some_and(|ch| ch.is_alphanumeric() || *ch == '_' || *ch == '$')
            {
                i += 1;
            }
            let w: String = chars[start..i].iter().collect();
            toks.push(Tok::Word(w.to_lowercase()));
        } else if c == ';' {
            toks.push(Tok::Semicolon);
            i += 1;
        } else {
            toks.push(Tok::Other);
            i += 1;
        }
    }
    Ok(toks)
}

#[cfg(test)]
mod tests {
    use super::check;

    #[test]
    fn accepts_reads() {
        for q in [
            "SELECT 1",
            "select 1 from crud.notes",
            "WITH t AS (SELECT id FROM ledger.outbox) SELECT count(*) FROM t",
            "(SELECT 1) UNION (SELECT 2)",
            "SELECT updated_at, 'UPDATE x SET y = 1' FROM t -- DELETE FROM t",
            "SELECT \"update\" FROM t /* INSERT INTO /* nested */ x */",
            "SELECT $1::text, $$ DELETE FROM t $$, E'it\\'s INSERT'",
            "SELECT 1;",
            "SELECT * FROM t ORDER BY a FOR",
        ] {
            assert_eq!(check(q), Ok(()), "{q}");
        }
    }

    #[test]
    fn rejects_writes_and_locks() {
        for q in [
            "UPDATE ledger.streams SET head = head + 1",
            "insert into crud.notes (id) values (1)",
            "WITH t AS (DELETE FROM ledger.outbox RETURNING id) SELECT count(*) FROM t",
            "SELECT head FROM ledger.streams FOR UPDATE",
            "SELECT head FROM ledger.streams FOR NO KEY UPDATE",
            "SELECT head FROM ledger.streams for share",
            "SELECT head FROM ledger.streams FOR KEY SHARE",
            "MERGE INTO proj.probes p USING (SELECT 1) s ON false WHEN NOT MATCHED THEN DO NOTHING",
            "SELECT * INTO t2 FROM t",
            "SELECT 1; SELECT 2",
            "TRUNCATE ledger.events",
            "",
            "SELECT 'unterminated",
        ] {
            assert!(check(q).is_err(), "{q}");
        }
    }
}
