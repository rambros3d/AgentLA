//! Driver for Texas Instruments PGA113 Zero-Drift Programmable Gain Amplifier.
//!
//! The PGA113 provides software-configurable gains in 1-2-5 oscilloscope increments
//! (1x, 2x, 5x, 10x, 20x, 50x, 100x, 200x) controlled via a 16-bit SPI transaction.
//! It also features an internal VCAL calibration channel outputting 0.05 * VDD.

/// PGA113 gain selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Pga113Gain {
    Gain1x = 0x00,
    Gain2x = 0x10,
    Gain5x = 0x20,
    Gain10x = 0x30,
    Gain20x = 0x40,
    Gain50x = 0x50,
    Gain100x = 0x60,
    Gain200x = 0x70,
}

impl Pga113Gain {
    /// Return the numerical gain multiplier (e.g. 50 for 50x).
    pub const fn multiplier(self) -> u16 {
        match self {
            Pga113Gain::Gain1x => 1,
            Pga113Gain::Gain2x => 2,
            Pga113Gain::Gain5x => 5,
            Pga113Gain::Gain10x => 10,
            Pga113Gain::Gain20x => 20,
            Pga113Gain::Gain50x => 50,
            Pga113Gain::Gain100x => 100,
            Pga113Gain::Gain200x => 200,
        }
    }

    /// Map a numerical gain multiplier to a PGA113 gain enum.
    pub const fn from_multiplier(multiplier: u16) -> Option<Self> {
        match multiplier {
            1 => Some(Pga113Gain::Gain1x),
            2 => Some(Pga113Gain::Gain2x),
            5 => Some(Pga113Gain::Gain5x),
            10 => Some(Pga113Gain::Gain10x),
            20 => Some(Pga113Gain::Gain20x),
            50 => Some(Pga113Gain::Gain50x),
            100 => Some(Pga113Gain::Gain100x),
            200 => Some(Pga113Gain::Gain200x),
            _ => None,
        }
    }

    /// Raw 4-bit nibble for the SPI payload.
    pub const fn raw_nibble(self) -> u8 {
        (self as u8) >> 4
    }
}

/// PGA113 analog input channel selection.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Pga113Channel {
    /// Channel 0: Internal calibration voltage VCAL = 0.05 * VDD.
    Vcal = 0x00,
    /// Channel 1: Primary analog input pin.
    Ch1 = 0x01,
}

/// SPI command opcodes for PGA113.
pub mod opcode {
    pub const WRITE: u8 = 0x2A;
    pub const READ: u8 = 0x6A;
    pub const SHUTDOWN: u8 = 0xE1;
}

/// PGA113 SPI frame encoder and configuration helper.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Pga113Config {
    pub gain: Pga113Gain,
    pub channel: Pga113Channel,
    pub is_shutdown: bool,
}

impl Default for Pga113Config {
    fn default() -> Self {
        Self {
            gain: Pga113Gain::Gain1x,
            channel: Pga113Channel::Ch1,
            is_shutdown: false,
        }
    }
}

impl Pga113Config {
    pub const fn new() -> Self {
        Self {
            gain: Pga113Gain::Gain1x,
            channel: Pga113Channel::Ch1,
            is_shutdown: false,
        }
    }

    /// Set gain and return updated config.
    pub const fn with_gain(mut self, gain: Pga113Gain) -> Self {
        self.gain = gain;
        self
    }

    /// Set channel and return updated config.
    pub const fn with_channel(mut self, channel: Pga113Channel) -> Self {
        self.channel = channel;
        self
    }

    /// Encode a 16-bit SPI write word for current configuration.
    pub const fn encode_write(&self) -> u16 {
        encode_write_frame(self.gain, self.channel)
    }

    /// Encode a 16-bit SPI shutdown frame.
    pub const fn encode_shutdown() -> u16 {
        ((opcode::SHUTDOWN as u16) << 8) | 0x0000
    }

    /// Calculate expected VCAL output voltage in millivolts given VDD in millivolts.
    ///
    /// VCAL is nominally 0.05 * VDD. The amplifier output is Gain * VCAL.
    pub const fn expected_vcal_mv(&self, vdd_mv: u32) -> u32 {
        let vcal_base_mv = (vdd_mv * 5) / 100;
        let mult = self.gain.multiplier() as u32;
        vcal_base_mv * mult
    }

    /// Calculate calibration gain error in parts-per-thousand (ppt).
    ///
    /// Positive means measured voltage is higher than theoretical.
    pub fn calculate_gain_error_ppt(&self, measured_mv: u32, vdd_mv: u32) -> i32 {
        let expected = self.expected_vcal_mv(vdd_mv) as i32;
        if expected == 0 {
            return 0;
        }
        let diff = (measured_mv as i32) - expected;
        (diff * 1000) / expected
    }
}

/// Encode a 16-bit SPI write word from gain and channel.
pub const fn encode_write_frame(gain: Pga113Gain, channel: Pga113Channel) -> u16 {
    let data_byte = (gain as u8) | (channel as u8);
    ((opcode::WRITE as u16) << 8) | (data_byte as u16)
}

/// Convert 16-bit frame into 2-byte SPI array (MSB first).
pub const fn frame_to_bytes(frame: u16) -> [u8; 2] {
    [(frame >> 8) as u8, (frame & 0xFF) as u8]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gain_conversions() {
        assert_eq!(Pga113Gain::from_multiplier(1), Some(Pga113Gain::Gain1x));
        assert_eq!(Pga113Gain::from_multiplier(50), Some(Pga113Gain::Gain50x));
        assert_eq!(Pga113Gain::from_multiplier(200), Some(Pga113Gain::Gain200x));
        assert_eq!(Pga113Gain::from_multiplier(3), None);

        assert_eq!(Pga113Gain::Gain100x.multiplier(), 100);
    }

    #[test]
    fn test_frame_encoding() {
        // Gain 1x (0x00), CH1 (0x01) -> Write opcode 0x2A -> Frame 0x2A01
        let frame = encode_write_frame(Pga113Gain::Gain1x, Pga113Channel::Ch1);
        assert_eq!(frame, 0x2A01);
        assert_eq!(frame_to_bytes(frame), [0x2A, 0x01]);

        // Gain 5x (0x20), VCAL (0x00) -> Frame 0x2A20
        let frame_vcal = encode_write_frame(Pga113Gain::Gain5x, Pga113Channel::Vcal);
        assert_eq!(frame_vcal, 0x2A20);

        // Shutdown frame
        assert_eq!(Pga113Config::encode_shutdown(), 0xE100);
    }

    #[test]
    fn test_vcal_calibration() {
        let mut cfg = Pga113Config::new().with_channel(Pga113Channel::Vcal);
        // At 3000 mV VDD, 0.05 * 3000 = 150 mV
        assert_eq!(cfg.expected_vcal_mv(3000), 150);

        // At Gain 2x, expected = 300 mV
        cfg.gain = Pga113Gain::Gain2x;
        assert_eq!(cfg.expected_vcal_mv(3000), 300);

        // Measured 303 mV at expected 300 mV -> +10 ppt (+1%)
        assert_eq!(cfg.calculate_gain_error_ppt(303, 3000), 10);
    }
}
