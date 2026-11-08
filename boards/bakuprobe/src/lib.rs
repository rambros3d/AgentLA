//! BakuProbe Board Support Package (RP2350B, 48 GPIOs, PGA113 front-end, INA180 current sense).

#![no_std]

use agentla_bsp::{
    AdapterPins, AnalogFrontendConfig, BoardFeatures, BoardProfile, CanPins, FlexiblePool,
    I2cPins, McuPackage, ProgrammerPins, Rs485Pins, SpiPins, UartPins,
};

pub const MANIFEST_JSON: &str = include_str!("../board.json");
pub const LOGO_SVG: &[u8] = include_bytes!("../logo.svg");
pub const PINOUT_SVG: &[u8] = include_bytes!("../pinout.svg");

/// BakuProbe instrument board profile.
pub struct BakuProbeBoard;

impl BakuProbeBoard {
    pub const fn manifest_json() -> &'static str {
        MANIFEST_JSON
    }
}

impl BoardProfile for BakuProbeBoard {
    const BOARD_ID: &'static str = "bakuprobe";
    const NAME: &'static str = "BakuProbe";
    const VENDOR: &'static str = "AgentLA";
    const VERSION: &'static str = "1.0.0";
    const MCU_PACKAGE: McuPackage = McuPackage::Qfn80;
    const GPIO_COUNT: u32 = 48;

    const PROGRAMMER: ProgrammerPins = ProgrammerPins {
        swclk: 14,
        swdio: 15,
        nrst: 18,
        boot0: 30,
        sdi: 15,
        uart_tx: 12,
        uart_rx: 13,
        trst: None,
        tdo: Some(17),
        tdi: Some(16),
    };

    const ADAPTERS: AdapterPins = AdapterPins {
        can: Some(CanPins {
            tx: 19,
            rx: 22,
            term: None,
        }),
        rs485: Some(Rs485Pins {
            tx: 20,
            rx: 21,
            de_re: 29,
        }),
        default_i2c: I2cPins { sda: 26, scl: 27 },
        default_spi: SpiPins {
            sck: 30,
            mosi: 31,
            miso: 28,
            cs: 29,
        },
        default_uart: UartPins { tx: 12, rx: 13 },
        default_ws2812: Some(6),
    };

    const FLEXIBLE: FlexiblePool = FlexiblePool::new(32, 39);

    const FEATURES: BoardFeatures = BoardFeatures {
        onboard_led: None,
        vref_volts: 3.0,
        has_hardware_can: true,
        has_hardware_rs485: true,
        target_current_sense: true,
        target_power_control: true,
        qwiic_stemma_qt: true,
    };

    fn analog_frontend() -> AnalogFrontendConfig {
        AnalogFrontendConfig::Direct {
            adc_pins: &[40, 41, 42, 43, 44, 45, 46, 47],
            adc_channels: &[0, 1, 2, 3, 4, 5, 6, 7],
            attenuation: 1,
            dual_personality: true,
        }
    }

    fn manifest_json() -> &'static str {
        MANIFEST_JSON
    }

    fn logo_svg() -> &'static [u8] {
        LOGO_SVG
    }

    fn pinout_svg() -> &'static [u8] {
        PINOUT_SVG
    }
}
