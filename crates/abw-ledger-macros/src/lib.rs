//! `read_query!` (target-system-design §3.9 layer 2, OD-09).
//!
//! Use it through `abw_ledger::read_query!`. It takes one string literal, checks at compile
//! time that the SQL is a read-only query (rules in `abw-ledger/src/sql_check.rs`), and
//! expands to a `::abw_ledger::ReadQuery`. A violation is a `compile_error!` spanned on the
//! literal, so the error is reported on the line of the call.

use proc_macro::{Delimiter, Group, Ident, Literal, Punct, Spacing, Span, TokenStream, TokenTree};

#[path = "../../abw-ledger/src/sql_check.rs"]
#[allow(dead_code)]
mod sql_check;

#[proc_macro]
pub fn read_query(input: TokenStream) -> TokenStream {
    match literal_of(input) {
        Ok(lit) => {
            let span = lit.span();
            match decode_str_literal(&lit.to_string()) {
                Ok(sql) => match sql_check::check(&sql) {
                    Ok(()) => expand(lit),
                    Err(reason) => compile_error(&reason, span),
                },
                Err(reason) => compile_error(&reason, span),
            }
        }
        Err((reason, span)) => compile_error(&reason, span),
    }
}

/// The single literal token of the input, looking through invisible groups that
/// `macro_rules!` forwarding introduces.
fn literal_of(input: TokenStream) -> Result<Literal, (String, Span)> {
    let mut tokens: Vec<TokenTree> = input.into_iter().collect();
    loop {
        match tokens.as_slice() {
            [TokenTree::Literal(lit)] => return Ok(lit.clone()),
            [TokenTree::Group(g)] if g.delimiter() == Delimiter::None => {
                tokens = g.stream().into_iter().collect();
            }
            [] => {
                return Err((
                    "read_query! takes one SQL string literal".to_string(),
                    Span::call_site(),
                ));
            }
            [first, ..] => {
                return Err((
                    "read_query! takes exactly one SQL string literal, with no other arguments"
                        .to_string(),
                    first.span(),
                ));
            }
        }
    }
}

/// The value of a Rust `"…"` or `r#"…"#` literal from its source text.
fn decode_str_literal(src: &str) -> Result<String, String> {
    if let Some(rest) = src.strip_prefix('r') {
        let hashes = rest.chars().take_while(|c| *c == '#').count();
        let fence = "#".repeat(hashes);
        let body = rest
            .strip_prefix(&fence)
            .and_then(|r| r.strip_prefix('"'))
            .and_then(|r| r.strip_suffix(&fence))
            .and_then(|r| r.strip_suffix('"'));
        return body
            .map(str::to_string)
            .ok_or_else(|| "read_query! needs a string literal".to_string());
    }
    let body = src
        .strip_prefix('"')
        .and_then(|r| r.strip_suffix('"'))
        .ok_or_else(|| "read_query! needs a string literal (not a byte or C string)".to_string())?;
    let mut out = String::with_capacity(body.len());
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        if c != '\\' {
            out.push(c);
            continue;
        }
        match chars.next() {
            Some('n') => out.push('\n'),
            Some('r') => out.push('\r'),
            Some('t') => out.push('\t'),
            Some('0') => out.push('\0'),
            Some('\\') => out.push('\\'),
            Some('\'') => out.push('\''),
            Some('"') => out.push('"'),
            Some('x') => {
                let hex: String = chars.by_ref().take(2).collect();
                let v = u8::from_str_radix(&hex, 16).map_err(|_| "bad \\x escape".to_string())?;
                out.push(char::from(v));
            }
            Some('u') => {
                let mut hex = String::new();
                if chars.next() != Some('{') {
                    return Err("bad \\u escape".to_string());
                }
                for h in chars.by_ref() {
                    if h == '}' {
                        break;
                    }
                    if h != '_' {
                        hex.push(h);
                    }
                }
                let cp = u32::from_str_radix(&hex, 16).map_err(|_| "bad \\u escape".to_string())?;
                out.push(char::from_u32(cp).ok_or_else(|| "bad \\u escape".to_string())?);
            }
            Some('\n') => {
                // Line continuation: skip the newline and leading whitespace.
                while chars.peek().is_some_and(|c| c.is_whitespace()) {
                    chars.next();
                }
            }
            _ => return Err("unsupported escape in SQL literal".to_string()),
        }
    }
    Ok(out)
}

/// `::abw_ledger::ReadQuery::__from_checked_literal(<lit>)`
fn expand(lit: Literal) -> TokenStream {
    let span = lit.span();
    let mut path = TokenStream::new();
    for seg in ["abw_ledger", "ReadQuery", "__from_checked_literal"] {
        path.extend(colons(span));
        path.extend([TokenTree::Ident(Ident::new(seg, span))]);
    }
    let mut args = Group::new(
        Delimiter::Parenthesis,
        TokenStream::from(TokenTree::Literal(lit)),
    );
    args.set_span(span);
    path.extend([TokenTree::Group(args)]);
    path
}

fn colons(span: Span) -> [TokenTree; 2] {
    let mut a = Punct::new(':', Spacing::Joint);
    a.set_span(span);
    let mut b = Punct::new(':', Spacing::Alone);
    b.set_span(span);
    [TokenTree::Punct(a), TokenTree::Punct(b)]
}

/// `compile_error!("<msg>")` spanned on `span`.
fn compile_error(msg: &str, span: Span) -> TokenStream {
    let mut bang = Punct::new('!', Spacing::Alone);
    bang.set_span(span);
    let mut text = Literal::string(msg);
    text.set_span(span);
    let mut group = Group::new(
        Delimiter::Parenthesis,
        TokenStream::from(TokenTree::Literal(text)),
    );
    group.set_span(span);
    TokenStream::from_iter([
        TokenTree::Ident(Ident::new("compile_error", span)),
        TokenTree::Punct(bang),
        TokenTree::Group(group),
    ])
}
