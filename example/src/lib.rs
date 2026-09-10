use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;

slint_dom::include_ui!("ui/main.slint");

thread_local! {
    static APP: RefCell<Option<RunningApp>> = const { RefCell::new(None) };
}

struct RunningApp {
    _app: MainWindow,
    _chart: Rc<slint_dom::LineChart>,
    _chart_subscription: slint_dom::Subscription,
    _rpc: Option<Rc<slint_dom::rpc::RpcClient>>,
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let app = MainWindow::mount_to_body()?;
    let chart = Rc::new(slint_dom::LineChart::mount(app.graph())?);
    let chart_target = chart.clone();
    let chart_subscription = app.progress_property().observe(move |value| {
        let baseline = *value;
        let values = [
            baseline * 0.35,
            baseline * 0.62,
            baseline * 0.44,
            baseline * 0.82,
            baseline,
        ];
        let _ = chart_target.set_points(&values);
    });

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

    let rpc = connect_rpc();
    if let Some(rpc) = &rpc {
        let status = app.status_property();
        rpc.on_message(move |message| {
            if let Some(error) = message.error {
                status.set(format!("RPC error: {error}"));
            } else if let Some(result) = message.result {
                status.set(format!("RPC: {result}"));
            }
        });
        let rpc = rpc.clone();
        let command = app.command_property();
        app.on_execute(move || {
            let value = command.get();
            let _ = rpc.request(1, "command", &value);
        });
    } else {
        let status = app.status_property();
        app.on_execute(move || status.set("RPC server is not connected".into()));
    }

    APP.with(|slot| {
        *slot.borrow_mut() = Some(RunningApp {
            _app: app,
            _chart: chart,
            _chart_subscription: chart_subscription,
            _rpc: rpc,
        })
    });
    Ok(())
}

fn connect_rpc() -> Option<Rc<slint_dom::rpc::RpcClient>> {
    let location = web_sys::window()?.location();
    let scheme = if location.protocol().ok()?.as_str() == "https:" {
        "wss"
    } else {
        "ws"
    };
    let host = location.host().ok()?;
    slint_dom::rpc::RpcClient::connect(&format!("{scheme}://{host}/rpc"))
        .ok()
        .map(Rc::new)
}
