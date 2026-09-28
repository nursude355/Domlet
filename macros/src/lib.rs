use proc_macro::TokenStream;
use proc_macro2::Span;
use std::{env, fs, path::PathBuf};
use syn::{parse_macro_input, LitStr};
mod ast;
mod generate;
mod lexer;
mod parser;

#[proc_macro]
pub fn include_ui(input: TokenStream) -> TokenStream {
    let relative = parse_macro_input!(input as LitStr);
    match expand(&relative.value()) {
        Ok(tokens) => tokens.into(),
        Err(message) => syn::Error::new(relative.span(), message)
            .to_compile_error()
            .into(),
    }
}

fn expand(relative: &str) -> Result<proc_macro2::TokenStream, String> {
    let manifest = env::var_os("CARGO_MANIFEST_DIR").ok_or("CARGO_MANIFEST_DIR is unavailable")?;
    let path = PathBuf::from(manifest).join(relative);
    let source = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    let component =
        parser::parse(&source).map_err(|error| format_source_error(relative, &source, error))?;
    let tokens = generate::component(&component, &LitStr::new(relative, Span::call_site()));
    match tokens {
        Ok(t) => Ok(t),
        Err(error) => {
            let message = format_diagnostic(relative, &source, error.offset, &error.message);
            let stars = "*".repeat(message.lines().next().map_or(20, |line| line.len()) + 2);
            Err(format!("slint-dom:\n{}\n{}\n{}", stars, message, stars))
        }
    }
}

fn format_source_error(relative: &str, source: &str, error: parser::ParseError) -> String {
    format_diagnostic(relative, source, error.offset, &error.message)
}

fn format_diagnostic(relative: &str, source: &str, offset: usize, message: &str) -> String {
    let mut line: usize = 1;
    let mut column: usize = 1;
    let mut line_start: usize = 0;
    for (index, character) in source.chars().enumerate() {
        if index == offset {
            break;
        }
        if character == '\n' {
            line += 1;
            column = 1;
            line_start = index + 1;
        } else {
            column += 1;
        }
    }
    let source_line: String = source
        .chars()
        .skip(line_start)
        .take_while(|character| *character != '\n' && *character != '\r')
        .collect();
    let gutter_width = line.to_string().len();
    format!(
        "{relative}:{line}:{column}: {}\n{:gutter_width$} |\n{line:gutter_width$} | {source_line}\n{:gutter_width$} | {}^",
        message,
        "",
        "",
        " ".repeat(column.saturating_sub(1).try_into().unwrap()),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parser_diagnostics_name_the_slint_line_and_column() {
        let source = "export component App {\n    Text { text: status; XXX}\n}";
        let error = parser::parse(source).unwrap_err();
        let diagnostic = format_source_error("ui/main.slint", source, error);
        assert!(diagnostic.contains("ui/main.slint:2:"), "{diagnostic}");
        assert!(
            diagnostic.contains("Text { text: status; XXX}"),
            "{diagnostic}"
        );
        assert!(diagnostic.contains('^'), "{diagnostic}");
    }
}
