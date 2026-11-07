//! Universal Target Diagnostic Engine for AgentLA MCP Server.
//!
//! Provides:
//! 1. Physical / electrical rail sensing and dynamic pad pull-down perturbation tests.
//! 2. CoreSight fault analysis & register reader (DHCSR, CFSR, HFSR, BFAR, PC, LR) + RISC-V DM fallback.
//! 3. Firmware vector table inspector and dynamic SRAM boundary validation.

use core::cell::RefCell;
use core::fmt::Write as _;
use rp_pac as pac;

use crate::board::ActiveBoard;
use agentla_bsp::BoardProfile;
use crate::programmer::pins::{TargetPort, PIN_RESET, PIN_SWCLK, PIN_SWDIO};
use crate::programmer::target::cortex_m::*;
use crate::programmer::target::qingke::*;
use crate::programmer::PROGRAMMER_PINS;

// ============================================================================
// Tier 1: Electrical Layer & Dynamic Pad Perturbation
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RailStatus {
    Nominal,
    Brownout,
    Unpowered,
}

impl RailStatus {
    pub const fn as_str(&self) -> &'static str {
        match self {
            RailStatus::Nominal => "nominal",
            RailStatus::Brownout => "brownout",
            RailStatus::Unpowered => "unpowered",
        }
    }
}

pub fn classify_rail_voltage(mv: u32) -> RailStatus {
    if mv < 500 {
        RailStatus::Unpowered
    } else if mv < 2700 {
        RailStatus::Brownout
    } else {
        RailStatus::Nominal
    }
}

/// Measures target rail voltage in millivolts using the RP2350 SAR ADC.
pub fn measure_target_rail_mv() -> u32 {
    // Check if board has dedicated vtarget sense with divider or direct ADC
    #[cfg(feature = "board-bakuprobe")]
    {
        // Bakuprobe pin 47 (ADC channel 7) with 10x attenuation
        let raw = read_adc_raw(7);
        ((raw as u32) * 33000) / 4095
    }
    #[cfg(not(feature = "board-bakuprobe"))]
    {
        // Direct ADC channel 2 (GPIO 28) or channel 0 (GPIO 26)
        let raw = read_adc_raw(2);
        ((raw as u32) * 3300) / 4095
    }
}

/// Reads a single raw 12-bit ADC conversion from the specified channel.
pub fn read_adc_raw(channel: u8) -> u16 {
    pac::RESETS.reset().modify(|w| w.set_adc(false));
    while !pac::RESETS.reset_done().read().adc() {}

    pac::ADC.cs().write(|w| w.set_en(true));
    while !pac::ADC.cs().read().ready() {}

    pac::ADC.cs().modify(|w| {
        w.set_ainsel(channel);
        w.set_start_once(true);
    });

    while !pac::ADC.cs().read().ready() {}
    pac::ADC.result().read().result()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PinElectricalState {
    DrivenHigh,
    PulledUp,
    Floating,
    ShortToGnd,
}

impl PinElectricalState {
    pub const fn as_str(&self) -> &'static str {
        match self {
            PinElectricalState::DrivenHigh => "driven_high",
            PinElectricalState::PulledUp => "pulled_up",
            PinElectricalState::Floating => "floating",
            PinElectricalState::ShortToGnd => "short_to_gnd",
        }
    }
}

/// Pure classification function for pad perturbation test.
pub fn classify_pin_perturbation(v_float: u8, v_pd: u8, v_pu: u8, v_drive_low: u8) -> PinElectricalState {
    let _ = v_float;
    if v_pu == 0 {
        PinElectricalState::ShortToGnd
    } else if v_pd == 0 && v_pu == 1 {
        PinElectricalState::Floating
    } else if v_drive_low != 0 {
        PinElectricalState::DrivenHigh
    } else {
        PinElectricalState::PulledUp
    }
}

/// Executes dynamic pad pull-down perturbation test on a specific GPIO pin (< 30 µs).
pub fn test_pin_perturbation(pin: usize) -> PinElectricalState {
    pac::SIO.gpio_oe(0).value_clr().write_value(1 << pin);
    pac::IO_BANK0.gpio(pin).ctrl().write(|w| w.set_funcsel(5));

    pac::PADS_BANK0.gpio(pin).modify(|w| {
        w.set_pue(false);
        w.set_pde(false);
    });
    cortex_m::asm::delay(750); // 5 µs
    let v_float = ((pac::SIO.gpio_in(0).read() >> pin) & 1) as u8;

    pac::PADS_BANK0.gpio(pin).modify(|w| {
        w.set_pue(false);
        w.set_pde(true);
    });
    cortex_m::asm::delay(750); // 5 µs
    let v_pd = ((pac::SIO.gpio_in(0).read() >> pin) & 1) as u8;

    pac::PADS_BANK0.gpio(pin).modify(|w| {
        w.set_pue(true);
        w.set_pde(false);
    });
    cortex_m::asm::delay(750); // 5 µs
    let v_pu = ((pac::SIO.gpio_in(0).read() >> pin) & 1) as u8;

    let mut v_drive = 0u8;
    if v_pd == 1 && v_pu == 1 {
        pac::PADS_BANK0.gpio(pin).modify(|w| w.set_drive(0.into()));
        pac::SIO.gpio_out(0).value_clr().write_value(1 << pin);
        pac::SIO.gpio_oe(0).value_set().write_value(1 << pin);
        cortex_m::asm::delay(150); // 1 µs
        v_drive = ((pac::SIO.gpio_in(0).read() >> pin) & 1) as u8;
        pac::SIO.gpio_oe(0).value_clr().write_value(1 << pin);
    }

    pac::PADS_BANK0.gpio(pin).modify(|w| {
        w.set_pue(true);
        w.set_pde(false);
    });

    classify_pin_perturbation(v_float, v_pd, v_pu, v_drive)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SwdPinReport {
    pub swclk: PinElectricalState,
    pub swdio: PinElectricalState,
    pub nrst: PinElectricalState,
}

impl SwdPinReport {
    pub fn is_healthy(&self) -> bool {
        self.swclk != PinElectricalState::ShortToGnd
            && self.swdio != PinElectricalState::ShortToGnd
            && self.nrst != PinElectricalState::ShortToGnd
    }
}

pub fn probe_swd_pins(swclk_pin: usize, swdio_pin: usize, nrst_pin: usize) -> SwdPinReport {
    SwdPinReport {
        swclk: test_pin_perturbation(swclk_pin),
        swdio: test_pin_perturbation(swdio_pin),
        nrst: test_pin_perturbation(nrst_pin),
    }
}

// ============================================================================
// Tier 2 & Tier 3: CoreSight SCB Reader, RISC-V Fallback, & Root Cause Synthesis
// ============================================================================

pub const REG_CPUID: u32 = 0xE000ED00;
pub const REG_CFSR: u32 = 0xE000ED28;
pub const REG_HFSR: u32 = 0xE000ED2C;
pub const REG_BFAR: u32 = 0xE000ED38;
pub const REG_DHCSR: u32 = 0xE000EDF0;

pub fn synthesize_root_cause(
    dhcsr: u32,
    cfsr: u32,
    hfsr: u32,
    bfar: u32,
    pc: u32,
    _lr: u32,
    out: &mut heapless::String<192>,
) {
    if (dhcsr & (1 << 19)) != 0 {
        let _ = write!(out, "HardFault Lockup: Core in double-fault lockup at PC 0x{:08X}", pc);
        return;
    }
    if cfsr == 0 && hfsr == 0 {
        if (dhcsr & (1 << 17)) != 0 {
            let _ = write!(out, "Core halted cleanly by debugger at PC 0x{:08X}", pc);
        } else {
            let _ = write!(out, "Core executing normally without faults");
        }
        return;
    }

    // UsageFault analysis
    if (cfsr & (1 << 25)) != 0 {
        let _ = write!(out, "UsageFault: Division by zero at PC 0x{:08X}", pc);
        return;
    }
    if (cfsr & (1 << 24)) != 0 {
        let _ = write!(out, "UsageFault: Unaligned memory access at PC 0x{:08X}", pc);
        return;
    }
    if (cfsr & (1 << 17)) != 0 {
        let _ = write!(out, "UsageFault: Invalid Thumb state (LSB=0) at PC 0x{:08X}", pc);
        return;
    }
    if (cfsr & (1 << 16)) != 0 {
        let _ = write!(out, "UsageFault: Undefined instruction at PC 0x{:08X}", pc);
        return;
    }
    if (cfsr & (1 << 19)) != 0 {
        let _ = write!(out, "UsageFault: Coprocessor disabled (FPU access) at PC 0x{:08X}", pc);
        return;
    }

    // BusFault analysis
    if (cfsr & (1 << 9)) != 0 {
        if (cfsr & (1 << 15)) != 0 {
            let _ = write!(out, "BusFault: Precise data bus error accessing 0x{:08X} (unclocked peripheral or unmapped memory) at PC 0x{:08X}", bfar, pc);
        } else {
            let _ = write!(out, "BusFault: Precise data bus error at PC 0x{:08X}", pc);
        }
        return;
    }
    if (cfsr & (1 << 8)) != 0 {
        let _ = write!(out, "BusFault: Instruction bus fetch error at PC 0x{:08X}", pc);
        return;
    }
    if (cfsr & (1 << 10)) != 0 {
        let _ = write!(out, "BusFault: Imprecise data bus error at PC 0x{:08X}", pc);
        return;
    }

    // MemManage Fault analysis
    if (cfsr & 0xFF) != 0 {
        let _ = write!(out, "MemManage: Memory protection fault (MPU violation) at PC 0x{:08X}", pc);
        return;
    }

    // HardFault Status Register
    if (hfsr & (1 << 1)) != 0 {
        let _ = write!(out, "HardFault: Vector table fetch failed (bad vector table offset)");
        return;
    }
    if (hfsr & (1 << 30)) != 0 {
        let _ = write!(out, "HardFault: Escalated from unhandled exception at PC 0x{:08X}", pc);
        return;
    }

    let _ = write!(out, "Fault: CFSR=0x{:08X} HFSR=0x{:08X} at PC 0x{:08X}", cfsr, hfsr, pc);
}

pub fn synthesize_riscv_root_cause(
    dmstatus: u32,
    mcause: u32,
    mepc: u32,
    mtval: u32,
    out: &mut heapless::String<192>,
) {
    if (dmstatus & (1 << 9)) == 0 && (dmstatus & (3 << 10)) != 0 {
        let _ = write!(out, "RISC-V core running normally");
        return;
    }
    match mcause & 0x7FFFFFFF {
        1 => { let _ = write!(out, "Instruction access fault at PC 0x{:08X}", mepc); }
        2 => { let _ = write!(out, "Illegal instruction 0x{:08X} at PC 0x{:08X}", mtval, mepc); }
        4 => { let _ = write!(out, "Load address misaligned accessing 0x{:08X} at PC 0x{:08X}", mtval, mepc); }
        5 => { let _ = write!(out, "Load access fault accessing 0x{:08X} at PC 0x{:08X}", mtval, mepc); }
        6 => { let _ = write!(out, "Store address misaligned accessing 0x{:08X} at PC 0x{:08X}", mtval, mepc); }
        7 => { let _ = write!(out, "Store access fault accessing 0x{:08X} at PC 0x{:08X}", mtval, mepc); }
        11 => { let _ = write!(out, "Environment call from M-mode at PC 0x{:08X}", mepc); }
        c => { let _ = write!(out, "RISC-V Exception cause={} at PC 0x{:08X} (mtval=0x{:08X})", c, mepc, mtval); }
    }
}

#[derive(Debug, Clone)]
pub struct TargetDiagnosticReport {
    pub rail_voltage_mv: u32,
    pub rail_status: RailStatus,
    pub pins: SwdPinReport,
    pub pin_integrity: bool,
    pub target_connected: bool,
    pub architecture: Option<&'static str>,
    pub dpidr: Option<u32>,
    pub cpuid: Option<u32>,
    pub dhcsr: Option<u32>,
    pub cfsr: Option<u32>,
    pub hfsr: Option<u32>,
    pub bfar: Option<u32>,
    pub pc: Option<u32>,
    pub lr: Option<u32>,
    pub status: &'static str,
    pub root_cause: heapless::String<192>,
}

impl TargetDiagnosticReport {
    pub fn to_json(&self) -> heapless::String<1024> {
        let mut out = heapless::String::new();
        let _ = write!(
            out,
            r#"{{"rail_voltage_mv":{},"rail_status":"{}","pins":{{"swclk":"{}","swdio":"{}","nrst":"{}"}},"pin_integrity":{},"target_connected":{}"#,
            self.rail_voltage_mv,
            self.rail_status.as_str(),
            self.pins.swclk.as_str(),
            self.pins.swdio.as_str(),
            self.pins.nrst.as_str(),
            self.pin_integrity,
            self.target_connected
        );
        if let Some(arch) = self.architecture {
            let _ = write!(out, r#","architecture":"{}""#, arch);
        }
        if let Some(dpidr) = self.dpidr {
            let _ = write!(out, r#","dpidr":"0x{:08x}""#, dpidr);
        }
        if let Some(cpuid) = self.cpuid {
            let _ = write!(out, r#","cpuid":"0x{:08x}""#, cpuid);
        }
        if self.dhcsr.is_some() || self.cfsr.is_some() || self.hfsr.is_some() || self.bfar.is_some() || self.pc.is_some() || self.lr.is_some() {
            let _ = write!(out, r#","registers":{{"#);
            let mut first = true;
            if let Some(v) = self.dhcsr {
                let _ = write!(out, r#""dhcsr":"0x{:08x}""#, v);
                first = false;
            }
            if let Some(v) = self.cfsr {
                if !first { let _ = out.push(','); }
                let _ = write!(out, r#""cfsr":"0x{:08x}""#, v);
                first = false;
            }
            if let Some(v) = self.hfsr {
                if !first { let _ = out.push(','); }
                let _ = write!(out, r#""hfsr":"0x{:08x}""#, v);
                first = false;
            }
            if let Some(v) = self.bfar {
                if !first { let _ = out.push(','); }
                let _ = write!(out, r#""bfar":"0x{:08x}""#, v);
                first = false;
            }
            if let Some(v) = self.pc {
                if !first { let _ = out.push(','); }
                let _ = write!(out, r#""pc":"0x{:08x}""#, v);
                first = false;
            }
            if let Some(v) = self.lr {
                if !first { let _ = out.push(','); }
                let _ = write!(out, r#""lr":"0x{:08x}""#, v);
            }
            let _ = out.push('}');
        }
        let _ = write!(
            out,
            r#","status":"{}","root_cause":"{}"}}"#,
            self.status,
            self.root_cause.as_str()
        );
        out
    }
}

/// Executes full one-shot target diagnostic.
pub fn run_target_diagnose(port_hint: Option<&str>) -> TargetDiagnosticReport {
    let rail_voltage_mv = measure_target_rail_mv();
    let rail_status = classify_rail_voltage(rail_voltage_mv);
    let (swclk_pin, swdio_pin, nrst_pin) = match port_hint {
        Some("ch32v003") => (14, 12, 13),
        Some("ch32v203") => (8, 9, 10),
        _ => (PIN_SWCLK, PIN_SWDIO, PIN_RESET),
    };
    let pins = probe_swd_pins(swclk_pin, swdio_pin, nrst_pin);
    let pin_integrity = pins.is_healthy();

    // 1. If board has dedicated target power control/sense and rail is unpowered (< 0.5V), return early
    if ActiveBoard::FEATURES.target_power_control && rail_status == RailStatus::Unpowered {
        let mut root_cause = heapless::String::new();
        let _ = write!(root_cause, "Target supply rail unpowered (< 0.5V)");
        return TargetDiagnosticReport {
            rail_voltage_mv,
            rail_status,
            pins,
            pin_integrity,
            target_connected: false,
            architecture: None,
            dpidr: None,
            cpuid: None,
            dhcsr: None,
            cfsr: None,
            hfsr: None,
            bfar: None,
            pc: None,
            lr: None,
            status: "unpowered",
            root_cause,
        };
    }

    // 2. Connect to target using PROGRAMMER_PINS lock
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins_ref = cell.borrow_mut();

        // Check if port_hint requests RISC-V specifically
        let try_rvswd = port_hint == Some("ch32v203") || port_hint == Some("ch32v003");

        if port_hint == Some("ch32v003") {
            // Fast-path: RISC-V 1-wire SWIO on Port 3 (GPIO 12 SWIO, 13 nRST)
            let mut swio_session = pins_ref.into_swio_port(TargetPort::Ch32v003);
            if let Ok(dmstatus) = swio_session.attach() {
                if dmstatus != 0 && dmstatus != 0xFFFFFFFF {
                    let mut qingke = QingKe::new_1wire(&mut swio_session.transport);
                    let mcause = qingke.read_reg(0x342).unwrap_or(0);
                    let mepc = qingke.read_reg(0x341).unwrap_or(0);
                    let mtval = qingke.read_reg(0x343).unwrap_or(0);
                    let pc = qingke.read_reg(0x7B1).ok();

                    let status = if (dmstatus & (1 << 9)) != 0 { "halted" } else { "running" };
                    let mut root_cause = heapless::String::new();
                    synthesize_riscv_root_cause(dmstatus, mcause, mepc, mtval, &mut root_cause);

                    *pins_ref = swio_session.release();
                    return TargetDiagnosticReport {
                        rail_voltage_mv,
                        rail_status,
                        pins,
                        pin_integrity,
                        target_connected: true,
                        architecture: Some("RISC-V (QingKe V2 1-Wire)"),
                        dpidr: None,
                        cpuid: None,
                        dhcsr: Some(dmstatus),
                        cfsr: Some(mcause),
                        hfsr: None,
                        bfar: Some(mtval),
                        pc,
                        lr: None,
                        status,
                        root_cause,
                    };
                }
            }
            *pins_ref = swio_session.release();
        }

        if !try_rvswd {
            // First try ARM SWD on BluePill (Port 1)
            let mut swd_session = pins_ref.into_swd_port(TargetPort::BluePill, 1000);
            if let Ok(dpidr) = swd_session.attach() {
                if dpidr != 0 && dpidr != 0xFFFFFFFF {
                    let mut cortex = CortexM::new(&mut swd_session.transport);
                    let cpuid = cortex.mem_read32(REG_CPUID).unwrap_or(0);
                    let dhcsr = cortex.mem_read32(REG_DHCSR).unwrap_or(0);
                    let cfsr = cortex.mem_read32(REG_CFSR).unwrap_or(0);
                    let hfsr = cortex.mem_read32(REG_HFSR).unwrap_or(0);
                    let bfar = cortex.mem_read32(REG_BFAR).unwrap_or(0);

                    // Read PC and LR
                    let pc = cortex.read_core_reg(15).ok();
                    let lr = cortex.read_core_reg(14).ok();

                    let status = if (dhcsr & (1 << 19)) != 0 {
                        "lockup"
                    } else if cfsr != 0 || (hfsr & 0xC0000002) != 0 {
                        "hardfault"
                    } else if (dhcsr & (1 << 17)) != 0 {
                        "halted"
                    } else {
                        "running"
                    };

                    let mut root_cause = heapless::String::new();
                    synthesize_root_cause(
                        dhcsr,
                        cfsr,
                        hfsr,
                        bfar,
                        pc.unwrap_or(0),
                        lr.unwrap_or(0),
                        &mut root_cause,
                    );

                    *pins_ref = swd_session.release();

                    return TargetDiagnosticReport {
                        rail_voltage_mv,
                        rail_status,
                        pins,
                        pin_integrity,
                        target_connected: true,
                        architecture: Some("ARM Cortex-M"),
                        dpidr: Some(dpidr),
                        cpuid: Some(cpuid),
                        dhcsr: Some(dhcsr),
                        cfsr: Some(cfsr),
                        hfsr: Some(hfsr),
                        bfar: Some(bfar),
                        pc,
                        lr,
                        status,
                        root_cause,
                    };
                }
            }
            *pins_ref = swd_session.release();
        }

        // Try RISC-V 2-wire RVSWD (Port 2)
        let mut rvswd_session = pins_ref.into_rvswd_port(TargetPort::Ch32v203);
        if let Ok(dmstatus) = rvswd_session.attach() {
            if dmstatus != 0 && dmstatus != 0xFFFFFFFF {
                let mut qingke = QingKe::new_2wire(&mut rvswd_session.transport);
                let mcause = qingke.read_reg(0x342).unwrap_or(0);
                let mepc = qingke.read_reg(0x341).unwrap_or(0);
                let mtval = qingke.read_reg(0x343).unwrap_or(0);
                let pc = qingke.read_reg(0x7B1).ok();

                let status = if (dmstatus & (1 << 9)) != 0 { "halted" } else { "running" };
                let mut root_cause = heapless::String::new();
                synthesize_riscv_root_cause(dmstatus, mcause, mepc, mtval, &mut root_cause);

                *pins_ref = rvswd_session.release();
                return TargetDiagnosticReport {
                    rail_voltage_mv,
                    rail_status,
                    pins,
                    pin_integrity,
                    target_connected: true,
                    architecture: Some("RISC-V (QingKe)"),
                    dpidr: None,
                    cpuid: None,
                    dhcsr: Some(dmstatus),
                    cfsr: Some(mcause),
                    hfsr: None,
                    bfar: Some(mtval),
                    pc,
                    lr: None,
                    status,
                    root_cause,
                };
            }
        }
        *pins_ref = rvswd_session.release();

        // Try RISC-V 1-wire SWIO (Port 3)
        let mut swio_session = pins_ref.into_swio_port(TargetPort::Ch32v003);
        if let Ok(dmstatus) = swio_session.attach() {
            if dmstatus != 0 && dmstatus != 0xFFFFFFFF {
                let mut qingke = QingKe::new_1wire(&mut swio_session.transport);
                let mcause = qingke.read_reg(0x342).unwrap_or(0);
                let mepc = qingke.read_reg(0x341).unwrap_or(0);
                let mtval = qingke.read_reg(0x343).unwrap_or(0);
                let pc = qingke.read_reg(0x7B1).ok();

                let status = if (dmstatus & (1 << 9)) != 0 { "halted" } else { "running" };
                let mut root_cause = heapless::String::new();
                synthesize_riscv_root_cause(dmstatus, mcause, mepc, mtval, &mut root_cause);

                *pins_ref = swio_session.release();
                return TargetDiagnosticReport {
                    rail_voltage_mv,
                    rail_status,
                    pins,
                    pin_integrity,
                    target_connected: true,
                    architecture: Some("RISC-V (QingKe)"),
                    dpidr: None,
                    cpuid: None,
                    dhcsr: Some(dmstatus),
                    cfsr: Some(mcause),
                    hfsr: None,
                    bfar: Some(mtval),
                    pc,
                    lr: None,
                    status,
                    root_cause,
                };
            }
        }
        *pins_ref = swio_session.release();

        let mut root_cause = heapless::String::new();
        let _ = write!(
            root_cause,
            "Target not responding over SWD/RVSWD (check clock, reset, or connect-under-reset)"
        );
        TargetDiagnosticReport {
            rail_voltage_mv,
            rail_status,
            pins,
            pin_integrity,
            target_connected: false,
            architecture: None,
            dpidr: None,
            cpuid: None,
            dhcsr: None,
            cfsr: None,
            hfsr: None,
            bfar: None,
            pc: None,
            lr: None,
            status: "no_target",
            root_cause,
        }
    })
}

// ============================================================================
// Tier 3: Vector Table & Layout Inspector (mcp_inspect_firmware)
// ============================================================================

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VectorTable {
    pub initial_sp: u32,
    pub reset_vector: u32,
    pub nmi_vector: u32,
    pub hardfault_vector: u32,
}

pub fn parse_vector_table(bytes: &[u8]) -> Option<VectorTable> {
    if bytes.len() < 16 {
        return None;
    }
    Some(VectorTable {
        initial_sp: u32::from_le_bytes(bytes[0..4].try_into().ok()?),
        reset_vector: u32::from_le_bytes(bytes[4..8].try_into().ok()?),
        nmi_vector: u32::from_le_bytes(bytes[8..12].try_into().ok()?),
        hardfault_vector: u32::from_le_bytes(bytes[12..16].try_into().ok()?),
    })
}

pub fn validate_stack_pointer(sp: u32, ram_base: u32, ram_size: u32) -> bool {
    // Stack pointer must be within [ram_base, ram_base + ram_size]
    // and word-aligned
    (sp & 3) == 0 && sp >= ram_base && sp <= (ram_base + ram_size)
}

pub fn validate_thumb_bit(reset_vector: u32) -> bool {
    // Cortex-M mandates bit 0 must be 1 (Thumb mode invariant)
    (reset_vector & 1) == 1
}

#[derive(Debug, Clone)]
pub struct FirmwareInspectReport {
    pub flash_address: u32,
    pub initial_sp: u32,
    pub reset_vector: u32,
    pub nmi_vector: u32,
    pub hardfault_vector: u32,
    pub sram_base: u32,
    pub sram_size: u32,
    pub sp_valid: bool,
    pub reset_vector_thumb_valid: bool,
    pub rdp_level: &'static str,
    pub valid: bool,
    pub warning: Option<heapless::String<192>>,
}

impl FirmwareInspectReport {
    pub fn to_json(&self) -> heapless::String<1024> {
        let mut out = heapless::String::new();
        let _ = write!(
            out,
            r#"{{"flash_address":"0x{:08x}","initial_sp":"0x{:08x}","reset_vector":"0x{:08x}","nmi_vector":"0x{:08x}","hardfault_vector":"0x{:08x}","sram_base":"0x{:08x}","sram_size":{},"sp_valid":{},"reset_vector_thumb_valid":{},"rdp_level":"{}","valid":{}"#,
            self.flash_address,
            self.initial_sp,
            self.reset_vector,
            self.nmi_vector,
            self.hardfault_vector,
            self.sram_base,
            self.sram_size,
            self.sp_valid,
            self.reset_vector_thumb_valid,
            self.rdp_level,
            self.valid
        );
        if let Some(w) = &self.warning {
            let _ = write!(out, r#","warning":"{}""#, w.as_str());
        } else {
            let _ = write!(out, r#","warning":null"#);
        }
        let _ = out.push('}');
        out
    }
}

pub fn run_inspect_firmware(address_opt: Option<u32>, target_opt: Option<&str>) -> (heapless::String<1024>, bool) {
    let flash_addr = address_opt.unwrap_or(0x08000000);
    let mut vec_buf = [0u8; 16];
    let res = crate::programmer::read_flash_global(flash_addr, &mut vec_buf);
    if res.is_err() {
        let mut err = heapless::String::new();
        let _ = write!(err, r#"{{"error":"Failed to read target flash at 0x{:08X}"}}"#, flash_addr);
        return (err, true);
    }

    let vt = match parse_vector_table(&vec_buf) {
        Some(v) => v,
        None => {
            let mut err = heapless::String::new();
            let _ = write!(err, r#"{{"error":"Failed to parse vector table at 0x{:08X}"}}"#, flash_addr);
            return (err, true);
        }
    };

    // Dynamic resolution of SRAM boundaries via LittleFS AlgoBlobHeader
    let mut ram_base = 0x20000000u32;
    let mut ram_size = 0x80000u32; // Default 512KB ARM universal fallback

    if let Some(target) = target_opt {
        let mut algo_buf = [0u8; 512];
        if let Ok((hdr, _)) = crate::programmer::algo::FlashAlgorithmRunner::load_algo_from_fs(target, &mut algo_buf) {
            ram_base = hdr.ram_base;
            ram_size = hdr.ram_size;
        } else if target == "stm32f1" {
            ram_base = crate::programmer::algo::STM32F1_ALGO_HDR.ram_base;
            ram_size = crate::programmer::algo::STM32F1_ALGO_HDR.ram_size;
        }
    }

    let sp_valid = validate_stack_pointer(vt.initial_sp, ram_base, ram_size);
    let reset_vector_thumb_valid = validate_thumb_bit(vt.reset_vector);

    // Option byte readout (e.g. at 0x1FFFF800)
    let mut opt_buf = [0u8; 4];
    let rdp_level = if crate::programmer::read_flash_global(0x1FFFF800, &mut opt_buf).is_ok() {
        if opt_buf[0] == 0xA5 || opt_buf[0] == 0xAA {
            "Level 0 (unprotected)"
        } else if opt_buf[0] == 0xCC {
            "Level 2 (locked)"
        } else {
            "Level 1 (readout protected)"
        }
    } else {
        "Level 0 (unprotected)"
    };

    let valid = sp_valid && reset_vector_thumb_valid;

    let mut warning = None;
    let mut warn_str = heapless::String::<192>::new();
    if !sp_valid {
        let _ = write!(
            warn_str,
            "Initial SP 0x{:08X} out of SRAM bounds [0x{:08X} - 0x{:08X}]",
            vt.initial_sp,
            ram_base,
            ram_base + ram_size
        );
    }
    if !reset_vector_thumb_valid {
        if !warn_str.is_empty() {
            let _ = warn_str.push_str("; ");
        }
        let _ = write!(
            warn_str,
            "Reset Vector 0x{:08X} missing Thumb bit (bit 0 must be 1)",
            vt.reset_vector
        );
    }
    if !warn_str.is_empty() {
        warning = Some(warn_str);
    }

    let report = FirmwareInspectReport {
        flash_address: flash_addr,
        initial_sp: vt.initial_sp,
        reset_vector: vt.reset_vector,
        nmi_vector: vt.nmi_vector,
        hardfault_vector: vt.hardfault_vector,
        sram_base: ram_base,
        sram_size: ram_size,
        sp_valid,
        reset_vector_thumb_valid,
        rdp_level,
        valid,
        warning,
    };

    (report.to_json(), false)
}

