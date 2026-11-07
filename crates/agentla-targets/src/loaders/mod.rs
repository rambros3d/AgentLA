//! Embedded and dynamic RAM algorithm loaders.

pub mod blobs;

pub use blobs::{
    AlgoBlobHeader, EmbeddedLoader, ALGO_MAGIC, ALGO_VERSION, EMBEDDED_LOADERS,
};

/// Lookup an embedded RAM loader for a given target name.
pub fn find_embedded_loader(target_name: &str) -> Option<&'static EmbeddedLoader> {
    for loader in EMBEDDED_LOADERS {
        if loader.target_name.eq_ignore_ascii_case(target_name) {
            return Some(loader);
        }
    }
    None
}
