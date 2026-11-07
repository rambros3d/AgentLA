//! Arm ADIv5 SWD protocol engine and sequence handlers.

use crate::delay::delay_cycles;
use crate::traits::SwdIo;

pub const SWD_ACK_OK: u8 = 0x01;
pub const SWD_ACK_WAIT: u8 = 0x02;
pub const SWD_ACK_FAULT: u8 = 0x04;
pub const SWD_ACK_PARITY_ERR: u8 = 0x05;
pub const SWD_ACK_TIMEOUT: u8 = 0x07;

pub const DP_DPIDR: u8 = 0x00;
pub const DP_ABORT: u8 = 0x00;
pub const DP_CTRLSTAT: u8 = 0x04;
pub const DP_SELECT: u8 = 0x08;
pub const DP_RDBUFF: u8 = 0x0C;

pub const AP_CSW: u8 = 0x00;
pub const AP_TAR: u8 = 0x04;
pub const AP_DRW: u8 = 0x0C;
pub const AP_IDR: u8 = 0xFC;

pub const ADIV5_AP_CSW_DEFAULT: u32 = 0x23000012;
pub const ADIV5_AP_CSW_16BIT: u32 = 0x23000011;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SwdError {
    AckWait,
    AckFault,
    ParityError,
    ProtocolError,
    Timeout,
    NoTarget,
}

/// Builds an 8-bit SWD request packet.
pub fn build_request(is_ap: bool, is_read: bool, a23: u8) -> u8 {
    let mut prq = 1u8; // Start bit
    let ap = if is_ap { 1u8 } else { 0u8 };
    let rnw = if is_read { 1u8 } else { 0u8 };
    let a2 = a23 & 1;
    let a3 = (a23 >> 1) & 1;

    prq |= ap << 1;
    prq |= rnw << 2;
    prq |= a2 << 3;
    prq |= a3 << 4;

    let parity = (ap + rnw + a2 + a3) & 1;
    prq |= parity << 5;
    // Stop bit = 0 (bit 6)
    prq |= 1 << 7; // Park bit = 1
    prq
}

/// Abstract ADIv5 interface trait allowing Cortex-M and FLM runners
/// to access Debug Port (DP) and Access Port (AP) registers.
pub trait Adiv5Access {
    fn write_dp(&mut self, reg: u8, value: u32) -> Result<(), SwdError>;
    fn read_dp(&mut self, reg: u8, value: &mut u32) -> Result<(), SwdError>;
    fn select_ap(&mut self, apsel: u8, apbank: u8) -> Result<(), SwdError>;
    fn power_up_debug(&mut self) -> Result<(), SwdError>;
    fn recover_from_ack_error(&mut self) -> Result<(), SwdError>;
    fn write_ap(&mut self, reg: u8, value: u32) -> Result<(), SwdError>;
    fn write_ap_port(&mut self, apsel: u8, reg: u8, value: u32) -> Result<(), SwdError>;
    fn read_ap(&mut self, reg: u8, value: &mut u32) -> Result<(), SwdError>;
    fn read_ap_port(&mut self, apsel: u8, reg: u8, value: &mut u32) -> Result<(), SwdError>;
    fn read_dpidr_retry(&mut self, idcode: &mut u32) -> Result<(), SwdError>;
    fn assert_reset(&mut self, assert: bool) -> Result<(), SwdError>;
}

/// Generic ADIv5 state machine operating over any `SwdIo` transport.
pub struct Adiv5Engine<T: SwdIo> {
    pub transport: T,
    pub ap_select: u32,
}

impl<T: SwdIo> Adiv5Engine<T> {
    pub const fn new(transport: T) -> Self {
        Self {
            transport,
            ap_select: 0xFFFFFFFF,
        }
    }
}

impl<T: SwdIo> Adiv5Access for Adiv5Engine<T> {
    fn write_dp(&mut self, reg: u8, value: u32) -> Result<(), SwdError> {
        let req = build_request(false, false, (reg >> 2) & 3);
        for _ in 0..200 {
            match self.transport.transfer(req, Some(value)) {
                Ok((SWD_ACK_OK, _)) => return Ok(()),
                Ok((SWD_ACK_WAIT, _)) => {
                    delay_cycles(300);
                    continue;
                }
                Ok((SWD_ACK_FAULT, _)) => return Err(SwdError::AckFault),
                _ => return Err(SwdError::ProtocolError),
            }
        }
        Err(SwdError::Timeout)
    }

    fn read_dp(&mut self, reg: u8, value: &mut u32) -> Result<(), SwdError> {
        let req = build_request(false, true, (reg >> 2) & 3);
        for _ in 0..200 {
            match self.transport.transfer(req, None) {
                Ok((SWD_ACK_OK, data)) => {
                    *value = data;
                    return Ok(());
                }
                Ok((SWD_ACK_WAIT, _)) => {
                    delay_cycles(300);
                    continue;
                }
                Ok((SWD_ACK_FAULT, _)) => return Err(SwdError::AckFault),
                Ok((SWD_ACK_PARITY_ERR, _)) => return Err(SwdError::ParityError),
                _ => return Err(SwdError::ProtocolError),
            }
        }
        Err(SwdError::Timeout)
    }

    fn select_ap(&mut self, apsel: u8, apbank: u8) -> Result<(), SwdError> {
        let select = ((apsel as u32) << 24) | (((apbank as u32) & 0x0F) << 4);
        if self.ap_select != select {
            self.write_dp(DP_SELECT, select)?;
            self.ap_select = select;
        }
        Ok(())
    }

    fn power_up_debug(&mut self) -> Result<(), SwdError> {
        let _ = self.write_dp(DP_ABORT, 0x0000001E);
        let _ = self.write_dp(DP_CTRLSTAT, 0x50000000);
        for _ in 0..100 {
            let mut val = 0u32;
            if self.read_dp(DP_CTRLSTAT, &mut val).is_ok() && (val & 0xA0000000) == 0xA0000000 {
                return Ok(());
            }
        }
        Ok(())
    }

    fn recover_from_ack_error(&mut self) -> Result<(), SwdError> {
        let _ = self.transport.jtag_to_swd();
        let mut idcode = 0u32;
        if self.read_dpidr_retry(&mut idcode).is_err() || idcode == 0 || idcode == 0xFFFFFFFF {
            return Err(SwdError::ProtocolError);
        }
        let _ = self.write_dp(DP_ABORT, 0x0000001E);
        let _ = self.write_dp(DP_CTRLSTAT, 0x50000000);
        self.power_up_debug()?;
        self.ap_select = 0xFFFFFFFF;
        Ok(())
    }

    fn write_ap(&mut self, reg: u8, value: u32) -> Result<(), SwdError> {
        self.write_ap_port(0, reg, value)
    }

    fn write_ap_port(&mut self, apsel: u8, reg: u8, value: u32) -> Result<(), SwdError> {
        for _attempt in 0..2 {
            let apbank = (reg >> 4) & 0x0F;
            self.select_ap(apsel, apbank)?;
            let req = build_request(true, false, (reg >> 2) & 3);
            for _ in 0..200 {
                match self.transport.transfer(req, Some(value)) {
                    Ok((SWD_ACK_OK, _)) => return Ok(()),
                    Ok((SWD_ACK_WAIT, _)) => {
                        delay_cycles(300);
                        continue;
                    }
                    Ok((SWD_ACK_FAULT, _)) => {
                        let _ = self.recover_from_ack_error();
                        break;
                    }
                    _ => {
                        let _ = self.recover_from_ack_error();
                        break;
                    }
                }
            }
        }
        Err(SwdError::ProtocolError)
    }

    fn read_ap(&mut self, reg: u8, value: &mut u32) -> Result<(), SwdError> {
        self.read_ap_port(0, reg, value)
    }

    fn read_ap_port(&mut self, apsel: u8, reg: u8, value: &mut u32) -> Result<(), SwdError> {
        for _attempt in 0..2 {
            let apbank = (reg >> 4) & 0x0F;
            self.select_ap(apsel, apbank)?;
            let req = build_request(true, true, (reg >> 2) & 3);
            let mut ok = false;
            for _ in 0..200 {
                match self.transport.transfer(req, None) {
                    Ok((SWD_ACK_OK, _)) => {
                        ok = true;
                        break;
                    }
                    Ok((SWD_ACK_WAIT, _)) => {
                        delay_cycles(300);
                        continue;
                    }
                    Ok((SWD_ACK_FAULT, _)) => {
                        let _ = self.recover_from_ack_error();
                        break;
                    }
                    _ => {
                        let _ = self.recover_from_ack_error();
                        break;
                    }
                }
            }
            if ok {
                return self.read_dp(DP_RDBUFF, value);
            }
        }
        Err(SwdError::ProtocolError)
    }

    fn read_dpidr_retry(&mut self, idcode: &mut u32) -> Result<(), SwdError> {
        let mut res = self.read_dp(DP_DPIDR, idcode);
        let freqs = [100, 200, 500, 1000];
        let mut freq_idx = 0;

        for retry in 0..30 {
            if res.is_ok() && *idcode != 0 && *idcode != 0xFFFFFFFF {
                return Ok(());
            }
            if retry > 0 && retry % 5 == 0 {
                if freq_idx < 3 {
                    freq_idx += 1;
                    let _ = self.transport.set_clock(freqs[freq_idx]);
                }
                let _ = self.transport.jtag_to_swd();
            }
            let _ = self.write_dp(DP_ABORT, 0x0000001E);
            let _ = self.write_dp(DP_CTRLSTAT, 0x50000000);
            res = self.read_dp(DP_DPIDR, idcode);
        }
        res
    }

    fn assert_reset(&mut self, assert: bool) -> Result<(), SwdError> {
        self.transport
            .assert_reset(assert)
            .map_err(|_| SwdError::ProtocolError)
    }
}

impl<T: Adiv5Access + ?Sized> Adiv5Access for &mut T {
    fn write_dp(&mut self, reg: u8, value: u32) -> Result<(), SwdError> {
        (**self).write_dp(reg, value)
    }
    fn read_dp(&mut self, reg: u8, value: &mut u32) -> Result<(), SwdError> {
        (**self).read_dp(reg, value)
    }
    fn select_ap(&mut self, apsel: u8, apbank: u8) -> Result<(), SwdError> {
        (**self).select_ap(apsel, apbank)
    }
    fn power_up_debug(&mut self) -> Result<(), SwdError> {
        (**self).power_up_debug()
    }
    fn recover_from_ack_error(&mut self) -> Result<(), SwdError> {
        (**self).recover_from_ack_error()
    }
    fn write_ap(&mut self, reg: u8, value: u32) -> Result<(), SwdError> {
        (**self).write_ap(reg, value)
    }
    fn write_ap_port(&mut self, apsel: u8, reg: u8, value: u32) -> Result<(), SwdError> {
        (**self).write_ap_port(apsel, reg, value)
    }
    fn read_ap(&mut self, reg: u8, value: &mut u32) -> Result<(), SwdError> {
        (**self).read_ap(reg, value)
    }
    fn read_ap_port(&mut self, apsel: u8, reg: u8, value: &mut u32) -> Result<(), SwdError> {
        (**self).read_ap_port(apsel, reg, value)
    }
    fn read_dpidr_retry(&mut self, idcode: &mut u32) -> Result<(), SwdError> {
        (**self).read_dpidr_retry(idcode)
    }
    fn assert_reset(&mut self, assert: bool) -> Result<(), SwdError> {
        (**self).assert_reset(assert)
    }
}
