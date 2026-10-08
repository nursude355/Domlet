//! Raspberry Pi Pico 2 + W5500 no-std web and WebSocket/JSON-RPC server.
//!
//! Wiring follows Embassy's W5500 example: SPI0 MISO=GP16, CS=GP17,
//! SCK=GP18, MOSI=GP19, RESET=GP20, INT=GP21.

#![no_std]
#![no_main]
#![recursion_limit = "512"]
// picoserve hashes the embedded files at compile time (ETag); for the ~159 KB
// WASM file this exceeds rustc's default const-eval step limit.
#![allow(long_running_const_eval)]

use core::{
    cell::Cell, convert::Infallible, future::poll_fn, future::Future, pin::pin, task::Poll,
};
use defmt::{info, unwrap};
use defmt_rtt as _;
use embassy_executor::Spawner;
use embassy_net::{Config as NetConfig, StackResources};
use embassy_net_wiznet::{chip::W5500, Device, Runner, State};
use embassy_rp::{
    bind_interrupts,
    clocks::RoscRng,
    dma,
    gpio::{Input, Level, Output, Pull},
    peripherals::{DMA_CH0, DMA_CH1, SPI0},
    spi::{Async, Config as SpiConfig, Spi},
};
use embassy_time::{Delay, Instant, Timer};
use embedded_hal_bus::spi::ExclusiveDevice;
use panic_probe as _;
use picoserve::{
    extract::FromRequestParts,
    futures::Either,
    request::RequestParts,
    response::{ws, Directory, File, StatusCode, WebSocketUpgrade},
    routing::get,
};
use static_cell::StaticCell;

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>, dma::InterruptHandler<DMA_CH1>;
});

type EthernetRunner = Runner<
    'static,
    W5500,
    ExclusiveDevice<Spi<'static, SPI0, Async>, Output<'static>, Delay>,
    Input<'static>,
    Output<'static>,
>;

#[embassy_executor::task]
async fn ethernet_task(runner: EthernetRunner) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, Device<'static>>) -> ! {
    runner.run().await
}

/// Reject foreign/ambiguous browser origins before upgrading the connection.
struct SameOrigin;

impl<'r, State> FromRequestParts<'r, State> for SameOrigin {
    type Rejection = (StatusCode, &'static str);

    async fn from_request_parts(
        _state: &'r State,
        parts: &RequestParts<'r>,
    ) -> Result<Self, Self::Rejection> {
        let forbidden = (StatusCode::FORBIDDEN, "WebSocket origin rejected");
        let mut origin = None;
        let mut host = None;
        for (name, value) in parts.headers() {
            let field = if name == "Origin" {
                &mut origin
            } else if name == "Host" {
                &mut host
            } else {
                continue;
            };
            // An invalid or duplicate header must not become "missing Origin".
            if field.is_some() {
                return Err(forbidden);
            }
            *field = Some(value.as_str().map_err(|_| forbidden)?);
        }
        // Missing Origin is allowed for non-browser tools, like the desktop
        // example. It is not authentication; raw clients can forge headers.
        if domlet_example_server::websocket_origin_allowed(origin, host) {
            Ok(Self)
        } else {
            Err(forbidden)
        }
    }
}

struct RpcSocket;

impl ws::WebSocketCallback for RpcSocket {
    async fn run<R: picoserve::io::Read, W: picoserve::io::Write<Error = R::Error>>(
        self,
        mut rx: ws::SocketRx<R>,
        mut tx: ws::SocketTx<W>,
    ) -> Result<(), W::Error> {
        use domlet_example_server::{WebSocketHeartbeat, RPC_MESSAGE_LIMIT};

        let started_ms = Instant::now().as_millis();
        let heartbeat = Cell::new(WebSocketHeartbeat::new(started_ms));
        let mut io = pin!(async {
            let mut receive_buffer = [0; RPC_MESSAGE_LIMIT];
            let mut next_telemetry_ms = started_ms + 5000;

            let close_reason = loop {
                // Yield even when a hostile client keeps every read ready, so
                // the watchdog and the other two server slots can be polled.
                Timer::after_millis(0).await;
                let now_ms = Instant::now().as_millis();
                if heartbeat.get().expired(now_ms) {
                    return Ok(());
                }
                let mut state = heartbeat.get();
                if let Some(payload) = state.ping_if_due(now_ms) {
                    heartbeat.set(state);
                    tx.send_ping(&payload).await?;
                }
                if now_ms >= next_telemetry_ms {
                    let notification =
                        domlet_example_server::telemetry::<128>((now_ms - started_ms) / 1000);
                    tx.send_text(&notification).await?;
                    next_telemetry_ms = now_ms + 5000;
                }
                let wake_ms = heartbeat.get().wake_ms().min(next_telemetry_ms);
                match rx
                    .next_message(
                        &mut receive_buffer,
                        Timer::at(Instant::from_millis(wake_ms)),
                    )
                    .await?
                {
                    Either::First(Ok(ws::Message::Text(request))) => {
                        if let Some(response) =
                            domlet_example_server::reply::<RPC_MESSAGE_LIMIT>(request)
                        {
                            tx.send_text(&response).await?;
                        }
                    }
                    Either::First(Ok(ws::Message::Ping(data))) => tx.send_pong(data).await?,
                    Either::First(Ok(ws::Message::Pong(data))) => {
                        let mut state = heartbeat.get();
                        state.accept_pong(Instant::now().as_millis(), data);
                        heartbeat.set(state);
                    }
                    Either::First(Ok(ws::Message::Close(reason))) => break reason,
                    Either::First(Ok(ws::Message::Binary(_))) => {
                        break Some((1003, "text JSON-RPC only"));
                    }
                    Either::First(Err(error)) => break Some((error.code(), "WebSocket error")),
                    Either::Second(()) => {}
                }
            };

            tx.close(close_reason).await
        });
        let mut watchdog = pin!(async {
            loop {
                Timer::at(Instant::from_millis(heartbeat.get().deadline_ms())).await;
                if heartbeat.get().expired(Instant::now().as_millis()) {
                    return;
                }
            }
        });

        // On expiry, drop BOTH socket halves without writing another frame:
        // cancelled reads/writes may be mid-frame and must never be reused.
        // picoserve then closes TCP; its existing 3 s read/1 s write timeouts
        // bound shutdown. Its 1 s per-write timeout is unchanged throughout.
        poll_fn(|context| {
            // Deadline wins even if I/O is ready. picoserve's next_message
            // signal only applies BEFORE a frame starts: race the entire I/O
            // session instead, including partial frames and blocked writes.
            if watchdog.as_mut().poll(context).is_ready() {
                return Poll::Ready(Ok(()));
            }
            io.as_mut().poll(context)
        })
        .await
    }
}

static CONFIG: picoserve::Config = picoserve::Config::const_default();

/// Serves one connection at a time on port 80 until the device resets.
async fn serve<P: picoserve::routing::PathRouter>(
    task_id: u32,
    app: &picoserve::Router<P>,
    stack: embassy_net::Stack<'_>,
) -> picoserve::NoGracefulShutdown {
    let mut tcp_rx_buffer = [0; 2048];
    let mut tcp_tx_buffer = [0; 4096];
    let mut http_buffer = [0; 2048];
    picoserve::Server::new(app, &CONFIG, &mut http_buffer)
        .listen_and_serve(task_id, stack, 80, &mut tcp_rx_buffer, &mut tcp_tx_buffer)
        .await
}

#[embassy_executor::main(
    executor = "embassy_rp::executor::Executor",
    entry = "cortex_m_rt::entry"
)]
async fn main(spawner: Spawner) {
    let peripherals = embassy_rp::init(Default::default());
    let mut rng = RoscRng;

    let mut spi_config = SpiConfig::default();
    spi_config.frequency = 50_000_000;
    let spi = Spi::new(
        peripherals.SPI0,
        peripherals.PIN_18,
        peripherals.PIN_19,
        peripherals.PIN_16,
        peripherals.DMA_CH0,
        peripherals.DMA_CH1,
        Irqs,
        spi_config,
    );
    let chip_select = Output::new(peripherals.PIN_17, Level::High);
    let interrupt = Input::new(peripherals.PIN_21, Pull::Up);
    let reset = Output::new(peripherals.PIN_20, Level::High);

    static W5500_STATE: StaticCell<State<8, 8>> = StaticCell::new();
    let (device, runner) = embassy_net_wiznet::new(
        [0x02, 0x00, 0x00, 0x00, 0x00, 0x02],
        W5500_STATE.init(State::new()),
        ExclusiveDevice::new(spi, chip_select, Delay).unwrap(),
        interrupt,
        reset,
    )
    .await
    .unwrap();
    let task = ethernet_task(runner).unwrap_or_else(|_| panic!("W5500 task is already spawned"));
    spawner.spawn(task);

    static STACK_RESOURCES: StaticCell<StackResources<4>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        device,
        NetConfig::dhcpv4(Default::default()),
        STACK_RESOURCES.init(StackResources::new()),
        rng.next_u64(),
    );
    let task = net_task(runner).unwrap_or_else(|_| panic!("network task is already spawned"));
    spawner.spawn(task);

    info!("waiting for DHCP");
    stack.wait_config_up().await;
    let config = unwrap!(stack.config_v4());
    let address = config.address.address();
    let octets = address.octets();
    info!(
        "open http://{}.{}.{}.{}",
        octets[0], octets[1], octets[2], octets[3]
    );

    let app = picoserve::Router::from_service(
        const {
            Directory {
                files: &[("", File::html(include_str!("../../../example/index.html")))],
                sub_directories: &[(
                    "pkg",
                    Directory {
                        files: &[
                            (
                                "domlet_example.js",
                                File::javascript(include_str!(
                                    "../../../example/pkg/domlet_example.js"
                                )),
                            ),
                            (
                                "domlet_example_bg.wasm",
                                File::with_content_type(
                                    "application/wasm",
                                    include_bytes!("../../../example/pkg/domlet_example_bg.wasm"),
                                ),
                            ),
                        ],
                        sub_directories: &[],
                    },
                )],
            }
        },
    )
    .route(
        "/rpc",
        get(async |_: SameOrigin, upgrade: WebSocketUpgrade| upgrade.on_upgrade(RpcSocket)),
    );

    // Each open page holds one connection for its WebSocket, so a single
    // server would stall reloads and further tabs. Three servers plus the
    // DHCP socket use every socket in `StackResources<4>`. The router's type
    // cannot be named, so they are polled together in this task rather than
    // spawned; none of them ever completes.
    let mut first = pin!(serve(0, &app, stack));
    let mut second = pin!(serve(1, &app, stack));
    let mut third = pin!(serve(2, &app, stack));
    poll_fn(|context| {
        let _ = first.as_mut().poll(context);
        let _ = second.as_mut().poll(context);
        let _ = third.as_mut().poll(context);
        Poll::<Infallible>::Pending
    })
    .await;
}
