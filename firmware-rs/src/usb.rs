use embassy_rp::peripherals::USB;
use embassy_rp::usb::Driver;
use embassy_usb::class::cdc_acm::{CdcAcmClass, State as CdcAcmState};
use embassy_usb::class::cdc_ncm::embassy_net::State as NetState;
use embassy_usb::class::cdc_ncm::{CdcNcmClass, State as CdcNcmState};
use embassy_usb::class::cmsis_dap_v2::{CmsisDapV2Class, State as DapV2State};
use embassy_usb::class::hid::{
    Config as HidConfig, HidBootProtocol, HidReader, HidReaderWriter, HidSubclass, HidWriter,
    State as HidState,
};
use embassy_usb::{Builder, UsbDevice};
use static_cell::StaticCell;

pub const CMSIS_DAP_REPORT_DESC: &[u8] = &[
    0x06, 0x00, 0xFF, // Usage Page (Vendor Defined 0xFF00)
    0x09, 0x01,       // Usage (0x01)
    0xA1, 0x01,       // Collection (Application)
    0x15, 0x00,       //   Logical Minimum (0)
    0x26, 0xFF, 0x00, //   Logical Maximum (255)
    0x75, 0x08,       //   Report Size (8)
    0x95, 0x40,       //   Report Count (64)
    0x09, 0x01,       //   Usage (0x01)
    0x81, 0x02,       //   Input (Data,Var,Abs)
    0x95, 0x40,       //   Report Count (64)
    0x09, 0x01,       //   Usage (0x01)
    0x91, 0x02,       //   Output (Data,Var,Abs)
    0xC0,             // End Collection
];

pub const HOST_MAC_ADDR: [u8; 6] = [0x02, 0x02, 0x84, 0x6A, 0x96, 0x00];
pub const DEVICE_MAC_ADDR: [u8; 6] = [0x02, 0x02, 0x84, 0x6A, 0x96, 0x01];

pub struct CompositeUsb {
    pub device: UsbDevice<'static, Driver<'static, USB>>,
    pub cdc_cli: CdcAcmClass<'static, Driver<'static, USB>>,
    pub cdc_uart: CdcAcmClass<'static, Driver<'static, USB>>,
    pub dap_v1_reader: HidReader<'static, Driver<'static, USB>, 64>,
    pub dap_v1_writer: HidWriter<'static, Driver<'static, USB>, 64>,
    pub dap_v2: CmsisDapV2Class<'static, Driver<'static, USB>>,
    pub ncm_runner: embassy_usb::class::cdc_ncm::embassy_net::Runner<'static, Driver<'static, USB>, 1514>,
    pub net_device: embassy_usb::class::cdc_ncm::embassy_net::Device<'static, 1514>,
}

pub fn create_composite_usb(driver: Driver<'static, USB>) -> CompositeUsb {
    static CONFIG_DESC: StaticCell<[u8; 512]> = StaticCell::new();
    static BOS_DESC: StaticCell<[u8; 256]> = StaticCell::new();
    static MSOS_DESC: StaticCell<[u8; 512]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; 128]> = StaticCell::new();

    static CDC0_STATE: StaticCell<CdcAcmState> = StaticCell::new();
    static CDC1_STATE: StaticCell<CdcAcmState> = StaticCell::new();
    static HID_STATE: StaticCell<HidState> = StaticCell::new();
    static DAP_V2_STATE: StaticCell<DapV2State> = StaticCell::new();
    static NCM_STATE: StaticCell<CdcNcmState> = StaticCell::new();
    static NET_STATE: StaticCell<NetState<1514, 4, 4>> = StaticCell::new();

    let config_desc = CONFIG_DESC.init([0; 512]);
    let bos_desc = BOS_DESC.init([0; 256]);
    let msos_desc = MSOS_DESC.init([0; 512]);
    let control_buf = CONTROL_BUF.init([0; 128]);

    let mut config = embassy_usb::Config::new(0x1209, 0x414D);
    config.manufacturer = Some("AgentLA");
    config.product = Some("AgentLA RP2350 Probe");
    config.serial_number = Some("00000001");
    config.max_power = 500;
    config.max_packet_size_0 = 64;
    config.device_class = 0xEF;
    config.device_sub_class = 0x02;
    config.device_protocol = 0x01;
    config.composite_with_iads = true;

    let mut builder = Builder::new(
        driver,
        config,
        config_desc,
        bos_desc,
        msos_desc,
        control_buf,
    );

    builder.msos_descriptor(embassy_usb::msos::windows_version::WIN8_1, 1);

    // 1 & 2: CDC-ACM 0 (CLI / REPL)
    let cdc0_state = CDC0_STATE.init(CdcAcmState::new());
    let cdc_cli = CdcAcmClass::new(&mut builder, cdc0_state, 64);

    // 3 & 4: CDC-ACM 1 (UART bridge)
    let cdc1_state = CDC1_STATE.init(CdcAcmState::new());
    let cdc_uart = CdcAcmClass::new(&mut builder, cdc1_state, 64);

    // 5: CMSIS-DAP v1 (HID)
    let hid_state = HID_STATE.init(HidState::new());
    let hid_config = HidConfig {
        report_descriptor: CMSIS_DAP_REPORT_DESC,
        request_handler: None,
        poll_ms: 1,
        max_packet_size: 64,
        hid_subclass: HidSubclass::No,
        hid_boot_protocol: HidBootProtocol::None,
    };
    let hid_rw = HidReaderWriter::<_, 64, 64>::new(&mut builder, hid_state, hid_config);
    let (dap_v1_reader, dap_v1_writer) = hid_rw.split();

    // 6: CMSIS-DAP v2 (Bulk)
    let dap_v2_state = DAP_V2_STATE.init(DapV2State::new());
    let dap_v2 = CmsisDapV2Class::new(&mut builder, dap_v2_state, 64, false);

    // 7 & 8: CDC-NCM (Ethernet)
    let ncm_state = NCM_STATE.init(CdcNcmState::new());
    let ncm_class = CdcNcmClass::new(&mut builder, ncm_state, HOST_MAC_ADDR, 64);
    let net_state = NET_STATE.init(NetState::new());
    let (ncm_runner, net_device) =
        ncm_class.into_embassy_net_device::<1514, 4, 4>(net_state, DEVICE_MAC_ADDR);

    let device = builder.build();

    CompositeUsb {
        device,
        cdc_cli,
        cdc_uart,
        dap_v1_reader,
        dap_v1_writer,
        dap_v2,
        ncm_runner,
        net_device,
    }
}

static DAP_HANDLER: embassy_sync::blocking_mutex::Mutex<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    core::cell::RefCell<crate::programmer::dap::CmsisDapHandler>,
> = embassy_sync::blocking_mutex::Mutex::new(core::cell::RefCell::new(
    crate::programmer::dap::CmsisDapHandler::new(),
));

pub fn handle_dap_request(req: &[u8], resp: &mut [u8]) -> usize {
    DAP_HANDLER.lock(|h| h.borrow_mut().handle_packet(req, resp))
}

#[embassy_executor::task]
pub async fn usb_task(mut device: UsbDevice<'static, Driver<'static, USB>>) {
    device.run().await;
}

#[embassy_executor::task]
pub async fn ncm_task(runner: embassy_usb::class::cdc_ncm::embassy_net::Runner<'static, Driver<'static, USB>, 1514>) {
    runner.run().await;
}

async fn write_str<D: embassy_usb::driver::Driver<'static>>(
    class: &mut CdcAcmClass<'static, D>,
    s: &str,
) {
    let bytes = s.as_bytes();
    for chunk in bytes.chunks(64) {
        let _ = class.write_packet(chunk).await;
    }
}

async fn print_target_info<D: embassy_usb::driver::Driver<'static>>(
    class: &mut CdcAcmClass<'static, D>,
    target: &crate::programmer::target::TargetInfo,
) {
    write_str(class, "  Name:     ").await;
    write_str(class, target.name).await;
    write_str(class, "\r\n  Family:   ").await;
    match target.family {
        crate::programmer::target::TargetFamily::Stm32F1 => write_str(class, "STM32F1\r\n").await,
        crate::programmer::target::TargetFamily::Stm32F4 => write_str(class, "STM32F4\r\n").await,
        crate::programmer::target::TargetFamily::Stm32H5 => write_str(class, "STM32H5\r\n").await,
        crate::programmer::target::TargetFamily::Rp2040 => write_str(class, "RP2040\r\n").await,
        crate::programmer::target::TargetFamily::Rp2350 => write_str(class, "RP2350\r\n").await,
        crate::programmer::target::TargetFamily::Ch32V003 => write_str(class, "CH32V003 (1-Wire)\r\n").await,
        crate::programmer::target::TargetFamily::Ch32V203 => write_str(class, "CH32V203 (2-Wire)\r\n").await,
        crate::programmer::target::TargetFamily::Py32F0 => write_str(class, "PY32F0\r\n").await,
        _ => write_str(class, "Unknown\r\n").await,
    }
    write_str(class, "  Flash:    ").await;
    let mut kb_buf = [0u8; 16];
    let kb = target.flash_size / 1024;
    let s = itoa_str(kb, &mut kb_buf);
    write_str(class, s).await;
    write_str(class, " KB\r\n  RAM:      ").await;
    let ram_kb = target.ram_size / 1024;
    let s2 = itoa_str(ram_kb, &mut kb_buf);
    write_str(class, s2).await;
    write_str(class, " KB\r\n").await;
}

fn itoa_str(mut val: u32, buf: &mut [u8; 16]) -> &str {
    if val == 0 {
        buf[0] = b'0';
        return core::str::from_utf8(&buf[..1]).unwrap_or("0");
    }
    let mut idx = 15;
    while val > 0 && idx > 0 {
        buf[idx] = b'0' + (val % 10) as u8;
        val /= 10;
        idx -= 1;
    }
    core::str::from_utf8(&buf[idx + 1..]).unwrap_or("")
}

async fn write_hex_u32<D: embassy_usb::driver::Driver<'static>>(
    class: &mut CdcAcmClass<'static, D>,
    val: u32,
) {
    let mut buf = [0u8; 10];
    buf[0] = b'0';
    buf[1] = b'x';
    for i in 0..8 {
        let nibble = ((val >> ((7 - i) * 4)) & 0xF) as usize;
        buf[2 + i] = b"0123456789ABCDEF"[nibble];
    }
    let s = core::str::from_utf8(&buf).unwrap_or("");
    write_str(class, s).await;
}

#[embassy_executor::task]
pub async fn cdc_cli_task(mut class: CdcAcmClass<'static, Driver<'static, USB>>) {
    loop {
        class.wait_connection().await;
        write_str(
            &mut class,
            "\r\n=== AgentLA RP2350 Interactive Console ===\r\nType 'help' for command list.\r\nagentla> ",
        )
        .await;
        let mut line_buf = [0u8; 64];
        let mut line_len = 0;
        let mut rx_buf = [0u8; 64];
        loop {
            let n = match class.read_packet(&mut rx_buf).await {
                Ok(n) => n,
                Err(_) => break,
            };
            for &b in &rx_buf[..n] {
                let _ = class.write_packet(&[b]).await;
                if b == b'\r' || b == b'\n' {
                    let _ = class.write_packet(b"\r\n").await;
                    let cmd = core::str::from_utf8(&line_buf[..line_len])
                        .unwrap_or("")
                        .trim();
                    match cmd {
                        "help" | "HELP" => {
                            write_str(
                                &mut class,
                                "Commands:\r\n  help                     - Show command list\r\n  status                   - System and core status\r\n  version                  - Firmware version\r\n  identify                 - MCU identity\r\n  bootsel                  - Reboot probe into USB BOOTSEL mode\r\n  target detect            - Scan all 3 ports (BluePill, CH32V203, CH32V003)\r\n  target detect bluepill   - Detect BluePill on Port 1 (GPIO 4 SWCLK, 5 SWDIO, 6 nRST)\r\n  target detect ch32v203   - Detect CH32V203 on Port 2 (GPIO 8 SWCLK, 9 SWDIO, 10 nRST)\r\n  target detect ch32v003   - Detect CH32V003 on Port 3 (GPIO 12 SWIO, 13 nRST, 14 TX)\r\n  target halt [port]       - Halt core (bluepill, ch32v203, ch32v003)\r\n  target resume [port]     - Resume core (bluepill, ch32v203, ch32v003)\r\n  flash read [port]        - Read flash vector table / header (default: bluepill)\r\n  flash erase [port]       - Erase test flash page/sector (default: bluepill)\r\n  flash write [port]       - Write & verify test pattern (default: bluepill)\r\n",
                            )
                            .await;
                        }
                        "bootsel" | "BOOTSEL" | "boot" | "BOOT" | "reset" | "RESET" => {
                            write_str(&mut class, "OK ENTERING BOOTSEL\r\n").await;
                            embassy_time::Timer::after(embassy_time::Duration::from_millis(100)).await;
                            embassy_rp::rom_data::reset_to_usb_boot(0, 0);
                        }
                        "version" | "VERSION" => {
                            write_str(&mut class, "AgentLA firmware v1.0.0 (RP2350 Embassy Rust)\r\n").await;
                        }
                        "identify" | "IDENTIFY" => {
                            write_str(
                                &mut class,
                                "CHIP: RP2350 (ARM Cortex-M33 @ 150MHz)\r\nBOARD: AgentLA BakuProbe / Pico 2\r\n",
                            )
                            .await;
                        }
                        "status" | "STATUS" => {
                            write_str(
                                &mut class,
                                "SYSTEM: OK\r\nCLOCKS: 150 MHz sys_clk\r\nNET: CDC-NCM 10.10.10.10/24 up\r\nPROGRAMMER: Ready (SWD/SWIO/RVSWD multi-port)\r\n",
                            )
                            .await;
                            let gpio_in = rp_pac::SIO.gpio_in(0).read();
                            write_str(&mut class, "GPIO IN: ").await;
                            write_hex_u32(&mut class, gpio_in).await;
                            write_str(&mut class, "\r\n  P1: CLK4=").await;
                            write_str(&mut class, if (gpio_in & (1 << 4)) != 0 { "1" } else { "0" }).await;
                            write_str(&mut class, " DIO5=").await;
                            write_str(&mut class, if (gpio_in & (1 << 5)) != 0 { "1" } else { "0" }).await;
                            write_str(&mut class, " RST6=").await;
                            write_str(&mut class, if (gpio_in & (1 << 6)) != 0 { "1" } else { "0" }).await;
                            write_str(&mut class, "\r\n  P2: CLK8=").await;
                            write_str(&mut class, if (gpio_in & (1 << 8)) != 0 { "1" } else { "0" }).await;
                            write_str(&mut class, " DIO9=").await;
                            write_str(&mut class, if (gpio_in & (1 << 9)) != 0 { "1" } else { "0" }).await;
                            write_str(&mut class, " RST10=").await;
                            write_str(&mut class, if (gpio_in & (1 << 10)) != 0 { "1" } else { "0" }).await;
                            write_str(&mut class, "\r\n  P3: SWIO12=").await;
                            write_str(&mut class, if (gpio_in & (1 << 12)) != 0 { "1" } else { "0" }).await;
                            write_str(&mut class, " RST13=").await;
                            write_str(&mut class, if (gpio_in & (1 << 13)) != 0 { "1" } else { "0" }).await;
                            write_str(&mut class, " TX14=").await;
                            write_str(&mut class, if (gpio_in & (1 << 14)) != 0 { "1" } else { "0" }).await;
                            write_str(&mut class, "\r\n").await;
                        }
                        cmd if cmd.eq_ignore_ascii_case("target detect bluepill") => {
                            write_str(&mut class, "Probing Port 1 [BluePill (GPIO 4 SWCLK, 5 SWDIO, 6 nRST)]...\r\n").await;
                            let (dpidr, ack, req) = crate::programmer::debug_probe_bluepill_global();
                            write_str(&mut class, "  Req: ").await;
                            write_hex_u32(&mut class, req).await;
                            write_str(&mut class, "  ACK: ").await;
                            write_hex_u32(&mut class, ack as u32).await;
                            write_str(&mut class, "  DPIDR: ").await;
                            write_hex_u32(&mut class, dpidr).await;
                            write_str(&mut class, "\r\n").await;
                            match crate::programmer::detect_target_port_global(crate::programmer::TargetPort::BluePill) {
                                Ok(t) => print_target_info(&mut class, &t).await,
                                Err(_) => write_str(&mut class, "Error: No target detected on Port 1.\r\n").await,
                            }
                        }
                        cmd if cmd.eq_ignore_ascii_case("target detect ch32v203") => {
                            write_str(&mut class, "Probing Port 2 [CH32V203 (GPIO 8 SWCLK, 9 SWDIO, 10 nRST)]...\r\n").await;
                            let (dmstatus, _) = crate::programmer::debug_probe_ch32v203_global();
                            write_str(&mut class, "  DMSTATUS: ").await;
                            write_hex_u32(&mut class, dmstatus).await;
                            write_str(&mut class, "\r\n").await;
                            match crate::programmer::detect_target_port_global(crate::programmer::TargetPort::Ch32v203) {
                                Ok(t) => print_target_info(&mut class, &t).await,
                                Err(_) => write_str(&mut class, "Error: No target detected on Port 2.\r\n").await,
                            }
                        }
                        cmd if cmd.eq_ignore_ascii_case("target detect ch32v003") => {
                            write_str(&mut class, "Probing Port 3 [CH32V003 (GPIO 12 SWIO, 13 nRST, 14 TX)]...\r\n").await;
                            let (dmstatus, partid) = crate::programmer::debug_probe_ch32v003_global();
                            write_str(&mut class, "  DMSTATUS: ").await;
                            write_hex_u32(&mut class, dmstatus).await;
                            write_str(&mut class, "  PARTID: ").await;
                            write_hex_u32(&mut class, partid).await;
                            write_str(&mut class, "\r\n").await;
                            match crate::programmer::detect_target_port_global(crate::programmer::TargetPort::Ch32v003) {
                                Ok(t) => print_target_info(&mut class, &t).await,
                                Err(_) => write_str(&mut class, "Error: No target detected on Port 3.\r\n").await,
                            }
                        }
                        cmd if cmd.eq_ignore_ascii_case("target detect") || cmd.eq_ignore_ascii_case("target detect all") => {
                            write_str(&mut class, "Scanning all target ports...\r\n").await;
                            write_str(&mut class, "[Port 1: BluePill (GPIO 4 SWCLK, 5 SWDIO, 6 nRST)]\r\n").await;
                            match crate::programmer::detect_target_port_global(crate::programmer::TargetPort::BluePill) {
                                Ok(t) => print_target_info(&mut class, &t).await,
                                Err(_) => write_str(&mut class, "  No target detected on Port 1.\r\n").await,
                            }
                            write_str(&mut class, "[Port 2: CH32V203 (GPIO 8 SWCLK, 9 SWDIO, 10 nRST)]\r\n").await;
                            match crate::programmer::detect_target_port_global(crate::programmer::TargetPort::Ch32v203) {
                                Ok(t) => print_target_info(&mut class, &t).await,
                                Err(_) => write_str(&mut class, "  No target detected on Port 2.\r\n").await,
                            }
                            write_str(&mut class, "[Port 3: CH32V003 (GPIO 12 SWIO, 13 nRST, 14 TX)]\r\n").await;
                            match crate::programmer::detect_target_port_global(crate::programmer::TargetPort::Ch32v003) {
                                Ok(t) => print_target_info(&mut class, &t).await,
                                Err(_) => write_str(&mut class, "  No target detected on Port 3.\r\n").await,
                            }
                        }
                        cmd if cmd.eq_ignore_ascii_case("target halt") || cmd.eq_ignore_ascii_case("target halt bluepill") => {
                            match crate::programmer::halt_target_port_global(crate::programmer::TargetPort::BluePill) {
                                Ok(_) => write_str(&mut class, "Port 1 [BluePill] core halted successfully.\r\n").await,
                                Err(_) => write_str(&mut class, "Error: Failed to halt Port 1 [BluePill] core.\r\n").await,
                            }
                        }
                        cmd if cmd.eq_ignore_ascii_case("target halt ch32v203") => {
                            match crate::programmer::halt_target_port_global(crate::programmer::TargetPort::Ch32v203) {
                                Ok(_) => write_str(&mut class, "Port 2 [CH32V203] core halted successfully.\r\n").await,
                                Err(_) => write_str(&mut class, "Error: Failed to halt Port 2 [CH32V203] core.\r\n").await,
                            }
                        }
                        cmd if cmd.eq_ignore_ascii_case("target halt ch32v003") => {
                            match crate::programmer::halt_target_port_global(crate::programmer::TargetPort::Ch32v003) {
                                Ok(_) => write_str(&mut class, "Port 3 [CH32V003] core halted successfully.\r\n").await,
                                Err(_) => write_str(&mut class, "Error: Failed to halt Port 3 [CH32V003] core.\r\n").await,
                            }
                        }
                        cmd if cmd.eq_ignore_ascii_case("target resume") || cmd.eq_ignore_ascii_case("target resume bluepill") => {
                            match crate::programmer::resume_target_port_global(crate::programmer::TargetPort::BluePill) {
                                Ok(_) => write_str(&mut class, "Port 1 [BluePill] core resumed.\r\n").await,
                                Err(_) => write_str(&mut class, "Error: Failed to resume Port 1 [BluePill] core.\r\n").await,
                            }
                        }
                        cmd if cmd.eq_ignore_ascii_case("target resume ch32v203") => {
                            match crate::programmer::resume_target_port_global(crate::programmer::TargetPort::Ch32v203) {
                                Ok(_) => write_str(&mut class, "Port 2 [CH32V203] core resumed.\r\n").await,
                                Err(_) => write_str(&mut class, "Error: Failed to resume Port 2 [CH32V203] core.\r\n").await,
                            }
                        }
                        cmd if cmd.eq_ignore_ascii_case("target resume ch32v003") => {
                            match crate::programmer::resume_target_port_global(crate::programmer::TargetPort::Ch32v003) {
                                Ok(_) => write_str(&mut class, "Port 3 [CH32V003] core resumed.\r\n").await,
                                Err(_) => write_str(&mut class, "Error: Failed to resume Port 3 [CH32V003] core.\r\n").await,
                            }
                        }
                        cmd if cmd.len() >= 5 && cmd[..5].eq_ignore_ascii_case("flash") => {
                            let sub = cmd[5..].trim();
                            if sub.is_empty() || sub.eq_ignore_ascii_case("help") {
                                write_str(&mut class, "Usage:\r\n  flash read [bluepill|ch32v203|ch32v003]\r\n  flash erase [bluepill|ch32v203|ch32v003]\r\n  flash write [bluepill|ch32v203|ch32v003]\r\n").await;
                            } else if sub.len() >= 4 && sub[..4].eq_ignore_ascii_case("read") {
                                let arg = sub[4..].trim();
                                let (port, label, is_arm) = match arg {
                                    a if a.eq_ignore_ascii_case("ch32v203") => (crate::programmer::TargetPort::Ch32v203, "Port 2 [CH32V203]", false),
                                    a if a.eq_ignore_ascii_case("ch32v003") => (crate::programmer::TargetPort::Ch32v003, "Port 3 [CH32V003]", false),
                                    _ => (crate::programmer::TargetPort::BluePill, "Port 1 [BluePill]", true),
                                };
                                let mut buf = [0u8; 16];
                                match crate::programmer::read_flash_port_global(port, 0x08000000, &mut buf) {
                                    Ok(_) => {
                                        write_str(&mut class, label).await;
                                        write_str(&mut class, " Flash [0x08000000..0x08000010]:\r\n").await;
                                        let w0 = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]);
                                        let w1 = u32::from_le_bytes([buf[4], buf[5], buf[6], buf[7]]);
                                        let w2 = u32::from_le_bytes([buf[8], buf[9], buf[10], buf[11]]);
                                        let w3 = u32::from_le_bytes([buf[12], buf[13], buf[14], buf[15]]);
                                        if is_arm {
                                            write_str(&mut class, "  0x08000000 (SP):           ").await;
                                            write_hex_u32(&mut class, w0).await;
                                            write_str(&mut class, "\r\n  0x08000004 (Reset Vector): ").await;
                                            write_hex_u32(&mut class, w1).await;
                                            write_str(&mut class, "\r\n  0x08000008 (NMI Vector):   ").await;
                                            write_hex_u32(&mut class, w2).await;
                                            write_str(&mut class, "\r\n  0x0800000C (HardFault):    ").await;
                                            write_hex_u32(&mut class, w3).await;
                                        } else {
                                            write_str(&mut class, "  0x08000000 (Word 0):       ").await;
                                            write_hex_u32(&mut class, w0).await;
                                            write_str(&mut class, "\r\n  0x08000004 (Word 1):       ").await;
                                            write_hex_u32(&mut class, w1).await;
                                            write_str(&mut class, "\r\n  0x08000008 (Word 2):       ").await;
                                            write_hex_u32(&mut class, w2).await;
                                            write_str(&mut class, "\r\n  0x0800000C (Word 3):       ").await;
                                            write_hex_u32(&mut class, w3).await;
                                        }
                                        write_str(&mut class, "\r\n  Raw bytes: ").await;
                                        for &b in &buf {
                                            let hex_chars = [
                                                b"0123456789ABCDEF"[(b >> 4) as usize],
                                                b"0123456789ABCDEF"[(b & 0x0F) as usize],
                                                b' ',
                                            ];
                                            let _ = class.write_packet(&hex_chars).await;
                                        }
                                        write_str(&mut class, "\r\n").await;
                                    }
                                    Err(_) => {
                                        write_str(&mut class, "Error reading target flash.\r\n").await;
                                    }
                                }
                            } else if sub.len() >= 5 && sub[..5].eq_ignore_ascii_case("erase") {
                                let arg = sub[5..].trim();
                                let (port, label, addr, len) = match arg {
                                    a if a.eq_ignore_ascii_case("ch32v203") => (crate::programmer::TargetPort::Ch32v203, "Port 2 [CH32V203]", 0x0800F000u32, 256u32),
                                    a if a.eq_ignore_ascii_case("ch32v003") => (crate::programmer::TargetPort::Ch32v003, "Port 3 [CH32V003]", 0x08003F00u32, 64u32),
                                    _ => (crate::programmer::TargetPort::BluePill, "Port 1 [BluePill]", 0x0800FC00u32, 1024u32),
                                };
                                write_str(&mut class, "Erasing ").await;
                                write_str(&mut class, label).await;
                                write_str(&mut class, " test page at ").await;
                                write_hex_u32(&mut class, addr).await;
                                write_str(&mut class, "...\r\n").await;
                                match crate::programmer::erase_flash_port_global(port, addr, len) {
                                    Ok(_) => {
                                        write_str(&mut class, label).await;
                                        write_str(&mut class, " flash erase successful.\r\n").await;
                                    }
                                    Err(e) => {
                                        write_str(&mut class, "Error: Flash erase failed (").await;
                                        match e {
                                            crate::programmer::target::TargetError::FlashErrorCode(c) => write_hex_u32(&mut class, c).await,
                                            _ => write_str(&mut class, "err").await,
                                        }
                                        write_str(&mut class, ").\r\n").await;
                                    }
                                }
                            } else if sub.len() >= 5 && sub[..5].eq_ignore_ascii_case("write") {
                                let arg = sub[5..].trim();
                                let (port, label, addr, erase_len) = match arg {
                                    a if a.eq_ignore_ascii_case("ch32v203") => (crate::programmer::TargetPort::Ch32v203, "Port 2 [CH32V203]", 0x0800F000u32, 256u32),
                                    a if a.eq_ignore_ascii_case("ch32v003") => (crate::programmer::TargetPort::Ch32v003, "Port 3 [CH32V003]", 0x08003F00u32, 64u32),
                                    _ => (crate::programmer::TargetPort::BluePill, "Port 1 [BluePill]", 0x0800FC00u32, 1024u32),
                                };
                                let pattern: [u8; 16] = [
                                    0xAA, 0x55, 0x12, 0x34, 0xBE, 0xEF, 0xCA, 0xFE,
                                    0xDE, 0xAD, 0x00, 0x11, 0x22, 0x33, 0x44, 0x55,
                                ];
                                write_str(&mut class, "Erasing & programming test pattern to ").await;
                                write_str(&mut class, label).await;
                                write_str(&mut class, " at ").await;
                                write_hex_u32(&mut class, addr).await;
                                write_str(&mut class, "...\r\n").await;
                                if let Err(e) = crate::programmer::erase_flash_port_global(port, addr, erase_len) {
                                    write_str(&mut class, "Error: Pre-write erase failed (").await;
                                    match e {
                                        crate::programmer::target::TargetError::FlashErrorCode(c) => write_hex_u32(&mut class, c).await,
                                        _ => write_str(&mut class, "err").await,
                                    }
                                    write_str(&mut class, ").\r\n").await;
                                } else {
                                    match crate::programmer::flash_target_port_global(port, addr, &pattern) {
                                        Ok(_) => {
                                            write_str(&mut class, label).await;
                                            write_str(&mut class, " flash program & verify successful.\r\n").await;
                                        }
                                        Err(e) => {
                                            write_str(&mut class, "Error: Flash program/verify failed (").await;
                                            match e {
                                                crate::programmer::target::TargetError::FlashErrorCode(c) => write_hex_u32(&mut class, c).await,
                                                _ => write_str(&mut class, "err").await,
                                            }
                                            write_str(&mut class, ").\r\n").await;
                                        }
                                    }
                                }
                            } else {
                                write_str(&mut class, "Unknown flash command. Type 'help'.\r\n").await;
                            }
                        }
                        "" => {}
                        _ => {
                            write_str(&mut class, "Unknown command. Type 'help'.\r\n").await;
                        }
                    }
                    line_len = 0;
                    write_str(&mut class, "agentla> ").await;
                } else if b == 0x08 || b == 0x7F {
                    if line_len > 0 {
                        line_len -= 1;
                    }
                } else if line_len < line_buf.len() {
                    line_buf[line_len] = b;
                    line_len += 1;
                }
            }
        }
    }
}

#[embassy_executor::task]
pub async fn cdc_uart_task(mut class: CdcAcmClass<'static, Driver<'static, USB>>) {
    loop {
        class.wait_connection().await;
        let mut buf = [0u8; 64];
        loop {
            let n = match class.read_packet(&mut buf).await {
                Ok(n) => n,
                Err(_) => break,
            };
            if let Err(_) = class.write_packet(&buf[..n]).await {
                break;
            }
        }
    }
}

#[embassy_executor::task]
pub async fn dap_v1_task(
    mut reader: HidReader<'static, Driver<'static, USB>, 64>,
    mut writer: HidWriter<'static, Driver<'static, USB>, 64>,
) {
    let mut req = [0u8; 64];
    let mut resp = [0u8; 64];
    loop {
        match reader.read(&mut req).await {
            Ok(n) => {
                let out_len = handle_dap_request(&req[..n], &mut resp);
                resp[out_len..].fill(0);
                let _ = writer.write(&resp).await;
            }
            Err(_) => {
                embassy_time::Timer::after(embassy_time::Duration::from_millis(50)).await;
            }
        }
    }
}

#[embassy_executor::task]
pub async fn dap_v2_task(mut class: CmsisDapV2Class<'static, Driver<'static, USB>>) {
    let mut req = [0u8; 512];
    let mut resp = [0u8; 512];
    loop {
        class.wait_connection().await;
        loop {
            match class.read_packet(&mut req).await {
                Ok(n) => {
                    let out_len = handle_dap_request(&req[..n], &mut resp);
                    if let Err(_) = class.write_packet(&resp[..out_len]).await {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    }
}
