//! Compressed radix prefix/pattern matcher for commercial part numbers.

use super::families::TARGET_CATALOG;
use crate::types::TargetDescriptor;

struct PrefixRule {
    prefix: &'static str,
    target_idx: usize,
}

static PREFIX_RULES: &[PrefixRule] = &[
    // STM32
    PrefixRule { prefix: "STM32F103", target_idx: 0 },
    PrefixRule { prefix: "STM32F101", target_idx: 1 },
    PrefixRule { prefix: "STM32F102", target_idx: 1 },
    PrefixRule { prefix: "STM32F105", target_idx: 3 },
    PrefixRule { prefix: "STM32F107", target_idx: 3 },
    PrefixRule { prefix: "STM32F303", target_idx: 4 },
    PrefixRule { prefix: "GD32F103", target_idx: 5 },
    PrefixRule { prefix: "AT32F403", target_idx: 6 },
    PrefixRule { prefix: "STM32F401", target_idx: 7 },
    PrefixRule { prefix: "STM32F411", target_idx: 7 },
    PrefixRule { prefix: "STM32F405", target_idx: 8 },
    PrefixRule { prefix: "STM32F407", target_idx: 8 },
    PrefixRule { prefix: "STM32F746", target_idx: 9 },
    PrefixRule { prefix: "STM32F767", target_idx: 9 },
    PrefixRule { prefix: "STM32G030", target_idx: 10 },
    PrefixRule { prefix: "STM32G031", target_idx: 10 },
    PrefixRule { prefix: "STM32G431", target_idx: 11 },
    PrefixRule { prefix: "STM32G474", target_idx: 11 },
    PrefixRule { prefix: "STM32L431", target_idx: 12 },
    PrefixRule { prefix: "STM32L476", target_idx: 12 },
    PrefixRule { prefix: "STM32C011", target_idx: 13 },
    PrefixRule { prefix: "STM32C031", target_idx: 13 },
    PrefixRule { prefix: "STM32H503", target_idx: 14 },
    PrefixRule { prefix: "STM32H563", target_idx: 14 },
    PrefixRule { prefix: "STM32H743", target_idx: 15 },
    PrefixRule { prefix: "STM32H753", target_idx: 15 },
    PrefixRule { prefix: "PY32F002", target_idx: 16 },
    PrefixRule { prefix: "PY32F003", target_idx: 16 },
    PrefixRule { prefix: "PY32F030", target_idx: 16 },
    // Nordic
    PrefixRule { prefix: "NRF52832", target_idx: 17 },
    PrefixRule { prefix: "NRF52840", target_idx: 18 },
    PrefixRule { prefix: "NRF5340",  target_idx: 19 },
    // Microchip
    PrefixRule { prefix: "ATSAMD21", target_idx: 20 },
    PrefixRule { prefix: "SAMD21",   target_idx: 20 },
    PrefixRule { prefix: "ATSAME54", target_idx: 21 },
    PrefixRule { prefix: "SAME54",   target_idx: 21 },
    // NXP
    PrefixRule { prefix: "LPC804",   target_idx: 22 },
    PrefixRule { prefix: "KL25",     target_idx: 23 },
    PrefixRule { prefix: "KL26",     target_idx: 23 },
    // TI
    PrefixRule { prefix: "MSPM0",    target_idx: 24 },
    // Silicon Labs
    PrefixRule { prefix: "EFM32",    target_idx: 25 },
    // Renesas
    PrefixRule { prefix: "RA4M1",    target_idx: 26 },
    // Raspberry Pi
    PrefixRule { prefix: "RP2040",   target_idx: 27 },
    PrefixRule { prefix: "RP2350",   target_idx: 28 },
    // RISC-V
    PrefixRule { prefix: "CH32V003", target_idx: 29 },
    PrefixRule { prefix: "CH32V203", target_idx: 30 },
    PrefixRule { prefix: "CH582",    target_idx: 31 },
    PrefixRule { prefix: "CH592",    target_idx: 31 },
    PrefixRule { prefix: "ESP32-C3", target_idx: 32 },
    PrefixRule { prefix: "GD32VF103", target_idx: 33 },
];

/// Match a commercial part number string using case-insensitive prefix matching.
pub fn match_part_number(part: &str) -> Option<&'static TargetDescriptor> {
    for rule in PREFIX_RULES {
        if starts_with_ignore_case(part, rule.prefix) {
            return TARGET_CATALOG.get(rule.target_idx);
        }
    }
    None
}

fn starts_with_ignore_case(s: &str, prefix: &str) -> bool {
    let s_bytes = s.as_bytes();
    let p_bytes = prefix.as_bytes();
    if s_bytes.len() < p_bytes.len() {
        return false;
    }
    for i in 0..p_bytes.len() {
        let b1 = s_bytes[i].to_ascii_uppercase();
        let b2 = p_bytes[i].to_ascii_uppercase();
        if b1 != b2 {
            return false;
        }
    }
    true
}
