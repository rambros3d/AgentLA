#![no_std]
#![no_main]
#![recursion_limit = "512"]

use defmt_rtt as _;
use embassy_rp::bind_interrupts;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use panic_probe as _;
use static_cell::StaticCell;

mod net;
mod storage;
mod usb;
mod web;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
});

#[embassy_executor::task]
async fn watchdog_task(mut watchdog: embassy_rp::watchdog::Watchdog) {
    watchdog.start(embassy_time::Duration::from_secs(5));
    loop {
        embassy_time::Timer::after(embassy_time::Duration::from_secs(1)).await;
        watchdog.feed(embassy_time::Duration::from_secs(5));
    }
}

#[embassy_executor::main]
async fn main(spawner: embassy_executor::Spawner) {
    let mut config = embassy_rp::config::Config::default();
    config.clocks = embassy_rp::clocks::ClockConfig::crystal(12_000_000);
    let p = embassy_rp::init(config);
    defmt::info!("AgentLA RP2350 booting at 150 MHz...");

    // 1. Hardware Watchdog Supervisor
    let watchdog = embassy_rp::watchdog::Watchdog::new(p.WATCHDOG);
    spawner.spawn(watchdog_task(watchdog).unwrap());
    defmt::info!("Watchdog supervisor active.");

    // 2. Storage Partition (LittleFS)
    let flash = embassy_rp::flash::Flash::<_, _, 4_194_304>::new_blocking(p.FLASH);
    storage::init_filesystem(flash);

    // 3. Composite USB Device
    let driver = Driver::new(p.USB, Irqs);
    let composite = usb::create_composite_usb(driver);

    // 4. Async Network Stack (embassy-net over CDC-NCM)
    let mut dns_servers = heapless::Vec::<embassy_net::Ipv4Address, 3>::new();
    let _ = dns_servers.push(net::STATIC_IP);

    let net_config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: embassy_net::Ipv4Cidr::new(net::STATIC_IP, 24),
        dns_servers,
        gateway: Some(net::STATIC_IP),
    });

    static STACK_RESOURCES: StaticCell<embassy_net::StackResources<8>> = StaticCell::new();
    let resources = STACK_RESOURCES.init(embassy_net::StackResources::new());
    let (stack, net_runner) =
        embassy_net::new(composite.net_device, net_config, resources, 0x12345678);

    // 5. Spawn all background tasks
    spawner.spawn(usb::usb_task(composite.device).unwrap());
    spawner.spawn(usb::ncm_task(composite.ncm_runner).unwrap());
    spawner.spawn(usb::cdc_cli_task(composite.cdc_cli).unwrap());
    spawner.spawn(usb::cdc_uart_task(composite.cdc_uart).unwrap());
    spawner
        .spawn(usb::dap_v1_task(composite.dap_v1_reader, composite.dap_v1_writer).unwrap());
    spawner.spawn(usb::dap_v2_task(composite.dap_v2).unwrap());
    spawner.spawn(net::net_task(net_runner).unwrap());
    spawner.spawn(net::dhcp_server_task(stack).unwrap());
    spawner.spawn(net::dns_server_task(stack).unwrap());
    spawner.spawn(web::web_server_task(stack).unwrap());

    defmt::info!("All AgentLA subsystems initialized and running.");
}
