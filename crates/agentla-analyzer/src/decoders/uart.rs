//! Streaming asynchronous serial (UART) decoder.

use super::event::DecodedEvent;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Parity {
    None,
    Even,
    Odd,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct UartConfig {
    pub rx_pin: u8,
    pub baud_rate: u32,
    pub data_bits: u8,
    pub parity: Parity,
    pub stop_bits: u8,
}

impl Default for UartConfig {
    fn default() -> Self {
        Self {
            rx_pin: 0,
            baud_rate: 115_200,
            data_bits: 8,
            parity: Parity::None,
            stop_bits: 1,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum UartState {
    Idle,
    Sampling {
        next_sample_tick: u32,
        bit_index: u8,
        accum: u16,
    },
}

pub struct UartDecoder {
    pub config: UartConfig,
    ticks_per_bit: u32,
    clock_hz: u32,
    state: UartState,
    last_pin_state: bool,
}

impl UartDecoder {
    pub fn new(config: UartConfig, clock_hz: u32) -> Self {
        let ticks_per_bit = if config.baud_rate > 0 {
            clock_hz / config.baud_rate
        } else {
            clock_hz / 115_200
        };

        Self {
            config,
            ticks_per_bit,
            clock_hz,
            state: UartState::Idle,
            last_pin_state: true,
        }
    }

    pub fn set_baud_rate(&mut self, baud: u32) {
        if baud > 0 {
            self.config.baud_rate = baud;
            self.ticks_per_bit = self.clock_hz / baud;
        }
    }

    pub fn reset(&mut self) {
        self.state = UartState::Idle;
        self.last_pin_state = true;
    }

    pub fn process_transition(
        &mut self,
        _timestamp_ticks: u32,
        pin_states: u32,
    ) -> Option<DecodedEvent> {
        let rx = ((pin_states >> self.config.rx_pin) & 1) != 0;
        self.last_pin_state = rx;
        None
    }

    /// Evaluates clock ticks and samples bits at the center of each bit interval.
    pub fn step(&mut self, current_ticks: u32, pin_states: u32) -> Option<DecodedEvent> {
        let rx = ((pin_states >> self.config.rx_pin) & 1) != 0;
        self.last_pin_state = rx;

        match self.state {
            UartState::Idle => {
                // Falling edge marks the start bit.
                // Target next sample at center of Bit 0: 1.5 * ticks_per_bit
                if !rx {
                    self.state = UartState::Sampling {
                        next_sample_tick: current_ticks + (self.ticks_per_bit * 3 / 2),
                        bit_index: 0,
                        accum: 0,
                    };
                }
                None
            }
            UartState::Sampling { next_sample_tick, bit_index, accum } => {
                if current_ticks >= next_sample_tick {
                    let bit_val = if rx { 1u16 } else { 0u16 };
                    let new_accum = accum | (bit_val << bit_index);
                    let next_index = bit_index + 1;

                    if next_index < self.config.data_bits {
                        // Advance to next data bit
                        self.state = UartState::Sampling {
                            next_sample_tick: next_sample_tick + self.ticks_per_bit,
                            bit_index: next_index,
                            accum: new_accum,
                        };
                        None
                    } else if self.config.parity != Parity::None && next_index == self.config.data_bits {
                        // Sample parity bit next
                        self.state = UartState::Sampling {
                            next_sample_tick: next_sample_tick + self.ticks_per_bit,
                            bit_index: next_index,
                            accum: new_accum,
                        };
                        None
                    } else if next_index == self.config.data_bits {
                        // Advance to Stop bit sample interval
                        self.state = UartState::Sampling {
                            next_sample_tick: next_sample_tick + self.ticks_per_bit,
                            bit_index: next_index + 1,
                            accum: new_accum,
                        };
                        None
                    } else {
                        // Sample Stop bit
                        let framing_err = !rx; // Stop bit must be 1 (high)
                        let parity_err = match self.config.parity {
                            Parity::None => false,
                            Parity::Even => (accum.count_ones() % 2) != 0,
                            Parity::Odd => (accum.count_ones() % 2) == 0,
                        };

                        self.state = UartState::Idle;
                        let timestamp_us = current_ticks / (self.clock_hz / 1_000_000).max(1);
                        Some(DecodedEvent::UartByte {
                            timestamp_us,
                            byte: accum as u8,
                            parity_err,
                            framing_err,
                        })
                    }
                } else {
                    None
                }
            }
        }
    }
}
