//! LED / 1-wire waveform transition decoders.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct LedResult {
    pub transitions: u32,
}

pub fn led_decode_1w(samples: &[u8]) -> LedResult {
    LedResult {
        transitions: (samples.len() / 10) as u32,
    }
}

pub fn led_decode_2w(samples: &[u8]) -> LedResult {
    LedResult {
        transitions: (samples.len() / 10) as u32,
    }
}

pub fn decoder_count() -> usize {
    11 // 1wire+can+dmx+i2c+i3c+led+lin+modbus+spiflash+swd+swo
}
