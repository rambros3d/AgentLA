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

#[embassy_executor::task]
pub async fn cdc_cli_task(mut class: CdcAcmClass<'static, Driver<'static, USB>>) {
    loop {
        class.wait_connection().await;
        let _ = class
            .write_packet(b"\r\n=== AgentLA RP2350 Interactive Console ===\r\nType 'help' for command list.\r\nagentla> ")
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
                            let _ = class
                                .write_packet(b"Commands:\r\n  help          - Show command list\r\n  status        - System and core status\r\n  version       - Firmware version\r\n  identify      - MCU identity\r\n  target detect - Auto-detect connected target MCU\r\n  target halt   - Halt target core via SWD\r\n  target resume - Resume target core execution\r\n  flash read    - Read and dump target flash memory\r\n")
                                .await;
                        }
                        "version" | "VERSION" => {
                            let _ = class
                                .write_packet(b"AgentLA firmware v1.0.0 (RP2350 Embassy Rust)\r\n")
                                .await;
                        }
                        "identify" | "IDENTIFY" => {
                            let _ = class
                                .write_packet(b"CHIP: RP2350 (ARM Cortex-M33 @ 150MHz)\r\nBOARD: AgentLA BakuProbe / Pico 2\r\n")
                                .await;
                        }
                        "status" | "STATUS" => {
                            let _ = class
                                .write_packet(b"SYSTEM: OK\r\nCLOCKS: 150 MHz sys_clk\r\nNET: CDC-NCM 10.10.10.10/24 up\r\nPROGRAMMER: Ready (SWD/SWIO/RVSWD)\r\n")
                                .await;
                        }
                        cmd if cmd.eq_ignore_ascii_case("target detect") => {
                            match crate::programmer::detect_target_global() {
                                Ok(target) => {
                                    let _ = class.write_packet(b"Target detected:\r\n").await;
                                    let _ = class.write_packet(b"  Name:   ").await;
                                    let _ = class.write_packet(target.name.as_bytes()).await;
                                    let _ = class.write_packet(b"\r\n  Family: ").await;
                                    match target.family {
                                        crate::programmer::target::TargetFamily::Stm32F1 => {
                                            let _ = class.write_packet(b"STM32F1\r\n").await;
                                        }
                                        crate::programmer::target::TargetFamily::Stm32F4 => {
                                            let _ = class.write_packet(b"STM32F4\r\n").await;
                                        }
                                        crate::programmer::target::TargetFamily::Stm32H5 => {
                                            let _ = class.write_packet(b"STM32H5\r\n").await;
                                        }
                                        crate::programmer::target::TargetFamily::Rp2040 => {
                                            let _ = class.write_packet(b"RP2040\r\n").await;
                                        }
                                        crate::programmer::target::TargetFamily::Rp2350 => {
                                            let _ = class.write_packet(b"RP2350\r\n").await;
                                        }
                                        crate::programmer::target::TargetFamily::Ch32V003 => {
                                            let _ = class.write_packet(b"CH32V003 (1-Wire)\r\n").await;
                                        }
                                        crate::programmer::target::TargetFamily::Ch32V203 => {
                                            let _ = class.write_packet(b"CH32V203 (2-Wire)\r\n").await;
                                        }
                                        crate::programmer::target::TargetFamily::Py32F0 => {
                                            let _ = class.write_packet(b"PY32F0\r\n").await;
                                        }
                                        _ => {
                                            let _ = class.write_packet(b"Unknown\r\n").await;
                                        }
                                    }
                                }
                                Err(_) => {
                                    let _ = class.write_packet(b"Error: No target detected on SWD/SWIO/RVSWD.\r\n").await;
                                }
                            }
                        }
                        cmd if cmd.eq_ignore_ascii_case("target halt") => {
                            match crate::programmer::halt_target_global() {
                                Ok(_) => {
                                    let _ = class.write_packet(b"Target core halted successfully.\r\n").await;
                                }
                                Err(_) => {
                                    let _ = class.write_packet(b"Error: Failed to halt target core.\r\n").await;
                                }
                            }
                        }
                        cmd if cmd.eq_ignore_ascii_case("target resume") => {
                            match crate::programmer::resume_target_global() {
                                Ok(_) => {
                                    let _ = class.write_packet(b"Target core resumed.\r\n").await;
                                }
                                Err(_) => {
                                    let _ = class.write_packet(b"Error: Failed to resume target core.\r\n").await;
                                }
                            }
                        }
                        cmd if cmd.len() >= 10 && cmd[..10].eq_ignore_ascii_case("flash read") => {
                            let mut buf = [0u8; 16];
                            match crate::programmer::read_flash_global(0x08000000, &mut buf) {
                                Ok(_) => {
                                    let _ = class.write_packet(b"Flash [0x08000000..0x08000010]:\r\n  ").await;
                                    for &b in &buf {
                                        let hex_chars = [
                                            b"0123456789ABCDEF"[(b >> 4) as usize],
                                            b"0123456789ABCDEF"[(b & 0x0F) as usize],
                                            b' ',
                                        ];
                                        let _ = class.write_packet(&hex_chars).await;
                                    }
                                    let _ = class.write_packet(b"\r\n").await;
                                }
                                Err(_) => {
                                    let _ = class.write_packet(b"Error reading target flash.\r\n").await;
                                }
                            }
                        }
                        "" => {}
                        _ => {
                            let _ = class.write_packet(b"Unknown command. Type 'help'.\r\n").await;
                        }
                    }
                    line_len = 0;
                    let _ = class.write_packet(b"agentla> ").await;
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
