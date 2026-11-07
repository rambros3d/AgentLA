#![no_std]

#[cfg(feature = "std")]
extern crate std;

pub mod cascade;
pub mod catalog;
pub mod engines;
pub mod loaders;
pub mod traits;
pub mod types;

pub use cascade::{flash_cascade_execute, unbrick_cascade_execute, CascadeResult, ExecutionTier};
pub use catalog::{lookup_by_dev_id, lookup_by_name, match_part_number, TARGET_CATALOG};
pub use engines::FlashEngineRunner;
pub use loaders::{find_embedded_loader, AlgoBlobHeader, EmbeddedLoader, EMBEDDED_LOADERS};
pub use traits::TargetMemAccess;
pub use types::{DirectEngineKind, SiliconFamily, TargetDescriptor, TargetError, TargetProtection};
