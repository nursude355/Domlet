//! Raspberry Pi Pico 2 + W5500 no-std web and WebSocket/JSON-RPC server.
//!
//! Wiring follows Embassy's W5500 example: SPI0 MISO=GP16, CS=GP17,
//! SCK=GP18, MOSI=GP19, RESET=GP20, INT=GP21.

#![no_std]
#![no_main]
#![recursion_limit = "512"]

use core::{convert::Infallible, future::poll_fn, future::Future, pin::pin, task::Poll};
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
use embassy_time::{Delay, Timer};
use embedded_hal_bus::spi::ExclusiveDevice;
use panic_probe as _;
use picoserve::{
    futures::Either,
    response::{ws, Directory, File, WebSocketUpgrade},
    routing::get,
};
use static_cell::StaticCell;

bind_interrupts!(struct Irqs {
    DMA_IRQ_0 => dma::InterruptHandler<DMA_CH0>, dma::InterruptHandler<DMA_CH1>;
});

#[embassy_executor::task]
async fn ethernet_task(
    runner: Runner<
        'static,
        W5500,
        ExclusiveDevice<Spi<'static, SPI0, Async>, Output<'static>, Delay>,
        Input<'static>,
        Output<'static>,
    >,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, Device<'static>>) -> ! {
    runner.run().await
}

struct RpcSocket;

impl ws::WebSocketCallback for RpcSocket {
    async fn run<R: picoserve::io::Read, W: picoserve::io::Write<Error = R::Error>>(
        self,
        mut rx: ws::SocketRx<R>,
        mut tx: ws::SocketTx<W>,
    ) -> Result<(), W::Error> {
        let mut receive_buffer = [0; 1024];
        let mut uptime_seconds = 0_u64;

        let close_reason = loop {
            match rx
                .next_message(&mut receive_buffer, Timer::after_secs(5))
                .await?
            {
                Either::First(Ok(ws::Message::Text(request))) => {
                    if let Some(response) = slint_dom_example_server::reply::<1024>(request) {
                        tx.send_text(&response).await?;
                    }
                }
                Either::First(Ok(ws::Message::Ping(data))) => tx.send_pong(data).await?,
                Either::First(Ok(ws::Message::Pong(_))) => {}
                Either::First(Ok(ws::Message::Close(reason))) => break reason,
                Either::First(Ok(ws::Message::Binary(_))) => {
                    break Some((1003, "text JSON-RPC only"));
                }
                Either::First(Err(error)) => break Some((error.code(), "WebSocket error")),
                Either::Second(()) => {
                    uptime_seconds += 5;
                    let notification = slint_dom_example_server::telemetry::<128>(uptime_seconds);
                    tx.send_text(&notification).await?;
                }
            }
        };

        tx.close(close_reason).await
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
                                "slint_dom_example.js",
                                File::javascript(include_str!(
                                    "../../../example/pkg/slint_dom_example.js"
                                )),
                            ),
                            (
                                "slint_dom_example_bg.wasm",
                                File::with_content_type(
                                    "application/wasm",
                                    include_bytes!(
                                        "../../../example/pkg/slint_dom_example_bg.wasm"
                                    ),
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
        get(async |upgrade: WebSocketUpgrade| upgrade.on_upgrade(RpcSocket)),
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
