#![no_std]
#![no_main]
#![recursion_limit = "512"]

use defmt_rtt as _;
use embassy_rp::bind_interrupts;
use embassy_rp::flash::Flash;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use panic_probe as _;
use static_cell::StaticCell;

mod analyzer;
mod decoder;
mod bridge;
mod mcp;
mod apps;
mod fs_http;
mod net;
mod pin_owner;
mod rest;
mod profile;
mod servo;
mod storage;
mod usb;
mod web;
mod web_assets;
mod web_desktop;
pub mod programmer;

bind_interrupts!(struct Irqs {
    USBCTRL_IRQ => InterruptHandler<USB>;
    PIO0_IRQ_0 => embassy_rp::pio::InterruptHandler<embassy_rp::peripherals::PIO0>;
    DMA_IRQ_0 => embassy_rp::dma::InterruptHandler<embassy_rp::peripherals::DMA_CH0>;
});

// ---------------------------------------------------------------------------
// libc string shims for the littlefs2 C core
//
// `littlefs2-sys` compiles `lfs.c` with `lfs_util.h` stripped (see its
// build.rs), so the C core emits plain `strcpy`/`strchr`/`strlen` calls. The
// mem* variants are provided by `compiler_builtins`; the str* ones are not,
// and a no_std firmware links no libc — so we supply minimal implementations
// matching the C semantics exactly. Only used by the littlefs C code.
// ---------------------------------------------------------------------------

/// strcpy: copies `src` (incl. NUL) into `dst`, returns `dst`.
#[unsafe(no_mangle)]
unsafe extern "C" fn strcpy(dst: *mut core::ffi::c_char, src: *const core::ffi::c_char) -> *mut core::ffi::c_char {
    unsafe {
        let mut i = 0usize;
        loop {
            let b = *src.add(i);
            *dst.add(i) = b;
            if b == 0 {
                break;
            }
            i += 1;
        }
    }
    dst
}

/// strchr: returns a pointer to the first occurrence of `c` in `s`,
/// including the terminating NUL, or null when absent.
#[unsafe(no_mangle)]
unsafe extern "C" fn strchr(s: *const core::ffi::c_char, c: core::ffi::c_int) -> *mut core::ffi::c_char {
    unsafe {
        let mut i = 0usize;
        loop {
            let b = *s.add(i);
            if b as core::ffi::c_int == c {
                return s.add(i) as *mut core::ffi::c_char;
            }
            if b == 0 {
                return core::ptr::null_mut();
            }
            i += 1;
        }
    }
}

/// strlen: length of the NUL-terminated string.
#[unsafe(no_mangle)]
unsafe extern "C" fn strlen(s: *const core::ffi::c_char) -> usize {
    unsafe {
        let mut n = 0usize;
        while *s.add(n) != 0 {
            n += 1;
        }
        n
    }
}

/// strspn: length of the initial segment of `s` made only of `accept` chars.
#[unsafe(no_mangle)]
unsafe extern "C" fn strspn(s: *const core::ffi::c_char, accept: *const core::ffi::c_char) -> usize {
    unsafe {
        let mut n = 0usize;
        while *s.add(n) != 0 {
            let mut found = false;
            let mut i = 0usize;
            while *accept.add(i) != 0 {
                if *accept.add(i) == *s.add(n) {
                    found = true;
                    break;
                }
                i += 1;
            }
            if !found {
                break;
            }
            n += 1;
        }
        n
    }
}

/// strcspn: length of the initial segment of `s` without any `reject` char.
#[unsafe(no_mangle)]
unsafe extern "C" fn strcspn(s: *const core::ffi::c_char, reject: *const core::ffi::c_char) -> usize {
    unsafe {
        let mut n = 0usize;
        while *s.add(n) != 0 {
            let mut i = 0usize;
            while *reject.add(i) != 0 {
                if *reject.add(i) == *s.add(n) {
                    return n;
                }
                i += 1;
            }
            n += 1;
        }
        n
    }
}

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
    config.clocks.rosc = None;
    let p = embassy_rp::init(config);

    // Turn on status LED (GPIO 25) immediately to show CPU is running
    let mut led = embassy_rp::gpio::Output::new(p.PIN_25, embassy_rp::gpio::Level::High);
    led.set_high();

    // Pre-take PIO0/DMA0 for the analyzer capture task.
    let pio0 = p.PIO0;
    let dma0 = p.DMA_CH0;

    defmt::info!("AgentLA RP2350 booting at 150 MHz...");

    // 1. Pin-ownership blank state (GPIO pin-ownership spec): register the fixed
    //    programmer bridge claims and quiesce every flexible pin (SIO funcsel, input
    //    direction, no pulls) before any peripheral task can drive a GPIO.
    pin_owner::init();

    // 2. Composite USB Device (Start USB FIRST so host enumerates immediately)
    let driver = Driver::new(p.USB, Irqs);
    let composite = usb::create_composite_usb(driver);

    // 3. Spawn core USB tasks immediately
    spawner.spawn(usb::usb_task(composite.device).unwrap());
    spawner.spawn(usb::cdc_uart_task(composite.cdc_uart).unwrap());
    spawner.spawn(usb::dap_v1_task(composite.dap_v1_reader, composite.dap_v1_writer).unwrap());
    spawner.spawn(usb::dap_v2_task(composite.dap_v2).unwrap());
    spawner.spawn(usb::ncm_task(composite.ncm_runner).unwrap());
    spawner.spawn(usb::gs_usb_task(composite.gs_usb_read_ep, composite.gs_usb_write_ep).unwrap());

    // 3b. Async Network Stack (embassy-net over CDC-NCM)
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

    spawner.spawn(net::net_task(net_runner).unwrap());
    spawner.spawn(net::dhcp_server_task(stack).unwrap());
    spawner.spawn(net::dns_server_task(stack).unwrap());
    spawner.spawn(web::web_server_task(stack).unwrap());

    // 3c. LittleFS partition on internal flash. Must complete before the
    // watchdog task starts: a first-boot format erases 512 x 4 KiB sectors
    // and can take seconds (the watchdog feeds on 5 s windows). The FLASH
    // peripheral is shared from here on between LittleFS and profile ops
    // (profile window 0x180000..0x190000, see storage.rs).
    let flash = Flash::new_blocking(p.FLASH);

    // Profile boot-default scan BEFORE the LittleFS mount: purely XIP (read-only,
    // zero-copy) and never erases at boot (design decision 5).
    profile::scan_boot_default();

    let mounted = storage::init_filesystem(flash);
    defmt::info!(
        "LittleFS subsystem {}",
        if mounted { "ready" } else { "FAILED to mount (web fallback active)" }
    );

    // Initialize and tristate all programmer pins
    crate::programmer::pins::tristate_pins();

    // 4. Hardware Watchdog Supervisor
    let watchdog = embassy_rp::watchdog::Watchdog::new(p.WATCHDOG);
    spawner.spawn(watchdog_task(watchdog).unwrap());

    // Spawn analyzer capture task (PIO0/DMA0) to fill bank.
    spawner.spawn(crate::analyzer::capture_task(pio0, dma0).unwrap());
    let _ = crate::decoder::swd_decode(&crate::decoder::SWD_TEST_SAMPLE, 1_000_000, 0, 1, 0);
defmt::info!("SWD KAT: port smoke");
defmt::info!("All AgentLA subsystems initialized and running.");
}
