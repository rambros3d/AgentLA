//! Real-time Hardware Trigger Matrix for protocol-qualified actions.

use super::event::{DecodedEvent, ProtocolKind};

pub const MAX_RULES: usize = 8;

#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct TriggerActionOutput {
    pub rule_id: u8,
    pub freeze_capture: bool,
    pub assert_pin: Option<u8>,
    pub halt_swd: bool,
    pub ws_alert: bool,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct HardwareTriggerRule {
    pub rule_id: u8,
    pub enabled: bool,
    pub bus: ProtocolKind,
    pub match_addr: u16,
    pub match_addr_mask: u16,
    pub match_ack: Option<bool>,
    pub match_id: u32,
    pub match_id_mask: u32,
    pub match_data_byte0: Option<u8>,
    pub match_error: bool,
    pub action_freeze_capture: bool,
    pub action_assert_pin: Option<u8>,
    pub action_halt_swd: bool,
    pub action_ws_alert: bool,
}

impl Default for HardwareTriggerRule {
    fn default() -> Self {
        Self {
            rule_id: 0,
            enabled: false,
            bus: ProtocolKind::I2c,
            match_addr: 0,
            match_addr_mask: 0,
            match_ack: None,
            match_id: 0,
            match_id_mask: 0,
            match_data_byte0: None,
            match_error: false,
            action_freeze_capture: true,
            action_assert_pin: None,
            action_halt_swd: false,
            action_ws_alert: true,
        }
    }
}

pub struct HardwareTriggerMatrix {
    pub rules: [HardwareTriggerRule; MAX_RULES],
}

impl HardwareTriggerMatrix {
    pub const fn new() -> Self {
        Self {
            rules: [
                HardwareTriggerRule {
                    rule_id: 0,
                    enabled: false,
                    bus: ProtocolKind::I2c,
                    match_addr: 0,
                    match_addr_mask: 0,
                    match_ack: None,
                    match_id: 0,
                    match_id_mask: 0,
                    match_data_byte0: None,
                    match_error: false,
                    action_freeze_capture: false,
                    action_assert_pin: None,
                    action_halt_swd: false,
                    action_ws_alert: false,
                },
                HardwareTriggerRule {
                    rule_id: 1,
                    enabled: false,
                    bus: ProtocolKind::I2c,
                    match_addr: 0,
                    match_addr_mask: 0,
                    match_ack: None,
                    match_id: 0,
                    match_id_mask: 0,
                    match_data_byte0: None,
                    match_error: false,
                    action_freeze_capture: false,
                    action_assert_pin: None,
                    action_halt_swd: false,
                    action_ws_alert: false,
                },
                HardwareTriggerRule {
                    rule_id: 2,
                    enabled: false,
                    bus: ProtocolKind::I2c,
                    match_addr: 0,
                    match_addr_mask: 0,
                    match_ack: None,
                    match_id: 0,
                    match_id_mask: 0,
                    match_data_byte0: None,
                    match_error: false,
                    action_freeze_capture: false,
                    action_assert_pin: None,
                    action_halt_swd: false,
                    action_ws_alert: false,
                },
                HardwareTriggerRule {
                    rule_id: 3,
                    enabled: false,
                    bus: ProtocolKind::I2c,
                    match_addr: 0,
                    match_addr_mask: 0,
                    match_ack: None,
                    match_id: 0,
                    match_id_mask: 0,
                    match_data_byte0: None,
                    match_error: false,
                    action_freeze_capture: false,
                    action_assert_pin: None,
                    action_halt_swd: false,
                    action_ws_alert: false,
                },
                HardwareTriggerRule {
                    rule_id: 4,
                    enabled: false,
                    bus: ProtocolKind::I2c,
                    match_addr: 0,
                    match_addr_mask: 0,
                    match_ack: None,
                    match_id: 0,
                    match_id_mask: 0,
                    match_data_byte0: None,
                    match_error: false,
                    action_freeze_capture: false,
                    action_assert_pin: None,
                    action_halt_swd: false,
                    action_ws_alert: false,
                },
                HardwareTriggerRule {
                    rule_id: 5,
                    enabled: false,
                    bus: ProtocolKind::I2c,
                    match_addr: 0,
                    match_addr_mask: 0,
                    match_ack: None,
                    match_id: 0,
                    match_id_mask: 0,
                    match_data_byte0: None,
                    match_error: false,
                    action_freeze_capture: false,
                    action_assert_pin: None,
                    action_halt_swd: false,
                    action_ws_alert: false,
                },
                HardwareTriggerRule {
                    rule_id: 6,
                    enabled: false,
                    bus: ProtocolKind::I2c,
                    match_addr: 0,
                    match_addr_mask: 0,
                    match_ack: None,
                    match_id: 0,
                    match_id_mask: 0,
                    match_data_byte0: None,
                    match_error: false,
                    action_freeze_capture: false,
                    action_assert_pin: None,
                    action_halt_swd: false,
                    action_ws_alert: false,
                },
                HardwareTriggerRule {
                    rule_id: 7,
                    enabled: false,
                    bus: ProtocolKind::I2c,
                    match_addr: 0,
                    match_addr_mask: 0,
                    match_ack: None,
                    match_id: 0,
                    match_id_mask: 0,
                    match_data_byte0: None,
                    match_error: false,
                    action_freeze_capture: false,
                    action_assert_pin: None,
                    action_halt_swd: false,
                    action_ws_alert: false,
                },
            ],
        }
    }

    pub fn install_rule(&mut self, rule: HardwareTriggerRule) -> Result<(), ()> {
        let idx = rule.rule_id as usize;
        if idx < MAX_RULES {
            self.rules[idx] = rule;
            Ok(())
        } else {
            Err(())
        }
    }

    pub fn clear_rule(&mut self, rule_id: u8) {
        let idx = rule_id as usize;
        if idx < MAX_RULES {
            self.rules[idx].enabled = false;
        }
    }

    pub fn clear_all(&mut self) {
        for rule in &mut self.rules {
            rule.enabled = false;
        }
    }

    /// Evaluates an incoming DecodedEvent against all enabled rules.
    /// Returns the combined action output for the first matching rule, if any.
    pub fn evaluate(&self, event: &DecodedEvent) -> Option<TriggerActionOutput> {
        for rule in &self.rules {
            if !rule.enabled {
                continue;
            }

            let matches = match *event {
                DecodedEvent::UartByte { byte, parity_err, framing_err, .. } => {
                    if rule.bus != ProtocolKind::Uart {
                        false
                    } else if rule.match_error && (parity_err || framing_err) {
                        true
                    } else if let Some(target_b) = rule.match_data_byte0 {
                        byte == target_b
                    } else {
                        false
                    }
                }
                DecodedEvent::I2cTransaction { addr, ack, data, data_len, .. } => {
                    if rule.bus != ProtocolKind::I2c {
                        false
                    } else {
                        let addr_match = if rule.match_addr_mask != 0 {
                            (addr & rule.match_addr_mask) == (rule.match_addr & rule.match_addr_mask)
                        } else {
                            addr == rule.match_addr
                        };

                        let ack_match = match rule.match_ack {
                            Some(req_ack) => ack == req_ack,
                            None => true,
                        };

                        let data_match = match rule.match_data_byte0 {
                            Some(b) => data_len > 0 && data[0] == b,
                            None => true,
                        };

                        addr_match && ack_match && data_match
                    }
                }
                DecodedEvent::SpiTransfer { mosi, miso, len, .. } => {
                    if rule.bus != ProtocolKind::Spi {
                        false
                    } else {
                        match rule.match_data_byte0 {
                            Some(b) => len > 0 && (mosi[0] == b || miso[0] == b),
                            None => true,
                        }
                    }
                }
                DecodedEvent::CanFrame { id, rtr: _, dlc, data, crc_ok, .. } => {
                    if rule.bus != ProtocolKind::Can {
                        false
                    } else if rule.match_error && !crc_ok {
                        true
                    } else {
                        let id_match = if rule.match_id_mask != 0 {
                            (id & rule.match_id_mask) == (rule.match_id & rule.match_id_mask)
                        } else {
                            id == rule.match_id
                        };

                        let data_match = match rule.match_data_byte0 {
                            Some(b) => dlc > 0 && data[0] == b,
                            None => true,
                        };

                        id_match && data_match
                    }
                }
                DecodedEvent::Error { kind, .. } => {
                    rule.match_error && rule.bus == kind
                }
            };

            if matches {
                return Some(TriggerActionOutput {
                    rule_id: rule.rule_id,
                    freeze_capture: rule.action_freeze_capture,
                    assert_pin: rule.action_assert_pin,
                    halt_swd: rule.action_halt_swd,
                    ws_alert: rule.action_ws_alert,
                });
            }
        }

        None
    }
}
