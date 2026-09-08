#![cfg(target_arch = "wasm32")]

use slint_dom::{Callback, DomBuilder, Property};
use std::{cell::Cell, rc::Rc};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::*;
use web_sys::{Event, HtmlInputElement};

wasm_bindgen_test_configure!(run_in_browser);

#[wasm_bindgen_test]
fn property_updates_dom_text() -> Result<(), JsValue> {
    let dom = DomBuilder::from_browser()?;
    let text = dom.element("span", "test-text")?;
    let property = Property::new("first".to_owned());
    dom.bind_text(&text, &property);
    assert_eq!(text.text_content().as_deref(), Some("first"));
    property.set("second".to_owned());
    assert_eq!(text.text_content().as_deref(), Some("second"));
    Ok(())
}

#[wasm_bindgen_test]
fn input_binding_is_two_way() -> Result<(), JsValue> {
    let dom = DomBuilder::from_browser()?;
    let element = dom.element("input", "test-input")?;
    let input = element.clone().dyn_into::<HtmlInputElement>()?;
    let property = Property::new("from-rust".to_owned());
    let _binding = dom.bind_input(&element, &property)?;
    assert_eq!(input.value(), "from-rust");

    input.set_value("from-browser");
    input.dispatch_event(&Event::new("input")?)?;
    assert_eq!(property.get(), "from-browser");
    Ok(())
}

#[wasm_bindgen_test]
fn listener_invokes_callback_and_unregisters_on_drop() -> Result<(), JsValue> {
    let dom = DomBuilder::from_browser()?;
    let button = dom.element("button", "test-button")?;
    let count = Rc::new(Cell::new(0));
    let callback = Callback::default();
    let output = count.clone();
    callback.set(move || output.set(output.get() + 1));

    let binding = dom.listen(&button, "click", callback)?;
    button.dispatch_event(&Event::new("click")?)?;
    assert_eq!(count.get(), 1);

    drop(binding);
    button.dispatch_event(&Event::new("click")?)?;
    assert_eq!(count.get(), 1);
    Ok(())
}
