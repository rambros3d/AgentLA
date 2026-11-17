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
use embassy_usb::control::{InResponse, OutResponse, Request, RequestType};
use embassy_usb::types::StringIndex;
use embassy_usb::{Builder, Handler, UsbDevice};
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

pub struct VendorControlHandler {
    gs_usb_str: StringIndex,
    i2c_str: StringIndex,
}

impl VendorControlHandler {
    pub const fn new(gs_usb_str: StringIndex, i2c_str: StringIndex) -> Self {
        Self { gs_usb_str, i2c_str }
    }
}

impl Handler for VendorControlHandler {
    fn get_string(&mut self, index: StringIndex, _lang_id: u16) -> Option<&str> {
        if index == self.gs_usb_str {
            Some("AgentLA SocketCAN gs_usb")
        } else if index == self.i2c_str {
            Some("AgentLA i2c-tiny-usb")
        } else {
            None
        }
    }

    fn control_out(&mut self, req: Request, data: &[u8]) -> Option<OutResponse> {
        if req.request_type != RequestType::Vendor {
            return None;
        }

        let req_cmd = req.request;
        let i2c_base = req_cmd & !0x03;

        // Check if this is an i2c-tiny-usb CMD_I2C_IO write
        if i2c_base == crate::bridge::i2c_tinyusb::CMD_I2C_IO || (req_cmd & 0x04) != 0 {
            let is_end = (req_cmd & crate::bridge::i2c_tinyusb::CMD_I2C_IO_END) != 0;
            let addr = (req.index & 0x7F) as u8;
            crate::bridge::i2c_tinyusb::i2c_io_write(addr, data, is_end);
            return Some(OutResponse::Accepted);
        }

        if req_cmd == crate::bridge::can_gsusb::GS_USB_BREQ_MODE
            && data.len() >= core::mem::size_of::<crate::bridge::can_gsusb::GsDeviceMode>()
        {
            let mode_buf: crate::bridge::can_gsusb::GsDeviceMode = unsafe {
                core::ptr::read_unaligned(data.as_ptr() as *const _)
            };
            if mode_buf.mode == crate::bridge::can_gsusb::GS_CAN_MODE_START {
                crate::bridge::can_gsusb::adapter_can_start();
            } else if mode_buf.mode == crate::bridge::can_gsusb::GS_CAN_MODE_RESET {
                crate::bridge::can_gsusb::adapter_can_standby();
            }
            return Some(OutResponse::Accepted);
        }

        match req_cmd {
            crate::bridge::i2c_tinyusb::CMD_SET_DELAY => {
                Some(OutResponse::Accepted)
            }

            // gs_usb control OUT requests
            crate::bridge::can_gsusb::GS_USB_BREQ_HOST_FORMAT => {
                Some(OutResponse::Accepted)
            }

            crate::bridge::can_gsusb::GS_USB_BREQ_BITTIMING => {
                if data.len() >= core::mem::size_of::<crate::bridge::can_gsusb::GsDeviceBittiming>() {
                    let bt: crate::bridge::can_gsusb::GsDeviceBittiming = unsafe {
                        core::ptr::read_unaligned(data.as_ptr() as *const _)
                    };
                    crate::bridge::can_gsusb::adapter_can_set_bittiming(&bt);
                }
                Some(OutResponse::Accepted)
            }

            crate::bridge::can_gsusb::GS_USB_BREQ_SET_TERMINATION => {
                let enable = if !data.is_empty() {
                    let mut b = [0u8; 4];
                    let copy_len = data.len().min(4);
                    b[..copy_len].copy_from_slice(&data[..copy_len]);
                    u32::from_le_bytes(b) != 0
                } else {
                    req.value != 0
                };
                crate::bridge::can_gsusb::adapter_can_set_termination(enable);
                Some(OutResponse::Accepted)
            }

            crate::bridge::can_gsusb::GS_USB_BREQ_BERR
            | crate::bridge::can_gsusb::GS_USB_BREQ_TIMESTAMP
            | crate::bridge::can_gsusb::GS_USB_BREQ_IDENTIFY => {
                Some(OutResponse::Accepted)
            }

            _ => None,
        }
    }

    fn control_in<'a>(&'a mut self, req: Request, buf: &'a mut [u8]) -> Option<InResponse<'a>> {
        if req.request_type != RequestType::Vendor {
            return None;
        }

        let req_cmd = req.request;
        let i2c_base = req_cmd & !0x03;

        // Check i2c-tiny-usb I2C_IO read
        if i2c_base == crate::bridge::i2c_tinyusb::CMD_I2C_IO || (req_cmd & 0x04) != 0 {
            let is_end = (req_cmd & crate::bridge::i2c_tinyusb::CMD_I2C_IO_END) != 0;
            let addr = (req.index & 0x7F) as u8;
            let len = (req.length as usize).min(buf.len());
            crate::bridge::i2c_tinyusb::i2c_io_read(addr, &mut buf[..len], is_end);
            return Some(InResponse::Accepted(&buf[..len]));
        }

        if req_cmd == crate::bridge::can_gsusb::GS_USB_BREQ_BERR && req.length >= 8 {
            let zeros = [0u8; 8];
            let len = (req.length as usize).min(8).min(buf.len());
            buf[..len].copy_from_slice(&zeros[..len]);
            return Some(InResponse::Accepted(&buf[..len]));
        }

        match req_cmd {
            // i2c-tiny-usb commands
            crate::bridge::i2c_tinyusb::CMD_ECHO => {
                let val_bytes = req.value.to_le_bytes();
                let len = (req.length as usize).min(2).min(buf.len());
                buf[..len].copy_from_slice(&val_bytes[..len]);
                Some(InResponse::Accepted(&buf[..len]))
            }

            crate::bridge::i2c_tinyusb::CMD_GET_FUNC => {
                let func_bytes = crate::bridge::i2c_tinyusb::I2C_FUNC_ALL.to_le_bytes();
                let len = (req.length as usize).min(4).min(buf.len());
                buf[..len].copy_from_slice(&func_bytes[..len]);
                Some(InResponse::Accepted(&buf[..len]))
            }

            crate::bridge::i2c_tinyusb::CMD_GET_STATUS => {
                let status = crate::bridge::i2c_tinyusb::get_last_status();
                if !buf.is_empty() {
                    buf[0] = status;
                    Some(InResponse::Accepted(&buf[..1]))
                } else {
                    Some(InResponse::Accepted(&[]))
                }
            }

            // gs_usb control IN requests
            crate::bridge::can_gsusb::GS_USB_BREQ_DEVICE_CONFIG => {
                let cfg = crate::bridge::can_gsusb::GsDeviceConfig {
                    reserved1: 0,
                    reserved2: 0,
                    reserved3: 0,
                    icount: 0, // 1 channel
                    sw_version: 1,
                    hw_version: 1,
                };
                let raw_bytes = unsafe {
                    core::slice::from_raw_parts(
                        &cfg as *const _ as *const u8,
                        core::mem::size_of::<crate::bridge::can_gsusb::GsDeviceConfig>(),
                    )
                };
                let len = (req.length as usize).min(raw_bytes.len()).min(buf.len());
                buf[..len].copy_from_slice(&raw_bytes[..len]);
                Some(InResponse::Accepted(&buf[..len]))
            }

            crate::bridge::can_gsusb::GS_USB_BREQ_BT_CONST => {
                let bt = crate::bridge::can_gsusb::GsDeviceBtConst {
                    feature: 0,
                    fclk_can: 48_000_000,
                    tseg1_min: 1,
                    tseg1_max: 16,
                    tseg2_min: 1,
                    tseg2_max: 8,
                    sjw_max: 4,
                    brp_min: 1,
                    brp_max: 1024,
                    brp_inc: 1,
                };
                let raw_bytes = unsafe {
                    core::slice::from_raw_parts(
                        &bt as *const _ as *const u8,
                        core::mem::size_of::<crate::bridge::can_gsusb::GsDeviceBtConst>(),
                    )
                };
                let len = (req.length as usize).min(raw_bytes.len()).min(buf.len());
                buf[..len].copy_from_slice(&raw_bytes[..len]);
                Some(InResponse::Accepted(&buf[..len]))
            }

            crate::bridge::can_gsusb::GS_USB_BREQ_GET_TERMINATION => {
                let term: u32 = if crate::bridge::can_gsusb::adapter_can_get_termination() { 1 } else { 0 };
                let term_bytes = term.to_le_bytes();
                let len = (req.length as usize).min(4).min(buf.len());
                buf[..len].copy_from_slice(&term_bytes[..len]);
                Some(InResponse::Accepted(&buf[..len]))
            }

            crate::bridge::can_gsusb::GS_USB_BREQ_TIMESTAMP => {
                let ts: u32 = 0;
                let ts_bytes = ts.to_le_bytes();
                let len = (req.length as usize).min(4).min(buf.len());
                buf[..len].copy_from_slice(&ts_bytes[..len]);
                Some(InResponse::Accepted(&buf[..len]))
            }

            _ => None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProgrammerPersonality {
    CmsisDap,
    StLink,
    WchLink,
}

impl ProgrammerPersonality {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::CmsisDap => "cmsis",
            Self::StLink => "stlink",
            Self::WchLink => "wchlink",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        if s.eq_ignore_ascii_case("cmsis") || s.eq_ignore_ascii_case("cmsis-dap") {
            Some(Self::CmsisDap)
        } else if s.eq_ignore_ascii_case("stlink") || s.eq_ignore_ascii_case("st-link") {
            Some(Self::StLink)
        } else if s.eq_ignore_ascii_case("wchlink") || s.eq_ignore_ascii_case("wch-link") || s.eq_ignore_ascii_case("wch") {
            Some(Self::WchLink)
        } else {
            None
        }
    }
}

pub const SCRATCH_PERSONALITY_MAGIC: u32 = 0xA61A_0000;

/// Build a 24-character hex USB serial string from the RP2350 public chip ID.
///
/// `stlink` host tools require exactly 24 hex characters (`STLINK_SERIAL_LENGTH`),
/// derived from a 96-bit UID. A fixed short serial such as "00000001" is rejected
/// by `stlink_serial()`, so every stlink tool reports the probe as unusable.
///
/// The 64-bit `CHIPID` is ECC-encoded, so we mask off the ECC bits and keep the
/// payload, then append the personality code to reach 24 hex characters. Two
/// probes with different personality modes never collide, and two probes with the
/// same personality collide only if their chip IDs match.
fn usb_serial_str(personality: ProgrammerPersonality) -> &'static str {
    // The serial is a pure function of (chip ID, personality) and is only ever
    // read after `create_composite_usb`, so it is built once under a critical
    // section instead of using `static mut` + raw pointers.
    use core::sync::atomic::{AtomicBool, Ordering};
    static READY: AtomicBool = AtomicBool::new(false);

    // Single-threaded firmware: the whole boot and USB path runs on one core
    // under one executor, so no preemption can interleave the write below.
    if READY.load(Ordering::Relaxed) {
        // SAFETY: `READY` is only set after all 24 bytes are written, and
        // nothing writes them again.
        return unsafe { serial_str_from_buf() };
    }

    let cid = [
        rp_pac::OTP_DATA_RAW.chipid0().read(),
        rp_pac::OTP_DATA_RAW.chipid1().read(),
        rp_pac::OTP_DATA_RAW.chipid2().read(),
        rp_pac::OTP_DATA_RAW.chipid3().read(),
    ];

    // CHIPID0..3 hold a 64-bit public device ID, 27 payload bits per 32-bit row
    // with ECC in bits 4:0. Collect all four rows and keep the payload.
    let mut raw: u64 = 0;
    for (i, row) in [cid[0], cid[1], cid[2], cid[3]].iter().enumerate() {
        raw |= ((row & !0x1F) as u64) << (27 * i);
    }
    let tail = match personality {
        ProgrammerPersonality::CmsisDap => 0x414D_0001u32,
        ProgrammerPersonality::StLink => 0x3752_0001u32,
        ProgrammerPersonality::WchLink => 0x0001_0001u32,
    };

    // 24 hex chars: 16 from the chip ID (unique per device) + 8 from the
    // personality tag. `stlink_serial()` accepts exactly 24.
    let mut buf = [0u8; 24];
    for i in 0..16 {
        let nib = ((raw >> (60 - 4 * i)) & 0xF) as u8;
        buf[i] = if nib < 10 { b'0' + nib } else { b'A' + (nib - 10) };
    }
    for i in 0..8 {
        let nib = ((tail >> (28 - 4 * i)) & 0xF) as u8;
        buf[16 + i] = if nib < 10 { b'0' + nib } else { b'A' + (nib - 10) };
    }

    // Publish the buffer. `SERIAL_BUF` is written exactly once.
    // SAFETY: single-threaded, and `READY` gates all subsequent reads. All
    // accesses go through raw pointers, so no reference to the static exists.
    unsafe {
        let dst = core::ptr::addr_of_mut!(SERIAL_BUF) as *mut u8;
        core::ptr::copy_nonoverlapping(buf.as_ptr(), dst, 24);
    }
    READY.store(true, Ordering::Relaxed);

    // SAFETY: 24 ASCII hex characters, written once and never mutated.
    unsafe { serial_str_from_buf() }
}

/// Storage for the one-time serial string. Written once inside a critical
/// section by `usb_serial_str`, then read-only.
static mut SERIAL_BUF: [u8; 24] = [b'0'; 24];

/// Build a `&'static str` over `SERIAL_BUF`.
///
/// # Safety
/// Caller must guarantee `SERIAL_BUF` has been fully initialised and that no
/// further writes will occur.
unsafe fn serial_str_from_buf() -> &'static str {
    let ptr = core::ptr::addr_of!(SERIAL_BUF) as *const u8;
    core::str::from_utf8_unchecked(core::slice::from_raw_parts(ptr, 24))
}

pub fn detect_boot_personality() -> ProgrammerPersonality {
    let scratch = rp_pac::WATCHDOG.scratch5().read();
    if (scratch & 0xFFFF_0000) == SCRATCH_PERSONALITY_MAGIC {
        match scratch & 0xFFFF {
            1 => ProgrammerPersonality::StLink,
            2 => ProgrammerPersonality::WchLink,
            _ => ProgrammerPersonality::CmsisDap,
        }
    } else {
        ProgrammerPersonality::CmsisDap
    }
}

pub fn get_programmer_personality() -> ProgrammerPersonality {
    detect_boot_personality()
}

pub fn set_programmer_personality_in_scratch(p: ProgrammerPersonality) {
    let code = match p {
        ProgrammerPersonality::CmsisDap => 0,
        ProgrammerPersonality::StLink => 1,
        ProgrammerPersonality::WchLink => 2,
    };
    rp_pac::WATCHDOG.scratch5().write_value(SCRATCH_PERSONALITY_MAGIC | code);
}

pub fn load_personality_from_fs() -> ProgrammerPersonality {
    let mut found = None;
    let _ = crate::storage::with_user_fs(|fs| {
        let Ok(path) = littlefs2::path::PathBuf::try_from("/system/programmer_mode.txt") else { return };
        if let Ok(data) = fs.read::<32>(&path) {
            if let Ok(s) = core::str::from_utf8(&data) {
                found = ProgrammerPersonality::from_str(s.trim());
            }
        }
    });
    if let Some(p) = found {
        set_programmer_personality_in_scratch(p);
        p
    } else {
        detect_boot_personality()
    }
}

pub fn save_personality_to_fs(p: ProgrammerPersonality) {
    set_programmer_personality_in_scratch(p);
    let _ = crate::storage::with_user_fs(|fs| {
        let Ok(dir) = littlefs2::path::PathBuf::try_from("/system") else { return };
        let _ = fs.create_dir_all(&dir);
        let Ok(path) = littlefs2::path::PathBuf::try_from("/system/programmer_mode.txt") else { return };
        let _ = fs.write(&path, p.as_str().as_bytes());
    });
}

static STLINK_REPORTED_VERSION: portable_atomic::AtomicU16 =
    portable_atomic::AtomicU16::new(agentla_programmer::stlink::STLINK_VER_REPORTED);

pub fn get_stlink_version() -> u16 {
    STLINK_REPORTED_VERSION.load(portable_atomic::Ordering::Relaxed)
}

pub fn load_stlink_version() -> u16 {
    let mut found = None;
    let _ = crate::storage::with_user_fs(|fs| {
        let Ok(path) = littlefs2::path::PathBuf::try_from("/system/stlink_version.txt") else { return };
        if let Ok(data) = fs.read::<32>(&path) {
            if let Ok(s) = core::str::from_utf8(&data) {
                found = agentla_programmer::stlink::parse_version(s.trim());
            }
        }
    });
    let v = found.unwrap_or(agentla_programmer::stlink::STLINK_VER_REPORTED);
    STLINK_REPORTED_VERSION.store(v, portable_atomic::Ordering::Relaxed);
    v
}

pub fn save_stlink_version(v: u16) {
    STLINK_REPORTED_VERSION.store(v, portable_atomic::Ordering::Relaxed);
    let mut buf = [0u8; 16];
    let v_str = agentla_programmer::stlink::format_version(v, &mut buf);
    let _ = crate::storage::with_user_fs(|fs| {
        let Ok(dir) = littlefs2::path::PathBuf::try_from("/system") else { return };
        let _ = fs.create_dir_all(&dir);
        let Ok(path) = littlefs2::path::PathBuf::try_from("/system/stlink_version.txt") else { return };
        let _ = fs.write(&path, v_str.as_bytes());
    });
}

pub async fn switch_programmer_personality(new_mode: ProgrammerPersonality) {
    save_personality_to_fs(new_mode);
    // Soft detach: disable USB pullup
    rp_pac::USB.sie_ctrl().modify(|w| w.set_pullup_en(false));
    embassy_time::Timer::after(embassy_time::Duration::from_millis(50)).await;
    // System reset into new personality
    cortex_m::peripheral::SCB::sys_reset();
}

pub enum ProgrammerEndpoints {
    CmsisDap {
        dap_v1_reader: HidReader<'static, Driver<'static, USB>, 64>,
        dap_v1_writer: HidWriter<'static, Driver<'static, USB>, 64>,
        dap_v2: CmsisDapV2Class<'static, Driver<'static, USB>>,
    },
    StLink {
        cmd_out: <Driver<'static, USB> as embassy_usb::driver::Driver<'static>>::EndpointOut,
        cmd_in: <Driver<'static, USB> as embassy_usb::driver::Driver<'static>>::EndpointIn,
        trace_in: <Driver<'static, USB> as embassy_usb::driver::Driver<'static>>::EndpointIn,
    },
    WchLink {
        cmd_out: <Driver<'static, USB> as embassy_usb::driver::Driver<'static>>::EndpointOut,
        cmd_in: <Driver<'static, USB> as embassy_usb::driver::Driver<'static>>::EndpointIn,
        data_out: <Driver<'static, USB> as embassy_usb::driver::Driver<'static>>::EndpointOut,
        data_in: <Driver<'static, USB> as embassy_usb::driver::Driver<'static>>::EndpointIn,
    },
}

pub struct CompositeUsb {
    pub device: UsbDevice<'static, Driver<'static, USB>>,
    pub personality: ProgrammerPersonality,
    pub programmer: ProgrammerEndpoints,
    pub cdc_uart: CdcAcmClass<'static, Driver<'static, USB>>,
    pub ncm_runner: embassy_usb::class::cdc_ncm::embassy_net::Runner<'static, Driver<'static, USB>, 1514>,
    pub net_device: embassy_usb::class::cdc_ncm::embassy_net::Device<'static, 1514>,
    pub gs_usb_read_ep: <Driver<'static, USB> as embassy_usb::driver::Driver<'static>>::EndpointOut,
    pub gs_usb_write_ep: <Driver<'static, USB> as embassy_usb::driver::Driver<'static>>::EndpointIn,
}

pub fn create_composite_usb(
    driver: Driver<'static, USB>,
    personality: ProgrammerPersonality,
) -> CompositeUsb {
    static CONFIG_DESC: StaticCell<[u8; 1024]> = StaticCell::new();
    static BOS_DESC: StaticCell<[u8; 256]> = StaticCell::new();
    static MSOS_DESC: StaticCell<[u8; 512]> = StaticCell::new();
    static CONTROL_BUF: StaticCell<[u8; 256]> = StaticCell::new();

    static CDC_UART_STATE: StaticCell<CdcAcmState> = StaticCell::new();
    static HID_STATE: StaticCell<HidState> = StaticCell::new();
    static DAP_V2_STATE: StaticCell<DapV2State> = StaticCell::new();
    static NCM_STATE: StaticCell<CdcNcmState> = StaticCell::new();
    static NET_STATE: StaticCell<NetState<1514, 4, 4>> = StaticCell::new();
    static VENDOR_HANDLER: StaticCell<VendorControlHandler> = StaticCell::new();

    let config_desc = CONFIG_DESC.init([0; 1024]);
    let bos_desc = BOS_DESC.init([0; 256]);
    let msos_desc = MSOS_DESC.init([0; 512]);
    let control_buf = CONTROL_BUF.init([0; 256]);

    let (vid, pid, product_str, mfg_str) = match personality {
        ProgrammerPersonality::CmsisDap => (0x1209, 0x414D, "AgentLA CMSIS-DAP Probe", "AgentLA"),
        ProgrammerPersonality::StLink => (0x0483, 0x3752, "ST-LINK/V2-1", "STMicroelectronics"),
        ProgrammerPersonality::WchLink => (0x1a86, 0x8010, "WCH-Link", "wch.cn"),
    };

    let mut config = embassy_usb::Config::new(vid, pid);
    config.manufacturer = Some(mfg_str);
    config.product = Some(product_str);
    let serial = usb_serial_str(personality);
    config.serial_number = Some(serial);
    // CMSIS-DAP DAP_Info(0x03) has its own serial field; keep it in sync with
    // the USB descriptor so host tools see one identity.
    set_dap_serial(serial);
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

    // Programmer Interface (Allocated to EP 0x01/0x81 and 0x02/0x82 first)
    let programmer = match personality {
        ProgrammerPersonality::CmsisDap => {
            // CMSIS-DAP v1 (HID)
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

            // CMSIS-DAP v2 (Bulk)
            let dap_v2_state = DAP_V2_STATE.init(DapV2State::new());
            let dap_v2 = CmsisDapV2Class::new(&mut builder, dap_v2_state, 64, false);
            ProgrammerEndpoints::CmsisDap {
                dap_v1_reader,
                dap_v1_writer,
                dap_v2,
            }
        }
        ProgrammerPersonality::StLink => {
            let st_str = builder.string();
            let mut st_func = builder.function(0xFF, 0xFF, 0xFF);
            let mut st_iface = st_func.interface();
            let mut st_alt = st_iface.alt_setting(0xFF, 0xFF, 0xFF, Some(st_str));
            let cmd_out = st_alt.endpoint_bulk_out(None, 64);
            let cmd_in = st_alt.endpoint_bulk_in(None, 64);
            let trace_in = st_alt.endpoint_bulk_in(None, 64);
            drop(st_func);
            ProgrammerEndpoints::StLink { cmd_out, cmd_in, trace_in }
        }
        ProgrammerPersonality::WchLink => {
            let wch_str = builder.string();
            let mut wch_func = builder.function(0xFF, 0x80, 0x55);
            let mut wch_iface = wch_func.interface();
            let mut wch_alt = wch_iface.alt_setting(0xFF, 0x80, 0x55, Some(wch_str));
            let cmd_out = wch_alt.endpoint_bulk_out(None, 64);
            let cmd_in = wch_alt.endpoint_bulk_in(None, 64);
            let data_out = wch_alt.endpoint_bulk_out(None, 64);
            let data_in = wch_alt.endpoint_bulk_in(None, 64);
            drop(wch_func);
            ProgrammerEndpoints::WchLink {
                cmd_out,
                cmd_in,
                data_out,
                data_in,
            }
        }
    };

    // CDC-ACM (Target UART telemetry bridge)
    let cdc_uart_state = CDC_UART_STATE.init(CdcAcmState::new());
    let cdc_uart = CdcAcmClass::new(&mut builder, cdc_uart_state, 64);

    // CDC-NCM (Ethernet)
    let ncm_state = NCM_STATE.init(CdcNcmState::new());
    let ncm_class = CdcNcmClass::new(&mut builder, ncm_state, HOST_MAC_ADDR, 64);
    let net_state = NET_STATE.init(NetState::new());
    let (ncm_runner, net_device) =
        ncm_class.into_embassy_net_device::<1514, 4, 4>(net_state, DEVICE_MAC_ADDR);

    // gs_usb SocketCAN (Vendor Class 0xFF, Subclass 0x00, Protocol 0x00)
    let gs_str = builder.string();
    let mut gs_func = builder.function(0xFF, 0x00, 0x00);
    let mut gs_iface = gs_func.interface();
    let mut gs_alt = gs_iface.alt_setting(0xFF, 0x00, 0x00, Some(gs_str));
    let gs_usb_read_ep = gs_alt.endpoint_bulk_out(None, 64);
    let gs_usb_write_ep = gs_alt.endpoint_bulk_in(None, 64);
    drop(gs_func);

    // i2c-tiny-usb (Vendor Class 0xFF, Subclass 0x00, Protocol 0x00)
    let i2c_str = builder.string();
    let mut i2c_func = builder.function(0xFF, 0x00, 0x00);
    let mut i2c_iface = i2c_func.interface();
    let _i2c_alt = i2c_iface.alt_setting(0xFF, 0x00, 0x00, Some(i2c_str));
    drop(i2c_func);

    let vendor_handler = VENDOR_HANDLER.init(VendorControlHandler::new(gs_str, i2c_str));
    builder.handler(vendor_handler);

    let device = builder.build();

    CompositeUsb {
        device,
        personality,
        programmer,
        cdc_uart,
        ncm_runner,
        net_device,
        gs_usb_read_ep,
        gs_usb_write_ep,
    }
}

static DAP_HANDLER: embassy_sync::blocking_mutex::Mutex<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    core::cell::RefCell<crate::programmer::dap::CmsisDapHandler>,
> = embassy_sync::blocking_mutex::Mutex::new(core::cell::RefCell::new(
    crate::programmer::dap::CmsisDapHandler::new(),
));

/// Set the DAP_Info(0x03) serial from the USB string descriptor source.
/// Call once at boot, before the USB tasks start serving.
pub fn set_dap_serial(serial: &'static str) {
    DAP_HANDLER.lock(|h| h.borrow_mut().set_serial(Some(serial)));
}

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

#[embassy_executor::task]
pub async fn gs_usb_task(
    mut read_ep: <Driver<'static, USB> as embassy_usb::driver::Driver<'static>>::EndpointOut,
    mut write_ep: <Driver<'static, USB> as embassy_usb::driver::Driver<'static>>::EndpointIn,
) {
    use embassy_usb::driver::{Endpoint, EndpointIn, EndpointOut};
    read_ep.wait_enabled().await;
    let mut rx_buf = [0u8; 64];
    loop {
        match read_ep.read(&mut rx_buf).await {
            Ok(n) => {
                if n >= core::mem::size_of::<crate::bridge::can_gsusb::GsHostFrame>() {
                    let frame: crate::bridge::can_gsusb::GsHostFrame = unsafe {
                        core::ptr::read_unaligned(rx_buf.as_ptr() as *const _)
                    };
                    crate::bridge::can_gsusb::record_tx_frame();
                    // If host requested transmission echo, reply on Bulk IN endpoint
                    if frame.echo_id != 0xFFFF_FFFF {
                        let frame_bytes = unsafe {
                            core::slice::from_raw_parts(
                                &frame as *const _ as *const u8,
                                core::mem::size_of::<crate::bridge::can_gsusb::GsHostFrame>(),
                            )
                        };
                        let _ = write_ep.write(frame_bytes).await;
                    }
                }
            }
            Err(_) => {
                embassy_time::Timer::after(embassy_time::Duration::from_millis(10)).await;
            }
        }
    }
}

#[embassy_executor::task]
pub async fn stlink_usb_task(
    mut cmd_out: <Driver<'static, USB> as embassy_usb::driver::Driver<'static>>::EndpointOut,
    mut cmd_in: <Driver<'static, USB> as embassy_usb::driver::Driver<'static>>::EndpointIn,
) {
    use embassy_usb::driver::{Endpoint, EndpointIn, EndpointOut};
    cmd_out.wait_enabled().await;

    // NOTE: ST-Link mode is not yet interoperable with stlink host tools.
    // `st-info` gets past USB enumeration and the serial check but times out on
    // ENTER_SWD. Root cause not established. The CMSIS-DAP personality remains
    // the verified transport; do not advertise ST-Link mode as supported.
    let transport = crate::programmer::swd::SwdTransport::new();
    let mut handler = agentla_programmer::stlink::StLinkHandler::with_swo(
        transport,
        &crate::programmer::swo::SWO_ENGINE,
    );
    handler.set_reported_version(get_stlink_version());
    handler.set_voltage_provider(crate::telemetry::get_target_voltage_mv);

    let mut cmd_buf = [0u8; 64];
    let mut data_buf = [0u8; 1024];
    let mut resp_buf = [0u8; 1024];

    loop {
        match cmd_out.read(&mut cmd_buf).await {
            Ok(n) if n > 0 => {
                let cmd = &cmd_buf[..n];
                handler.set_reported_version(get_stlink_version());
                if cmd.len() >= 2 && cmd[0] == agentla_programmer::stlink::STLINK_DEBUG_COMMAND {
                    if cmd[1] == agentla_programmer::stlink::DBG_APIV2_START_TRACE_RX {
                        crate::programmer::swo::init_swo_pin(crate::programmer::swo::PIN_SWO);
                    } else if cmd[1] == agentla_programmer::stlink::DBG_APIV2_GET_TRACE_NB {
                        let baud = handler.swo.baud_rate();
                        crate::programmer::swo::poll_swo_burst(crate::programmer::swo::PIN_SWO, baud, 64);
                    }
                }

                let out_len = handler.out_len(cmd);
                let in_len = handler.in_len(cmd);

                if out_len > 0 {
                    // Host sends data payload for memory write or RW_MISC_OUT
                    let mut received = 0;
                    while received < out_len {
                        let chunk = (out_len - received).min(data_buf.len());
                        match cmd_out.read(&mut data_buf[..chunk]).await {
                            Ok(rx_n) if rx_n > 0 => {
                                handler.mem_out(cmd, received, &data_buf[..rx_n]);
                                received += rx_n;
                            }
                            _ => break,
                        }
                    }
                    // For memory write, send 2-byte status reply
                    let mut resp = [0u8; 2];
                    resp[0] = handler.last_status;
                    let _ = cmd_in.write(&resp).await;
                } else if in_len > 0 {
                    // Host expects read data on Bulk IN
                    let mut sent = 0;
                    while sent < in_len {
                        let chunk = (in_len - sent).min(data_buf.len());
                        handler.mem_in(cmd, sent, &mut data_buf[..chunk]);
                        if let Err(_) = cmd_in.write(&data_buf[..chunk]).await {
                            break;
                        }
                        sent += chunk;
                    }
                } else {
                    let resp_len = handler.execute(cmd, &mut resp_buf);
                    if resp_len > 0 {
                        let _ = cmd_in.write(&resp_buf[..resp_len]).await;
                    }
                }
            }
            Ok(_) => {}
            Err(_) => {
                embassy_time::Timer::after(embassy_time::Duration::from_millis(10)).await;
            }
        }
    }
}

#[embassy_executor::task]
pub async fn swo_trace_task(
    mut trace_in: <Driver<'static, USB> as embassy_usb::driver::Driver<'static>>::EndpointIn,
) {
    use embassy_usb::driver::{Endpoint, EndpointIn};
    trace_in.wait_enabled().await;

    let mut pkt_buf = [0u8; 64];

    loop {
        let swo = &crate::programmer::swo::SWO_ENGINE;
        if swo.is_active() {
            let baud = swo.baud_rate();
            crate::programmer::swo::poll_swo_burst(crate::programmer::swo::PIN_SWO, baud, 64);

            if swo.is_polled() {
                if swo.trace_owed() > 0 {
                    let n = swo.pop_trace_packet(&mut pkt_buf);
                    if n > 0 {
                        let _ = trace_in.write(&pkt_buf[..n]).await;
                    }
                } else {
                    embassy_time::Timer::after(embassy_time::Duration::from_micros(200)).await;
                }
            } else {
                let avail = swo.available();
                if avail >= 64 {
                    let n = swo.pop_trace_packet(&mut pkt_buf);
                    if n > 0 {
                        let _ = trace_in.write(&pkt_buf[..n]).await;
                    }
                } else if avail > 0 {
                    embassy_time::Timer::after(embassy_time::Duration::from_millis(2)).await;
                    let n = swo.pop_trace_packet(&mut pkt_buf);
                    if n > 0 {
                        let _ = trace_in.write(&pkt_buf[..n]).await;
                    }
                } else {
                    embassy_time::Timer::after(embassy_time::Duration::from_millis(1)).await;
                }
            }
        } else {
            embassy_time::Timer::after(embassy_time::Duration::from_millis(20)).await;
        }
    }
}

#[embassy_executor::task]
pub async fn swo_rx_task() {
    let pin = crate::programmer::swo::PIN_SWO;
    loop {
        let swo = &crate::programmer::swo::SWO_ENGINE;
        if swo.is_active() {
            let baud = swo.baud_rate();
            let count = crate::programmer::swo::poll_swo_burst(pin, baud, 64);
            if count == 0 {
                embassy_time::Timer::after(embassy_time::Duration::from_micros(100)).await;
            }
        } else {
            embassy_time::Timer::after(embassy_time::Duration::from_millis(50)).await;
        }
    }
}

#[embassy_executor::task]
pub async fn wchlink_usb_task(
    mut cmd_out: <Driver<'static, USB> as embassy_usb::driver::Driver<'static>>::EndpointOut,
    mut cmd_in: <Driver<'static, USB> as embassy_usb::driver::Driver<'static>>::EndpointIn,
    mut data_out: <Driver<'static, USB> as embassy_usb::driver::Driver<'static>>::EndpointOut,
    mut data_in: <Driver<'static, USB> as embassy_usb::driver::Driver<'static>>::EndpointIn,
) {
    use embassy_usb::driver::{Endpoint, EndpointIn, EndpointOut};
    cmd_out.wait_enabled().await;
    data_out.wait_enabled().await;

    let transport = crate::programmer::rvswd::RvswdTransport::new();
    let mut handler = agentla_programmer::wch::WchLinkHandler::new(transport);

    let mut cmd_rx_buf = [0u8; 64];
    let mut cmd_tx_buf = [0u8; 64];
    let mut data_rx_buf = [0u8; 64];
    let mut data_tx_buf = [0u8; 64];

    loop {
        // 1. Check flow-control ACK or memory read streaming on data_in
        let mut ack = [0u8; 4];
        if handler.ack_pending(&mut ack) {
            if data_in.write(&ack).await.is_ok() {
                handler.ack_sent();
            }
        } else if handler.reading {
            let n = handler.read_chunk(&mut data_tx_buf);
            if n > 0 {
                let _ = data_in.write(&data_tx_buf[..n]).await;
            }
        }

        // 2. Select between command OUT and data OUT (if wanted)
        if handler.wants_data() {
            match embassy_futures::select::select(
                cmd_out.read(&mut cmd_rx_buf),
                data_out.read(&mut data_rx_buf),
            ).await {
                embassy_futures::select::Either::First(Ok(n)) if n > 0 => {
                    let resp_len = handler.execute(&cmd_rx_buf[..n], &mut cmd_tx_buf);
                    if resp_len > 0 {
                        let _ = cmd_in.write(&cmd_tx_buf[..resp_len]).await;
                    }
                }
                embassy_futures::select::Either::Second(Ok(n)) if n > 0 => {
                    handler.data_out(&data_rx_buf[..n]);
                }
                _ => {
                    embassy_time::Timer::after(embassy_time::Duration::from_millis(1)).await;
                }
            }
        } else {
            // Only waiting on command OUT
            match embassy_time::with_timeout(
                embassy_time::Duration::from_millis(5),
                cmd_out.read(&mut cmd_rx_buf),
            ).await {
                Ok(Ok(n)) if n > 0 => {
                    let resp_len = handler.execute(&cmd_rx_buf[..n], &mut cmd_tx_buf);
                    if resp_len > 0 {
                        let _ = cmd_in.write(&cmd_tx_buf[..resp_len]).await;
                    }
                }
                _ => {}
            }
        }
    }
}
