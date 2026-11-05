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

    static CDC_UART_STATE: StaticCell<CdcAcmState> = StaticCell::new();
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

    // 1 & 2: CDC-ACM (Target UART telemetry bridge)
    let cdc_uart_state = CDC_UART_STATE.init(CdcAcmState::new());
    let cdc_uart = CdcAcmClass::new(&mut builder, cdc_uart_state, 64);

    // 3: CMSIS-DAP v1 (HID)
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

    // 4: CMSIS-DAP v2 (Bulk)
    let dap_v2_state = DAP_V2_STATE.init(DapV2State::new());
    let dap_v2 = CmsisDapV2Class::new(&mut builder, dap_v2_state, 64, false);

    // 5 & 6: CDC-NCM (Ethernet)
    let ncm_state = NCM_STATE.init(CdcNcmState::new());
    let ncm_class = CdcNcmClass::new(&mut builder, ncm_state, HOST_MAC_ADDR, 64);
    let net_state = NET_STATE.init(NetState::new());
    let (ncm_runner, net_device) =
        ncm_class.into_embassy_net_device::<1514, 4, 4>(net_state, DEVICE_MAC_ADDR);

    let device = builder.build();

    CompositeUsb {
        device,
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
pub async fn cdc_uart_task(mut class: CdcAcmClass<'static, Driver<'static, USB>>) {
    crate::bridge::uart::init_uart_pins(
        crate::bridge::uart::PIN_UART_TX_DEFAULT,
        crate::bridge::uart::PIN_UART_RX_DEFAULT,
    );

    loop {
        class.wait_connection().await;
        let mut host_rx_buf = [0u8; 64];
        let mut target_rx_buf = [0u8; 64];

        loop {
            // 1. Check if target MCU sent bytes on UART (GPIO 14)
            let n_target = crate::bridge::uart::read_rx_bytes(&mut target_rx_buf);
            if n_target > 0 {
                if let Err(_) = class.write_packet(&target_rx_buf[..n_target]).await {
                    break;
                }
            }

            // 2. Poll target UART pin for newly arriving byte
            if let Some(b) = crate::bridge::uart::uart_poll_rx(
                crate::bridge::uart::PIN_UART_RX_DEFAULT,
                115_200,
            ) {
                let _ = class.write_packet(&[b]).await;
            }

            // 3. Check for data from host PC on CDC-ACM with timeout
            match embassy_time::with_timeout(
                embassy_time::Duration::from_millis(5),
                class.read_packet(&mut host_rx_buf),
            ).await {
                Ok(Ok(n)) if n > 0 => {
                    // Forward host bytes to physical target TX pin (GPIO 8)
                    crate::bridge::uart::uart_write_bytes(
                        crate::bridge::uart::PIN_UART_TX_DEFAULT,
                        115_200,
                        &host_rx_buf[..n],
                    );
                }
                Ok(Err(_)) => break, // Disconnected
                _ => {} // Timeout: continue polling
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
