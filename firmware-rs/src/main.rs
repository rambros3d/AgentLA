#![no_std]
#![no_main]
#![recursion_limit = "1024"]

use defmt_rtt as _;
use embassy_rp::bind_interrupts;
use embassy_rp::flash::Flash;
use embassy_rp::peripherals::USB;
use embassy_rp::usb::{Driver, InterruptHandler};
use panic_probe as _;
use static_cell::StaticCell;

mod analyzer;
pub mod board;
mod decoder;
mod bridge;
pub mod instruments;
mod mcp;
mod apps;
mod fs_http;
mod net;
mod pin_owner;
mod rest;
mod profile;
mod servo;
mod tar_io;
mod storage;
mod usb;
mod web;
mod web_assets;
mod web_desktop;
pub mod device_html;
pub mod programmer;
pub mod ols;
pub mod telemetry;
pub mod benchmark;

static mut CORE1_STACK: embassy_rp::multicore::Stack<4096> = embassy_rp::multicore::Stack::new();

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
    // Long enough that a slow but progressing boot (LittleFS mount, profile
    // apply) is not reset, short enough that a genuine hang reboots instead of
    // leaving the probe reachable by ping but with no HTTP server.
    watchdog.start(embassy_time::Duration::from_secs(8));
    loop {
        embassy_time::Timer::after(embassy_time::Duration::from_secs(2)).await;
        watchdog.feed(embassy_time::Duration::from_secs(8));
    }
}

/// Applies the boot-default hardware profile off the critical boot path.
///
/// `profile::scan_boot_default` -> `apply_profile` -> `quiesce_and_reset` uses
/// blocking delays. Awaiting it inline stalled the executor before the network
/// stack was serving, which left the probe pingable with no HTTP listener.
#[embassy_executor::task]
async fn profile_boot_task() {
    crate::profile::scan_boot_default().await;
}

#[embassy_executor::main]
async fn main(spawner: embassy_executor::Spawner) {
    let mut config = embassy_rp::config::Config::default();
    config.clocks.rosc = None;
    let p = embassy_rp::init(config);

    // Start the watchdog before any blocking work. Previously it was spawned
    // last, so a hang during LittleFS mount or profile apply left the probe
    // alive (ping works, USB works) with no HTTP server and no way to recover
    // except power-cycling. Now a hang reboots.
    let watchdog = embassy_rp::watchdog::Watchdog::new(p.WATCHDOG);
    spawner.spawn(watchdog_task(watchdog).unwrap());

    // Read the boot clock requested for a warm reset from WATCHDOG.scratch4.
    // There is no persistent boot table in the single-slot layout, so a cold
    // boot always starts at the safe 150 MHz default.
    let scratch4 = rp_pac::WATCHDOG.scratch4().read();
    let boot_mhz = if (scratch4 & 0xFFFF_0000) == 0xC10C_0000 {
        rp_pac::WATCHDOG.scratch4().write_value(0);
        scratch4 & 0xFFFF
    } else {
        150
    };

    let target_mhz = match boot_mhz {
        300 => 300,
        500 => 500,
        _ => 150,
    };

    if target_mhz != 150 {
        if let Some(step) = crate::benchmark::step_for_mhz(target_mhz) {
            defmt::info!("Configuring RP2350 sys_clk: {} MHz", target_mhz);
            let ok = crate::benchmark::apply_system_clock_in_ram(step);
            if ok {
                defmt::info!("RP2350 sys_clk successfully locked at {} MHz", target_mhz);
            } else {
                defmt::error!("Failed to lock PLL at {} MHz", target_mhz);
                crate::benchmark::set_sys_clk_hz(150_000_000);
            }
        }
    }

    // Spawn Core 1 dedicated hard-real-time capture & DSP loop
    embassy_rp::multicore::spawn_core1(
        p.CORE1,
        unsafe { &mut *core::ptr::addr_of_mut!(CORE1_STACK) },
        || {
            crate::analyzer::core1_capture_loop();
        },
    );

    // Turn on status LED (GPIO 25) immediately to show CPU is running
    let mut led = embassy_rp::gpio::Output::new(p.PIN_25, embassy_rp::gpio::Level::High);
    led.set_high();

    // Pre-take PIO0/DMA0 for the analyzer capture task and PIO2 for the
    // I3C SDR master. PIO1 remains available for other adapter roles.
    let pio0 = p.PIO0;
    let dma0 = p.DMA_CH0;
    let pio2 = p.PIO2;

    defmt::info!("AgentLA RP2350 booting at 150 MHz...");

    // 1. Pin-ownership blank state (GPIO pin-ownership spec): register the fixed
    //    programmer bridge claims and quiesce every flexible pin (SIO funcsel, input
    //    direction, no pulls) before any peripheral task can drive a GPIO.
    pin_owner::init();

    // 2. Composite USB Device (Start USB FIRST so host enumerates immediately)
    let personality = usb::detect_boot_personality();
    defmt::info!("Active programmer personality: {}", personality.as_str());
    let driver = Driver::new(p.USB, Irqs);
    let composite = usb::create_composite_usb(driver, personality);

    // 3. Spawn core USB tasks immediately
    spawner.spawn(usb::usb_task(composite.device).unwrap());
    spawner.spawn(usb::cdc_uart_task(composite.cdc_uart).unwrap());
    match composite.programmer {
        usb::ProgrammerEndpoints::CmsisDap {
            dap_v1_reader,
            dap_v1_writer,
            dap_v2,
        } => {
            spawner.spawn(usb::dap_v1_task(dap_v1_reader, dap_v1_writer).unwrap());
            spawner.spawn(usb::dap_v2_task(dap_v2).unwrap());
        }
        usb::ProgrammerEndpoints::StLink { cmd_out, cmd_in, trace_in } => {
            spawner.spawn(usb::stlink_usb_task(cmd_out, cmd_in).unwrap());
            spawner.spawn(usb::swo_trace_task(trace_in).unwrap());
            spawner.spawn(usb::swo_rx_task().unwrap());
        }
        usb::ProgrammerEndpoints::WchLink {
            cmd_out,
            cmd_in,
            data_out,
            data_in,
        } => {
            spawner.spawn(usb::wchlink_usb_task(cmd_out, cmd_in, data_out, data_in).unwrap());
        }
    }
    spawner.spawn(usb::ncm_task(composite.ncm_runner).unwrap());
    spawner.spawn(usb::gs_usb_task(composite.gs_usb_read_ep, composite.gs_usb_write_ep).unwrap());

    // 3b. Async Network Stack (embassy-net over CDC-NCM)
    let mut dns_servers = heapless::Vec::<embassy_net::Ipv4Address, 3>::new();
    let _ = dns_servers.push(net::STATIC_IP);

    let net_config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: embassy_net::Ipv4Cidr::new(net::STATIC_IP, 24),
        dns_servers,
        // Host and device are peers on a direct USB NCM /24 link. A gateway
        // pointing at our own IP makes smoltcp treat on-subnet replies as
        // off-subnet, ARP for itself, and never return TCP responses.
        gateway: None,
    });

    static STACK_RESOURCES: StaticCell<embassy_net::StackResources<24>> = StaticCell::new();
    let resources = STACK_RESOURCES.init(embassy_net::StackResources::new());
    let (stack, net_runner) =
        embassy_net::new(composite.net_device, net_config, resources, 0x12345678);

    spawner.spawn(net::net_task(net_runner).unwrap());
    spawner.spawn(net::dhcp_server_task(stack).unwrap());
    spawner.spawn(net::dns_server_task(stack).unwrap());
    spawner.spawn(web::web_server_task(stack).unwrap());
    spawner.spawn(crate::programmer::gdb_server::gdb_server_task(stack).unwrap());
    spawner.spawn(crate::ols::ols_server_task(stack).unwrap());

    // 3c. LittleFS partitions on internal flash. The FLASH peripheral is
    // shared from here on between SYSTEM_FS/USER_FS and profile ops. OTA and
    // filesystem bounds in storage.rs keep every write above the app slot.
    let flash = Flash::new_blocking(p.FLASH);

    let mounted = storage::init_filesystem(flash);
    defmt::info!(
        "LittleFS subsystem {}",
        if mounted { "ready" } else { "FAILED to mount (web fallback active)" }
    );

    if mounted {
        rest::init_settings_from_fs();
        usb::load_personality_from_fs();
        usb::load_stlink_version();
        // Deferred: apply_profile() uses blocking delays and rewrites the pin
        // pool. Running it inline stalled the executor before the network stack
        // was serving. Spawn it instead.
        spawner.spawn(profile_boot_task().unwrap());
    }

    // Initialize and tristate all programmer pins
    crate::programmer::pins::tristate_pins();

    // Spawn analyzer capture task (PIO0/DMA0) to fill bank.
    spawner.spawn(crate::analyzer::capture_task(pio0, dma0).unwrap());
    // Spawn the I3C SDR master task (PIO2, GP10=SDA / GP11=SCL).
    spawner.spawn(crate::bridge::i3c::i3c_task(pio2).unwrap());
    let _ = crate::decoder::swd_decode(&crate::decoder::SWD_TEST_SAMPLE, 1_000_000, 0, 1, 0);
defmt::info!("SWD KAT: port smoke");
defmt::info!("All AgentLA subsystems initialized and running.");
}
