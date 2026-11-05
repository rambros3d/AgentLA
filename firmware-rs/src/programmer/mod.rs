pub mod algo;
pub mod dap;
pub mod pins;
pub mod pio_programs;
pub mod rvswd;
pub mod swd;
pub mod swio;
pub mod target;

use algo::FlashAlgorithmRunner;
use core::cell::RefCell;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::blocking_mutex::Mutex;
pub use pins::TargetPort;
use pins::ProgrammerPins;
use target::cortex_m::CortexM;
use target::qingke::QingKe;
use target::targets::TargetInfo;
use target::TargetError;

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

pub fn debug_probe_bluepill_global() -> (u32, u8, u32) {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        let mut swd_session = pins.into_swd_port(TargetPort::BluePill, 1000);
        swd_session.jtag_to_swd();
        let mut dpidr = 0u32;
        let res = swd_session.read_dpidr_retry(&mut dpidr);
        let req = swd::SwdTransport::build_request(false, true, 0);
        let ack = if res.is_ok() { swd::SWD_ACK_OK } else { 7 };
        *pins = swd_session.release();
        (dpidr, ack, req as u32)
    })
}

pub fn debug_probe_ch32v203_global() -> (u32, u32) {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        let rvswd_session = pins.into_rvswd_port(TargetPort::Ch32v203);
        let dmstatus = rvswd_session.read(0x11).unwrap_or(0xDEADBEEF);
        *pins = rvswd_session.release();
        (dmstatus, 0)
    })
}

pub fn debug_probe_ch32v003_global() -> (u32, u32) {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        let swio_session = pins.into_swio_port(TargetPort::Ch32v003);
        let dmstatus = swio_session.get(0x11).unwrap_or(0xDEADBEEF);
        let partid = swio_session.get(0x7F).unwrap_or(0xDEADBEEF);
        *pins = swio_session.release();
        (dmstatus, partid)
    })
}

pub fn halt_target_port_global(port: TargetPort) -> Result<(), TargetError> {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        match port {
            TargetPort::BluePill => {
                let mut swd_session = pins.into_swd_port(TargetPort::BluePill, 1000);
                swd_session.attach().map_err(|_| TargetError::ProtocolError)?;
                let mut cortex = CortexM::new(&mut swd_session.transport);
                let res = cortex.halt().map_err(|_| TargetError::Timeout);
                *pins = swd_session.release();
                res
            }
            TargetPort::Ch32v203 => {
                let mut rvswd_session = pins.into_rvswd_port(TargetPort::Ch32v203);
                let mut qk = QingKe::new_2wire(&mut rvswd_session.transport);
                let res = qk.halt().map_err(|_| TargetError::Timeout);
                *pins = rvswd_session.release();
                res
            }
            TargetPort::Ch32v003 => {
                let mut swio_session = pins.into_swio_port(TargetPort::Ch32v003);
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
                let mut swd_session = pins.into_swd_port(TargetPort::BluePill, 1000);
                swd_session.attach().map_err(|_| TargetError::ProtocolError)?;
                let mut cortex = CortexM::new(&mut swd_session.transport);
                let res = cortex.resume().map_err(|_| TargetError::Timeout);
                *pins = swd_session.release();
                res
            }
            TargetPort::Ch32v203 => {
                let mut rvswd_session = pins.into_rvswd_port(TargetPort::Ch32v203);
                let mut qk = QingKe::new_2wire(&mut rvswd_session.transport);
                let res = qk.resume().map_err(|_| TargetError::Timeout);
                *pins = rvswd_session.release();
                res
            }
            TargetPort::Ch32v003 => {
                let mut swio_session = pins.into_swio_port(TargetPort::Ch32v003);
                let mut qk = QingKe::new_1wire(&mut swio_session.transport);
                let res = qk.resume().map_err(|_| TargetError::Timeout);
                *pins = swio_session.release();
                res
            }
        }
    })
}

pub fn read_flash_port_global(port: TargetPort, addr: u32, buf: &mut [u8]) -> Result<(), TargetError> {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        match port {
            TargetPort::BluePill => {
                let mut swd_session = pins.into_swd_port(TargetPort::BluePill, 1000);
                swd_session.attach().map_err(|_| TargetError::ProtocolError)?;
                let mut cortex = CortexM::new(&mut swd_session.transport);
                let res = cortex
                    .mem_read_block(addr, buf)
                    .map_err(|_| TargetError::ProtocolError);
                *pins = swd_session.release();
                res
            }
            TargetPort::Ch32v203 => {
                let mut rvswd_session = pins.into_rvswd_port(TargetPort::Ch32v203);
                let mut qk = QingKe::new_2wire(&mut rvswd_session.transport);
                let res = qk
                    .mem_read_block(addr, buf)
                    .map_err(|_| TargetError::ProtocolError);
                *pins = rvswd_session.release();
                res
            }
            TargetPort::Ch32v003 => {
                let mut swio_session = pins.into_swio_port(TargetPort::Ch32v003);
                let mut qk = QingKe::new_1wire(&mut swio_session.transport);
                let res = qk
                    .mem_read_block(addr, buf)
                    .map_err(|_| TargetError::ProtocolError);
                *pins = swio_session.release();
                res
            }
        }
    })
}

pub fn erase_flash_port_global(port: TargetPort, addr: u32, len: u32) -> Result<(), TargetError> {
    PROGRAMMER_PINS.lock(|cell| {
        let mut pins = cell.borrow_mut();
        match port {
            TargetPort::BluePill => {
                let mut swd_session = pins.into_swd_port(TargetPort::BluePill, 1000);
                swd_session.attach().map_err(|_| TargetError::ProtocolError)?;
                let mut cortex = CortexM::new(&mut swd_session.transport);
                cortex.halt().map_err(|_| TargetError::Timeout)?;
                let sector_size = 1024u32;
                let start_addr = addr & !(sector_size - 1);
                let end_addr = (addr + len + sector_size - 1) & !(sector_size - 1);
                let mut curr = start_addr;
                while curr < end_addr {
                    FlashAlgorithmRunner::stm32f1_erase_sector(&mut cortex, curr)
                        .map_err(|e| match e {
                            algo::AlgoError::ExecutionFailed(code) => TargetError::FlashErrorCode(code),
                            _ => TargetError::FlashError,
                        })?;
                    curr += sector_size;
                }
                // Lock flash before resuming
                let _ = cortex.swd.write_ap(crate::programmer::swd::AP_CSW, crate::programmer::swd::ADIV5_AP_CSW_DEFAULT);
                let _ = cortex.swd.write_ap(crate::programmer::swd::AP_TAR, 0x40022010);
                let _ = cortex.swd.write_ap(crate::programmer::swd::AP_DRW, 0x80); // LOCK bit
                let _ = cortex.resume();
                *pins = swd_session.release();
                Ok(())
            }
            TargetPort::Ch32v203 => {
                let mut rvswd_session = pins.into_rvswd_port(TargetPort::Ch32v203);
                let mut qk = QingKe::new_2wire(&mut rvswd_session.transport);
                qk.halt().map_err(|_| TargetError::Timeout)?;
                // Unlock
                qk.mem_write32(0x40022004, 0x45670123).map_err(|_| TargetError::FlashErrorCode(101))?;
                qk.mem_write32(0x40022004, 0xCDEF89AB).map_err(|_| TargetError::FlashErrorCode(102))?;
                qk.mem_write32(0x40022024, 0x45670123).map_err(|_| TargetError::FlashErrorCode(103))?;
                qk.mem_write32(0x40022024, 0xCDEF89AB).map_err(|_| TargetError::FlashErrorCode(104))?;

                let page_sz = 256u32;
                let start_addr = addr & !(page_sz - 1);
                let end_addr = (addr + len + page_sz - 1) & !(page_sz - 1);
                let mut curr = start_addr;
                while curr < end_addr {
                    qk.mem_write32(0x40022010, 1 << 17).map_err(|_| TargetError::FlashErrorCode(105))?; // FTER
                    qk.mem_write32(0x40022014, curr).map_err(|_| TargetError::FlashErrorCode(106))?; // ADDR
                    qk.mem_write32(0x40022010, (1 << 17) | (1 << 6)).map_err(|_| TargetError::FlashErrorCode(107))?; // FTER | STRT

                    cortex_m::asm::delay(75_000); // 500 µs initial delay (matching ch32.c)
                    let mut busy = true;
                    for _ in 0..1000 {
                        let statr = qk.mem_read32(0x4002200C).map_err(|e| match e {
                            target::qingke::QingKeError::CommandErrorCode(c) => TargetError::FlashErrorCode(1080 + c),
                            target::qingke::QingKeError::Timeout => TargetError::FlashErrorCode(0x7771),
                            target::qingke::QingKeError::WireError => TargetError::FlashErrorCode(0x7772),
                            target::qingke::QingKeError::CommandError => TargetError::FlashErrorCode(0x7773),
                            target::qingke::QingKeError::RvswdParity => TargetError::FlashErrorCode(0x7774),
                            target::qingke::QingKeError::RvswdTimeout => TargetError::FlashErrorCode(0x7775),
                        })?;
                        if (statr & 1) == 0 {
                            if (statr & 0x14) != 0 {
                                let _ = qk.mem_write32(0x4002200C, 0x14); // W1C
                                *pins = rvswd_session.release();
                                return Err(TargetError::FlashErrorCode(statr));
                            }
                            busy = false;
                            break;
                        }
                        cortex_m::asm::delay(7500);
                    }
                    if busy {
                        *pins = rvswd_session.release();
                        return Err(TargetError::Timeout);
                    }
                    curr += page_sz;
                }

                let _ = qk.mem_write32(0x40022010, 0);
                let _ = qk.mem_write32(0x40022010, 1 << 15); // FLOCK
                let _ = qk.resume();
                *pins = rvswd_session.release();
                Ok(())
            }
            TargetPort::Ch32v003 => {
                let mut swio_session = pins.into_swio_port(TargetPort::Ch32v003);
                let mut qk = QingKe::new_1wire(&mut swio_session.transport);
                qk.halt().map_err(|_| TargetError::Timeout)?;
                // Unlock
                qk.mem_write32(0x40022004, 0x45670123).map_err(|_| TargetError::FlashErrorCode(301))?;
                qk.mem_write32(0x40022004, 0xCDEF89AB).map_err(|_| TargetError::FlashErrorCode(302))?;
                qk.mem_write32(0x40022024, 0x45670123).map_err(|_| TargetError::FlashErrorCode(303))?;
                qk.mem_write32(0x40022024, 0xCDEF89AB).map_err(|_| TargetError::FlashErrorCode(304))?;

                let page_sz = 64u32;
                let start_addr = addr & !(page_sz - 1);
                let end_addr = (addr + len + page_sz - 1) & !(page_sz - 1);
                let mut curr = start_addr;
                while curr < end_addr {
                    qk.mem_write32(0x40022014, curr).map_err(|_| TargetError::FlashErrorCode(305))?; // ADDR
                    qk.mem_write32(0x40022010, (1 << 1) | (1 << 6)).map_err(|_| TargetError::FlashErrorCode(306))?; // PER | STRT

                    cortex_m::asm::delay(75_000); // 500 µs initial delay
                    let mut busy = true;
                    for _ in 0..1000 {
                        let statr = qk.mem_read32(0x4002200C).map_err(|_| TargetError::FlashErrorCode(307))?;
                        if (statr & 1) == 0 {
                            busy = false;
                            break;
                        }
                        cortex_m::asm::delay(7500);
                    }
                    if busy {
                        *pins = swio_session.release();
                        return Err(TargetError::Timeout);
                    }
                    curr += page_sz;
                }

                let _ = qk.mem_write32(0x40022010, 0);
                let _ = qk.mem_write32(0x40022010, 1 << 7); // LOCK
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
                let mut swd_session = pins.into_swd_port(TargetPort::BluePill, 1000);
                swd_session.attach().map_err(|_| TargetError::ProtocolError)?;
                let mut cortex = CortexM::new(&mut swd_session.transport);
                cortex.halt().map_err(|_| TargetError::Timeout)?;
                let page_size = 1024usize;
                let mut offset = 0usize;
                while offset < data.len() {
                    let chunk_len = (data.len() - offset).min(page_size);
                    FlashAlgorithmRunner::stm32f1_program_page(&mut cortex, addr + offset as u32, &data[offset..offset + chunk_len])
                        .map_err(|e| match e {
                            algo::AlgoError::ExecutionFailed(code) => TargetError::FlashErrorCode(code),
                            _ => TargetError::FlashError,
                        })?;
                    offset += chunk_len;
                }
                // Lock flash before verify and resume
                let _ = cortex.swd.write_ap(crate::programmer::swd::AP_CSW, crate::programmer::swd::ADIV5_AP_CSW_DEFAULT);
                let _ = cortex.swd.write_ap(crate::programmer::swd::AP_TAR, 0x40022010);
                let _ = cortex.swd.write_ap(crate::programmer::swd::AP_DRW, 0x80); // LOCK bit

                // Verify
                let mut readback = [0u8; 64];
                let mut v_offset = 0usize;
                while v_offset < data.len() {
                    let chunk_len = (data.len() - v_offset).min(readback.len());
                    cortex.mem_read_block(addr + v_offset as u32, &mut readback[..chunk_len])
                        .map_err(|_| TargetError::ProtocolError)?;
                    if readback[..chunk_len] != data[v_offset..v_offset + chunk_len] {
                        let _ = cortex.resume();
                        *pins = swd_session.release();
                        return Err(TargetError::FlashErrorCode(0xDEADBEE1));
                    }
                    v_offset += chunk_len;
                }
                let _ = cortex.resume();
                *pins = swd_session.release();
                Ok(())
            }
            TargetPort::Ch32v203 => {
                let mut rvswd_session = pins.into_rvswd_port(TargetPort::Ch32v203);
                let mut qk = QingKe::new_2wire(&mut rvswd_session.transport);
                qk.halt().map_err(|_| TargetError::Timeout)?;
                qk.mem_write32(0x40022004, 0x45670123).map_err(|_| TargetError::FlashErrorCode(201))?;
                qk.mem_write32(0x40022004, 0xCDEF89AB).map_err(|_| TargetError::FlashErrorCode(202))?;
                qk.mem_write32(0x40022024, 0x45670123).map_err(|_| TargetError::FlashErrorCode(203))?;
                qk.mem_write32(0x40022024, 0xCDEF89AB).map_err(|_| TargetError::FlashErrorCode(204))?;

                let page_sz = 256usize;
                let mut offset = 0usize;
                while offset < data.len() {
                    let page_addr = addr + offset as u32;
                    let chunk_len = (data.len() - offset).min(page_sz);

                    qk.mem_write32(0x40022010, (1 << 16) | (1 << 19)).map_err(|_| TargetError::FlashErrorCode(205))?; // FTPG | BUFRST
                    qk.mem_write32(0x40022014, page_addr).map_err(|_| TargetError::FlashErrorCode(206))?;

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
                        qk.mem_write32(page_addr + b as u32, word).map_err(|_| TargetError::FlashErrorCode(207))?;
                    }

                    qk.mem_write32(0x40022010, (1 << 16) | (1 << 6)).map_err(|_| TargetError::FlashErrorCode(208))?; // FTPG | STRT

                    cortex_m::asm::delay(75_000); // 500 µs initial delay
                    let mut busy = true;
                    for _ in 0..1000 {
                        let statr = qk.mem_read32(0x4002200C).map_err(|_| TargetError::FlashErrorCode(209))?;
                        if (statr & 1) == 0 {
                            if (statr & 0x14) != 0 {
                                let _ = qk.mem_write32(0x4002200C, 0x14);
                                *pins = rvswd_session.release();
                                return Err(TargetError::FlashErrorCode(statr));
                            }
                            busy = false;
                            break;
                        }
                        cortex_m::asm::delay(7500);
                    }
                    if busy {
                        *pins = rvswd_session.release();
                        return Err(TargetError::Timeout);
                    }

                    offset += chunk_len;
                }

                let _ = qk.mem_write32(0x40022010, 0);
                let _ = qk.mem_write32(0x40022010, 1 << 15); // FLOCK

                // Verify
                let mut verify_buf = [0u8; 64];
                let mut v_offset = 0usize;
                while v_offset < data.len() {
                    let chunk = (data.len() - v_offset).min(verify_buf.len());
                    qk.mem_read_block(addr + v_offset as u32, &mut verify_buf[..chunk])
                        .map_err(|_| TargetError::FlashErrorCode(210))?;
                    if verify_buf[..chunk] != data[v_offset..v_offset + chunk] {
                        let _ = qk.resume();
                        *pins = rvswd_session.release();
                        return Err(TargetError::FlashErrorCode(0xDEADBEE2));
                    }
                    v_offset += chunk;
                }

                let _ = qk.resume();
                *pins = rvswd_session.release();
                Ok(())
            }
            TargetPort::Ch32v003 => {
                let mut swio_session = pins.into_swio_port(TargetPort::Ch32v003);
                let mut qk = QingKe::new_1wire(&mut swio_session.transport);
                qk.halt().map_err(|_| TargetError::Timeout)?;
                qk.mem_write32(0x40022004, 0x45670123).map_err(|_| TargetError::FlashErrorCode(301))?;
                qk.mem_write32(0x40022004, 0xCDEF89AB).map_err(|_| TargetError::FlashErrorCode(302))?;
                qk.mem_write32(0x40022024, 0x45670123).map_err(|_| TargetError::FlashErrorCode(303))?;
                qk.mem_write32(0x40022024, 0xCDEF89AB).map_err(|_| TargetError::FlashErrorCode(304))?;

                let page_sz = 64usize;
                let mut offset = 0usize;
                while offset < data.len() {
                    let page_addr = addr + offset as u32;
                    let chunk_len = (data.len() - offset).min(page_sz);

                    qk.mem_write32(0x40022010, (1 << 16) | (1 << 21)).map_err(|_| TargetError::FlashErrorCode(305))?; // FTPG | BUFLOAD

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
                        qk.mem_write32(page_addr + b as u32, word).map_err(|_| TargetError::FlashErrorCode(306))?;
                    }

                    qk.mem_write32(0x40022010, (1 << 16) | (1 << 6)).map_err(|_| TargetError::FlashErrorCode(307))?; // FTPG | STRT

                    cortex_m::asm::delay(75_000); // 500 µs initial delay
                    let mut busy = true;
                    for _ in 0..1000 {
                        let statr = qk.mem_read32(0x4002200C).map_err(|_| TargetError::FlashErrorCode(308))?;
                        if (statr & 1) == 0 {
                            busy = false;
                            break;
                        }
                        cortex_m::asm::delay(7500);
                    }
                    if busy {
                        *pins = swio_session.release();
                        return Err(TargetError::Timeout);
                    }

                    offset += chunk_len;
                }

                let _ = qk.mem_write32(0x40022010, 0);
                let _ = qk.mem_write32(0x40022010, 1 << 7); // LOCK

                // Verify
                let mut verify_buf = [0u8; 64];
                let mut v_offset = 0usize;
                while v_offset < data.len() {
                    let chunk = (data.len() - v_offset).min(verify_buf.len());
                    qk.mem_read_block(addr + v_offset as u32, &mut verify_buf[..chunk])
                        .map_err(|_| TargetError::FlashErrorCode(309))?;
                    if verify_buf[..chunk] != data[v_offset..v_offset + chunk] {
                        let _ = qk.resume();
                        *pins = swio_session.release();
                        return Err(TargetError::FlashErrorCode(0xDEADBEE3));
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

pub fn resume_target_global() -> Result<(), TargetError> {
    resume_target_port_global(TargetPort::BluePill)
}

pub fn read_flash_global(addr: u32, buf: &mut [u8]) -> Result<(), TargetError> {
    read_flash_port_global(TargetPort::BluePill, addr, buf)
}

pub fn flash_target_global(addr: u32, data: &[u8]) -> Result<(), TargetError> {
    flash_target_port_global(TargetPort::BluePill, addr, data)
}
