//! CMSIS-DAP v1 and v2 command parsing, execution, and response serialization.

use crate::delay::delay_cycles;
use crate::swd::{
    build_request, SWD_ACK_FAULT, SWD_ACK_OK, SWD_ACK_TIMEOUT, SWD_ACK_WAIT,
};
use crate::traits::SwdIo;

pub const DAP_INFO_ID: u8 = 0x00;
pub const DAP_HOST_STATUS: u8 = 0x01;
pub const DAP_CONNECT: u8 = 0x02;
pub const DAP_DISCONNECT: u8 = 0x03;
pub const DAP_TRANSFER_CONFIGURE: u8 = 0x04;
pub const DAP_TRANSFER: u8 = 0x05;
pub const DAP_TRANSFER_BLOCK: u8 = 0x06;
pub const DAP_TRANSFER_ABORT: u8 = 0x07;
pub const DAP_WRITE_ABORT: u8 = 0x08;
pub const DAP_DELAY: u8 = 0x09;
pub const DAP_RESET_TARGET: u8 = 0x0A;
pub const DAP_SWJ_PINS: u8 = 0x10;
pub const DAP_SWJ_CLOCK: u8 = 0x11;
pub const DAP_SWJ_SEQUENCE: u8 = 0x12;
pub const DAP_SWD_CONFIGURE: u8 = 0x13;
pub const DAP_SWD_SEQUENCE: u8 = 0x1D;

pub const DAP_PORT_SWD: u8 = 0x01;

pub const DAP_TRANSFER_OK: u8 = 0x01;
pub const DAP_TRANSFER_WAIT: u8 = 0x02;
pub const DAP_TRANSFER_FAULT: u8 = 0x04;
pub const DAP_TRANSFER_ERROR: u8 = 0x08;
pub const DAP_TRANSFER_NO_ACK: u8 = 0x10;

pub struct CmsisDapHandler<T: SwdIo> {
    pub transport: T,
    pub idle_cycles: u8,
    pub wait_retry: u16,
    pub connected: bool,
}

impl<T: SwdIo> CmsisDapHandler<T> {
    pub const fn new(transport: T) -> Self {
        Self {
            transport,
            idle_cycles: 0,
            wait_retry: 100,
            connected: false,
        }
    }

    pub fn handle_packet(&mut self, req: &[u8], resp: &mut [u8]) -> usize {
        if req.is_empty() {
            return 0;
        }

        match req[0] {
            DAP_INFO_ID => self.handle_info(req, resp),
            DAP_HOST_STATUS => {
                resp[0] = DAP_HOST_STATUS;
                resp[1] = 0; // OK
                2
            }
            DAP_CONNECT => self.handle_connect(req, resp),
            DAP_DISCONNECT => self.handle_disconnect(req, resp),
            DAP_TRANSFER_CONFIGURE => self.handle_transfer_configure(req, resp),
            DAP_TRANSFER => self.handle_transfer(req, resp),
            DAP_TRANSFER_BLOCK => self.handle_transfer_block(req, resp),
            DAP_WRITE_ABORT => self.handle_write_abort(req, resp),
            DAP_DELAY => {
                resp[0] = DAP_DELAY;
                if req.len() >= 3 {
                    let us = (req[1] as u32) | ((req[2] as u32) << 8);
                    delay_cycles(us * 150);
                }
                resp[1] = 0;
                2
            }
            DAP_RESET_TARGET => {
                resp[0] = DAP_RESET_TARGET;
                let _ = self.transport.reset_sequence();
                resp[1] = 0;
                2
            }
            DAP_SWJ_PINS => self.handle_swj_pins(req, resp),
            DAP_SWJ_CLOCK => self.handle_swj_clock(req, resp),
            DAP_SWJ_SEQUENCE => self.handle_swj_sequence(req, resp),
            DAP_SWD_CONFIGURE => {
                resp[0] = DAP_SWD_CONFIGURE;
                if req.len() >= 2 {
                    let turnaround = (req[1] & 0x03) + 1;
                    self.transport.set_turnaround(turnaround);
                }
                resp[1] = 0;
                2
            }
            cmd => {
                resp[0] = cmd;
                resp[1] = 0x00;
                2
            }
        }
    }

    fn handle_info(&self, req: &[u8], resp: &mut [u8]) -> usize {
        resp[0] = DAP_INFO_ID;
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
                let ver = b"1.0.0";
                resp[1] = ver.len() as u8;
                resp[2..2 + ver.len()].copy_from_slice(ver);
                2 + ver.len()
            }
            0x05 => {
                let v = b"ARM";
                resp[1] = v.len() as u8;
                resp[2..2 + v.len()].copy_from_slice(v);
                2 + v.len()
            }
            0x06 => {
                let n = b"Cortex-M";
                resp[1] = n.len() as u8;
                resp[2..2 + n.len()].copy_from_slice(n);
                2 + n.len()
            }
            0xF0 => {
                resp[1] = 1;
                resp[2] = 0x01; // SWD supported
                3
            }
            0xFE => {
                resp[1] = 1;
                resp[2] = 1;
                3
            }
            0xFF => {
                resp[1] = 2;
                resp[2] = 0x00;
                resp[3] = 0x02; // 512 = 0x0200
                4
            }
            _ => {
                resp[1] = 0;
                2
            }
        }
    }

    fn handle_connect(&mut self, req: &[u8], resp: &mut [u8]) -> usize {
        resp[0] = DAP_CONNECT;
        let port = if req.len() >= 2 { req[1] } else { 0 };

        if port == 0 || port == DAP_PORT_SWD {
            let _ = self.transport.jtag_to_swd();
            self.connected = true;
            resp[1] = DAP_PORT_SWD;
        } else {
            resp[1] = 0;
        }
        2
    }

    fn handle_disconnect(&mut self, _req: &[u8], resp: &mut [u8]) -> usize {
        resp[0] = DAP_DISCONNECT;
        self.connected = false;
        resp[1] = 0;
        2
    }

    fn handle_transfer_configure(&mut self, req: &[u8], resp: &mut [u8]) -> usize {
        resp[0] = DAP_TRANSFER_CONFIGURE;
        if req.len() >= 4 {
            self.idle_cycles = req[1];
            self.wait_retry = (req[2] as u16) | ((req[3] as u16) << 8);
            self.transport.set_idle_cycles(self.idle_cycles);
        }
        resp[1] = 0;
        2
    }

    fn handle_transfer(&mut self, req: &[u8], resp: &mut [u8]) -> usize {
        resp[0] = DAP_TRANSFER;
        if req.len() < 3 {
            resp[1] = 0;
            resp[2] = DAP_TRANSFER_ERROR;
            return 3;
        }

        let request_count = req[2] as usize;
        resp[1] = 0;
        resp[2] = DAP_TRANSFER_OK;

        let mut req_idx = 3;
        let mut resp_idx = 3;
        let mut response_count = 0u8;
        let mut post_read = false;
        let wait_retry = self.wait_retry;

        for _ in 0..request_count {
            if req_idx >= req.len() {
                resp[2] = DAP_TRANSFER_ERROR | DAP_TRANSFER_NO_ACK;
                break;
            }

            let req_byte = req[req_idx];
            req_idx += 1;

            let is_ap = (req_byte & 0x01) != 0;
            let is_read = (req_byte & 0x02) != 0;
            let a23 = (req_byte >> 2) & 0x03;
            let wire_req = build_request(is_ap, is_read, a23);

            if !is_read {
                // Write transfer
                if req_idx + 4 > req.len() {
                    resp[2] = DAP_TRANSFER_ERROR | DAP_TRANSFER_NO_ACK;
                    break;
                }
                let val = u32::from_le_bytes([
                    req[req_idx],
                    req[req_idx + 1],
                    req[req_idx + 2],
                    req[req_idx + 3],
                ]);
                req_idx += 4;

                let mut ack = SWD_ACK_OK;
                for _ in 0..wait_retry {
                    match self.transport.transfer(wire_req, Some(val)) {
                        Ok((a, _)) => {
                            ack = a;
                            if ack != SWD_ACK_WAIT {
                                break;
                            }
                        }
                        Err(_) => {
                            ack = SWD_ACK_TIMEOUT;
                            break;
                        }
                    }
                    delay_cycles(1500);
                }

                if ack != SWD_ACK_OK {
                    resp[2] = if ack == SWD_ACK_WAIT {
                        DAP_TRANSFER_WAIT
                    } else if ack == SWD_ACK_FAULT {
                        DAP_TRANSFER_FAULT
                    } else {
                        DAP_TRANSFER_ERROR | DAP_TRANSFER_NO_ACK
                    };
                    break;
                }
                response_count += 1;
            } else {
                // Read transfer
                if post_read {
                    let mut prev_data = 0u32;
                    let mut ack = SWD_ACK_OK;
                    for _ in 0..wait_retry {
                        match self.transport.transfer(wire_req, None) {
                            Ok((a, data)) => {
                                ack = a;
                                prev_data = data;
                                if ack != SWD_ACK_WAIT {
                                    break;
                                }
                            }
                            Err(_) => {
                                ack = SWD_ACK_TIMEOUT;
                                break;
                            }
                        }
                        delay_cycles(1500);
                    }
                    if ack != SWD_ACK_OK {
                        resp[2] = if ack == SWD_ACK_WAIT {
                            DAP_TRANSFER_WAIT
                        } else if ack == SWD_ACK_FAULT {
                            DAP_TRANSFER_FAULT
                        } else {
                            DAP_TRANSFER_ERROR | DAP_TRANSFER_NO_ACK
                        };
                        break;
                    }
                    if resp_idx + 4 <= resp.len() {
                        resp[resp_idx..resp_idx + 4].copy_from_slice(&prev_data.to_le_bytes());
                        resp_idx += 4;
                    }
                } else if is_ap {
                    let mut ack = SWD_ACK_OK;
                    for _ in 0..wait_retry {
                        match self.transport.transfer(wire_req, None) {
                            Ok((a, _)) => {
                                ack = a;
                                if ack != SWD_ACK_WAIT {
                                    break;
                                }
                            }
                            Err(_) => {
                                ack = SWD_ACK_TIMEOUT;
                                break;
                            }
                        }
                        delay_cycles(1500);
                    }
                    if ack != SWD_ACK_OK {
                        resp[2] = if ack == SWD_ACK_WAIT {
                            DAP_TRANSFER_WAIT
                        } else if ack == SWD_ACK_FAULT {
                            DAP_TRANSFER_FAULT
                        } else {
                            DAP_TRANSFER_ERROR | DAP_TRANSFER_NO_ACK
                        };
                        break;
                    }
                    post_read = true;
                } else {
                    let mut dp_data = 0u32;
                    let mut ack = SWD_ACK_OK;
                    for _ in 0..wait_retry {
                        match self.transport.transfer(wire_req, None) {
                            Ok((a, data)) => {
                                ack = a;
                                dp_data = data;
                                if ack != SWD_ACK_WAIT {
                                    break;
                                }
                            }
                            Err(_) => {
                                ack = SWD_ACK_TIMEOUT;
                                break;
                            }
                        }
                        delay_cycles(1500);
                    }
                    if ack != SWD_ACK_OK {
                        resp[2] = if ack == SWD_ACK_WAIT {
                            DAP_TRANSFER_WAIT
                        } else if ack == SWD_ACK_FAULT {
                            DAP_TRANSFER_FAULT
                        } else {
                            DAP_TRANSFER_ERROR | DAP_TRANSFER_NO_ACK
                        };
                        break;
                    }
                    if resp_idx + 4 <= resp.len() {
                        resp[resp_idx..resp_idx + 4].copy_from_slice(&dp_data.to_le_bytes());
                        resp_idx += 4;
                    }
                }
                response_count += 1;
            }
        }

        if resp[2] == DAP_TRANSFER_OK && post_read {
            let rdbuff_wire = build_request(false, true, 3);
            let mut final_data = 0u32;
            let mut ack = SWD_ACK_OK;
            for _ in 0..wait_retry {
                match self.transport.transfer(rdbuff_wire, None) {
                    Ok((a, data)) => {
                        ack = a;
                        final_data = data;
                        if ack != SWD_ACK_WAIT {
                            break;
                        }
                    }
                    Err(_) => {
                        ack = SWD_ACK_TIMEOUT;
                        break;
                    }
                }
                delay_cycles(1500);
            }
            if ack == SWD_ACK_OK {
                if resp_idx + 4 <= resp.len() {
                    resp[resp_idx..resp_idx + 4].copy_from_slice(&final_data.to_le_bytes());
                    resp_idx += 4;
                }
            } else {
                resp[2] = if ack == SWD_ACK_WAIT {
                    DAP_TRANSFER_WAIT
                } else if ack == SWD_ACK_FAULT {
                    DAP_TRANSFER_FAULT
                } else {
                    DAP_TRANSFER_ERROR | DAP_TRANSFER_NO_ACK
                };
            }
        }

        resp[1] = response_count;
        resp_idx
    }

    fn handle_transfer_block(&mut self, req: &[u8], resp: &mut [u8]) -> usize {
        resp[0] = DAP_TRANSFER_BLOCK;
        if req.len() < 5 {
            resp[1] = 0;
            resp[2] = 0;
            resp[3] = DAP_TRANSFER_ERROR;
            return 4;
        }

        let count = ((req[2] as usize) | ((req[3] as usize) << 8)) as u16;
        resp[1] = 0;
        resp[2] = 0;
        resp[3] = DAP_TRANSFER_OK;

        if count == 0 {
            return 4;
        }

        let req_byte = req[4];
        let is_ap = (req_byte & 0x01) != 0;
        let is_read = (req_byte & 0x02) != 0;
        let a23 = (req_byte >> 2) & 0x03;
        let wire_req = build_request(is_ap, is_read, a23);

        let mut req_idx = 5;
        let mut resp_idx = 4;
        let mut completed = 0u16;
        let wait_retry = self.wait_retry;

        if is_read {
            for _ in 0..count {
                let mut data = 0u32;
                let mut ack = SWD_ACK_OK;
                for _ in 0..wait_retry {
                    match self.transport.transfer(wire_req, None) {
                        Ok((a, d)) => {
                            ack = a;
                            data = d;
                            if ack != SWD_ACK_WAIT {
                                break;
                            }
                        }
                        Err(_) => {
                            ack = SWD_ACK_TIMEOUT;
                            break;
                        }
                    }
                    delay_cycles(1500);
                }
                if ack != SWD_ACK_OK {
                    resp[3] = if ack == SWD_ACK_WAIT {
                        DAP_TRANSFER_WAIT
                    } else if ack == SWD_ACK_FAULT {
                        DAP_TRANSFER_FAULT
                    } else {
                        DAP_TRANSFER_ERROR | DAP_TRANSFER_NO_ACK
                    };
                    break;
                }
                if resp_idx + 4 <= resp.len() {
                    resp[resp_idx..resp_idx + 4].copy_from_slice(&data.to_le_bytes());
                    resp_idx += 4;
                }
                completed += 1;
            }
        } else {
            for _ in 0..count {
                if req_idx + 4 > req.len() {
                    resp[3] = DAP_TRANSFER_ERROR | DAP_TRANSFER_NO_ACK;
                    break;
                }
                let value = u32::from_le_bytes([
                    req[req_idx],
                    req[req_idx + 1],
                    req[req_idx + 2],
                    req[req_idx + 3],
                ]);
                req_idx += 4;

                let mut ack = SWD_ACK_OK;
                for _ in 0..wait_retry {
                    match self.transport.transfer(wire_req, Some(value)) {
                        Ok((a, _)) => {
                            ack = a;
                            if ack != SWD_ACK_WAIT {
                                break;
                            }
                        }
                        Err(_) => {
                            ack = SWD_ACK_TIMEOUT;
                            break;
                        }
                    }
                    delay_cycles(1500);
                }
                if ack != SWD_ACK_OK {
                    resp[3] = if ack == SWD_ACK_WAIT {
                        DAP_TRANSFER_WAIT
                    } else if ack == SWD_ACK_FAULT {
                        DAP_TRANSFER_FAULT
                    } else {
                        DAP_TRANSFER_ERROR | DAP_TRANSFER_NO_ACK
                    };
                    break;
                }
                completed += 1;
            }
        }

        resp[1] = (completed & 0xFF) as u8;
        resp[2] = ((completed >> 8) & 0xFF) as u8;
        if !is_read {
            4
        } else {
            resp_idx
        }
    }

    fn handle_swj_pins(&mut self, req: &[u8], resp: &mut [u8]) -> usize {
        resp[0] = DAP_SWJ_PINS;
        if req.len() < 7 {
            resp[1] = 0;
            return 2;
        }

        let pin_output = req[1];
        let pin_select = req[2];
        let wait_us = (req[3] as u32)
            | ((req[4] as u32) << 8)
            | ((req[5] as u32) << 16)
            | ((req[6] as u32) << 24);

        let in_val = self
            .transport
            .swj_pins(pin_output, pin_select, wait_us)
            .unwrap_or(0);
        resp[1] = in_val;
        2
    }

    fn handle_swj_sequence(&mut self, req: &[u8], resp: &mut [u8]) -> usize {
        resp[0] = DAP_SWJ_SEQUENCE;
        if req.len() < 2 {
            resp[1] = 1;
            return 2;
        }

        let bit_count = if req[1] == 0 {
            256
        } else {
            req[1] as usize
        };
        let byte_count = (bit_count + 7) / 8;

        if req.len() < 2 + byte_count {
            resp[1] = 1;
            return 2;
        }

        let mut bits_left = bit_count;
        for i in 0..byte_count {
            let chunk_bits = if bits_left >= 8 { 8 } else { bits_left };
            let _ = self
                .transport
                .write_bits(chunk_bits as u32, req[2 + i] as u32);
            bits_left -= chunk_bits;
        }

        resp[1] = 0;
        2
    }

    fn handle_swj_clock(&mut self, req: &[u8], resp: &mut [u8]) -> usize {
        resp[0] = DAP_SWJ_CLOCK;
        if req.len() >= 5 {
            let freq_hz = (req[1] as u32)
                | ((req[2] as u32) << 8)
                | ((req[3] as u32) << 16)
                | ((req[4] as u32) << 24);
            let freq_khz = (freq_hz / 1000).max(1);
            let _ = self.transport.set_clock(freq_khz);
        }
        resp[1] = 0;
        2
    }

    fn handle_write_abort(&mut self, req: &[u8], resp: &mut [u8]) -> usize {
        resp[0] = DAP_WRITE_ABORT;
        let abort_val = if req.len() >= 6 {
            (req[2] as u32)
                | ((req[3] as u32) << 8)
                | ((req[4] as u32) << 16)
                | ((req[5] as u32) << 24)
        } else {
            0x1E
        };
        let abort_req = build_request(false, false, 0);
        let _ = self.transport.transfer(abort_req, Some(abort_val));
        resp[1] = 0;
        2
    }
}
