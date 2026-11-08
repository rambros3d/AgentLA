use std::env;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() {
    let out = &PathBuf::from(env::var_os("OUT_DIR").unwrap());

    let is_slot_b = env::var("CARGO_FEATURE_SLOT_B").is_ok();
    let flash_origin = if is_slot_b {
        "0x10104000"
    } else {
        "0x10004000"
    };

    let memory_x = format!(
r#"MEMORY {{
    FLASH : ORIGIN = {flash_origin}, LENGTH = 1024K
    RAM   : ORIGIN = 0x20000000, LENGTH = 520K
}}
"#);

    File::create(out.join("memory.x"))
        .unwrap()
        .write_all(memory_x.as_bytes())
        .unwrap();
    println!("cargo:rustc-link-search={}", out.display());
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_SLOT_B");
    println!("cargo:rerun-if-changed=memory.x");
}
