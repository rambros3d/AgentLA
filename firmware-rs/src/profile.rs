//! # Declarative Hardware Profile Manager (LittleFS USER_FS JSON persistence)
//!
//! Replaces legacy raw 64 KB sector-erasing flash slots with declarative JSON
//! configuration files stored under `/profiles/<id>.json` in `USER_FS`.
//! Provides automatic ROM template re-seeding, marker-file boot default resolution,
//! LittleFS atomic transactions, and full hardware state handover (pins & power rails).

use core::fmt::Write as _;
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use heapless::String;
use littlefs2::path::PathBuf;

pub const TEMPLATE_SAFE_STATE: &str = include_str!("../assets/profiles/safe_state.json");
pub const TEMPLATE_LOGIC_ANALYZER: &str = include_str!("../assets/profiles/logic_analyzer.json");
pub const TEMPLATE_CAN_SNIFFER: &str = include_str!("../assets/profiles/can_sniffer.json");
pub const TEMPLATE_UART_PASSTHROUGH: &str = include_str!("../assets/profiles/uart_passthrough.json");

pub const SAFE_STATE_ID: &str = "safe_state";
pub const SAFE_STATE_NAME: &str = "Safe State (Hi-Z)";

#[derive(Clone, Debug)]
pub struct ProfileMetadata {
    pub id: String<32>,
    pub name: String<48>,
    pub description: String<96>,
    pub is_boot: bool,
    pub read_only: bool,
}

#[derive(Debug, PartialEq, Eq)]
pub enum ProfileError {
    NotFound,
    FsError,
    ReadOnly,
    InvalidPayload,
}

static ACTIVE_PROFILE_ID: Mutex<CriticalSectionRawMutex, String<32>> = Mutex::new(String::new());
static ACTIVE_PROFILE_NAME: Mutex<CriticalSectionRawMutex, String<48>> = Mutex::new(String::new());

/// Backwards compatibility stub for MCP tools expecting integer slot ID.
pub fn active_slot() -> u32 {
    0
}

/// Returns current active profile ID.
pub fn active_id() -> String<32> {
    let guard = ACTIVE_PROFILE_ID.try_lock();
    match guard {
        Ok(g) if !g.is_empty() => g.clone(),
        _ => String::try_from(SAFE_STATE_ID).unwrap_or_default(),
    }
}

/// Returns current active profile human-readable name.
pub fn active_name() -> String<48> {
    let guard = ACTIVE_PROFILE_NAME.try_lock();
    match guard {
        Ok(g) if !g.is_empty() => g.clone(),
        _ => String::try_from(SAFE_STATE_NAME).unwrap_or_default(),
    }
}

/// Sets active profile metadata.
pub fn set_active(id: &str, name: &str) {
    if let Ok(mut g_id) = ACTIVE_PROFILE_ID.try_lock() {
        g_id.clear();
        let _ = g_id.push_str(id);
    }
    if let Ok(mut g_nm) = ACTIVE_PROFILE_NAME.try_lock() {
        g_nm.clear();
        let _ = g_nm.push_str(name);
    }
}

/// Resolves boot-default profile ID from `/profiles/boot_default` file.
pub fn boot_id() -> String<32> {
    let mut out = String::<32>::new();
    let _ = crate::storage::with_user_fs(|fs| {
        let Ok(path) = PathBuf::try_from("/profiles/boot_default") else { return; };
        if let Ok(data) = fs.read::<32>(&path) {
            let s = core::str::from_utf8(&data).unwrap_or("").trim();
            let _ = out.push_str(s);
        }
    });
    if out.is_empty() {
        let _ = out.push_str(SAFE_STATE_ID);
    }
    out
}

/// Auto-seeds built-in factory profile templates from ROM if `/profiles` is empty or missing.
pub fn seed_default_templates() {
    let _ = crate::storage::with_user_fs(|fs| {
        let Ok(dir_path) = PathBuf::try_from("/profiles") else { return; };
        let _ = fs.create_dir_all(&dir_path);

        let Ok(safe_path) = PathBuf::try_from("/profiles/safe_state.json") else { return; };
        if fs.metadata(&safe_path).is_err() {
            let _ = fs.write(&safe_path, TEMPLATE_SAFE_STATE.as_bytes());
            if let Ok(p) = PathBuf::try_from("/profiles/logic_analyzer.json") {
                let _ = fs.write(&p, TEMPLATE_LOGIC_ANALYZER.as_bytes());
            }
            if let Ok(p) = PathBuf::try_from("/profiles/can_sniffer.json") {
                let _ = fs.write(&p, TEMPLATE_CAN_SNIFFER.as_bytes());
            }
            if let Ok(p) = PathBuf::try_from("/profiles/uart_passthrough.json") {
                let _ = fs.write(&p, TEMPLATE_UART_PASSTHROUGH.as_bytes());
            }
            defmt::info!("Seeded factory hardware profiles in USER_FS:/profiles");
        }
    });
}

/// Inspects boot configuration on power-up and applies designated boot profile.
pub async fn scan_boot_default() {
    seed_default_templates();
    let b_id = boot_id();
    defmt::info!("Boot profile: {}", b_id.as_str());
    if b_id.as_str() != SAFE_STATE_ID && !b_id.is_empty() {
        let _ = apply_profile(b_id.as_str()).await;
    } else {
        set_active(SAFE_STATE_ID, SAFE_STATE_NAME);
    }
}

/// Iterates profiles stored on LittleFS and yields metadata.
pub fn list_profiles<F: FnMut(&ProfileMetadata)>(mut cb: F) {
    let current_boot = boot_id();
    let _ = crate::storage::with_user_fs(|fs| {
        let Ok(dir_path) = PathBuf::try_from("/profiles") else { return; };
        let _ = fs.read_dir_and_then(&dir_path, |dir| {
            for entry in dir.by_ref() {
                let Ok(entry) = entry else { continue; };
                let fname = entry.file_name().as_str();
                if !fname.ends_with(".json") {
                    continue;
                }
                let id = &fname[..fname.len() - 5];
                if id.is_empty() {
                    continue;
                }

                let mut path_str = String::<64>::new();
                let _ = write!(path_str, "/profiles/{}", fname);
                let Ok(pbuf) = PathBuf::try_from(path_str.as_str()) else { continue; };

                if let Ok(data) = fs.read::<512>(&pbuf) {
                    if let Ok(json_str) = core::str::from_utf8(&data) {
                        let name = find_json_str(json_str, "name").unwrap_or_else(|| String::try_from(id).unwrap_or_default());
                        let desc = find_json_str(json_str, "description").unwrap_or_default();
                        let ro = find_json_bool(json_str, "read_only").unwrap_or(false) || id == SAFE_STATE_ID;
                        let is_boot = id == current_boot.as_str();

                        let meta = ProfileMetadata {
                            id: String::try_from(id).unwrap_or_default(),
                            name: String::try_from(name.as_str()).unwrap_or_default(),
                            description: String::try_from(desc.as_str()).unwrap_or_default(),
                            is_boot,
                            read_only: ro,
                        };
                        cb(&meta);
                    }
                }
            }
            Ok(())
        });
    });
}

/// Reads full JSON document for a profile.
pub fn read_profile_doc(id: &str, buf: &mut [u8]) -> Result<usize, ProfileError> {
    if id == SAFE_STATE_ID {
        let bytes = TEMPLATE_SAFE_STATE.as_bytes();
        if bytes.len() > buf.len() {
            return Err(ProfileError::FsError);
        }
        buf[..bytes.len()].copy_from_slice(bytes);
        return Ok(bytes.len());
    }

    let mut path_str = String::<64>::new();
    let _ = write!(path_str, "/profiles/{}.json", id);
    let Ok(pbuf) = PathBuf::try_from(path_str.as_str()) else {
        return Err(ProfileError::NotFound);
    };

    let data = crate::storage::with_user_fs(|fs| {
        fs.read::<1536>(&pbuf).map_err(|_| ProfileError::NotFound)
    }).ok_or(ProfileError::FsError)??;

    if data.len() > buf.len() {
        return Err(ProfileError::FsError);
    }
    buf[..data.len()].copy_from_slice(&data);
    Ok(data.len())
}

/// Saves or updates a JSON profile document in LittleFS.
pub fn save_profile_doc(id: &str, doc: &[u8]) -> Result<(), ProfileError> {
    if id.is_empty() || id == SAFE_STATE_ID {
        return Err(ProfileError::ReadOnly);
    }

    let mut path_str = String::<64>::new();
    let _ = write!(path_str, "/profiles/{}.json", id);
    let Ok(pbuf) = PathBuf::try_from(path_str.as_str()) else {
        return Err(ProfileError::InvalidPayload);
    };

    crate::storage::with_user_fs(|fs| {
        let Ok(dir_path) = PathBuf::try_from("/profiles") else { return Err(ProfileError::FsError); };
        let _ = fs.create_dir_all(&dir_path);
        fs.write(&pbuf, doc).map_err(|_| ProfileError::FsError)
    }).ok_or(ProfileError::FsError)?
}

/// Deletes a profile document from LittleFS.
pub fn delete_profile_doc(id: &str) -> Result<(), ProfileError> {
    if id == SAFE_STATE_ID {
        return Err(ProfileError::ReadOnly);
    }

    let mut path_str = String::<64>::new();
    let _ = write!(path_str, "/profiles/{}.json", id);
    let Ok(pbuf) = PathBuf::try_from(path_str.as_str()) else {
        return Err(ProfileError::NotFound);
    };

    crate::storage::with_user_fs(|fs| {
        // Check read_only
        if let Ok(data) = fs.read::<256>(&pbuf) {
            if let Ok(json_str) = core::str::from_utf8(&data) {
                if find_json_bool(json_str, "read_only").unwrap_or(false) {
                    return Err(ProfileError::ReadOnly);
                }
            }
        }
        fs.remove(&pbuf).map_err(|_| ProfileError::NotFound)
    }).ok_or(ProfileError::FsError)??;

    // If active or boot profile was deleted, revert to Safe State
    if active_id().as_str() == id {
        set_active(SAFE_STATE_ID, SAFE_STATE_NAME);
    }
    if boot_id().as_str() == id {
        let _ = set_boot_default("");
    }

    Ok(())
}

/// Designates default boot profile in `/profiles/boot_default`.
pub fn set_boot_default(id: &str) -> Result<(), ProfileError> {
    let Ok(pbuf) = PathBuf::try_from("/profiles/boot_default") else {
        return Err(ProfileError::FsError);
    };

    crate::storage::with_user_fs(|fs| {
        if id.is_empty() || id == SAFE_STATE_ID {
            let _ = fs.remove(&pbuf);
            Ok(())
        } else {
            let Ok(dir_path) = PathBuf::try_from("/profiles") else { return Err(ProfileError::FsError); };
            let _ = fs.create_dir_all(&dir_path);
            fs.write(&pbuf, id.as_bytes()).map_err(|_| ProfileError::FsError)
        }
    }).ok_or(ProfileError::FsError)?
}

/// Applies a hardware profile: quiesces pins, discharges rails, configures power and claims pins.
pub async fn apply_profile(id: &str) -> Result<String<96>, ProfileError> {
    // 1. Quiesce all flexible pin claims into safe High-Z
    crate::pin_owner::quiesce_and_reset();

    // 2. Rail discharge settle
    embassy_time::Timer::after(embassy_time::Duration::from_millis(5)).await;

    // Handle Safe State baseline
    if id == SAFE_STATE_ID || id.is_empty() {
        set_active(SAFE_STATE_ID, SAFE_STATE_NAME);
        crate::telemetry::set_overcurrent_limit_ma(500);
        return Ok(String::try_from(SAFE_STATE_NAME).unwrap_or_default());
    }

    // 3. Read profile document
    let mut buf = [0u8; 1536];
    let n = read_profile_doc(id, &mut buf)?;
    let Ok(json_str) = core::str::from_utf8(&buf[..n]) else {
        return Err(ProfileError::InvalidPayload);
    };

    let name = find_json_str(json_str, "name").unwrap_or_else(|| String::try_from(id).unwrap_or_default());

    // 4. Configure power settings if present
    if let Some(limit_ma) = find_json_u32(json_str, "overcurrent_limit_ma") {
        crate::telemetry::set_overcurrent_limit_ma(limit_ma);
    }

    // 5. Apply pin claims if present
    parse_and_claim_pins(json_str);

    set_active(id, name.as_str());
    defmt::info!("Applied hardware profile: {} ({})", id, name.as_str());
    Ok(name)
}

// ---------------------------------------------------------------------------
// Zero-allocation JSON search helpers
// ---------------------------------------------------------------------------

fn find_json_str(json: &str, key: &str) -> Option<String<96>> {
    let mut pattern = String::<64>::new();
    let _ = write!(pattern, "\"{}\"", key);
    let key_pos = json.find(pattern.as_str())?;
    let after_key = &json[key_pos + pattern.len()..];
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();
    if !after_colon.starts_with('"') {
        return None;
    }
    let val_start = &after_colon[1..];
    let quote_end = val_start.find('"')?;
    let val_str = &val_start[..quote_end];

    let mut out = String::<96>::new();
    let _ = out.push_str(val_str);
    Some(out)
}

fn find_json_bool(json: &str, key: &str) -> Option<bool> {
    let mut pattern = String::<64>::new();
    let _ = write!(pattern, "\"{}\"", key);
    let key_pos = json.find(pattern.as_str())?;
    let after_key = &json[key_pos + pattern.len()..];
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();
    if after_colon.starts_with("true") {
        Some(true)
    } else if after_colon.starts_with("false") {
        Some(false)
    } else {
        None
    }
}

fn find_json_u32(json: &str, key: &str) -> Option<u32> {
    let mut pattern = String::<64>::new();
    let _ = write!(pattern, "\"{}\"", key);
    let key_pos = json.find(pattern.as_str())?;
    let after_key = &json[key_pos + pattern.len()..];
    let colon_pos = after_key.find(':')?;
    let after_colon = after_key[colon_pos + 1..].trim_start();

    let mut end = 0;
    for c in after_colon.chars() {
        if c.is_ascii_digit() {
            end += 1;
        } else {
            break;
        }
    }
    if end == 0 {
        return None;
    }
    after_colon[..end].parse::<u32>().ok()
}

fn parse_and_claim_pins(json: &str) {
    let pins_start = match json.find("\"pins\"") {
        Some(p) => p,
        None => return,
    };
    let slice = &json[pins_start..];
    let arr_start = match slice.find('[') {
        Some(a) => a,
        None => return,
    };
    let arr_end = match slice.find(']') {
        Some(e) => e,
        None => return,
    };
    let pins_body = &slice[arr_start + 1..arr_end];

    // Scan for objects `{ ... }`
    for item in pins_body.split('{') {
        let end_obj = match item.find('}') {
            Some(e) => e,
            None => continue,
        };
        let obj = &item[..end_obj];
        let pin = match find_json_u32(obj, "pin") {
            Some(p) if p < 30 => p,
            _ => continue,
        };
        let driver_name = find_json_str(obj, "driver").unwrap_or_default();
        let driver = match driver_name.as_str() {
            "capture" => crate::pin_owner::Driver::Capture,
            "servo" => crate::pin_owner::Driver::Servo,
            "gpio" => crate::pin_owner::Driver::Gpio,
            "ws2812" => crate::pin_owner::Driver::Ws2812,
            "generator" => crate::pin_owner::Driver::Generator,
            "oscilloscope" => crate::pin_owner::Driver::Oscilloscope,
            "glitch" => crate::pin_owner::Driver::Glitch,
            _ => crate::pin_owner::Driver::Adapter,
        };
        let _ = crate::pin_owner::claim(driver, &[pin], 0);
    }
}
