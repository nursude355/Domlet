#![cfg(target_arch = "wasm32")]

use domlet::{Callback, DomBuilder, Property};
use std::{cell::Cell, rc::Rc};
use wasm_bindgen::{JsCast, JsValue};
use wasm_bindgen_test::*;
use web_sys::{Event, HtmlInputElement};

wasm_bindgen_test_configure!(run_in_browser);

domlet::include_ui!("tests/ui/lifecycle.slint");

#[wasm_bindgen_test]
fn generated_events_respect_enter_and_disabled_state() -> Result<(), JsValue> {
    let dom = DomBuilder::from_browser()?;
    let host = dom.element("div", "host")?;
    let app = Lifecycle::mount(&host)?;
    let count = Rc::new(Cell::new(0));
    let captured = count.clone();
    app.on_start(move || captured.set(captured.get() + 1));
    app.entry().dispatch_event(&Event::new("change")?)?;
    app.touch().dispatch_event(&Event::new("click")?)?;
    assert_eq!(count.get(), 0);
    let options = web_sys::KeyboardEventInit::new();
    options.set_key("Enter");
    let event = web_sys::KeyboardEvent::new_with_keyboard_event_init_dict("keydown", &options)?;
    app.entry().dispatch_event(&event)?;
    assert_eq!(count.get(), 1);
    assert_eq!(
        app.button().get_attribute("type").as_deref(),
        Some("button")
    );
    app.unmount();
    Ok(())
}

#[wasm_bindgen_test]
fn generated_component_updates_and_unmounts_cleanly() -> Result<(), JsValue> {
    let dom = DomBuilder::from_browser()?;
    let host = dom.element("div", "host")?;
    dom.append(&dom.body()?, &host)?;
    let app = Lifecycle::mount(&host)?;
    let status = app.status_property();
    let label = app.label().clone();
    let button = app.button().clone();
    let callback_status = status.clone();
    app.on_start(move || callback_status.set("Running".into()));
    button.dispatch_event(&Event::new("click")?)?;
    assert_eq!(label.text_content().as_deref(), Some("Running"));
    let input = app.entry().clone().dyn_into::<HtmlInputElement>()?;
    assert_eq!(input.value(), "Running");
    input.set_value("Edited");
    input.dispatch_event(&Event::new("input")?)?;
    assert_eq!(app.status(), "Edited");
    let toggle = app.toggle().clone().dyn_into::<HtmlInputElement>()?;
    toggle.set_checked(true);
    toggle.dispatch_event(&Event::new("change")?)?;
    assert!(app.active());
    let slider = app.slider().clone().dyn_into::<HtmlInputElement>()?;
    slider.set_value("73.5");
    slider.dispatch_event(&Event::new("input")?)?;
    assert_eq!(app.progress(), 73.5);
    assert!(app.inactive().has_attribute("hidden"));
    app.unmount();
    assert!(!host.has_child_nodes());
    status.set("After unmount".into());
    assert_eq!(label.text_content().as_deref(), Some("Edited"));
    assert_eq!(input.value(), "Edited");
    button.dispatch_event(&Event::new("click")?)?;
    assert_eq!(status.get(), "After unmount");
    host.remove();
    Ok(())
}

#[wasm_bindgen_test]
fn property_updates_dom_text() -> Result<(), JsValue> {
    let dom = DomBuilder::from_browser()?;
    let text = dom.element("span", "test-text")?;
    let property = Property::new("first".to_owned());
    let _subscription = dom.bind_text(&text, &property);
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
fn input_event_fired_during_notification_is_ignored() -> Result<(), JsValue> {
    let dom = DomBuilder::from_browser()?;
    let element = dom.element("input", "test-checkbox")?;
    element.set_attribute("type", "checkbox")?;
    let property = Property::new(false);
    let _binding = dom.bind_checked(&element, &property)?;
    let target = element.clone();
    // Re-firing the bound input's event from an observer used to panic.
    let _feedback = property.observe(move |_| {
        let _ = target.dispatch_event(&Event::new("change").unwrap());
    });
    property.set(true);
    assert!(property.get());
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
