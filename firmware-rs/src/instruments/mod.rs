//! # Mixed-Signal Instruments Suite (`analog-and-mixed-signal-instruments`)
//!
//! Subsystems:
//! - [`oscilloscope`]: RP2350 SAR ADC 4-channel DMA capture & triggering engine
//! - [`generator`]: Hardware PWM & PIO clock/signal generator (1 Hz – 75 MHz & bursts)
//! - [`ws2812`]: WS2812B/SK6812 PIO pattern transmitter & timing margin analyzer
//! - [`glitch`]: Cycle-accurate 150 MHz nanosecond fault & clock glitch injector

pub mod oscilloscope;
pub mod generator;
pub mod ws2812;
pub mod glitch;
