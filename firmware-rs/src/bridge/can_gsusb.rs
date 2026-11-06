#![allow(dead_code)]
//! SocketCAN gs_usb protocol handler and CAN transceiver management for AgentLA.
//!
//! Implements native Linux kernel `gs_usb` protocol (drivers/net/can/usb/gs_usb.c)
//! and manages physical CAN transceiver pins:
//! - GPIO 18: CAN_TX (SIO output, high for recessive bus idle)
//! - GPIO 19: CAN_RX (SIO input with pull-up)
//! - GPIO 22: CAN_TERM (SIO output high for 120 Ω termination, High-Z when open)

use core::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
use rp_pac as pac;

use crate::board::ActiveBoard;
use crate::pin_owner::{self, ClaimError, Driver, SlotId};
use agentla_bsp::BoardProfile;

pub const CAN_TX_PIN: u32 = match ActiveBoard::ADAPTERS.can {
    Some(can) => can.tx,
    None => 18,
};
pub const CAN_RX_PIN: u32 = match ActiveBoard::ADAPTERS.can {
    Some(can) => can.rx,
    None => 19,
};
pub const CAN_TERM_PIN: u32 = match ActiveBoard::ADAPTERS.can {
    Some(can) => match can.term {
        Some(term) => term,
        None => 22,
    },
    None => 22,
};
pub const CAN_PINS: [u32; 3] = [CAN_TX_PIN, CAN_RX_PIN, CAN_TERM_PIN];

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
pub const GS_USB_BREQ_SET_TERMINATION: u8 = 12;
pub const GS_USB_BREQ_GET_TERMINATION: u8 = 13;

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GsHostConfig {
    pub byte_order: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GsDeviceConfig {
    pub reserved1: u8,
    pub reserved2: u8,
    pub reserved3: u8,
    pub icount: u8,
    pub sw_version: u32,
    pub hw_version: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default)]
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
#[derive(Clone, Copy, Debug, Default)]
pub struct GsDeviceBittiming {
    pub prop_seg: u32,
    pub phase_seg1: u32,
    pub phase_seg2: u32,
    pub sjw: u32,
    pub brp: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GsDeviceMode {
    pub mode: u32,
    pub flags: u32,
}

#[repr(C, packed)]
#[derive(Clone, Copy, Debug, Default)]
pub struct GsHostFrame {
    pub echo_id: u32,
    pub can_id: u32,
    pub can_dlc: u8,
    pub channel: u8,
    pub flags: u8,
    pub reserved: u8,
    pub data: [u8; 8],
}

static CAN_MODE: AtomicU32 = AtomicU32::new(GS_CAN_MODE_RESET);
static CAN_TERMINATION: AtomicBool = AtomicBool::new(false);
static TX_FRAMES: AtomicU32 = AtomicU32::new(0);
static RX_FRAMES: AtomicU32 = AtomicU32::new(0);
static BITRATE: AtomicU32 = AtomicU32::new(500_000);

static CAN_SLOT: Mutex<CriticalSectionRawMutex, core::cell::RefCell<Option<SlotId>>> =
    Mutex::new(core::cell::RefCell::new(None));

fn gpio_high_z(pin: u32) {
    unsafe {
        pac::IO_BANK0.gpio(pin as usize).ctrl().write(|w| {
            w.set_funcsel(5);
        });
        pac::PADS_BANK0.gpio(pin as usize).modify(|w| {
            w.set_iso(false);
            w.set_ie(false);
            w.set_od(true);
            w.set_pue(false);
            w.set_pde(false);
        });
        pac::SIO.gpio_oe(0).value_clr().write_value(1 << pin);
    }
}

fn gpio_out_high(pin: u32) {
    unsafe {
        pac::IO_BANK0.gpio(pin as usize).ctrl().write(|w| {
            w.set_funcsel(5);
        });
        pac::PADS_BANK0.gpio(pin as usize).modify(|w| {
            w.set_iso(false);
            w.set_ie(false);
            w.set_od(false);
            w.set_pue(false);
            w.set_pde(false);
        });
        pac::SIO.gpio_out(0).value_set().write_value(1 << pin);
        pac::SIO.gpio_oe(0).value_set().write_value(1 << pin);
    }
}

fn gpio_in_pullup(pin: u32) {
    unsafe {
        pac::IO_BANK0.gpio(pin as usize).ctrl().write(|w| {
            w.set_funcsel(5);
        });
        pac::PADS_BANK0.gpio(pin as usize).modify(|w| {
            w.set_iso(false);
            w.set_ie(true);
            w.set_od(true);
            w.set_pue(true);
            w.set_pde(false);
        });
        pac::SIO.gpio_oe(0).value_clr().write_value(1 << pin);
    }
}

pub fn adapter_can_init() {
    adapter_can_standby();
}

pub fn adapter_can_is_active() -> bool {
    CAN_MODE.load(Ordering::Relaxed) == GS_CAN_MODE_START
}

pub fn adapter_can_get_termination() -> bool {
    CAN_TERMINATION.load(Ordering::Relaxed)
}

pub fn adapter_can_set_termination(enable: bool) {
    CAN_TERMINATION.store(enable, Ordering::Relaxed);
    if enable {
        gpio_out_high(CAN_TERM_PIN);
    } else {
        gpio_high_z(CAN_TERM_PIN);
    }
}

pub fn adapter_can_get_bitrate() -> u32 {
    BITRATE.load(Ordering::Relaxed)
}

pub fn adapter_can_set_bittiming(bt: &GsDeviceBittiming) {
    let t_total = 1 + bt.prop_seg + bt.phase_seg1 + bt.phase_seg2;
    if bt.brp > 0 && t_total > 0 {
        let br = 48_000_000 / (bt.brp * t_total);
        BITRATE.store(br, Ordering::Relaxed);
    }
}

pub fn adapter_can_get_tx_frames() -> u32 {
    TX_FRAMES.load(Ordering::Relaxed)
}

pub fn adapter_can_get_rx_frames() -> u32 {
    RX_FRAMES.load(Ordering::Relaxed)
}

pub fn record_tx_frame() {
    TX_FRAMES.fetch_add(1, Ordering::Relaxed);
}

pub fn record_rx_frame() {
    RX_FRAMES.fetch_add(1, Ordering::Relaxed);
}

pub fn adapter_can_start() {
    CAN_MODE.store(GS_CAN_MODE_START, Ordering::Relaxed);

    CAN_SLOT.lock(|cell| {
        let mut slot_opt = cell.borrow_mut();
        if slot_opt.is_none() {
            if let Ok(slot) = pin_owner::claim(Driver::Adapter, &CAN_PINS, pin_owner::CAP_SIO) {
                *slot_opt = Some(slot);
            }
        }
    });

    // Drive CAN_TX high (recessive idle state)
    gpio_out_high(CAN_TX_PIN);

    // CAN_RX input with pull-up
    gpio_in_pullup(CAN_RX_PIN);

    // Configure termination if engaged
    if adapter_can_get_termination() {
        gpio_out_high(CAN_TERM_PIN);
    } else {
        gpio_high_z(CAN_TERM_PIN);
    }
}

pub fn adapter_can_standby() {
    CAN_MODE.store(GS_CAN_MODE_RESET, Ordering::Relaxed);

    CAN_SLOT.lock(|cell| {
        let mut slot_opt = cell.borrow_mut();
        if let Some(slot) = slot_opt.take() {
            let _ = pin_owner::release(slot);
        }
    });

    // Tri-state all CAN transceiver pins into High-Z
    gpio_high_z(CAN_TX_PIN);
    gpio_high_z(CAN_RX_PIN);
    gpio_high_z(CAN_TERM_PIN);
}
