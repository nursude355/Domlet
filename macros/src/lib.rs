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
    let component = parser::parse(&source)?;
    generate::component(&component, &LitStr::new(relative, Span::call_site()))
}
