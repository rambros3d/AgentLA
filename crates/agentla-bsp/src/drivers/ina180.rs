//! Helper for Texas Instruments INA180 Current Sense Amplifier.
//!
//! The INA180 amplifies the differential voltage across a shunt resistor:
//! VOUT = I_LOAD * R_SHUNT * Gain.
//! Available gain options: A1 (20 V/V), A2 (50 V/V), A3 (100 V/V), A4 (200 V/V).

/// INA180 fixed voltage gain options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Ina180Gain {
    /// INA180A1: 20 V/V
    A1_20x = 20,
    /// INA180A2: 50 V/V
    A2_50x = 50,
    /// INA180A3: 100 V/V
    A3_100x = 100,
    /// INA180A4: 200 V/V
    A4_200x = 200,
}

impl Ina180Gain {
    pub const fn multiplier(self) -> u32 {
        self as u32
    }
}

/// INA180 sensor model and telemetry calculator.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Ina180 {
    /// Shunt resistor value in milliohms (e.g. 50 for 0.050 Ohm, 10 for 0.010 Ohm).
    pub shunt_milliohms: u32,
    /// Gain multiplier variant.
    pub gain: Ina180Gain,
}

impl Ina180 {
    pub const fn new(shunt_milliohms: u32, gain: Ina180Gain) -> Self {
        Self {
            shunt_milliohms,
            gain,
        }
    }

    /// Calculate load current in microamps (uA) given output voltage in microvolts (uV).
    pub const fn voltage_to_current_ua(&self, vout_uv: u64) -> u32 {
        let denominator = (self.shunt_milliohms as u64) * (self.gain.multiplier() as u64);
        if denominator == 0 {
            return 0;
        }
        // Vout_uv = I_ua * 1e-6 * R_mohm * 1e-3 * Gain * 1e6
        // Vout_uv = I_ua * R_mohm * Gain / 1000
        // I_ua = (Vout_uv * 1000) / (R_mohm * Gain)
        ((vout_uv * 1000) / denominator) as u32
    }

    /// Calculate load current in milliamps (mA) given output voltage in millivolts (mV).
    pub const fn voltage_to_current_ma(&self, vout_mv: u32) -> u32 {
        let denominator = (self.shunt_milliohms as u64) * (self.gain.multiplier() as u64);
        if denominator == 0 {
            return 0;
        }
        // Vout_mv = I_ma * R_mohm * Gain / 1000
        // I_ma = (Vout_mv * 1000) / (R_mohm * Gain)
        (((vout_mv as u64) * 1000) / denominator) as u32
    }

    /// Convert raw ADC counts into load current in milliamps (mA).
    ///
    /// `vref_mv`: Reference voltage of the ADC in millivolts (e.g. 3000 or 3300).
    /// `max_adc`: Full-scale ADC count (e.g. 4095 for 12-bit SAR ADC).
    pub const fn adc_raw_to_current_ma(&self, raw: u16, vref_mv: u32, max_adc: u16) -> u32 {
        if max_adc == 0 {
            return 0;
        }
        let vout_mv = ((raw as u64) * (vref_mv as u64)) / (max_adc as u64);
        self.voltage_to_current_ma(vout_mv as u32)
    }

    /// Convert raw ADC counts into load current in microamps (uA).
    pub const fn adc_raw_to_current_ua(&self, raw: u16, vref_mv: u32, max_adc: u16) -> u32 {
        if max_adc == 0 {
            return 0;
        }
        let vout_uv = ((raw as u64) * (vref_mv as u64) * 1000) / (max_adc as u64);
        self.voltage_to_current_ua(vout_uv)
    }

    /// Calculate maximum measurable current in milliamps (mA) before ADC/amplifier saturation.
    pub const fn max_measurable_current_ma(&self, vref_mv: u32) -> u32 {
        self.voltage_to_current_ma(vref_mv)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_current_calculation() {
        // 50 mOhm shunt with INA180A2 (50x gain)
        // Gain * Rshunt = 50 * 50 = 2500 mOhm = 2.5 V/A = 2.5 mV/mA
        let ina = Ina180::new(50, Ina180Gain::A2_50x);

        // At 100 mA: Vout = 100 mA * 2.5 mV/mA = 250 mV
        assert_eq!(ina.voltage_to_current_ma(250), 100);

        // At 1000 uV (1 mV): I_ua = (1000 * 1000) / 2500 = 400 uA
        assert_eq!(ina.voltage_to_current_ua(1000), 400);

        // With 3.0V (3000 mV) Vref: max current = 3000 / 2.5 = 1200 mA
        assert_eq!(ina.max_measurable_current_ma(3000), 1200);

        // Half scale ADC on 12-bit (raw ~2048 / 4095) with 3000 mV Vref -> ~600 mA
        let ma = ina.adc_raw_to_current_ma(2048, 3000, 4095);
        assert!((ma as i32 - 600).abs() <= 2);
    }
}
