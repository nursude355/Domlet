use std::cell::RefCell;
use wasm_bindgen::prelude::*;

slint_dom::include_ui!("ui/main.slint");

thread_local! {
    static APP: RefCell<Option<MainWindow>> = const { RefCell::new(None) };
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let app = MainWindow::mount_to_body()?;

    let status = app.status_property();
    let enabled = app.enabled_property();
    app.on_start(move || {
        status.set("Running".into());
        enabled.set(false);
    });

    let status = app.status_property();
    let enabled = app.enabled_property();
    app.on_stop(move || {
        status.set("Stopped".into());
        enabled.set(true);
    });

    APP.with(|slot| *slot.borrow_mut() = Some(app));
    Ok(())
}
