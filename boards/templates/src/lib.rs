//! Boilerplate template for custom carrier boards implementing [`BoardProfile`].

#![no_std]

use agentla_bsp::{
    AdapterPins, AnalogFrontendConfig, BoardFeatures, BoardProfile, FlexiblePool, I2cPins,
    McuPackage, ProgrammerPins, SpiPins, UartPins,
};

pub const MANIFEST_JSON: &str = include_str!("../template_board.json");

/// Custom hardware board profile template.
pub struct CustomBoard;

impl CustomBoard {
    pub const fn manifest_json() -> &'static str {
        MANIFEST_JSON
    }
}

impl BoardProfile for CustomBoard {
    const BOARD_ID: &'static str = "custom_board";
    const NAME: &'static str = "Custom Hardware Probe";
    const VENDOR: &'static str = "Custom Vendor";
    const VERSION: &'static str = "1.0.0";
    const MCU_PACKAGE: McuPackage = McuPackage::Qfn60;
    const GPIO_COUNT: u32 = 30;

    const PROGRAMMER: ProgrammerPins = ProgrammerPins {
        swclk: 4,
        swdio: 5,
        nrst: 6,
        boot0: 2,
        sdi: 7,
        uart_tx: 8,
        uart_rx: 9,
        trst: None,
        tdo: None,
        tdi: None,
    };

    const ADAPTERS: AdapterPins = AdapterPins {
        can: None,
        rs485: None,
        default_i2c: I2cPins { sda: 16, scl: 17 },
        default_spi: SpiPins {
            sck: 18,
            mosi: 19,
            miso: 16,
            cs: 17,
        },
        default_uart: UartPins { tx: 8, rx: 14 },
        default_ws2812: None,
    };

    const FLEXIBLE: FlexiblePool = FlexiblePool::new(0, 29);

    const FEATURES: BoardFeatures = BoardFeatures {
        onboard_led: Some(25),
        vref_volts: 3.3,
        has_hardware_can: false,
        has_hardware_rs485: false,
        target_current_sense: false,
        target_power_control: false,
        qwiic_stemma_qt: false,
    };

    fn analog_frontend() -> AnalogFrontendConfig {
        AnalogFrontendConfig::Direct {
            adc_pins: &[26, 27, 28, 29],
            adc_channels: &[0, 1, 2, 3],
            attenuation: 1,
            dual_personality: true,
        }
    }

    fn manifest_json() -> &'static str {
        MANIFEST_JSON
    }
}
