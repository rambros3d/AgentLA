//! Target catalog query and lookup functions.

pub mod families;
pub mod radix;

pub use families::TARGET_CATALOG;
pub use radix::match_part_number;

use crate::types::TargetDescriptor;

/// Lookup a target descriptor from hardware device ID (e.g. Cortex-M DBGMCU_IDCODE or RISC-V ID).
pub fn lookup_by_dev_id(dev_id: u16) -> Option<&'static TargetDescriptor> {
    for desc in TARGET_CATALOG {
        if desc.matches_dev_id(dev_id) {
            return Some(desc);
        }
    }
    None
}

/// Lookup a target descriptor by exact or prefix part number string.
pub fn lookup_by_name(name: &str) -> Option<&'static TargetDescriptor> {
    // 1. Try exact match against catalog name
    for desc in TARGET_CATALOG {
        if desc.name.eq_ignore_ascii_case(name) {
            return Some(desc);
        }
    }
    // 2. Try prefix radix matcher for commercial part numbers
    match_part_number(name)
}
