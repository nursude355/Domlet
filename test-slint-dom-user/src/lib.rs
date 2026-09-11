use wasm_bindgen::prelude::*;

slint_dom::include_ui!("ui/main.slint");

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let app = MainWindow::mount_to_body()?;
    let status = app.status_property();
    app.on_start(move || status.set("Running".into()));
    std::mem::forget(app);
    Ok(())
}
