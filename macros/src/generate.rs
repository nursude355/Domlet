use crate::ast::{Component, Element, PropertyKind, Value};
use proc_macro2::TokenStream;
use quote::{format_ident, quote};
use std::collections::{HashMap, HashSet};
use syn::LitStr;

pub fn component(component: &Component, source_path: &LitStr) -> Result<TokenStream, String> {
    let component_name = rust_ident(&component.name)?;
    validate(component)?;

    let mut property_types = HashMap::new();
    let mut fields = Vec::new();
    let mut initializers = Vec::new();
    let mut constructors = Vec::new();
    let mut methods = Vec::new();
    for property in &component.properties {
        let name = rust_ident(&property.name)?;
        let setter = format_ident!("set_{}", normalized(&property.name));
        let property_handle = format_ident!("{}_property", normalized(&property.name));
        let ty = rust_type(property.kind);
        let initial = literal(&property.initial, property.kind)?;
        property_types.insert(property.name.clone(), property.kind);
        fields.push(quote!(#name: ::slint_dom::Property<#ty>));
        initializers.push(quote!(let #name = ::slint_dom::Property::new(#initial);));
        constructors.push(quote!(#name));
        methods.push(quote! {
            pub fn #name(&self) -> #ty { self.#name.get() }
            pub fn #setter(&self, value: #ty) { self.#name.set(value); }
            pub fn #property_handle(&self) -> ::slint_dom::Property<#ty> { self.#name.clone() }
        });
    }

    let mut callback_names = HashSet::new();
    for callback in &component.callbacks {
        let name = rust_ident(callback)?;
        let on_name = format_ident!("on_{}", normalized(callback));
        callback_names.insert(callback.clone());
        fields.push(quote!(#name: ::slint_dom::Callback));
        initializers.push(quote!(let #name = ::slint_dom::Callback::default();));
        constructors.push(quote!(#name));
        methods.push(quote! {
            pub fn #on_name(&self, handler: impl FnMut() + 'static) { self.#name.set(handler); }
        });
    }

    let mut ids = Vec::new();
    collect_ids(&component.children, &mut ids);
    for id in &ids {
        let name = rust_ident(id)?;
        fields.push(quote!(#name: ::slint_dom::__private::Element));
        constructors.push(quote!(#name));
        methods.push(quote! {
            pub fn #name(&self) -> &::slint_dom::__private::Element { &self.#name }
        });
    }

    let mut sequence = 0;
    let nodes = emit_nodes(
        &component.children,
        quote!(root),
        &property_types,
        &callback_names,
        &mut sequence,
    )?;
    let root_tag = component.root_tag;
    let title = component
        .title
        .as_ref()
        .map(|title| quote!(dom.set_title(#title);));

    Ok(quote! {
        const _: &str = include_str!(concat!(env!("CARGO_MANIFEST_DIR"), "/", #source_path));

        pub struct #component_name {
            root: ::slint_dom::__private::Element,
            #(#fields,)*
            _events: ::std::vec::Vec<::slint_dom::EventBinding>,
        }

        impl #component_name {
            pub fn mount(parent: &::slint_dom::__private::Element) -> Result<Self, ::slint_dom::__private::JsValue> {
                let dom = ::slint_dom::DomBuilder::from_browser()?;
                dom.install_default_style()?;
                #title
                #(#initializers)*
                let mut events = ::std::vec::Vec::new();
                let root = dom.element(#root_tag, "sd-component")?;
                #nodes
                dom.append(parent, &root)?;
                Ok(Self { root, #(#constructors,)* _events: events })
            }

            pub fn mount_to_body() -> Result<Self, ::slint_dom::__private::JsValue> {
                let dom = ::slint_dom::DomBuilder::from_browser()?;
                let body = dom.body()?;
                Self::mount(&body)
            }

            pub fn root(&self) -> &::slint_dom::__private::Element { &self.root }
            pub fn unmount(self) { self.root.remove(); }
            #(#methods)*
        }
    })
}

fn emit_nodes(
    nodes: &[Element],
    parent: TokenStream,
    properties: &HashMap<String, PropertyKind>,
    callbacks: &HashSet<String>,
    sequence: &mut usize,
) -> Result<TokenStream, String> {
    let mut output = TokenStream::new();
    for node in nodes {
        let index = *sequence;
        *sequence += 1;
        let variable = node
            .id
            .as_ref()
            .map(|id| rust_ident(id))
            .transpose()?
            .unwrap_or_else(|| format_ident!("__node_{index}"));
        let spec = widget(&node.kind)?;
        if matches!(spec.tag, "input" | "img") && !node.children.is_empty() {
            return Err(format!(
                "void element `{}` cannot contain children",
                node.kind
            ));
        }
        let mut seen_properties = HashSet::new();
        let mut seen_events = HashSet::new();
        let mut setup = TokenStream::new();
        if let Some(input_type) = spec.input_type {
            setup.extend(quote!(dom.attribute(&#variable, "type", #input_type)?;));
        }

        for (name, value) in &node.properties {
            if !seen_properties.insert(name.as_str()) {
                return Err(format!(
                    "property `{name}` is assigned more than once on `{}`",
                    node.kind
                ));
            }
            if !spec.properties.contains(&name.as_str())
                && !COMMON_PROPERTIES.contains(&name.as_str())
            {
                return Err(format!(
                    "property `{name}` is not supported on `{}`",
                    node.kind
                ));
            }
            setup.extend(emit_property(
                &variable, &node.kind, name, value, properties,
            )?);
        }
        for handler in &node.handlers {
            if !seen_events.insert(handler.event.as_str()) {
                return Err(format!(
                    "event `{}` is handled more than once on `{}`",
                    handler.event, node.kind
                ));
            }
            if !callbacks.contains(&handler.callback) {
                return Err(format!(
                    "event `{}` references undeclared callback `{}`",
                    handler.event, handler.callback
                ));
            }
            let event = event_name(&node.kind, &handler.event)?;
            let callback = rust_ident(&handler.callback)?;
            setup.extend(quote!(events.push(dom.listen(&#variable, #event, #callback.clone())?);));
        }
        let children = emit_nodes(
            &node.children,
            quote!(#variable),
            properties,
            callbacks,
            sequence,
        )?;
        let tag = spec.tag;
        let class = spec.class;
        output.extend(quote! {
            let #variable = dom.element(#tag, #class)?;
            #setup
            #children
            dom.append(&#parent, &#variable)?;
        });
    }
    Ok(output)
}

fn emit_property(
    variable: &syn::Ident,
    kind: &str,
    name: &str,
    value: &Value,
    properties: &HashMap<String, PropertyKind>,
) -> Result<TokenStream, String> {
    match name {
        "text" => match value {
            Value::String(text) if matches!(kind, "LineEdit" | "TextInput") => {
                Ok(quote!(dom.attribute(&#variable, "value", #text)?;))
            }
            Value::String(text) => Ok(quote!(dom.text(&#variable, #text);)),
            Value::Identifier(binding) => {
                require_binding(properties, binding, PropertyKind::String, name)?;
                let binding = rust_ident(binding)?;
                if matches!(kind, "LineEdit" | "TextInput") {
                    Ok(quote!(events.push(dom.bind_input(&#variable, &#binding)?);))
                } else {
                    Ok(quote!(dom.bind_text(&#variable, &#binding);))
                }
            }
            _ => Err(format!(
                "`text` on `{kind}` requires a string or string property"
            )),
        },
        "enabled" | "visible" => {
            let attribute = if name == "enabled" {
                "disabled"
            } else {
                "hidden"
            };
            match value {
                Value::Bool(value) => {
                    let present = !value;
                    Ok(quote!(dom.boolean_attribute(&#variable, #attribute, #present);))
                }
                Value::Identifier(binding) => {
                    require_binding(properties, binding, PropertyKind::Bool, name)?;
                    let binding = rust_ident(binding)?;
                    if name == "enabled" {
                        Ok(quote!(dom.bind_enabled(&#variable, &#binding);))
                    } else {
                        Ok(quote!(dom.bind_visible(&#variable, &#binding);))
                    }
                }
                _ => Err(format!("`{name}` requires a bool or bool property")),
            }
        }
        "checked" => match value {
            Value::Bool(v) => Ok(quote!(dom.boolean_attribute(&#variable, "checked", #v);)),
            Value::Identifier(binding) => {
                require_binding(properties, binding, PropertyKind::Bool, name)?;
                let binding = rust_ident(binding)?;
                Ok(quote!(events.push(dom.bind_checked(&#variable, &#binding)?);))
            }
            _ => Err("`checked` requires a bool or bool property".into()),
        },
        "placeholder-text" => string_attribute(variable, "placeholder", value),
        "source" => string_attribute(variable, "src", value),
        "value" | "minimum" | "maximum" => scalar_attribute(
            variable,
            match name {
                "minimum" => "min",
                "maximum" => "max",
                other => other,
            },
            value,
        ),
        "width" | "height" | "min-width" | "min-height" | "max-width" | "max-height"
        | "padding" | "spacing" | "background" | "border-radius" => {
            let css_name = if name == "spacing" { "gap" } else { name };
            let value = static_value(value)?;
            validate_css_value(css_name, &value)?;
            Ok(quote!(dom.style(&#variable, #css_name, #value)?;))
        }
        _ => Err(format!("unsupported property `{name}`")),
    }
}

fn string_attribute(
    variable: &syn::Ident,
    attribute: &str,
    value: &Value,
) -> Result<TokenStream, String> {
    match value {
        Value::String(v) => Ok(quote!(dom.attribute(&#variable, #attribute, #v)?;)),
        _ => Err(format!("`{attribute}` requires a string literal")),
    }
}
fn scalar_attribute(
    variable: &syn::Ident,
    attribute: &str,
    value: &Value,
) -> Result<TokenStream, String> {
    let value = match value {
        Value::Number(value) if value.parse::<f64>().is_ok() => value.clone(),
        _ => return Err(format!("`{attribute}` requires a unitless number literal")),
    };
    Ok(quote!(dom.attribute(&#variable, #attribute, #value)?;))
}

fn validate_css_value(name: &str, value: &str) -> Result<(), String> {
    if name == "background" {
        if let Some(digits) = value.strip_prefix('#') {
            if !matches!(digits.len(), 3 | 4 | 6 | 8)
                || !digits.chars().all(|c| c.is_ascii_hexdigit())
            {
                return Err(format!("invalid CSS color `{value}`"));
            }
        }
        return Ok(());
    }
    if value.parse::<f64>().is_ok() {
        return Ok(());
    }
    for unit in ["px", "rem", "em", "%", "vh", "vw"] {
        if value
            .strip_suffix(unit)
            .is_some_and(|number| number.parse::<f64>().is_ok())
        {
            return Ok(());
        }
    }
    Err(format!("invalid CSS size `{value}` for `{name}`"))
}
fn static_value(value: &Value) -> Result<String, String> {
    match value {
        Value::String(v) | Value::Number(v) => Ok(v.clone()),
        Value::Bool(v) => Ok(v.to_string()),
        Value::Identifier(v) => Err(format!(
            "dynamic binding `{v}` is not supported for this property"
        )),
    }
}

fn require_binding(
    properties: &HashMap<String, PropertyKind>,
    binding: &str,
    expected: PropertyKind,
    target: &str,
) -> Result<(), String> {
    match properties.get(binding) {
        Some(actual) if *actual == expected => Ok(()),
        Some(_) => Err(format!(
            "property `{binding}` has the wrong type for `{target}`"
        )),
        None => Err(format!("unknown property binding `{binding}`")),
    }
}

fn validate(component: &Component) -> Result<(), String> {
    rust_ident(&component.name)?;
    let mut fields: HashSet<String> = ["root", "_events"].into_iter().map(str::to_owned).collect();
    let mut methods: HashSet<String> = ["mount", "mount_to_body", "root", "unmount"]
        .into_iter()
        .map(str::to_owned)
        .collect();
    for property in &component.properties {
        rust_ident(&property.name)?;
        let name = normalized(&property.name);
        if !fields.insert(name.clone()) {
            return Err(format!("duplicate component member `{}`", property.name));
        }
        for method in [&name, &format!("set_{name}"), &format!("{name}_property")] {
            if !methods.insert(method.clone()) {
                return Err(format!(
                    "generated method name `{method}` is used more than once"
                ));
            }
        }
    }
    for callback in &component.callbacks {
        rust_ident(callback)?;
        let name = normalized(callback);
        if !fields.insert(name.clone()) {
            return Err(format!("duplicate component member `{callback}`"));
        }
        let method = format!("on_{name}");
        if !methods.insert(method.clone()) {
            return Err(format!(
                "generated method name `{method}` is used more than once"
            ));
        }
    }
    let mut ids = Vec::new();
    collect_ids(&component.children, &mut ids);
    for id in ids {
        rust_ident(&id)?;
        let name = normalized(&id);
        if !fields.insert(name.clone()) || !methods.insert(name) {
            return Err(format!("duplicate element id or component member `{id}`"));
        }
    }
    Ok(())
}

fn collect_ids(elements: &[Element], output: &mut Vec<String>) {
    for element in elements {
        if let Some(id) = &element.id {
            output.push(id.clone());
        }
        collect_ids(&element.children, output);
    }
}

fn rust_ident(name: &str) -> Result<syn::Ident, String> {
    syn::parse_str::<syn::Ident>(&normalized(name))
        .map_err(|_| format!("`{name}` cannot be exposed as a Rust identifier"))
}
fn normalized(name: &str) -> String {
    name.replace('-', "_")
}

fn rust_type(kind: PropertyKind) -> TokenStream {
    match kind {
        PropertyKind::String => quote!(::std::string::String),
        PropertyKind::Bool => quote!(bool),
        PropertyKind::Int => quote!(i32),
        PropertyKind::Float => quote!(f64),
    }
}
fn literal(value: &Value, kind: PropertyKind) -> Result<TokenStream, String> {
    match (value, kind) {
        (Value::String(v), PropertyKind::String) => Ok(quote!(::std::string::String::from(#v))),
        (Value::Bool(v), PropertyKind::Bool) => Ok(quote!(#v)),
        (Value::Number(v), PropertyKind::Int) => v
            .parse::<i32>()
            .map(|v| quote!(#v))
            .map_err(|_| format!("`{v}` is not a valid int")),
        (Value::Number(v), PropertyKind::Float) => v
            .parse::<f64>()
            .map(|v| quote!(#v))
            .map_err(|_| format!("`{v}` is not a valid float")),
        _ => Err("property initial value has the wrong type".into()),
    }
}

struct Widget {
    tag: &'static str,
    class: &'static str,
    input_type: Option<&'static str>,
    properties: &'static [&'static str],
}
const COMMON_PROPERTIES: &[&str] = &[
    "enabled",
    "visible",
    "width",
    "height",
    "min-width",
    "min-height",
    "max-width",
    "max-height",
    "background",
    "border-radius",
];

fn widget(kind: &str) -> Result<Widget, String> {
    let result = match kind {
        "VerticalLayout" => Widget {
            tag: "div",
            class: "sd-column",
            input_type: None,
            properties: &["padding", "spacing"],
        },
        "HorizontalLayout" => Widget {
            tag: "div",
            class: "sd-row",
            input_type: None,
            properties: &["padding", "spacing"],
        },
        "GridLayout" => Widget {
            tag: "div",
            class: "sd-grid",
            input_type: None,
            properties: &["padding", "spacing"],
        },
        "Text" => Widget {
            tag: "span",
            class: "sd-text",
            input_type: None,
            properties: &["text"],
        },
        "Button" => Widget {
            tag: "button",
            class: "sd-button",
            input_type: None,
            properties: &["text"],
        },
        "LineEdit" | "TextInput" => Widget {
            tag: "input",
            class: "sd-input",
            input_type: Some("text"),
            properties: &["text", "placeholder-text"],
        },
        "CheckBox" => Widget {
            tag: "input",
            class: "sd-checkbox",
            input_type: Some("checkbox"),
            properties: &["checked"],
        },
        "Slider" => Widget {
            tag: "input",
            class: "sd-slider",
            input_type: Some("range"),
            properties: &["value", "minimum", "maximum"],
        },
        "Image" => Widget {
            tag: "img",
            class: "sd-image",
            input_type: None,
            properties: &["source"],
        },
        "Rectangle" => Widget {
            tag: "div",
            class: "sd-rectangle",
            input_type: None,
            properties: &[],
        },
        "TouchArea" => Widget {
            tag: "div",
            class: "sd-touch",
            input_type: None,
            properties: &[],
        },
        other => return Err(format!("unsupported Slint element `{other}`")),
    };
    Ok(result)
}

fn event_name(kind: &str, event: &str) -> Result<&'static str, String> {
    match (kind, event) {
        ("Button" | "TouchArea", "clicked") => Ok("click"),
        ("LineEdit" | "TextInput", "accepted") => Ok("change"),
        ("LineEdit" | "TextInput", "edited") => Ok("input"),
        ("CheckBox", "toggled") => Ok("change"),
        _ => Err(format!("event `{event}` is not supported on `{kind}`")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser;
    #[test]
    fn output_contains_state_callback_and_real_dom_calls() {
        let input = parser::parse(r#"export component App inherits Window { property <bool> enabled: true; callback go(); Button { enabled: enabled; clicked => { root.go(); } } }"#).unwrap();
        let output = component(
            &input,
            &LitStr::new("ui.slint", proc_macro2::Span::call_site()),
        )
        .unwrap()
        .to_string();
        assert!(
            output.contains("set_enabled")
                && output.contains("on_go")
                && output.contains("bind_enabled")
                && output.contains("dom . listen")
        );
    }

    #[test]
    fn rejects_generated_api_name_collisions() {
        let input =
            parser::parse("export component App { property <bool> root: true; Rectangle {} }")
                .unwrap();
        let error = component(
            &input,
            &LitStr::new("ui.slint", proc_macro2::Span::call_site()),
        )
        .unwrap_err();
        assert!(error.contains("duplicate component member"));
    }

    #[test]
    fn validates_css_sizes_and_colors() {
        assert!(validate_css_value("width", "12px").is_ok());
        assert!(validate_css_value("width", "12oops").is_err());
        assert!(validate_css_value("background", "#12xx00").is_err());
    }

    #[test]
    fn rejects_children_of_void_elements() {
        let input =
            parser::parse("export component App { LineEdit { Text { text: \"invalid\"; } } }")
                .unwrap();
        let error = component(
            &input,
            &LitStr::new("ui.slint", proc_macro2::Span::call_site()),
        )
        .unwrap_err();
        assert!(error.contains("cannot contain children"));
    }
}
