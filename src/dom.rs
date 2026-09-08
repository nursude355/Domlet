use crate::{Callback, Property};
use wasm_bindgen::{closure::Closure, JsCast, JsValue};
use web_sys::{Document, Element, Event, HtmlInputElement};

/// Low-level DOM construction context used by generated components.
pub struct DomBuilder {
    document: Document,
}

impl DomBuilder {
    pub fn from_browser() -> Result<Self, JsValue> {
        let window = web_sys::window().ok_or_else(|| JsValue::from_str("window is unavailable"))?;
        let document = window
            .document()
            .ok_or_else(|| JsValue::from_str("document is unavailable"))?;
        Ok(Self { document })
    }

    pub fn element(&self, tag: &str, class: &str) -> Result<Element, JsValue> {
        let element = self.document.create_element(tag)?;
        element.set_class_name(class);
        Ok(element)
    }
    pub fn set_title(&self, title: &str) {
        self.document.set_title(title);
    }
    pub fn text(&self, element: &Element, value: &str) {
        element.set_text_content(Some(value));
    }
    pub fn attribute(&self, element: &Element, name: &str, value: &str) -> Result<(), JsValue> {
        element.set_attribute(name, value)
    }
    pub fn boolean_attribute(&self, element: &Element, name: &str, value: bool) {
        if value {
            let _ = element.set_attribute(name, "");
        } else {
            let _ = element.remove_attribute(name);
        }
    }
    pub fn style(&self, element: &Element, name: &str, value: &str) -> Result<(), JsValue> {
        element
            .dyn_ref::<web_sys::HtmlElement>()
            .ok_or_else(|| JsValue::from_str("element has no style"))?
            .style()
            .set_property(name, value)
    }
    pub fn append(&self, parent: &Element, child: &Element) -> Result<(), JsValue> {
        parent.append_child(child).map(|_| ())
    }
    pub fn body(&self) -> Result<Element, JsValue> {
        self.document
            .body()
            .map(Into::into)
            .ok_or_else(|| JsValue::from_str("document body is unavailable"))
    }
    pub fn bind_text(&self, element: &Element, property: &Property<String>) {
        let element = element.clone();
        property.observe(move |value| element.set_text_content(Some(value)));
    }
    pub fn bind_input(
        &self,
        element: &Element,
        property: &Property<String>,
    ) -> Result<EventBinding, JsValue> {
        let input = element.clone().dyn_into::<HtmlInputElement>()?;
        let observed_input = input.clone();
        property.observe(move |value| {
            if observed_input.value() != *value {
                observed_input.set_value(value);
            }
        });
        let property = property.clone();
        EventBinding::new(element, "input", move |_| property.set(input.value()))
    }
    pub fn bind_checked(
        &self,
        element: &Element,
        property: &Property<bool>,
    ) -> Result<EventBinding, JsValue> {
        let input = element.clone().dyn_into::<HtmlInputElement>()?;
        let observed_input = input.clone();
        property.observe(move |value| observed_input.set_checked(*value));
        let property = property.clone();
        EventBinding::new(element, "change", move |_| property.set(input.checked()))
    }
    pub fn bind_enabled(&self, element: &Element, property: &Property<bool>) {
        let element = element.clone();
        property.observe(move |enabled| {
            if *enabled {
                let _ = element.remove_attribute("disabled");
            } else {
                let _ = element.set_attribute("disabled", "");
            }
        });
    }
    pub fn bind_visible(&self, element: &Element, property: &Property<bool>) {
        let element = element.clone();
        property.observe(move |visible| {
            if *visible {
                let _ = element.remove_attribute("hidden");
            } else {
                let _ = element.set_attribute("hidden", "");
            }
        });
    }
    pub fn listen(
        &self,
        element: &Element,
        event: &str,
        callback: Callback,
    ) -> Result<EventBinding, JsValue> {
        EventBinding::new(element, event, move |_| callback.invoke())
    }
    pub fn install_default_style(&self) -> Result<(), JsValue> {
        const ID: &str = "slint-dom-style";
        if self.document.get_element_by_id(ID).is_some() {
            return Ok(());
        }
        let style = self.document.create_element("style")?;
        style.set_id(ID);
        style.set_text_content(Some(DEFAULT_CSS));
        let head = self
            .document
            .head()
            .ok_or_else(|| JsValue::from_str("document head is unavailable"))?;
        head.append_child(&style).map(|_| ())
    }
}

/// Keeps a browser event listener alive and unregisters it when dropped.
pub struct EventBinding {
    element: Element,
    event: String,
    closure: Closure<dyn FnMut(Event)>,
}

impl EventBinding {
    fn new(
        element: &Element,
        event: &str,
        handler: impl FnMut(Event) + 'static,
    ) -> Result<Self, JsValue> {
        let closure = Closure::wrap(Box::new(handler) as Box<dyn FnMut(Event)>);
        element.add_event_listener_with_callback(event, closure.as_ref().unchecked_ref())?;
        Ok(Self {
            element: element.clone(),
            event: event.into(),
            closure,
        })
    }
}

impl Drop for EventBinding {
    fn drop(&mut self) {
        let _ = self.element.remove_event_listener_with_callback(
            &self.event,
            self.closure.as_ref().unchecked_ref(),
        );
    }
}

const DEFAULT_CSS: &str = r#"
.sd-component { box-sizing: border-box; width: 100%; min-height: 100%; font-family: system-ui, sans-serif; }
.sd-column, .sd-row, .sd-grid { display: flex; box-sizing: border-box; gap: .5rem; }
.sd-column { flex-direction: column; }
.sd-row { flex-direction: row; align-items: center; }
.sd-grid { display: grid; }
.sd-button { cursor: pointer; }
.sd-button:disabled { cursor: default; opacity: .55; }
.sd-input, .sd-slider, .sd-rectangle { box-sizing: border-box; }
.sd-image { max-width: 100%; }
[hidden] { display: none !important; }
"#;
