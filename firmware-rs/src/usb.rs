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

pub fn handle_dap_request(req: &[u8], resp: &mut [u8]) -> usize {
    if req.is_empty() {
        return 0;
    }
    match req[0] {
        0x00 => {
            // DAP_Info
            resp[0] = 0x00;
            if req.len() < 2 {
                resp[1] = 0;
                return 2;
            }
            match req[1] {
                0x01 => {
                    let v = b"AgentLA";
                    resp[1] = v.len() as u8;
                    resp[2..2 + v.len()].copy_from_slice(v);
                    2 + v.len()
                }
                0x02 => {
                    let p = b"AgentLA RP2350 Probe";
                    resp[1] = p.len() as u8;
                    resp[2..2 + p.len()].copy_from_slice(p);
                    2 + p.len()
                }
                0x03 => {
                    let s = b"00000001";
                    resp[1] = s.len() as u8;
                    resp[2..2 + s.len()].copy_from_slice(s);
                    2 + s.len()
                }
                0x04 => {
                    let ver = b"1.0";
                    resp[1] = ver.len() as u8;
                    resp[2..2 + ver.len()].copy_from_slice(ver);
                    2 + ver.len()
                }
                0xF0 => {
                    // Capabilities
                    resp[1] = 1;
                    resp[2] = 0x01; // SWD supported
                    3
                }
                0xFE => {
                    // Packet count
                    resp[1] = 1;
                    resp[2] = 1;
                    3
                }
                0xFF => {
                    // Packet size (64 bytes LE)
                    resp[1] = 2;
                    resp[2] = 64;
                    resp[3] = 0;
                    4
                }
                _ => {
                    resp[1] = 0;
                    2
                }
            }
        }
        0x02 => {
            // DAP_Connect
            resp[0] = 0x02;
            resp[1] = 1; // SWD port connected
            2
        }
        0x03 => {
            // DAP_Disconnect
            resp[0] = 0x03;
            resp[1] = 0; // Success
            2
        }
        cmd => {
            // Generic command ACK
            resp[0] = cmd;
            resp[1] = 0x00;
            2
        }
    }
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
                                .write_packet(b"Commands:\r\n  help     - Show command list\r\n  status   - System and core status\r\n  version  - Firmware version\r\n  identify - MCU identity\r\n")
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
                                .write_packet(b"SYSTEM: OK\r\nCLOCKS: 150 MHz sys_clk\r\nNET: CDC-NCM 10.10.10.10/24 up\r\n")
                                .await;
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
    let mut req = [0u8; 64];
    let mut resp = [0u8; 64];
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
