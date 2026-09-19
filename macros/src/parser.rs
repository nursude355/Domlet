use crate::{
    ast::{Component, Element, Handler, Property, PropertyKind, Value},
    lexer::{lex_spanned, SpannedToken, Token},
};

#[derive(Debug)]
pub struct ParseError {
    pub message: String,
    pub offset: usize,
}

impl ParseError {
    #[cfg(test)]
    fn contains(&self, needle: &str) -> bool {
        self.message.contains(needle)
    }
}

pub fn parse(source: &str) -> Result<Component, ParseError> {

    // for testing
    let error_message = format!("testing compile_error! in parser.rs -{}- ", source);
    compile_error!(error_message);


    Parser {
        tokens: lex_spanned(source).map_err(|error| ParseError {
            message: error.message,
            offset: error.offset,
        })?,
        at: 0,
    }
    .component()
}

struct Parser {
    tokens: Vec<SpannedToken>,
    at: usize,
}

impl Parser {
    fn current(&self) -> &Token {
        &self.tokens[self.at].0
    }
    fn offset(&self) -> usize {
        self.tokens[self.at].1
    }
    fn error(&self, message: impl Into<String>) -> ParseError {
        self.error_at(self.offset(), message)
    }
    fn error_at(&self, offset: usize, message: impl Into<String>) -> ParseError {
        ParseError {
            message: message.into(),
            offset,
        }
    }
    fn next_is(&self, symbol: char) -> bool {
        self.tokens.get(self.at + 1).map(|token| &token.0) == Some(&Token::Symbol(symbol))
    }
    fn bump(&mut self) {
        if self.at + 1 < self.tokens.len() {
            self.at += 1;
        }
    }
    fn is_ident(&self, expected: &str) -> bool {
        matches!(self.current(), Token::Ident(v) if v == expected)
    }
    fn ident(&mut self) -> Result<String, ParseError> {
        match self.current().clone() {
            Token::Ident(v) => {
                self.bump();
                Ok(v)
            }
            found => Err(self.error(format!("expected identifier, found {found:?}"))),
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
    fn expect(&mut self, symbol: char) -> Result<(), ParseError> {
        if self.eat(symbol) {
            Ok(())
        } else {
            Err(self.error(format!("expected `{symbol}`, found {:?}", self.current())))
        }
    }

    fn component(mut self) -> Result<Component, ParseError> {
        while self.is_ident("import") {
            self.bump();
            self.expect('{')?;
            loop {
                let widget = self.ident()?;
                if !matches!(
                    widget.as_str(),
                    "Button" | "LineEdit" | "CheckBox" | "Slider"
                ) {
                    return Err(self.error(format!("unsupported imported widget `{widget}`")));
                }
                if self.eat('}') {
                    break;
                }
                self.expect(',')?;
                if self.eat('}') {
                    break;
                }
            }
            if self.ident()? != "from" {
                return Err(self.error("expected `from` in import"));
            }
            if self.value()? != Value::String("std-widgets.slint".into()) {
                return Err(self.error("only std-widgets.slint imports are supported"));
            }
            self.expect(';')?;
        }
        if self.is_ident("export") {
            self.bump();
        }
        if self.ident()? != "component" {
            return Err(self.error("expected `component` declaration"));
        }
        let name = self.ident()?;
        let mut root_tag = "div";
        if self.is_ident("inherits") {
            self.bump();
            let base = self.ident()?;
            if base == "Window" {
                root_tag = "main";
            } else {
                return Err(self.error(format!("unsupported component base `{base}`")));
            }
        }
        self.expect('{')?;
        let mut properties = Vec::new();
        let mut callbacks = Vec::new();
        let mut children = Vec::new();
        let mut title = None;
        while !self.eat('}') {
            if self.current() == &Token::End {
                return Err(self.error("unclosed component body"));
            }
            if self.is_ident("property") {
                properties.push(self.property()?);
                continue;
            }
            if self.is_ident("callback") {
                callbacks.push(self.callback()?);
                continue;
            }
            let first_offset = self.offset();
            let first = self.ident()?;
            if self.current() == &Token::Symbol(':') && self.next_is('=') {
                children.push(self.element(first, first_offset)?);
            } else if self.eat(':') {
                if first != "title" {
                    return Err(self.error(format!("unsupported component property `{first}`")));
                }
                if title.is_some() {
                    return Err(self.error("component `title` is assigned more than once"));
                }
                match self.value()? {
                    Value::String(value) => title = Some(value),
                    _ => return Err(self.error("component `title` requires a string literal")),
                }
                self.expect(';')?;
            } else {
                children.push(self.element(first, first_offset)?);
            }
        }
        if self.current() != &Token::End {
            return Err(self.error(format!(
                "unexpected tokens after component: {:?}",
                self.current()
            )));
        }
        for property in &properties {
            validate_value_type(
                &property.initial,
                property.kind,
                &format!("property `{}`", property.name),
            )
            .map_err(|message| self.error(message))?;
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

    fn property(&mut self) -> Result<Property, ParseError> {
        self.bump();
        self.expect('<')?;
        let ty = self.ident()?;
        self.expect('>')?;
        let kind = match ty.as_str() {
            "string" => PropertyKind::String,
            "bool" => PropertyKind::Bool,
            "int" => PropertyKind::Int,
            "float" => PropertyKind::Float,
            _ => return Err(self.error(format!("unsupported property type `{ty}`"))),
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

    fn callback(&mut self) -> Result<String, ParseError> {
        self.bump();
        let name = self.ident()?;
        self.expect('(')?;
        if !self.eat(')') {
            return Err(self.error(format!(
                "callback `{name}` has arguments; only zero-argument callbacks are supported"
            )));
        }
        self.expect(';')?;
        Ok(name)
    }

    fn element(&mut self, first: String, first_offset: usize) -> Result<Element, ParseError> {
        let (id, kind, kind_offset) = if self.eat(':') {
            self.expect('=')?;
            let kind_offset = self.offset();
            (Some(first), self.ident()?, kind_offset)
        } else {
            (None, first, first_offset)
        };
        if !self.eat('{') {
            return Err(self.error_at(
                kind_offset,
                format!(
                    "expected `{{` after element `{kind}`, found {:?}",
                    self.current()
                ),
            ));
        }
        let mut properties = Vec::new();
        let mut handlers = Vec::new();
        let mut children = Vec::new();
        while !self.eat('}') {
            if self.current() == &Token::End {
                return Err(self.error(format!("unclosed `{kind}` element")));
            }
            let name_offset = self.offset();
            let name = self.ident()?;
            if self.current() == &Token::Symbol(':') && self.next_is('=') {
                children.push(self.element(name, name_offset)?);
            } else if self.eat(':') {
                let value = self.value()?;
                self.expect(';')?;
                properties.push((name, value));
            } else if self.current() == &Token::Arrow {
                self.bump();
                handlers.push(self.handler(name)?);
            } else {
                children.push(self.element(name, name_offset)?);
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

    fn handler(&mut self, event: String) -> Result<Handler, ParseError> {
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

    fn value(&mut self) -> Result<Value, ParseError> {
        if self.eat('!') {
            return Ok(Value::NotIdentifier(self.ident()?));
        }
        let value = match self.current().clone() {
            Token::String(v) => Value::String(v),
            Token::Ident(v) if v == "true" => Value::Bool(true),
            Token::Ident(v) if v == "false" => Value::Bool(false),
            Token::Ident(v) => Value::Identifier(v),
            Token::Number(v) => Value::Number(v),
            found => return Err(self.error(format!("expected value, found {found:?}"))),
        };
        self.bump();
        Ok(value)
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
    fn rejects_unresolved_imports() {
        for source in [
            "import { Button } from \"custom.slint\"; export component App {}",
            "import garbage; export component App {}",
            "import { Unknown } from \"std-widgets.slint\"; export component App {}",
        ] {
            assert!(parse(source).is_err());
        }
    }
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
        println!("------------------ Result: {:?}", parse("export component App { mystery: true; }"));
        assert!(parse("export component App { mystery: true; }")
            .unwrap_err()
            .contains("unsupported"));
    }

    #[test]
    fn rejects_unsupported_elements() {
        let input = "export component App { Text { text: \"invalid\"; } }";
        let r = parse(
            &input);

        //        println!("------------------ Result: {r:?}");

        match r {
            Ok(_) => panic!("-------------- expected error"),
            Err(e) => assert!(e.contains("unsupported Slint element")),
        }
    }    
}
