#[derive(Clone, Debug, PartialEq)]
pub enum Token {
    Ident(String),
    String(String),
    Number(String),
    Symbol(char),
    Arrow,
    End,
}

pub fn lex(source: &str) -> Result<Vec<Token>, String> {
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
                return Err("unterminated block comment".into());
            }
            at += 2;
            continue;
        }
        if at + 1 < chars.len() && chars[at] == '=' && chars[at + 1] == '>' {
            tokens.push(Token::Arrow);
            at += 2;
            continue;
        }
        if chars[at] == '"' {
            at += 1;
            let mut value = String::new();
            while at < chars.len() && chars[at] != '"' {
                if chars[at] == '\\' {
                    at += 1;
                    if at >= chars.len() {
                        return Err("unterminated string escape".into());
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
                return Err("unterminated string".into());
            }
            at += 1;
            tokens.push(Token::String(value));
            continue;
        }
        if chars[at] == '#' {
            let start = at;
            at += 1;
            while at < chars.len() && chars[at].is_ascii_hexdigit() {
                at += 1;
            }
            if at - start == 1 {
                return Err(format!("invalid color at character {start}"));
            }
            tokens.push(Token::String(chars[start..at].iter().collect()));
            continue;
        }
        if chars[at].is_ascii_alphabetic() || chars[at] == '_' {
            let start = at;
            while at < chars.len()
                && (chars[at].is_ascii_alphanumeric() || matches!(chars[at], '_' | '-'))
            {
                at += 1;
            }
            tokens.push(Token::Ident(chars[start..at].iter().collect()));
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
            tokens.push(Token::Number(chars[start..at].iter().collect()));
            continue;
        }
        if "{}:;<>,().=[]!".contains(chars[at]) {
            tokens.push(Token::Symbol(chars[at]));
            at += 1;
            continue;
        }
        return Err(format!(
            "unexpected character `{}` at character {at}",
            chars[at]
        ));
    }
    tokens.push(Token::End);
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
