use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

fn main() {
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());

    // Single-slot layout: the application is always linked at XIP 0x10008000.
    // The recovery bootloader owns flash 0x00000000..0x00008000 and SYSTEM_FS
    // starts at 0x00140000; neither may ever be touched by this binary.
    let memory_x = r#"MEMORY {
    FLASH : ORIGIN = 0x10008000, LENGTH = 1248K
    RAM   : ORIGIN = 0x20000000, LENGTH = 520K
}
"#;

    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(memory_x.as_bytes())
        .unwrap();

    // Build marker: a short hash of the working tree plus the dirty flag and a
    // UTC timestamp. Emitted as a Rust constant so FirmwareStatus can report
    // exactly which binary is running. There is only one slot now, so the
    // marker identifies the single application image rather than a slot pair.
    let git_hash = Command::new("git")
        .args(["rev-parse", "--short=8", "HEAD"])
        .output()
        .ok()
        .and_then(|o| {
            if o.status.success() {
                String::from_utf8(o.stdout).ok()
            } else {
                None
            }
        })
        .map(|s| s.trim().to_string())
        .unwrap_or_else(|| "nogit".to_string());

    let dirty = Command::new("git")
        .arg("status")
        .arg("--porcelain")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .map(|s| if s.trim().is_empty() { 0u8 } else { 1u8 })
        .unwrap_or(2u8);

    let stamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as u32)
        .unwrap_or(0);

    let marker = format!("{}-{}-{}", git_hash, if dirty == 0 { "clean" } else { "dirty" }, stamp);
    File::create(out.join("build_marker.rs"))
        .unwrap()
        .write_all(format!("pub const BUILD_MARKER: &str = \"{}\";\n", marker).as_bytes())
        .unwrap();

    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-changed=memory.x");
    // Recompute the marker on every build so a code change always produces a
    // fresh stamp, even when git HEAD has not moved.
    println!("cargo:rerun-if-changed=build.rs");
}
