//! Linux SocketCAN gs_usb protocol engine.
//!
//! Conforms to Linux kernel `gs_usb` protocol (`drivers/net/can/usb/gs_usb.c`).
//! Provides request decoding, bit timing math, and CAN frame serialization.

use embedded_can::{ExtendedId, Frame, Id, StandardId};
use crate::can::CanFrame;

pub const GS_CAN_MODE_RESET: u32 = 0;
pub const GS_CAN_MODE_START: u32 = 1;

pub const GS_USB_BREQ_HOST_FORMAT: u8 = 0;
pub const GS_USB_BREQ_BITTIMING: u8 = 1;
pub const GS_USB_BREQ_MODE: u8 = 2;
pub const GS_USB_BREQ_BERR: u8 = 3;
pub const GS_USB_BREQ_BT_CONST: u8 = 4;
pub const GS_USB_BREQ_DEVICE_CONFIG: u8 = 5;
pub const GS_USB_BREQ_TIMESTAMP: u8 = 6;
pub const GS_USB_BREQ_IDENTIFY: u8 = 7;
pub const GS_USB_BREQ_USER_ID: u8 = 8;
pub const GS_USB_BREQ_DATA_BITTIMING: u8 = 9;
pub const GS_USB_BREQ_BT_CONST_EXT: u8 = 10;
pub const GS_USB_BREQ_SET_TERMINATION: u8 = 12;
pub const GS_USB_BREQ_GET_TERMINATION: u8 = 13;
pub const GS_USB_BREQ_GET_BERR: u8 = 14;

pub const GS_CAN_EFF_FLAG: u32 = 0x8000_0000;
pub const GS_CAN_RTR_FLAG: u32 = 0x4000_0000;
pub const GS_CAN_ERR_FLAG: u32 = 0x2000_0000;
pub const GS_CAN_SFF_MASK: u32 = 0x0000_07FF;
pub const GS_CAN_EFF_MASK: u32 = 0x1FFF_FFFF;
pub const GS_CAN_ERR_MASK: u32 = 0x1FFF_FFFF;

pub const GS_CAN_FEATURE_LISTEN_ONLY: u32 = 1 << 0;
pub const GS_CAN_FEATURE_LOOP_BACK: u32 = 1 << 1;
pub const GS_CAN_FEATURE_TRIPLE_SAMPLE: u32 = 1 << 2;
pub const GS_CAN_FEATURE_ONE_SHOT: u32 = 1 << 3;
pub const GS_CAN_FEATURE_HW_TIMESTAMP: u32 = 1 << 4;
pub const GS_CAN_FEATURE_IDENTIFY: u32 = 1 << 5;
pub const GS_CAN_FEATURE_USER_ID: u32 = 1 << 6;
pub const GS_CAN_FEATURE_PAD_PKTS_TO_MAX_PKT_SIZE: u32 = 1 << 7;
pub const GS_CAN_FEATURE_FD: u32 = 1 << 8;
pub const GS_CAN_FEATURE_REQ_USB_QUIRK_LPC546XX: u32 = 1 << 9;
pub const GS_CAN_FEATURE_BT_CONST_EXT: u32 = 1 << 10;
pub const GS_CAN_FEATURE_TERMINATION: u32 = 1 << 11;
pub const GS_CAN_FEATURE_BERR_REPORTING: u32 = 1 << 12;
pub const GS_CAN_FEATURE_GET_STATE: u32 = 1 << 13;

pub const GS_CAN_HOST_FORMAT_LITTLE_ENDIAN: u32 = 0x0000_BEEF;

pub const GS_CAN_BASIC_FEATURES: u32 = GS_CAN_FEATURE_LISTEN_ONLY
    | GS_CAN_FEATURE_LOOP_BACK
    | GS_CAN_FEATURE_TRIPLE_SAMPLE;

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GsHostConfig {
    pub byte_order: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GsDeviceConfig {
    pub reserved1: u8,
    pub reserved2: u8,
    pub reserved3: u8,
    pub icount: u8,
    pub sw_version: u32,
    pub hw_version: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GsDeviceBtConst {
    pub feature: u32,
    pub fclk_can: u32,
    pub tseg1_min: u32,
    pub tseg1_max: u32,
    pub tseg2_min: u32,
    pub tseg2_max: u32,
    pub sjw_max: u32,
    pub brp_min: u32,
    pub brp_max: u32,
    pub brp_inc: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GsDeviceBittiming {
    pub prop_seg: u32,
    pub phase_seg1: u32,
    pub phase_seg2: u32,
    pub sjw: u32,
    pub brp: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GsDeviceMode {
    pub mode: u32,
    pub flags: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GsHostFrame {
    pub echo_id: u32,
    pub can_id: u32,
    pub can_dlc: u8,
    pub channel: u8,
    pub flags: u8,
    pub reserved: u8,
    pub data: [u8; 8],
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct GsDeviceBerr {
    pub rxerr: u32,
    pub txerr: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum GsUsbError {
    InvalidRequest(u8),
    BufferTooShort,
    InvalidPayload,
    InvalidByteOrder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub enum GsControlAction {
    ModeChange(u32),
    BittimingUpdated(u32),
    TerminationChanged(bool),
    FormatNegotiated(u32),
}

impl GsHostFrame {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn echo_id(&self) -> u32 {
        self.echo_id
    }

    pub fn can_id(&self) -> u32 {
        self.can_id
    }

    pub fn can_dlc(&self) -> u8 {
        self.can_dlc
    }

    pub fn channel(&self) -> u8 {
        self.channel
    }

    pub fn data_bytes(&self) -> [u8; 8] {
        self.data
    }

    pub fn from_can_frame(frame: &CanFrame, echo_id: u32, channel: u8) -> Self {
        let mut can_id = match frame.id() {
            Id::Standard(sid) => (sid.as_raw() as u32) & GS_CAN_SFF_MASK,
            Id::Extended(eid) => (eid.as_raw() & GS_CAN_EFF_MASK) | GS_CAN_EFF_FLAG,
        };

        if frame.is_remote_frame() {
            can_id |= GS_CAN_RTR_FLAG;
        }

        let dlc = frame.dlc().min(8) as u8;
        let mut data = [0u8; 8];
        if !frame.is_remote_frame() {
            data[..dlc as usize].copy_from_slice(frame.data());
        }

        Self {
            echo_id,
            can_id,
            can_dlc: dlc,
            channel,
            flags: 0,
            reserved: 0,
            data,
        }
    }

    pub fn to_can_frame(&self) -> Option<CanFrame> {
        let dlc = self.can_dlc as usize;
        if dlc > 8 {
            return None;
        }

        let is_extended = (self.can_id & GS_CAN_EFF_FLAG) != 0;
        let is_rtr = (self.can_id & GS_CAN_RTR_FLAG) != 0;

        let id = if is_extended {
            let ext_raw = self.can_id & GS_CAN_EFF_MASK;
            Id::Extended(ExtendedId::new(ext_raw)?)
        } else {
            let std_raw = (self.can_id & GS_CAN_SFF_MASK) as u16;
            Id::Standard(StandardId::new(std_raw)?)
        };

        if is_rtr {
            CanFrame::new_remote(id, dlc)
        } else {
            CanFrame::new(id, &self.data[..dlc])
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts(
                self as *const _ as *const u8,
                core::mem::size_of::<Self>(),
            )
        }
    }

    pub fn from_bytes(bytes: &[u8]) -> Option<Self> {
        if bytes.len() < core::mem::size_of::<Self>() {
            return None;
        }
        let frame = unsafe { core::ptr::read_unaligned(bytes.as_ptr() as *const Self) };
        Some(frame)
    }
}

pub fn calculate_bitrate(fclk_can: u32, bt: &GsDeviceBittiming) -> Option<u32> {
    let t_total = 1 + bt.prop_seg + bt.phase_seg1 + bt.phase_seg2;
    if bt.brp > 0 && t_total > 0 {
        Some(fclk_can / (bt.brp * t_total))
    } else {
        None
    }
}

pub fn validate_host_format(byte_order: u32) -> Result<u32, GsUsbError> {
    if byte_order == GS_CAN_HOST_FORMAT_LITTLE_ENDIAN || byte_order == 1 {
        Ok(GS_CAN_BASIC_FEATURES)
    } else {
        Err(GsUsbError::InvalidByteOrder)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "defmt", derive(defmt::Format))]
pub struct GsUsbEngine {
    pub mode: u32,
    pub flags: u32,
    pub bitrate: u32,
    pub termination: bool,
    pub fclk_can: u32,
    pub tx_frames: u32,
    pub rx_frames: u32,
}

impl GsUsbEngine {
    pub const fn new(fclk_can: u32) -> Self {
        Self {
            mode: GS_CAN_MODE_RESET,
            flags: 0,
            bitrate: 500_000,
            termination: false,
            fclk_can,
            tx_frames: 0,
            rx_frames: 0,
        }
    }

    pub fn is_active(&self) -> bool {
        self.mode == GS_CAN_MODE_START
    }

    pub fn record_tx(&mut self) {
        self.tx_frames = self.tx_frames.saturating_add(1);
    }

    pub fn record_rx(&mut self) {
        self.rx_frames = self.rx_frames.saturating_add(1);
    }

    pub fn handle_control_out(
        &mut self,
        request: u8,
        value: u16,
        _index: u16,
        data: &[u8],
    ) -> Result<Option<GsControlAction>, GsUsbError> {
        match request {
            GS_USB_BREQ_HOST_FORMAT => {
                let byte_order = if data.len() >= core::mem::size_of::<GsHostConfig>() {
                    let cfg = unsafe { core::ptr::read_unaligned(data.as_ptr() as *const GsHostConfig) };
                    cfg.byte_order
                } else {
                    value as u32
                };
                let features = validate_host_format(byte_order)?;
                Ok(Some(GsControlAction::FormatNegotiated(features)))
            }
            GS_USB_BREQ_BITTIMING => {
                if data.len() < core::mem::size_of::<GsDeviceBittiming>() {
                    return Err(GsUsbError::BufferTooShort);
                }
                let bt = unsafe { core::ptr::read_unaligned(data.as_ptr() as *const GsDeviceBittiming) };
                if let Some(br) = calculate_bitrate(self.fclk_can, &bt) {
                    self.bitrate = br;
                    Ok(Some(GsControlAction::BittimingUpdated(br)))
                } else {
                    Err(GsUsbError::InvalidPayload)
                }
            }
            GS_USB_BREQ_MODE => {
                if data.len() < core::mem::size_of::<GsDeviceMode>() {
                    return Err(GsUsbError::BufferTooShort);
                }
                let mode_buf = unsafe { core::ptr::read_unaligned(data.as_ptr() as *const GsDeviceMode) };
                self.mode = mode_buf.mode;
                self.flags = mode_buf.flags;
                Ok(Some(GsControlAction::ModeChange(mode_buf.mode)))
            }
            GS_USB_BREQ_SET_TERMINATION => {
                let enable = if !data.is_empty() {
                    let mut b = [0u8; 4];
                    let copy_len = data.len().min(4);
                    b[..copy_len].copy_from_slice(&data[..copy_len]);
                    u32::from_le_bytes(b) != 0
                } else {
                    value != 0
                };
                self.termination = enable;
                Ok(Some(GsControlAction::TerminationChanged(enable)))
            }
            GS_USB_BREQ_BERR | GS_USB_BREQ_TIMESTAMP | GS_USB_BREQ_IDENTIFY => Ok(None),
            _ => Err(GsUsbError::InvalidRequest(request)),
        }
    }

    pub fn handle_control_in(
        &self,
        request: u8,
        _value: u16,
        _index: u16,
        buf: &mut [u8],
    ) -> Result<usize, GsUsbError> {
        match request {
            GS_USB_BREQ_DEVICE_CONFIG => {
                let cfg = GsDeviceConfig {
                    reserved1: 0,
                    reserved2: 0,
                    reserved3: 0,
                    icount: 0, // 1 channel (channel 0)
                    sw_version: 1,
                    hw_version: 1,
                };
                let raw_bytes = unsafe {
                    core::slice::from_raw_parts(
                        &cfg as *const _ as *const u8,
                        core::mem::size_of::<GsDeviceConfig>(),
                    )
                };
                let len = raw_bytes.len().min(buf.len());
                buf[..len].copy_from_slice(&raw_bytes[..len]);
                Ok(len)
            }
            GS_USB_BREQ_BT_CONST => {
                let bt = GsDeviceBtConst {
                    feature: 0,
                    fclk_can: self.fclk_can,
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
                        core::mem::size_of::<GsDeviceBtConst>(),
                    )
                };
                let len = raw_bytes.len().min(buf.len());
                buf[..len].copy_from_slice(&raw_bytes[..len]);
                Ok(len)
            }
            GS_USB_BREQ_GET_TERMINATION => {
                let term: u32 = if self.termination { 1 } else { 0 };
                let bytes = term.to_le_bytes();
                let len = bytes.len().min(buf.len());
                buf[..len].copy_from_slice(&bytes[..len]);
                Ok(len)
            }
            GS_USB_BREQ_TIMESTAMP => {
                let ts: u32 = 0;
                let bytes = ts.to_le_bytes();
                let len = bytes.len().min(buf.len());
                buf[..len].copy_from_slice(&bytes[..len]);
                Ok(len)
            }
            GS_USB_BREQ_BERR => {
                let zeros = [0u8; 8];
                let len = zeros.len().min(buf.len());
                buf[..len].copy_from_slice(&zeros[..len]);
                Ok(len)
            }
            _ => Err(GsUsbError::InvalidRequest(request)),
        }
    }
}
