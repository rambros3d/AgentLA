pub mod algo;
pub mod bootloader;
pub mod dap;
pub mod gdb_server;
pub mod pins;
pub mod pio_programs;
pub mod rtt;
pub mod rvswd;
pub mod swd;
pub mod swio;
pub mod swo;
pub mod target;

use algo::FlashAlgorithmRunner;
use core::cell::RefCell;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
pub use pins::TargetPort;
pub use target::rescue_nrf_global;
use pins::{ProgrammerPins, pulse_reset_pin};
use target::cortex_m::CortexM;
use target::qingke::QingKe;
use target::targets::TargetInfo;
pub use target::TargetError;
use rp_pac as pac;
use swd::{AP_CSW, AP_TAR, AP_DRW};

pub static PROGRAMMER_PINS: Mutex<CriticalSectionRawMutex, RefCell<ProgrammerPins>> =
    Mutex::new(RefCell::new(ProgrammerPins::new()));

pub fn detect_target_global() -> Result<TargetInfo, TargetError> {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        target::detect_target(&mut *pins)
    })
}

pub fn detect_target_port_global(port: TargetPort) -> Result<TargetInfo, TargetError> {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        target::detect_target_port(&mut *pins, port)
    })
}

pub fn debug_probe_bluepill_global() -> (u32, u8, u32, u8) {
    let raw_in = pac::SIO.gpio_in(0).read();
    let clk = (raw_in & (1 << pins::PIN_SWCLK)) != 0;
    let dio = (raw_in & (1 << pins::PIN_SWDIO)) != 0;
    let rst = (raw_in & (1 << pins::PIN_RESET)) != 0;
    let mut pin_levels = 0u8;
    if clk { pin_levels |= 1; }
    if dio { pin_levels |= 2; }
    if rst { pin_levels |= 4; }

    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        let mut swd_session = pins.into_swd_port(TargetPort::BluePill, 100);
        let dpidr_res = swd_session.attach();
        let req = swd::SwdTransport::build_request(false, true, 0);
        let mut val = 0u32;
        let ack = swd_session.transport.transfer_raw(req, &mut val);
        let final_dpidr = match dpidr_res {
            Ok(d) => d,
            Err(_) => val,
        };
        *pins = swd_session.release();
        (final_dpidr, ack, req as u32, pin_levels)
    })
}

pub fn probe_swd_diagnostics() -> (u32, u32, u32, u32, u32, u32) {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        let mut swd = pins.into_swd_port(TargetPort::BluePill, 100);

        // 1. Assert nRST LOW to keep core in reset
        swd.transport.assert_reset(true);
        cortex_m::asm::delay(300_000); // 2 ms

        // 2. Wake target using dormant/line-reset sequence and read DPIDR
        let mut dpidr = 0u32;
        let dpidr_ok = swd.transport.read_dpidr_retry(&mut dpidr).is_ok();

        // 3. Power up debug: write DP_ABORT = 0x1E, write DP_CTRLSTAT = 0x50000000
        let req_abort = swd::SwdTransport::build_request(false, false, 0); // reg 0
        let mut val_abort = 0x0000001Eu32;
        let ack_abort = swd.transport.transfer_raw(req_abort, &mut val_abort);

        let req_w_ctrl = swd::SwdTransport::build_request(false, false, 1); // reg 4
        let mut val_w_ctrl = 0x50000000u32;
        let ack_w_ctrl = swd.transport.transfer_raw(req_w_ctrl, &mut val_w_ctrl);

        let req_r_ctrl = swd::SwdTransport::build_request(false, true, 1); // reg 4
        let mut ctrlstat = 0u32;
        let ack_r_ctrl = swd.transport.transfer_raw(req_r_ctrl, &mut ctrlstat);

        let mut ctrlstat2 = 0u32;
        let ack_r_ctrl2 = swd.transport.transfer_raw(req_r_ctrl, &mut ctrlstat2);

        // Now write DP_SELECT = 0
        let req_sel = swd::SwdTransport::build_request(false, false, 2); // reg 8
        let mut val_sel = 0u32;
        let ack_sel = swd.transport.transfer_raw(req_sel, &mut val_sel);

        // Pack ACKs into a diagnostic word: [ack_sel:4, ack_abort:4, ack_w_ctrl:4, ack_r_ctrl:4, ack_r_ctrl2:4, dpidr_ok:4]
        let acks = ((ack_sel as u32 & 0xF) << 20)
            | ((ack_abort as u32 & 0xF) << 16)
            | ((ack_w_ctrl as u32 & 0xF) << 12)
            | ((ack_r_ctrl as u32 & 0xF) << 8)
            | ((ack_r_ctrl2 as u32 & 0xF) << 4)
            | (if dpidr_ok { 1 } else { 0 });

        // 4. Read AP 0 IDR
        let mut ap0_idr = 0u32;
        let mut ap0_dummy = 0u32;
        let mut sel_val0 = 0x000000F0u32;
        let ack_ap0_sel = swd.transport.transfer_raw(req_sel, &mut sel_val0);
        let ack_ap0_req = swd.transport.transfer_raw(swd::SwdTransport::build_request(true, true, 3), &mut ap0_dummy);
        let req_rdbuff = swd::SwdTransport::build_request(false, true, 3); // reg 12
        let ack_ap0_rd = swd.transport.transfer_raw(req_rdbuff, &mut ap0_idr);

        let ap0_diag = ((ack_ap0_sel as u32 & 0xF) << 24)
            | ((ack_ap0_req as u32 & 0xF) << 20)
            | ((ack_ap0_rd as u32 & 0xF) << 16)
            | (ap0_idr & 0xFFFF);

        // 6. Release nRST
        swd.transport.assert_reset(false);

        *pins = swd.release();
        (dpidr, acks, ctrlstat, ctrlstat2, ap0_diag, ap0_idr)
    })
}

pub fn debug_probe_ch32v203_global() -> (u32, u32) {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        let mut rvswd_session = pins.into_rvswd_port(TargetPort::Ch32v203);
        let dmstatus = rvswd_session.attach().unwrap_or(0xDEADBEEF);
        let partid = rvswd_session.read(0x7F).unwrap_or(0);
        *pins = rvswd_session.release();
        (dmstatus, partid)
    })
}

pub fn debug_probe_ch32v003_global() -> (u32, u32, u32, u32) {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        let mut swio_session = pins.into_swio_port(TargetPort::Ch32v003);
        let dmstatus_res = swio_session.attach();
        let dmstatus = dmstatus_res.unwrap_or(0);
        let partid = swio_session.get(0x7F).unwrap_or(0);
        let sm_pc = pac::PIO0.sm(0).addr().read();
        let fstat = pac::PIO0.fstat().read().0;
        let gpio_in = pac::SIO.gpio_in(0).read();
        *pins = swio_session.release();
        (dmstatus, partid, (sm_pc.0 << 16) | (fstat & 0xFFFF), gpio_in)
    })
}

pub fn halt_target_port_global(port: TargetPort) -> Result<(), TargetError> {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        match port {
            TargetPort::BluePill => {
                let mut swd_session = pins.into_swd_port(TargetPort::BluePill, 100);
                let dpidr = swd_session.attach().map_err(|_| TargetError::ProtocolError)?;
                let mut cortex = CortexM::new(&mut swd_session.transport);
                if dpidr == 0x6BA02477 || dpidr == 0x0BE12477 {
                    let _ = cortex.select_ap(1);
                    cortex.csw = 0x0B000012;
                }
                let res = cortex.halt().map_err(|_| TargetError::Timeout);
                *pins = swd_session.release();
                res
            }
            TargetPort::Ch32v203 => {
                let mut rvswd_session = pins.into_rvswd_port(TargetPort::Ch32v203);
                let _ = rvswd_session.attach();
                let mut qk = QingKe::new_2wire(&mut rvswd_session.transport);
                let res = qk.halt().map_err(|_| TargetError::Timeout);
                *pins = rvswd_session.release();
                res
            }
            TargetPort::Ch32v003 => {
                let mut swio_session = pins.into_swio_port(TargetPort::Ch32v003);
                let _ = swio_session.attach();
                let mut qk = QingKe::new_1wire(&mut swio_session.transport);
                let res = qk.halt().map_err(|_| TargetError::Timeout);
                *pins = swio_session.release();
                res
            }
        }
    })
}

pub fn resume_target_port_global(port: TargetPort) -> Result<(), TargetError> {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        match port {
            TargetPort::BluePill => {
                let mut swd_session = pins.into_swd_port(TargetPort::BluePill, 100);
                let dpidr = swd_session.attach().map_err(|_| TargetError::ProtocolError)?;
                let mut cortex = CortexM::new(&mut swd_session.transport);
                if dpidr == 0x6BA02477 || dpidr == 0x0BE12477 {
                    let _ = cortex.select_ap(1);
                    cortex.csw = 0x0B000012;
                }
                let _ = cortex.mem_write32(0xE000_EDFC, 0); // Clear DEMCR VC_CORERESET
                let _ = cortex.mem_write32(0xE000_EDF0, 0xA05F_0000); // Clear C_DEBUGEN and release halt
                let res = cortex.resume().map_err(|_| TargetError::Timeout);
                *pins = swd_session.release();
                res
            }
            TargetPort::Ch32v203 => {
                let mut rvswd_session = pins.into_rvswd_port(TargetPort::Ch32v203);
                let _ = rvswd_session.attach();
                let mut qk = QingKe::new_2wire(&mut rvswd_session.transport);
                let res = qk.resume().map_err(|_| TargetError::Timeout);
                *pins = rvswd_session.release();
                res
            }
            TargetPort::Ch32v003 => {
                let mut swio_session = pins.into_swio_port(TargetPort::Ch32v003);
                let _ = swio_session.attach();
                let mut qk = QingKe::new_1wire(&mut swio_session.transport);
                let res = qk.resume().map_err(|_| TargetError::Timeout);
                *pins = swio_session.release();
                res
            }
        }
    })
}

pub fn read_flash_port_internal(pins: &mut ProgrammerPins, port: TargetPort, addr: u32, buf: &mut [u8]) -> Result<(), TargetError> {
    match port {
        TargetPort::BluePill => {
            let mut swd_session = pins.into_swd_port(TargetPort::BluePill, 100);
            let is_h5 = match swd_session.attach() {
                Ok(id) if !CortexM::is_armv8m_dpidr(id) => false,
                _ => true,
            };
            let mut cortex = CortexM::new(&mut swd_session.transport);
            if is_h5 {
                let _ = cortex.connect_under_reset();
                let mut id = 0u32;
                let _ = cortex.swd.read_dpidr_retry(&mut id);
                let _ = cortex.swd.write_dp(crate::programmer::swd::DP_ABORT, 0x0000001E);
                let _ = cortex.select_ap(1);
                cortex.csw = 0x0B000012;
                let _ = cortex.swd.write_ap_port(1, crate::programmer::swd::AP_CSW, 0x0B000012);
            } else {
                let _ = cortex.halt();
            }
            let res = if is_h5 {
                let mut offset = 0usize;
                let mut ok = true;
                let mut first_err = 0u32;
                while offset < buf.len() {
                    let to_read = (buf.len() - offset).min(4);
                    match cortex.mem_read32(addr + offset as u32) {
                        Ok(w) => {
                            let bytes = w.to_le_bytes();
                            buf[offset..offset + to_read].copy_from_slice(&bytes[..to_read]);
                            offset += to_read;
                        }
                        Err(e) => {
                            ok = false;
                            first_err = match e {
                                target::cortex_m::CortexError::Diag(step, err, _) => {
                                    ((step as u32) << 8) | (err as u32)
                                }
                                target::cortex_m::CortexError::SwdError => 1,
                                target::cortex_m::CortexError::Timeout => 2,
                                target::cortex_m::CortexError::HaltFailed => 3,
                                target::cortex_m::CortexError::NotHalted => 4,
                                target::cortex_m::CortexError::ResetTimeout => 5,
                                _ => 99,
                            };
                            break;
                        }
                    }
                }
                if ok {
                    Ok(())
                } else {
                    let mut ctrlstat = 0u32;
                    let _ = cortex.swd.read_dp(crate::programmer::swd::DP_CTRLSTAT, &mut ctrlstat);
                    Err(TargetError::Diag(1, first_err, ctrlstat))
                }
            } else {
                cortex
                    .mem_read_block(addr, buf)
                    .map_err(|_| TargetError::ProtocolError)
            };
            *pins = swd_session.release();
            res
        }
        TargetPort::Ch32v203 => {
            let mut rvswd_session = pins.into_rvswd_port(TargetPort::Ch32v203);
            let _ = rvswd_session.attach();
            let mut qk = QingKe::new_2wire(&mut rvswd_session.transport);
            let _ = qk.halt();

            // minichlink fast PROGBUF read
            let _ = qk.dmi_write(0x18, 0); // DMABSTRACTAUTO = 0
            let _ = qk.dmi_write(0x20, 0x40044180); // DMPROGBUF0
            let _ = qk.dmi_write(0x21, 0xc1040411); // DMPROGBUF1
            let _ = qk.dmi_write(0x22, 0x9002c180); // DMPROGBUF2
            let _ = qk.dmi_write(0x04, 0xe00000f4); // DMDATA0 = &DATA0
            let _ = qk.dmi_write(0x17, 0x0023100a); // x10 = &DATA0
            let _ = qk.dmi_write(0x04, 0xe00000f8); // DMDATA0 = &DATA1
            let _ = qk.dmi_write(0x17, 0x0023100b); // x11 = &DATA1
            let _ = qk.dmi_write(0x18, 1); // DMABSTRACTAUTO = 1
            let _ = qk.dmi_write(0x05, addr); // DMDATA1 = addr

            let mut offset = 0usize;
            while offset + 4 <= buf.len() {
                cortex_m::asm::delay(1500);
                if let Ok(w) = qk.dmi_read(0x04) {
                    buf[offset..offset + 4].copy_from_slice(&w.to_le_bytes());
                }
                offset += 4;
            }

            let _ = qk.dmi_write(0x18, 0);
            let _ = qk.resume();
            *pins = rvswd_session.release();
            Ok(())
        }
        TargetPort::Ch32v003 => {
            let mut swio_session = pins.into_swio_port(TargetPort::Ch32v003);
            let _ = swio_session.attach();
            let mut qk = QingKe::new_1wire(&mut swio_session.transport);
            let _ = qk.halt();

            // minichlink fast PROGBUF read
            let _ = qk.dmi_write(0x18, 0); // DMABSTRACTAUTO = 0
            let _ = qk.dmi_write(0x20, 0x40044180); // DMPROGBUF0
            let _ = qk.dmi_write(0x21, 0xc1040411); // DMPROGBUF1
            let _ = qk.dmi_write(0x22, 0x9002c180); // DMPROGBUF2
            let _ = qk.dmi_write(0x04, 0xe00000f4); // DMDATA0 = &DATA0
            let _ = qk.dmi_write(0x17, 0x0023100a); // x10 = &DATA0
            let _ = qk.dmi_write(0x04, 0xe00000f8); // DMDATA0 = &DATA1
            let _ = qk.dmi_write(0x17, 0x0023100b); // x11 = &DATA1
            let _ = qk.dmi_write(0x18, 1); // DMABSTRACTAUTO = 1
            let _ = qk.dmi_write(0x05, addr); // DMDATA1 = addr

            let mut offset = 0usize;
            while offset + 4 <= buf.len() {
                cortex_m::asm::delay(1500);
                if let Ok(w) = qk.dmi_read(0x04) {
                    buf[offset..offset + 4].copy_from_slice(&w.to_le_bytes());
                }
                offset += 4;
            }

            let _ = qk.dmi_write(0x18, 0);
            let _ = qk.resume();
            *pins = swio_session.release();
            Ok(())
        }
    }
}

pub fn read_flash_port_global(port: TargetPort, addr: u32, buf: &mut [u8]) -> Result<(), TargetError> {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        read_flash_port_internal(&mut *pins, port, addr, buf)
    })
}

pub fn write_mem_port_internal(pins: &mut ProgrammerPins, port: TargetPort, addr: u32, data: &[u8]) -> Result<(), TargetError> {
    match port {
        TargetPort::BluePill => {
            let mut swd_session = pins.into_swd_port(TargetPort::BluePill, 100);
            let (_dpidr, is_h5) = match swd_session.attach() {
                Ok(id) if !CortexM::is_armv8m_dpidr(id) => (id, false),
                Ok(id) => (id, true),
                Err(_) => {
                    let mut cortex = CortexM::new(&mut swd_session.transport);
                    let _ = cortex.connect_under_reset();
                    let mut id = 0u32;
                    let _ = cortex.swd.read_dpidr_retry(&mut id);
                    (id, CortexM::is_armv8m_dpidr(id))
                }
            };
            let mut cortex = CortexM::new(&mut swd_session.transport);
            if is_h5 {
                let _ = cortex.select_ap(1);
                cortex.csw = 0x0B000012;
            }
            let res = if is_h5 {
                let mut offset = 0usize;
                let mut ok = true;
                while offset < data.len() {
                    let to_write = (data.len() - offset).min(4);
                    let mut chunk = [0u8; 4];
                    chunk[..to_write].copy_from_slice(&data[offset..offset + to_write]);
                    let val = u32::from_le_bytes(chunk);
                    match cortex.mem_write32(addr + offset as u32, val) {
                        Ok(()) => {
                            offset += to_write;
                        }
                        Err(_) => {
                            ok = false;
                            break;
                        }
                    }
                }
                if ok { Ok(()) } else { Err(TargetError::ProtocolError) }
            } else {
                cortex
                    .mem_write_block(addr, data)
                    .map_err(|_| TargetError::ProtocolError)
            };
            *pins = swd_session.release();
            res
        }
        _ => Err(TargetError::ProtocolError),
    }
}

pub fn write_mem_port_global(port: TargetPort, addr: u32, data: &[u8]) -> Result<(), TargetError> {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        write_mem_port_internal(&mut *pins, port, addr, data)
    })
}

pub fn erase_flash_port_global(port: TargetPort, addr: u32, len: u32) -> Result<(), TargetError> {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        match port {
            TargetPort::BluePill => {
                let mut swd_session = pins.into_swd_port(TargetPort::BluePill, 100);
                let is_h5 = match swd_session.attach() {
                    Ok(id) if !CortexM::is_armv8m_dpidr(id) => false,
                    _ => true,
                };
                let mut cortex = CortexM::new(&mut swd_session.transport);
                if is_h5 {
                    let _ = cortex.connect_under_reset().map_err(|_| TargetError::FlashErrorCode(0x1001))?;
                    let mut id = 0u32;
                    let _ = cortex.swd.read_dpidr_retry(&mut id);
                    let _ = cortex.swd.write_dp(crate::programmer::swd::DP_ABORT, 0x0000001E);
                    let _ = cortex.select_ap(1);
                    cortex.csw = 0x0B000012;
                    let _ = cortex.swd.write_ap_port(1, crate::programmer::swd::AP_CSW, 0x0B000012);
                } else {
                    let _ = cortex.halt();
                }

                if is_h5 {
                    let sector_size = 8192u32;
                    let start_addr = addr & !(sector_size - 1);
                    let end_addr = addr + len;
                    let mut curr = start_addr;
                    while curr < end_addr {
                        FlashAlgorithmRunner::stm32h5_erase_sector(&mut cortex, curr)
                            .map_err(|e| match e {
                                algo::AlgoError::ExecutionFailed(code) => TargetError::FlashErrorCode(code),
                                algo::AlgoError::Timeout => TargetError::FlashErrorCode(0x30EE),
                                algo::AlgoError::SwdError => TargetError::FlashErrorCode(0x30ED),
                                _ => TargetError::FlashError,
                            })?;
                        curr += sector_size;
                    }
                    let _ = FlashAlgorithmRunner::stm32h5_lock(&mut cortex);
                } else {
                    let sector_size = 1024u32;
                    let start_addr = addr & !(sector_size - 1);
                    let end_addr = addr + len;
                    let mut curr = start_addr;
                    while curr < end_addr {
                        FlashAlgorithmRunner::stm32f1_erase_sector(&mut cortex, curr)
                            .map_err(|e| match e {
                                algo::AlgoError::ExecutionFailed(code) => TargetError::FlashErrorCode(code),
                                _ => TargetError::FlashError,
                            })?;
                        curr += sector_size;
                    }
                    // Lock flash before releasing
                    let _ = cortex.swd.write_ap(crate::programmer::swd::AP_CSW, crate::programmer::swd::ADIV5_AP_CSW_DEFAULT);
                    let _ = cortex.swd.write_ap(crate::programmer::swd::AP_TAR, 0x40022010);
                    let _ = cortex.swd.write_ap(crate::programmer::swd::AP_DRW, 0x80); // LOCK bit
                }
                if is_h5 {
                    let _ = cortex.sys_reset();
                } else {
                    let _ = cortex.resume();
                }
                *pins = swd_session.release();
                Ok(())
            }
            TargetPort::Ch32v203 => {
                let mut rvswd_session = pins.into_rvswd_port(TargetPort::Ch32v203);
                let _ = rvswd_session.attach();
                let mut qk = QingKe::new_2wire(&mut rvswd_session.transport);
                qk.halt().map_err(|_| TargetError::Timeout)?;
                // Unlock
                qk.mem_write32_halted(0x40022004, 0x45670123).map_err(|_| TargetError::FlashErrorCode(101))?;
                qk.mem_write32_halted(0x40022004, 0xCDEF89AB).map_err(|_| TargetError::FlashErrorCode(102))?;
                let _ = qk.mem_write32_halted(0x40022024, 0x45670123);
                let _ = qk.mem_write32_halted(0x40022024, 0xCDEF89AB);

                let page_sz = 256u32;
                let start_addr = addr & !(page_sz - 1);
                let end_addr = addr + len;
                let mut curr = start_addr;
                while curr < end_addr {
                    qk.mem_write32_halted(0x40022010, 1 << 17).map_err(|_| TargetError::FlashErrorCode(105))?; // FTER
                    qk.mem_write32_halted(0x40022014, curr).map_err(|_| TargetError::FlashErrorCode(106))?; // ADDR
                    qk.mem_write32_halted(0x40022010, (1 << 17) | (1 << 6)).map_err(|_| TargetError::FlashErrorCode(107))?; // FTER | STRT

                    cortex_m::asm::delay(75_000); // 500 µs initial delay (matching ch32.c)
                    let mut busy = true;
                    for _ in 0..1000 {
                        if let Ok(statr) = qk.mem_read32_halted(0x4002200C) {
                            if (statr & 1) == 0 {
                                if (statr & 0x14) != 0 {
                                    let _ = qk.mem_write32_halted(0x4002200C, 0x14); // W1C
                                    *pins = rvswd_session.release();
                                    return Err(TargetError::FlashErrorCode(statr));
                                }
                                busy = false;
                                break;
                            }
                        }
                        cortex_m::asm::delay(7500);
                    }
                    if busy {
                        *pins = rvswd_session.release();
                        return Err(TargetError::Timeout);
                    }
                    curr += page_sz;
                }

                let _ = qk.mem_write32_halted(0x40022010, 0);
                let _ = qk.mem_write32_halted(0x40022010, 1 << 15); // FLOCK
                let _ = qk.resume();
                *pins = rvswd_session.release();
                Ok(())
            }
            TargetPort::Ch32v003 => {
                let mut swio_session = pins.into_swio_port(TargetPort::Ch32v003);
                let _ = swio_session.attach();
                let mut qk = QingKe::new_1wire(&mut swio_session.transport);
                qk.halt().map_err(|_| TargetError::Timeout)?;
                // Unlock KEYR and MODEKEYR
                qk.mem_write32_halted(0x40022004, 0x45670123).map_err(|_| TargetError::FlashErrorCode(301))?;
                qk.mem_write32_halted(0x40022004, 0xCDEF89AB).map_err(|_| TargetError::FlashErrorCode(302))?;
                let _ = qk.mem_write32_halted(0x40022024, 0x45670123);
                let _ = qk.mem_write32_halted(0x40022024, 0xCDEF89AB);

                let page_sz = 64u32;
                let start_addr = addr & !(page_sz - 1);
                let end_addr = addr + len;
                let mut curr = start_addr;
                while curr < end_addr {
                    qk.mem_write32_halted(0x40022014, curr).map_err(|_| TargetError::FlashErrorCode(305))?; // ADDR
                    qk.mem_write32_halted(0x40022010, (1 << 1) | (1 << 6)).map_err(|_| TargetError::FlashErrorCode(306))?; // PER | STRT

                    cortex_m::asm::delay(450_000); // 3 ms initial delay
                    for _ in 0..100 {
                        if let Ok(statr) = qk.mem_read32_halted(0x4002200C) {
                            if (statr & 1) == 0 {
                                break;
                            }
                        }
                        cortex_m::asm::delay(15_000);
                    }
                    let _ = qk.mem_write32_halted(0x40022010, 0);
                    curr += page_sz;
                }

                let _ = qk.mem_write32_halted(0x40022010, 0);
                let _ = qk.mem_write32_halted(0x40022010, 1 << 7); // LOCK
                let _ = qk.resume();
                *pins = swio_session.release();
                Ok(())
            }
        }
    })
}

pub fn flash_target_port_global(port: TargetPort, addr: u32, data: &[u8]) -> Result<(), TargetError> {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        match port {
            TargetPort::BluePill => {
                let mut swd_session = pins.into_swd_port(TargetPort::BluePill, 100);
                let is_h5 = match swd_session.attach() {
                    Ok(id) if !CortexM::is_armv8m_dpidr(id) => false,
                    _ => true,
                };
                let mut cortex = CortexM::new(&mut swd_session.transport);
                if is_h5 {
                    let _ = cortex.connect_under_reset().map_err(|_| TargetError::FlashErrorCode(0x1001))?;
                    let mut id = 0u32;
                    let _ = cortex.swd.read_dpidr_retry(&mut id);
                    let _ = cortex.swd.write_dp(crate::programmer::swd::DP_ABORT, 0x0000001E);
                    let _ = cortex.select_ap(1);
                    cortex.csw = 0x0B000012;
                    let _ = cortex.swd.write_ap_port(1, crate::programmer::swd::AP_CSW, 0x0B000012);
                } else {
                    let _ = cortex.halt();
                }

                if is_h5 {
                    // STM32H5 Flash: 8 KB sectors, 128-bit (16-byte) quad-word programming
                    let sector_size = 8192u32;
                    let mut curr_sector = addr & !(sector_size - 1);
                    let end_addr = addr + data.len() as u32;
                    while curr_sector < end_addr {
                        FlashAlgorithmRunner::stm32h5_erase_sector(&mut cortex, curr_sector)
                            .map_err(|e| match e {
                                algo::AlgoError::ExecutionFailed(code) => TargetError::FlashErrorCode(code),
                                algo::AlgoError::Timeout => TargetError::FlashErrorCode(0x30EE),
                                algo::AlgoError::SwdError => TargetError::FlashErrorCode(0x30ED),
                                _ => TargetError::FlashErrorCode(0x30FF),
                            })?;
                        curr_sector += sector_size;
                    }

                    FlashAlgorithmRunner::stm32h5_program_page(&mut cortex, addr, data)
                        .map_err(|e| match e {
                            algo::AlgoError::ExecutionFailed(code) => TargetError::FlashErrorCode(code),
                            algo::AlgoError::Timeout => TargetError::FlashErrorCode(0x40EE),
                            algo::AlgoError::SwdError => TargetError::FlashErrorCode(0x40ED),
                            _ => TargetError::FlashErrorCode(0x40FF),
                        })?;
                    let _ = FlashAlgorithmRunner::stm32h5_lock(&mut cortex);
                } else {
                    // 1. Erase sector(s) covering destination
                    let sector_size = 1024u32;
                    let mut curr_sector = addr & !(sector_size - 1);
                    let end_addr = addr + data.len() as u32;
                    while curr_sector < end_addr {
                        FlashAlgorithmRunner::stm32f1_erase_sector(&mut cortex, curr_sector)
                            .map_err(|e| match e {
                                algo::AlgoError::ExecutionFailed(code) => TargetError::FlashErrorCode(0x3000 | (code & 0xFF)),
                                _ => TargetError::FlashErrorCode(0x30FF),
                            })?;
                        curr_sector += sector_size;
                    }

                    // 2. Program pages
                    let page_size = 1024usize;
                    let mut offset = 0usize;
                    while offset < data.len() {
                        let chunk_len = (data.len() - offset).min(page_size);
                        FlashAlgorithmRunner::stm32f1_program_page(&mut cortex, addr + offset as u32, &data[offset..offset + chunk_len])
                            .map_err(|e| match e {
                                algo::AlgoError::ExecutionFailed(code) => TargetError::FlashErrorCode(0x4000 | (code & 0xFF)),
                                _ => TargetError::FlashErrorCode(0x40FF),
                            })?;
                        offset += chunk_len;
                    }
                    // Lock flash before verify and resume
                    let _ = cortex.swd.write_ap(crate::programmer::swd::AP_CSW, crate::programmer::swd::ADIV5_AP_CSW_DEFAULT);
                    let _ = cortex.swd.write_ap(crate::programmer::swd::AP_TAR, 0x40022010);
                    let _ = cortex.swd.write_ap(crate::programmer::swd::AP_DRW, 0x80); // LOCK bit
                }

                if is_h5 {
                    let _ = cortex.halt();
                    let _ = cortex.swd.write_dp(crate::programmer::swd::DP_ABORT, 0x0000001E);
                    let _ = cortex.select_ap(1);
                    cortex.csw = 0x0B000012;
                    let _ = cortex.swd.write_ap_port(1, crate::programmer::swd::AP_CSW, 0x0B000012);
                }

                // Verify
                let mut v_offset = 0usize;
                let mut verify_buf = [0u8; 256];
                while v_offset < data.len() {
                    let chunk_len = (data.len() - v_offset).min(verify_buf.len());
                    if is_h5 {
                        for i in (0..chunk_len).step_by(4) {
                            let w = cortex
                                .mem_read32(addr + (v_offset + i) as u32)
                                .map_err(|_| TargetError::FlashErrorCode(0x6001))?;
                            let bytes = w.to_le_bytes();
                            let copy_len = (chunk_len - i).min(4);
                            verify_buf[i..i + copy_len].copy_from_slice(&bytes[..copy_len]);
                        }
                    } else {
                        cortex.mem_read_block(addr + v_offset as u32, &mut verify_buf[..chunk_len])
                            .map_err(|_| TargetError::FlashErrorCode(0x6001))?;
                    }
                    for i in 0..chunk_len {
                        if verify_buf[i] != data[v_offset + i] {
                            let _ = cortex.resume();
                            *pins = swd_session.release();
                            return Err(TargetError::FlashErrorCode(
                                ((verify_buf[i] as u32) << 24)
                                    | ((data[v_offset + i] as u32) << 16)
                                    | ((v_offset + i) as u32 & 0xFFFF),
                            ));
                        }
                    }
                    v_offset += chunk_len;
                }
                if is_h5 {
                    let _ = cortex.sys_reset();
                } else {
                    let _ = cortex.resume();
                }
                *pins = swd_session.release();
                Ok(())
            }
            TargetPort::Ch32v203 => {
                let mut rvswd_session = pins.into_rvswd_port(TargetPort::Ch32v203);
                let _ = rvswd_session.attach();
                let mut qk = QingKe::new_2wire(&mut rvswd_session.transport);
                qk.halt().map_err(|_| TargetError::Timeout)?;

                // Unlock flash
                qk.mem_write32_halted(0x40022004, 0x45670123).map_err(|_| TargetError::FlashErrorCode(201))?;
                qk.mem_write32_halted(0x40022004, 0xCDEF89AB).map_err(|_| TargetError::FlashErrorCode(202))?;
                let _ = qk.mem_write32_halted(0x40022024, 0x45670123);
                let _ = qk.mem_write32_halted(0x40022024, 0xCDEF89AB);

                // 1. Fast Page Erase
                let page_sz = 256usize;
                let start_addr = addr & !(page_sz as u32 - 1);
                let end_addr = (addr + data.len() as u32 + page_sz as u32 - 1) & !(page_sz as u32 - 1);
                let mut curr = start_addr;
                while curr < end_addr {
                    qk.mem_write32_halted(0x40022010, 1 << 17).map_err(|_| TargetError::FlashErrorCode(205))?; // FTER
                    qk.mem_write32_halted(0x40022014, curr).map_err(|_| TargetError::FlashErrorCode(206))?; // ADDR
                    qk.mem_write32_halted(0x40022010, (1 << 17) | (1 << 6)).map_err(|_| TargetError::FlashErrorCode(207))?; // FTER | STRT

                    cortex_m::asm::delay(75_000); // 500 µs
                    let mut busy = true;
                    for _ in 0..1000 {
                        if let Ok(statr) = qk.mem_read32_halted(0x4002200C) {
                            if (statr & 1) == 0 {
                                if (statr & 0x14) != 0 {
                                    let _ = qk.mem_write32_halted(0x4002200C, 0x14);
                                    *pins = rvswd_session.release();
                                    return Err(TargetError::FlashErrorCode(statr));
                                }
                                busy = false;
                                break;
                            }
                        }
                        cortex_m::asm::delay(7500);
                    }
                    if busy {
                        *pins = rvswd_session.release();
                        return Err(TargetError::Timeout);
                    }
                    curr += page_sz as u32;
                }

                // 2. Program pages via hardware fast page controller (matches ch32.c lines 600-635)
                let mut offset = 0usize;
                while offset < data.len() {
                    let page_addr = addr + offset as u32;
                    let chunk_len = (data.len() - offset).min(page_sz);

                    // Reset buffer & arm FTPG (FTPG | BUFRST), set page address
                    qk.mem_write32_halted(0x40022010, (1 << 16) | (1 << 19))
                        .map_err(|_| TargetError::FlashErrorCode(209))?;
                    qk.mem_write32_halted(0x40022014, page_addr)
                        .map_err(|_| TargetError::FlashErrorCode(210))?;

                    for b in (0..page_sz).step_by(4) {
                        let mut word = 0xFFFFFFFFu32;
                        if b + 4 <= chunk_len {
                            word = u32::from_le_bytes([
                                data[offset + b],
                                data[offset + b + 1],
                                data[offset + b + 2],
                                data[offset + b + 3],
                            ]);
                        } else if b < chunk_len {
                            let mut bytes = [0xFFu8; 4];
                            let rem = chunk_len - b;
                            bytes[..rem].copy_from_slice(&data[offset + b..offset + b + rem]);
                            word = u32::from_le_bytes(bytes);
                        }
                        qk.mem_write32_halted(page_addr + b as u32, word)
                            .map_err(|e| match e {
                                target::qingke::QingKeError::CommandErrorCode(c) => TargetError::FlashErrorCode(c),
                                _ => TargetError::FlashErrorCode(211),
                            })?;
                    }

                    // Trigger fast page write (FTPG | STRT)
                    qk.mem_write32_halted(0x40022010, (1 << 16) | (1 << 6))
                        .map_err(|_| TargetError::FlashErrorCode(212))?;

                    cortex_m::asm::delay(450_000); // 3 ms
                    for _ in 0..100 {
                        if let Ok(statr) = qk.mem_read32_halted(0x4002200C) {
                            if (statr & 1) == 0 {
                                if (statr & 0x14) != 0 {
                                    let _ = qk.mem_write32_halted(0x4002200C, 0x14);
                                }
                                break;
                            }
                        }
                        cortex_m::asm::delay(15_000);
                    }
                    offset += chunk_len;
                }

                // Clear programming mode and lock flash to re-enable read mode
                let _ = qk.mem_write32_halted(0x40022010, 0);
                let _ = qk.mem_write32_halted(0x40022010, 1 << 15); // FLOCK
                let _ = qk.mem_write32_halted(0x4002200C, 1 << 5); // Clear EOP
                let _ = qk.resume();
                *pins = rvswd_session.release();

                cortex_m::asm::delay(1_500_000); // 10 ms settling

                // Verify using clean read session
                let mut readback = [0u8; 64];
                let mut v_offset = 0usize;
                while v_offset < data.len() {
                    let chunk = (data.len() - v_offset).min(readback.len());
                    read_flash_port_internal(&mut *pins, TargetPort::Ch32v203, addr + v_offset as u32, &mut readback[..chunk])?;
                    for i in 0..chunk {
                        if readback[i] != data[v_offset + i] {
                            return Err(TargetError::FlashErrorCode(((readback[i] as u32) << 8) | (data[v_offset + i] as u32)));
                        }
                    }
                    v_offset += chunk;
                }
                Ok(())
            }
            TargetPort::Ch32v003 => {
                let mut swio_session = pins.into_swio_port(TargetPort::Ch32v003);
                let _ = swio_session.attach();
                let mut qk = QingKe::new_1wire(&mut swio_session.transport);
                qk.halt().map_err(|_| TargetError::Timeout)?;
                qk.mem_write32_halted(0x40022004, 0x45670123).map_err(|e| match e {
                    target::qingke::QingKeError::CommandErrorCode(c) => TargetError::FlashErrorCode(c),
                    _ => TargetError::FlashErrorCode(301),
                })?;
                qk.mem_write32_halted(0x40022004, 0xCDEF89AB).map_err(|e| match e {
                    target::qingke::QingKeError::CommandErrorCode(c) => TargetError::FlashErrorCode(c),
                    _ => TargetError::FlashErrorCode(302),
                })?;
                let _ = qk.mem_write32_halted(0x40022024, 0x45670123);
                let _ = qk.mem_write32_halted(0x40022024, 0xCDEF89AB);

                // 1. Page Erase (PER: 1 << 1, STRT: 1 << 6)
                let page_sz = 64usize;
                let start_addr = addr & !(page_sz as u32 - 1);
                let end_addr = (addr + data.len() as u32 + page_sz as u32 - 1) & !(page_sz as u32 - 1);
                let mut curr = start_addr;
                while curr < end_addr {
                    qk.mem_write32_halted(0x40022014, curr).map_err(|_| TargetError::FlashErrorCode(305))?; // ADDR
                    qk.mem_write32_halted(0x40022010, (1 << 1) | (1 << 6)).map_err(|_| TargetError::FlashErrorCode(306))?; // PER | STRT

                    cortex_m::asm::delay(450_000); // 3 ms
                    for _ in 0..100 {
                        if let Ok(statr) = qk.mem_read32_halted(0x4002200C) {
                            if (statr & 1) == 0 {
                                break;
                            }
                        }
                        cortex_m::asm::delay(15_000);
                    }
                    let _ = qk.mem_write32_halted(0x40022010, 0);
                    curr += page_sz as u32;
                }

                // 2. Fast Page Program (matches ch32_progbuf.c lines 315-375)
                let mut offset = 0usize;
                while offset < data.len() {
                    let page_addr = addr + offset as u32;
                    let chunk_len = (data.len() - offset).min(page_sz);

                    // FTPG & BUFRST
                    qk.mem_write32_halted(0x40022010, 1 << 16).map_err(|_| TargetError::FlashErrorCode(308))?;
                    qk.mem_write32_halted(0x40022010, (1 << 16) | (1 << 19)).map_err(|_| TargetError::FlashErrorCode(309))?;
                    qk.mem_write32_halted(0x40022014, page_addr).map_err(|_| TargetError::FlashErrorCode(310))?;

                    for _ in 0..100 {
                        if let Ok(statr) = qk.mem_read32_halted(0x4002200C) {
                            if (statr & 1) == 0 {
                                break;
                            }
                        }
                        cortex_m::asm::delay(15_000);
                    }

                    for b in (0..page_sz).step_by(4) {
                        let mut word = 0xFFFFFFFFu32;
                        if b + 4 <= chunk_len {
                            word = u32::from_le_bytes([
                                data[offset + b],
                                data[offset + b + 1],
                                data[offset + b + 2],
                                data[offset + b + 3],
                            ]);
                        } else if b < chunk_len {
                            let mut bytes = [0xFFu8; 4];
                            let rem = chunk_len - b;
                            bytes[..rem].copy_from_slice(&data[offset + b..offset + b + rem]);
                            word = u32::from_le_bytes(bytes);
                        }
                        qk.mem_write32_halted(page_addr + b as u32, word).map_err(|e| match e {
                            target::qingke::QingKeError::CommandErrorCode(c) => TargetError::FlashErrorCode(c),
                            _ => TargetError::FlashErrorCode(312),
                        })?;
                        // Latch word: FTPG | BUFLOAD (bit 18)
                        qk.mem_write32_halted(0x40022010, (1 << 16) | (1 << 18)).map_err(|_| TargetError::FlashErrorCode(313))?;
                        for _ in 0..50 {
                            if let Ok(statr) = qk.mem_read32_halted(0x4002200C) {
                                if (statr & 1) == 0 {
                                    break;
                                }
                            }
                            cortex_m::asm::delay(7500);
                        }
                    }

                    // Trigger Fast Page Program: FTPG | STRT (1 << 16 | 1 << 6)
                    qk.mem_write32_halted(0x40022014, page_addr).map_err(|_| TargetError::FlashErrorCode(314))?;
                    qk.mem_write32_halted(0x40022010, (1 << 16) | (1 << 6)).map_err(|_| TargetError::FlashErrorCode(315))?;

                    cortex_m::asm::delay(450_000); // 3 ms
                    for _ in 0..200 {
                        if let Ok(statr) = qk.mem_read32_halted(0x4002200C) {
                            if (statr & 1) == 0 {
                                if (statr & 0x14) != 0 {
                                    let _ = qk.mem_write32_halted(0x4002200C, 0x14);
                                }
                                break;
                            }
                        }
                        cortex_m::asm::delay(15_000);
                    }

                    offset += chunk_len;
                }

                let _ = qk.mem_write32_halted(0x40022010, 0);
                let _ = qk.mem_write32_halted(0x40022010, 1 << 7); // LOCK
                let _ = qk.mem_write32_halted(0x4002200C, 1 << 5); // Clear EOP
                cortex_m::asm::delay(300_000); // 2 ms settling

                // Verify
                let mut verify_buf = [0u8; 64];
                let mut v_offset = 0usize;
                while v_offset < data.len() {
                    let chunk = (data.len() - v_offset).min(verify_buf.len());
                    qk.mem_read_block(addr + v_offset as u32, &mut verify_buf[..chunk])
                        .map_err(|e| match e {
                            target::qingke::QingKeError::CommandErrorCode(c) => TargetError::FlashErrorCode(c),
                            target::qingke::QingKeError::Timeout => TargetError::Timeout,
                            _ => TargetError::FlashErrorCode(318),
                        })?;
                    for i in 0..chunk {
                        if verify_buf[i] != data[v_offset + i] {
                            let _ = qk.resume();
                            *pins = swio_session.release();
                            return Err(TargetError::FlashErrorCode(((verify_buf[i] as u32) << 8) | (data[v_offset + i] as u32)));
                        }
                    }
                    v_offset += chunk;
                }

                let _ = qk.resume();
                *pins = swio_session.release();
                Ok(())
            }
        }
    })
}

pub fn halt_target_global() -> Result<(), TargetError> {
    halt_target_port_global(TargetPort::BluePill)
}

pub fn connect_under_reset_port_global(port: TargetPort) -> Result<u32, TargetError> {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        let mut swd_session = pins.into_swd_port(port, 100);
        let res = match swd_session.connect_under_reset() {
            Ok(pc) => Ok(pc),
            Err(target::cortex_m::CortexError::ResetTimeout) => Err(TargetError::Timeout),
            Err(target::cortex_m::CortexError::Diag(step, v1, v2)) => Err(TargetError::Diag(step, v1, v2)),
            Err(_) => Err(TargetError::ProtocolError),
        };
        *pins = swd_session.release();
        res
    })
}

pub fn connect_under_reset_global() -> Result<u32, TargetError> {
    connect_under_reset_port_global(TargetPort::BluePill)
}

pub fn reset_and_catch_port_global(port: TargetPort) -> Result<u32, TargetError> {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        let mut swd_session = pins.into_swd_port(port, 100);
        let _ = swd_session.attach().map_err(|_| TargetError::ProtocolError)?;
        let res = match swd_session.reset_and_catch() {
            Ok(pc) => Ok(pc),
            Err(target::cortex_m::CortexError::ResetTimeout) => Err(TargetError::Timeout),
            Err(_) => Err(TargetError::ProtocolError),
        };
        *pins = swd_session.release();
        res
    })
}

pub fn reset_and_catch_global() -> Result<u32, TargetError> {
    reset_and_catch_port_global(TargetPort::BluePill)
}

pub fn resume_target_global() -> Result<(), TargetError> {
    resume_target_port_global(TargetPort::BluePill)
}

pub fn read_flash_global(addr: u32, buf: &mut [u8]) -> Result<(), TargetError> {
    read_flash_port_global(TargetPort::BluePill, addr, buf)
}

pub fn flash_target_global(addr: u32, data: &[u8]) -> Result<(), TargetError> {
    flash_target_port_global(TargetPort::BluePill, addr, data)
}

pub fn rescue_stm32_global() -> Result<(), bootloader::BootloaderError> {
    // 1. Conflict protection check:
    if matches!(crate::analyzer::state(), crate::analyzer::State::Armed | crate::analyzer::State::Capturing) {
        return Err(bootloader::BootloaderError::PinConflict);
    }
    for pin in [pins::PIN_BOOT0 as u32, pins::PIN_UART_TX as u32, pins::PIN_UART_RX as u32, pins::PIN_RESET as u32] {
        if let Some(owner) = crate::pin_owner::owner_of(pin) {
            if owner != crate::pin_owner::Driver::Programmer {
                return Err(bootloader::BootloaderError::PinConflict);
            }
        }
    }

    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();

        // 2. Hardware Orchestration:
        // Set BOOT0 = HIGH
        pins.set_boot0(true);

        // Pulse RESET low for 20ms, release high
        pins::drive_reset_pin(pins::PIN_RESET, true);
        cortex_m::asm::delay(3_000_000); // 20 ms
        pins::drive_reset_pin(pins::PIN_RESET, false);
        cortex_m::asm::delay(7_500_000); // 50 ms settling for bootloader

        // 3. Initialize UART 8E1 and execute AN3155 bootloader commands
        let mut bl = bootloader::Stm32Bootloader::default_programmer();
        bl.init_pins();

        let result = (|| {
            bl.sync()?;
            bl.readout_unprotect()?;
            Ok(())
        })();

        // 4. Teardown & cleanup:
        // Release BOOT0 = LOW
        pins.set_boot0(false);
        // Pulse reset to exit bootloader mode
        pins::drive_reset_pin(pins::PIN_RESET, true);
        cortex_m::asm::delay(3_000_000); // 20 ms
        pins::drive_reset_pin(pins::PIN_RESET, false);
        pins::tristate_pins();

        result
    })
}
