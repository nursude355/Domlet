use std::{cell::RefCell, rc::Rc};
use wasm_bindgen::prelude::*;

domlet::include_ui!("ui/main.slint");

thread_local! {
    static APP: RefCell<Option<RunningApp>> = const { RefCell::new(None) };
}

struct RunningApp {
    _app: MainWindow,
    _chart: Rc<domlet::LineChart>,
    _chart_subscription: domlet::Subscription,
    _rpc: Option<Rc<domlet::rpc::RpcClient>>,
}

#[wasm_bindgen(start)]
pub fn start() -> Result<(), JsValue> {
    let app = MainWindow::mount_to_body()?;
    let chart = Rc::new(domlet::LineChart::mount(app.graph())?);
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
        // A fixed scale matching the slider keeps the line height meaningful.
        let _ = chart_target.set_points_in_range(&values, 0.0, 100.0);
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
            } else if message.method.as_deref() == Some("telemetry") {
                let uptime = message
                    .params
                    .as_ref()
                    .and_then(|params| params.get("uptime_s"))
                    .and_then(|value| value.as_u64())
                    .unwrap_or_default();
                status.set(format!("Device uptime: {uptime}s"));
            }
        });
        let rpc = rpc.clone();
        let command = app.command_property();
        let status = app.status_property();
        let mut next_id = 0_u64;
        app.on_execute(move || {
            if !rpc.is_open() {
                status.set("RPC server is not connected".into());
                return;
            }
            next_id += 1;
            let request_id = domlet::rpc::Id::Number(next_id);
            if let Err(error) = rpc.request(request_id, "command", &command.get()) {
                status.set(format!("RPC send failed: {error:?}"));
            }
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

fn connect_rpc() -> Option<Rc<domlet::rpc::RpcClient>> {
    let location = web_sys::window()?.location();
    let scheme = if location.protocol().ok()?.as_str() == "https:" {
        "wss"
    } else {
        "ws"
    };
    let host = location.host().ok()?;
    domlet::rpc::RpcClient::connect(&format!("{scheme}://{host}/rpc"))
        .ok()
        .map(Rc::new)
}
