use crate::{
    ast::{Component, Element, Handler, Property, PropertyKind, Value},
    lexer::{lex, Token},
};

pub fn parse(source: &str) -> Result<Component, String> {
    Parser {
        tokens: lex(source)?,
        at: 0,
    }
    .component()
}

struct Parser {
    tokens: Vec<Token>,
    at: usize,
}

impl Parser {
    fn current(&self) -> &Token {
        &self.tokens[self.at]
    }
    fn next_is(&self, symbol: char) -> bool {
        self.tokens.get(self.at + 1) == Some(&Token::Symbol(symbol))
    }
    fn bump(&mut self) {
        if self.at + 1 < self.tokens.len() {
            self.at += 1;
        }
    }
    fn is_ident(&self, expected: &str) -> bool {
        matches!(self.current(), Token::Ident(v) if v == expected)
    }
    fn ident(&mut self) -> Result<String, String> {
        match self.current().clone() {
            Token::Ident(v) => {
                self.bump();
                Ok(v)
            }
            found => Err(format!("expected identifier, found {found:?}")),
        }
    }
    fn eat(&mut self, symbol: char) -> bool {
        if self.current() == &Token::Symbol(symbol) {
            self.bump();
            true
        } else {
            false
        }
    }
    fn expect(&mut self, symbol: char) -> Result<(), String> {
        if self.eat(symbol) {
            Ok(())
        } else {
            Err(format!("expected `{symbol}`, found {:?}", self.current()))
        }
    }

    fn component(mut self) -> Result<Component, String> {
        while self.is_ident("import") {
            self.until_semicolon("import")?;
        }
        if self.is_ident("export") {
            self.bump();
        }
        if self.ident()? != "component" {
            return Err("expected `component` declaration".into());
        }
        let name = self.ident()?;
        let mut root_tag = "div";
        if self.is_ident("inherits") {
            self.bump();
            let base = self.ident()?;
            if base == "Window" {
                root_tag = "main";
            } else {
                return Err(format!("unsupported component base `{base}`"));
            }
        }
        self.expect('{')?;
        let mut properties = Vec::new();
        let mut callbacks = Vec::new();
        let mut children = Vec::new();
        let mut title = None;
        while !self.eat('}') {
            if self.current() == &Token::End {
                return Err("unclosed component body".into());
            }
            if self.is_ident("property") {
                properties.push(self.property()?);
                continue;
            }
            if self.is_ident("callback") {
                callbacks.push(self.callback()?);
                continue;
            }
            let first = self.ident()?;
            if self.current() == &Token::Symbol(':') && self.next_is('=') {
                children.push(self.element(first)?);
            } else if self.eat(':') {
                if first != "title" {
                    return Err(format!("unsupported component property `{first}`"));
                }
                if title.is_some() {
                    return Err("component `title` is assigned more than once".into());
                }
                match self.value()? {
                    Value::String(value) => title = Some(value),
                    _ => return Err("component `title` requires a string literal".into()),
                }
                self.expect(';')?;
            } else {
                children.push(self.element(first)?);
            }
        }
        if self.current() != &Token::End {
            return Err(format!(
                "unexpected tokens after component: {:?}",
                self.current()
            ));
        }
        for property in &properties {
            validate_value_type(
                &property.initial,
                property.kind,
                &format!("property `{}`", property.name),
            )?;
        }
        Ok(Component {
            name,
            root_tag,
            title,
            properties,
            callbacks,
            children,
        })
    }

    fn property(&mut self) -> Result<Property, String> {
        self.bump();
        self.expect('<')?;
        let ty = self.ident()?;
        self.expect('>')?;
        let kind = match ty.as_str() {
            "string" => PropertyKind::String,
            "bool" => PropertyKind::Bool,
            "int" => PropertyKind::Int,
            "float" => PropertyKind::Float,
            _ => return Err(format!("unsupported property type `{ty}`")),
        };
        let name = self.ident()?;
        let initial = if self.eat(':') {
            self.value()?
        } else {
            default_value(kind)
        };
        self.expect(';')?;
        Ok(Property {
            name,
            kind,
            initial,
        })
    }

    fn callback(&mut self) -> Result<String, String> {
        self.bump();
        let name = self.ident()?;
        self.expect('(')?;
        if !self.eat(')') {
            return Err(format!(
                "callback `{name}` has arguments; only zero-argument callbacks are supported"
            ));
        }
        self.expect(';')?;
        Ok(name)
    }

    fn element(&mut self, first: String) -> Result<Element, String> {
        let (id, kind) = if self.eat(':') {
            self.expect('=')?;
            (Some(first), self.ident()?)
        } else {
            (None, first)
        };
        self.expect('{')?;
        let mut properties = Vec::new();
        let mut handlers = Vec::new();
        let mut children = Vec::new();
        while !self.eat('}') {
            if self.current() == &Token::End {
                return Err(format!("unclosed `{kind}` element"));
            }
            let name = self.ident()?;
            if self.current() == &Token::Symbol(':') && self.next_is('=') {
                children.push(self.element(name)?);
            } else if self.eat(':') {
                let value = self.value()?;
                self.expect(';')?;
                properties.push((name, value));
            } else if self.current() == &Token::Arrow {
                self.bump();
                handlers.push(self.handler(name)?);
            } else {
                children.push(self.element(name)?);
            }
        }
        Ok(Element {
            kind,
            id,
            properties,
            handlers,
            children,
        })
    }

    fn handler(&mut self, event: String) -> Result<Handler, String> {
        self.expect('{')?;
        if self.is_ident("root") {
            self.bump();
            self.expect('.')?;
        }
        let callback = self.ident()?;
        self.expect('(')?;
        self.expect(')')?;
        self.eat(';');
        self.expect('}')?;
        Ok(Handler { event, callback })
    }

    fn value(&mut self) -> Result<Value, String> {
        let value = match self.current().clone() {
            Token::String(v) => Value::String(v),
            Token::Ident(v) if v == "true" => Value::Bool(true),
            Token::Ident(v) if v == "false" => Value::Bool(false),
            Token::Ident(v) => Value::Identifier(v),
            Token::Number(v) => Value::Number(v),
            found => return Err(format!("expected value, found {found:?}")),
        };
        self.bump();
        Ok(value)
    }

    fn until_semicolon(&mut self, context: &str) -> Result<(), String> {
        while !self.eat(';') {
            if self.current() == &Token::End {
                return Err(format!("unterminated {context}"));
            }
            self.bump();
        }
        Ok(())
    }
}

fn default_value(kind: PropertyKind) -> Value {
    match kind {
        PropertyKind::String => Value::String(String::new()),
        PropertyKind::Bool => Value::Bool(false),
        PropertyKind::Int => Value::Number("0".into()),
        PropertyKind::Float => Value::Number("0.0".into()),
    }
}

fn validate_value_type(value: &Value, kind: PropertyKind, context: &str) -> Result<(), String> {
    let valid = matches!(
        (value, kind),
        (Value::String(_), PropertyKind::String)
            | (Value::Bool(_), PropertyKind::Bool)
            | (Value::Number(_), PropertyKind::Int | PropertyKind::Float)
    );
    if valid {
        Ok(())
    } else {
        Err(format!(
            "initial value of {context} does not match its type"
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn parses_properties_ids_bindings_and_callbacks() {
        let component = parse(
            r#"export component App inherits Window {
            property <bool> enabled: true; callback start();
            layout := VerticalLayout { Button { enabled: enabled; clicked => { root.start(); } } }
        }"#,
        )
        .unwrap();
        assert_eq!(component.properties.len(), 1);
        assert_eq!(component.callbacks, vec!["start"]);
        assert_eq!(component.children[0].id.as_deref(), Some("layout"));
        assert_eq!(
            component.children[0].children[0].handlers[0].callback,
            "start"
        );
    }
    #[test]
    fn rejects_unknown_root_properties() {
        assert!(parse("export component App { mystery: true; }")
            .unwrap_err()
            .contains("unsupported"));
    }
}
