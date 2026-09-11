#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Ident(String),
    String(String),
    Number(String),
    Symbol(char),
    Arrow,
    End,
}

#[derive(Debug)]
pub(crate) struct LexError {
    pub message: String,
    pub offset: usize,
}

pub(crate) type SpannedToken = (Token, usize);

#[cfg(test)]
pub fn lex(source: &str) -> Result<Vec<Token>, String> {
    lex_spanned(source)
        .map(|tokens| tokens.into_iter().map(|(token, _)| token).collect())
        .map_err(|error| error.message)
}

pub(crate) fn lex_spanned(source: &str) -> Result<Vec<SpannedToken>, LexError> {
    let chars: Vec<char> = source.chars().collect();
    let mut at = 0;
    let mut tokens = Vec::new();
    while at < chars.len() {
        if chars[at].is_whitespace() {
            at += 1;
            continue;
        }
        if at + 1 < chars.len() && chars[at] == '/' && chars[at + 1] == '/' {
            while at < chars.len() && chars[at] != '\n' {
                at += 1;
            }
            continue;
        }
        if at + 1 < chars.len() && chars[at] == '/' && chars[at + 1] == '*' {
            at += 2;
            while at + 1 < chars.len() && !(chars[at] == '*' && chars[at + 1] == '/') {
                at += 1;
            }
            if at + 1 >= chars.len() {
                return Err(LexError {
                    message: "unterminated block comment".into(),
                    offset: at,
                });
            }
            at += 2;
            continue;
        }
        if at + 1 < chars.len() && chars[at] == '=' && chars[at + 1] == '>' {
            tokens.push((Token::Arrow, at));
            at += 2;
            continue;
        }
        if chars[at] == '"' {
            let start = at;
            at += 1;
            let mut value = String::new();
            while at < chars.len() && chars[at] != '"' {
                if chars[at] == '\\' {
                    at += 1;
                    if at >= chars.len() {
                        return Err(LexError {
                            message: "unterminated string escape".into(),
                            offset: at,
                        });
                    }
                    value.push(match chars[at] {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        c => c,
                    });
                } else {
                    value.push(chars[at]);
                }
                at += 1;
            }
            if at == chars.len() {
                return Err(LexError {
                    message: "unterminated string".into(),
                    offset: at,
                });
            }
            at += 1;
            tokens.push((Token::String(value), start));
            continue;
        }
        if chars[at] == '#' {
            let start = at;
            at += 1;
            while at < chars.len() && chars[at].is_ascii_hexdigit() {
                at += 1;
            }
            if at - start == 1 {
                return Err(LexError {
                    message: "invalid color".into(),
                    offset: start,
                });
            }
            tokens.push((Token::String(chars[start..at].iter().collect()), start));
            continue;
        }
        if chars[at].is_ascii_alphabetic() || chars[at] == '_' {
            let start = at;
            while at < chars.len()
                && (chars[at].is_ascii_alphanumeric() || matches!(chars[at], '_' | '-'))
            {
                at += 1;
            }
            tokens.push((Token::Ident(chars[start..at].iter().collect()), start));
            continue;
        }
        if chars[at].is_ascii_digit()
            || (chars[at] == '-' && chars.get(at + 1).is_some_and(|c| c.is_ascii_digit()))
        {
            let start = at;
            at += 1;
            while at < chars.len()
                && (chars[at].is_ascii_alphanumeric() || matches!(chars[at], '.' | '%' | '-'))
            {
                at += 1;
            }
            tokens.push((Token::Number(chars[start..at].iter().collect()), start));
            continue;
        }
        if "{}:;<>,().=[]!".contains(chars[at]) {
            tokens.push((Token::Symbol(chars[at]), at));
            at += 1;
            continue;
        }
        return Err(LexError {
            message: format!("unexpected character `{}`", chars[at]),
            offset: at,
        });
    }
    tokens.push((Token::End, chars.len()));
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn handles_comments_arrows_units_and_escapes() {
        let tokens = lex("// x\n clicked => { root.go(); } width: 20px; text: \"a\\n\";").unwrap();
        assert!(tokens.contains(&Token::Arrow));
        assert!(tokens.contains(&Token::Number("20px".into())));
        assert!(tokens.contains(&Token::String("a\n".into())));
        assert!(lex("background: #1a2B3c;")
            .unwrap()
            .contains(&Token::String("#1a2B3c".into())));
    }
}
