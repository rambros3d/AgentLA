//! REST handlers for the peripherals arbiter.

use core::fmt::Write as _;

use picoserve::io::Read;
use picoserve::request::Request;
use picoserve::response::{ResponseWriter, StatusCode};
use picoserve::routing::MethodHandlerService;
use picoserve::response::IntoResponse;

use crate::pin_owner;
use crate::profile;
use crate::servo;
pub use crate::fs_http::{FsEntry, FsList, FsMkdir, FsRead, FsRename, FsStat, FsWrite};

// Minimal JSON/query helpers (no serde in no_std)
pub fn query_val(q: Option<picoserve::url_encoded::UrlEncodedString>, key: &str) -> Option<heapless::String<64>> {
    let q = q?;
    let s = q.0;
    for pair in s.split('&') {
        if let Some((k, v)) = pair.split_once('=') {
            if k == key && !v.is_empty() {
                let mut o = heapless::String::<64>::new();
                let _ = o.push_str(v);
                return Some(o);
            }
        }
    }
    None
}
fn key_int(q: Option<picoserve::url_encoded::UrlEncodedString>, body: &str, key: &str) -> Option<i32> {
    if let Some(v)=query_val(q,key){ return v.parse::<i32>().ok(); }
    let mut pat = heapless::String::<64>::new();
    let _ = pat.push('"'); let _ = pat.push_str(key); let _ = pat.push('"');
    let pat_str = pat.as_str();
    if let Some(i)=body.find(pat_str) {
        let b=&body[i+pat_str.len()..];
        if let Some(colon)=b.find(':') {
            let tail=b[colon+1..].trim_start();
            let digits: heapless::String<32> = tail.chars().take_while(|c| c.is_ascii_digit()||*c=='-').collect();
            if !digits.is_empty() { return digits.parse::<i32>().ok(); }
        }
    }
    None
}
fn key_u32(q: Option<picoserve::url_encoded::UrlEncodedString>, body: &str, key: &str) -> Option<u32> {
    if let Some(v) = key_str(q, body, key) {
        let s = v.as_str().trim();
        if let Some(hex) = s.strip_prefix("0x").or_else(|| s.strip_prefix("0X")) {
            return u32::from_str_radix(hex, 16).ok();
        } else {
            return s.parse::<u32>().ok();
        }
    }
    if let Some(i) = key_int(q, body, key) {
        if i >= 0 {
            return Some(i as u32);
        }
    }
    None
}
fn key_str(q: Option<picoserve::url_encoded::UrlEncodedString>, body: &str, key: &str) -> Option<heapless::String<64>> {
    if let Some(v)=query_val(q,key){ return Some(v); }
    let mut pat = heapless::String::<64>::new();
    let _ = pat.push('"'); let _ = pat.push_str(key); let _ = pat.push('"');
    let pat_str = pat.as_str();
    if let Some(i)=body.find(pat_str) {
        let b=&body[i+pat_str.len()..];
        if let Some(colon)=b.find(':') {
            let tail=b[colon+1..].trim_start();
            if tail.starts_with('"') {
                if let Some(end)=tail[1..].find('"') {
                    let mut o=heapless::String::<64>::new();
                    let _=o.push_str(&tail[1..1+end]);
                    return Some(o);
                }
            }
        }
    }
    None
}
fn parse_bool(s: &str) -> bool { s=="true"||s=="1" }
async fn read_body_str<R: Read>(request: &mut Request<'_, R>) -> heapless::String<512> {
    let mut reader = request.body_connection.body().reader();
    let mut buf = [0u8; 128];
    let mut out = heapless::String::<512>::new();
    loop {
        match reader.read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => {
                if let Ok(s) = core::str::from_utf8(&buf[..n]) {
                    let _ = out.push_str(s);
                }
            }
            Err(_) => break,
        }
    }
    out
}
async fn send_json<R: Read, W: ResponseWriter<Error=R::Error>>(mut request: Request<'_,R>, rw: W, code: StatusCode, body: &str) -> Result<picoserve::ResponseSent, W::Error> {
    let finalizer = request.body_connection.finalize().await?;
    (code, ("Content-Type","application/json"), body).write_to(finalizer, rw).await
}

// pin_owner query-arg parsing roles
fn owner_cap(name: &str) -> Option<u32> {
    match name {
        "programmer" => Some(pin_owner::CAP_SIO),
        "servo" => Some(pin_owner::CAP_PWM),
        "adapter" => Some(pin_owner::CAP_I2C),
        "capture" => Some(pin_owner::CAP_PIO),
        "gpio" | "sio" => Some(pin_owner::CAP_SIO),
        "spi" => Some(pin_owner::CAP_SPI),
        "uart" => Some(pin_owner::CAP_UART),
        "i2c" => Some(pin_owner::CAP_I2C),
        "pwm" => Some(pin_owner::CAP_PWM),
        "pio0" | "pio1" | "pio2" => Some(pin_owner::CAP_PIO),
        "oscilloscope" | "scope" => Some(pin_owner::CAP_ADC),
        "generator" => Some(pin_owner::CAP_PWM),
        "ws2812" => Some(pin_owner::CAP_PIO),
        "glitch" => Some(pin_owner::CAP_PIO),
        _ => None,
    }
}

// Required top-level structs wired in web.rs:
pub struct ProfilesList;
pub struct ProfilesApply;
pub struct ProfilesBoot;
pub struct ProfilesSave;
pub struct ProfilesDelete;
pub struct PinsClaim;
pub struct PinsRelease;
pub struct ServoList;
pub struct ServoAttach;
pub struct ServoWrite;
pub struct ServoWriteUs;
pub struct ServoDetach;
pub struct PwmList;
pub struct PwmClaim;
pub struct PwmSet;
pub struct PwmRelease;
pub struct LogicAnalyzerArm;
pub struct LogicAnalyzerStart;
pub struct LogicAnalyzerStop;
pub struct LogicAnalyzerStatus;
pub struct LogicAnalyzerConfig;
pub struct LogicAnalyzerData;
pub struct LogicAnalyzerVcd;
pub struct LogicAnalyzerDecodersConfigure;
pub struct LogicAnalyzerTriggersConfigure;
pub struct DecodedStreamWebSocket;
pub struct ProgrammerTarget;
pub struct ProgrammerReset;
pub struct ProgrammerResetCatch;
pub struct ProgrammerRescueStm32;
pub struct ProgrammerRescueNrf;
pub struct ProgrammerErase;
pub struct ProgrammerRead;
pub struct ProgrammerRttStatus;
pub struct ProgrammerRttStream;
pub struct BridgeState;
pub struct BridgeI2cOn;
pub struct BridgeI2cOff;
pub struct BridgeCanOn;
pub struct BridgeCanOff;
pub struct BridgeCan;
pub struct BridgeCanTermination;
pub struct BridgeLedOn;
pub struct BridgeLedOff;
pub struct BridgeI3cOn;
pub struct BridgeI3cOff;
pub struct BridgeI2cScan;
pub struct BridgeI2cRead;
pub struct BridgeI2cWrite;
pub struct BridgeLedTransmit;
pub struct BridgeSpiOn;
pub struct BridgeSpiOff;
pub struct BridgeSpiTransfer;
pub struct BridgeUartOn;
pub struct BridgeUartOff;
pub struct BridgeUartWrite;
pub struct BridgeUartRead;
pub struct ScopeData;
pub struct ScopeStart;
pub struct ScopeStop;
pub struct GeneratorStart;
pub struct GeneratorStop;
pub struct GeneratorStatus;
pub struct LedPattern;
pub struct LedAnalyze;
pub struct GlitchArm;
pub struct GlitchDisarm;
pub struct GlitchStatus;
pub struct SystemBootsel;
pub struct SystemReboot;
pub struct SystemPinsReleaseAll;
pub struct SystemPinsToggle;
pub struct SystemFsFormat;

macro_rules! service {
    ($t:ty, $m:expr) => {
        impl MethodHandlerService<(), ()> for $t {
            async fn call_method_handler_service<R: Read, W: ResponseWriter<Error=R::Error>>(
                &self, _state: &(), (): (), method: &str, mut request: Request<'_, R>, response_writer: W,
            ) -> Result<picoserve::ResponseSent, W::Error> {
                if method != $m { return send_json(request, response_writer, StatusCode::METHOD_NOT_ALLOWED, r#"{"error":"method_not_allowed"}"#).await; }
                <$t>::run(request, response_writer).await
            }
        }
    };
}

service!(ProfilesList, "GET"); service!(ProfilesApply, "POST"); service!(ProfilesBoot, "POST"); service!(ProfilesSave, "POST"); service!(ProfilesDelete, "POST"); service!(PinsClaim, "POST"); service!(PinsRelease, "POST"); service!(ServoList, "GET"); service!(ServoAttach, "POST"); service!(ServoWrite, "POST"); service!(ServoWriteUs, "POST"); service!(ServoDetach, "POST"); service!(PwmList, "GET"); service!(PwmClaim, "POST"); service!(PwmSet, "POST"); service!(PwmRelease, "POST"); service!(LogicAnalyzerArm, "POST"); service!(LogicAnalyzerStart, "POST"); service!(LogicAnalyzerStop, "POST"); service!(LogicAnalyzerStatus, "GET"); service!(LogicAnalyzerConfig, "POST"); service!(LogicAnalyzerData, "GET"); service!(LogicAnalyzerVcd, "GET"); service!(ProgrammerReset, "POST"); service!(ProgrammerResetCatch, "POST"); service!(ProgrammerRescueStm32, "POST"); service!(ProgrammerRescueNrf, "POST"); service!(ProgrammerErase, "POST"); service!(ProgrammerRead, "GET"); service!(ProgrammerRttStatus, "GET"); service!(ProgrammerRttStream, "GET");
service!(BridgeState, "GET"); service!(BridgeI2cOn, "POST"); service!(BridgeI2cOff, "POST"); service!(BridgeCanOn, "POST"); service!(BridgeCanOff, "POST"); service!(BridgeCan, "GET"); service!(BridgeCanTermination, "POST"); service!(BridgeLedOn, "POST"); service!(BridgeLedOff, "POST"); service!(BridgeI3cOn, "POST"); service!(BridgeI3cOff, "POST");
service!(BridgeI2cScan, "GET"); service!(BridgeI2cRead, "GET"); service!(BridgeI2cWrite, "POST"); service!(BridgeLedTransmit, "POST");
service!(BridgeSpiOn, "POST"); service!(BridgeSpiOff, "POST"); service!(BridgeSpiTransfer, "POST");
service!(BridgeUartOn, "POST"); service!(BridgeUartOff, "POST"); service!(BridgeUartWrite, "POST"); service!(BridgeUartRead, "GET");
service!(ScopeData, "GET"); service!(ScopeStart, "POST"); service!(ScopeStop, "POST");
service!(GeneratorStart, "POST"); service!(GeneratorStop, "POST"); service!(GeneratorStatus, "GET");
service!(LedPattern, "POST"); service!(LedAnalyze, "GET");
service!(GlitchArm, "POST"); service!(GlitchDisarm, "POST"); service!(GlitchStatus, "GET");
service!(FirmwareStatus, "GET"); service!(FirmwareRollback, "POST");
service!(SystemBootsel, "POST"); service!(SystemReboot, "POST"); service!(SystemPinsReleaseAll, "POST"); service!(SystemPinsToggle, "POST"); service!(SystemFsFormat, "POST");

// ============ implementations ============
impl ProfilesList {
    async fn run<R, W>(request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let mut body = heapless::String::<2048>::new();
        let _ = write!(
            body,
            r#"{{"active_profile":"{}","active_name":"{}","boot_profile":"{}","profiles":["#,
            profile::active_id().as_str(),
            profile::active_name().as_str(),
            profile::boot_id().as_str()
        );
        let mut first = true;
        profile::list_profiles(|p| {
            if !first {
                let _ = body.push(',');
            }
            first = false;
            let _ = write!(
                body,
                r#"{{"id":"{}","name":"{}","description":"{}","is_boot":{},"read_only":{}}}"#,
                p.id.as_str(),
                p.name.as_str(),
                p.description.as_str(),
                p.is_boot,
                p.read_only
            );
        });
        let _ = body.push_str("]}");
        send_json(request, response_writer, StatusCode::OK, body.as_str()).await
    }
}
impl ProfilesApply {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query = request.parts.query();
        let body = read_body_str(&mut request).await;
        let id = key_str(query, &body, "id").or_else(|| key_str(query, &body, "slot"));
        let Some(id) = id else {
            return send_json(request, response_writer, StatusCode::BAD_REQUEST, r#"{"error":"missing profile id"}"#).await;
        };
        match profile::apply_profile(id.as_str()).await {
            Ok(name) => {
                let mut b = heapless::String::<128>::new();
                let _ = write!(b, r#"{{"ok":true,"status":"applied","id":"{}","name":"{}"}}"#, id.as_str(), name.as_str());
                send_json(request, response_writer, StatusCode::OK, b.as_str()).await
            }
            Err(profile::ProfileError::NotFound) => {
                send_json(request, response_writer, StatusCode::NOT_FOUND, r#"{"ok":false,"error":"profile not found"}"#).await
            }
            Err(_) => {
                send_json(request, response_writer, StatusCode::INTERNAL_SERVER_ERROR, r#"{"ok":false,"error":"apply failed"}"#).await
            }
        }
    }
}
impl ProfilesBoot {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query = request.parts.query();
        let body = read_body_str(&mut request).await;
        let id = key_str(query, &body, "id").or_else(|| key_str(query, &body, "slot")).unwrap_or_default();
        match profile::set_boot_default(id.as_str()) {
            Ok(()) => {
                let mut b = heapless::String::<64>::new();
                let _ = write!(b, r#"{{"ok":true,"status":"boot_default_set","id":"{}"}}"#, id.as_str());
                send_json(request, response_writer, StatusCode::OK, b.as_str()).await
            }
            Err(_) => send_json(request, response_writer, StatusCode::BAD_REQUEST, r#"{"ok":false,"error":"set boot failed"}"#).await,
        }
    }
}
impl ProfilesSave {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query = request.parts.query();
        let body = read_body_str(&mut request).await;
        let id = key_str(query, &body, "id").or_else(|| key_str(query, &body, "name"));
        let Some(id) = id else {
            return send_json(request, response_writer, StatusCode::BAD_REQUEST, r#"{"ok":false,"error":"missing profile id"}"#).await;
        };
        match profile::save_profile_doc(id.as_str(), body.as_bytes()) {
            Ok(()) => {
                let mut b = heapless::String::<128>::new();
                let _ = write!(b, r#"{{"ok":true,"status":"saved","id":"{}"}}"#, id.as_str());
                send_json(request, response_writer, StatusCode::OK, b.as_str()).await
            }
            Err(profile::ProfileError::ReadOnly) => {
                send_json(request, response_writer, StatusCode::BAD_REQUEST, r#"{"ok":false,"error":"profile is read only"}"#).await
            }
            Err(_) => {
                send_json(request, response_writer, StatusCode::INTERNAL_SERVER_ERROR, r#"{"ok":false,"error":"save failed"}"#).await
            }
        }
    }
}
impl ProfilesDelete {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query = request.parts.query();
        let body = read_body_str(&mut request).await;
        let id = key_str(query, &body, "id").or_else(|| key_str(query, &body, "slot"));
        let Some(id) = id else {
            return send_json(request, response_writer, StatusCode::BAD_REQUEST, r#"{"ok":false,"error":"missing profile id"}"#).await;
        };
        match profile::delete_profile_doc(id.as_str()) {
            Ok(()) => {
                let mut b = heapless::String::<64>::new();
                let _ = write!(b, r#"{{"ok":true,"status":"deleted","id":"{}"}}"#, id.as_str());
                send_json(request, response_writer, StatusCode::OK, b.as_str()).await
            }
            Err(profile::ProfileError::ReadOnly) => {
                send_json(request, response_writer, StatusCode::BAD_REQUEST, r#"{"ok":false,"error":"cannot delete read only profile"}"#).await
            }
            Err(profile::ProfileError::NotFound) => {
                send_json(request, response_writer, StatusCode::NOT_FOUND, r#"{"ok":false,"error":"profile not found"}"#).await
            }
            Err(_) => {
                send_json(request, response_writer, StatusCode::INTERNAL_SERVER_ERROR, r#"{"ok":false,"error":"delete failed"}"#).await
            }
        }
    }
}
impl PinsClaim {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query=request.parts.query();
        let body=read_body_str(&mut request).await;
        // parse pins array from body or query csv
        let mut pins: heapless::Vec<u32,16>=heapless::Vec::new();
        if let Some(qpins)=query_val(query,"pins") {
            for t in qpins.as_str().split(',') { if let Ok(n)=t.parse::<u32>() { let _=pins.push(n); } }
        } else {
            // extract pins from body json array (naive)
            if let Some(i)=body.find("\"pins\"") {
                let b=&body[i..];
                if let Some(s)=b.find('[') {
                    for t in b[s+1..].split(',') {
                        let d: heapless::String<8>=t.trim().chars().filter(|c|c.is_ascii_digit()).collect();
                        if let Ok(n)=d.parse::<u32>() { let _=pins.push(n); }
                    }
                }
            }
        }
        let owner=key_str(query,&body,"owner").unwrap_or_else(|| { let mut x=heapless::String::<64>::new(); let _=x.push_str("SIO"); x });
        let mut lower = heapless::String::<64>::new();
        for ch in owner.as_str().chars() { let _ = lower.push(ch.to_ascii_lowercase()); }
        let driver = pin_owner::Driver::from_name(lower.as_str()).unwrap_or(pin_owner::Driver::Gpio);
        let caps = if pins.is_empty() { 0u32 } else {
            let base = match lower.as_str() {
                "programmer" => pin_owner::CAP_SIO,
                "servo"|"pwm" => pin_owner::CAP_PWM,
                "adapter"|"i2c" => pin_owner::CAP_I2C,
                "capture"|"pio0"|"pio1"|"pio2" => pin_owner::CAP_PIO,
                "spi" => pin_owner::CAP_SPI,
                "uart" => pin_owner::CAP_UART,
                _ => pin_owner::CAP_SIO,
            };
            base | pin_owner::CAP_SIO as u32
        };


        if pins.is_empty() { return send_json(request,response_writer,StatusCode::BAD_REQUEST,r#"{"error":"empty pins"}"#).await; }
        if pins.iter().any(|p|!pin_owner::is_flex(*p)) {
            let mut b=heapless::String::<256>::new(); let _=write!(b,r#"{{"error":"pin not available","pins":["#);
            for (i,p) in pins.iter().enumerate() { if i>0 {let _=b.push(','); if !pin_owner::is_flex(*p) { let _=write!(b,"{}",p); } } else if !pin_owner::is_flex(*p) { let _=write!(b,"{}",p);} } // simplify below
            let _=b.push_str("]}");
            let mut b=heapless::String::<256>::new();
            let _=b.push_str(r#"{"error":"pin not available","pins":[\"#);
            // rebuild cleanly
            return send_json(request,response_writer,StatusCode::CONFLICT,r#"{"error":"pin not available"}"#).await;
        }
        // conflict precheck
        let mut conflict: heapless::Vec<u32,16>=heapless::Vec::new();
        for &p in pins.iter() { if pin_owner::has_claim(p).is_some() { let _=conflict.push(p); } }
        if !conflict.is_empty() {
            let mut b=heapless::String::<256>::new(); let _=b.push_str(r#"{"error":"pin conflict","pins":["#);
            for (i,p) in conflict.iter().enumerate() { if i>0 {let _=b.push(',');} let _=write!(b,"{}",p); }
            let _=b.push_str("]}");
            return send_json(request,response_writer,StatusCode::CONFLICT,b.as_str()).await;
        }
        match pin_owner::claim(driver, pins.as_slice(), caps) {
            Ok(slot) => {
                let mut b=heapless::String::<256>::new();
                let _=b.push_str(r#"{"claimed":true,"pins":["#);
                for (i,p) in pins.iter().enumerate() { if i>0 {let _=b.push(',');} let _=write!(b,"{}",p); }
                let _=write!(b,r#"],"handle":{}}}"#,slot.0);
                send_json(request,response_writer,StatusCode::OK,b.as_str()).await
            }
            Err(_) => send_json(request,response_writer,StatusCode::CONFLICT,r#"{"error":"pin conflict"}"#).await,
        }
    }
}
impl PinsRelease {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query=request.parts.query();
        let body=read_body_str(&mut request).await;
        let h=key_int(query,&body,"handle").unwrap_or(-1);
        if h<0 { return send_json(request,response_writer,StatusCode::BAD_REQUEST,r#"{"error":"no handle"}"#).await; }
        match pin_owner::release(pin_owner::SlotId(h as u32)) {
            Ok(()) => { let mut b=heapless::String::<64>::new(); let _=write!(b,r#"{{"released":true,"handle":{}}}"#,h); send_json(request,response_writer,StatusCode::OK,b.as_str()).await }
            Err(_) => send_json(request,response_writer,StatusCode::NOT_FOUND,r#"{"error":"no such claim"}"#).await,
        }
    }
}
impl ServoList {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let chans=servo::servo_list();
        let mut b=heapless::String::<1024>::new();
        let _=b.push_str(r#"{"status":"ok","channels":["#);
        let mut first=true;
        for c in chans.iter() {
            if !first { let _=b.push(','); } first=false;
            let _=write!(b,r#"{{"pin":{},"angle":{},"pulse_us":{},"min_us":{},"max_us":{},"continuous":{}}}"#, c.pin,c.angle,c.pulse_us,c.min_us,c.max_us,c.continuous);
        }
        let _=b.push_str("]}");
        send_json(request,response_writer,StatusCode::OK,b.as_str()).await
    }
}
impl ServoAttach {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query=request.parts.query();
        let body=read_body_str(&mut request).await;
        let pin=key_int(query,&body,"pin").unwrap_or(-1);
        if pin<0 { return send_json(request,response_writer,StatusCode::BAD_REQUEST,r#"{"error":"no pin"}"#).await; }
        let min=key_int(query,&body,"min_us").map(|v|v as u32);
        let max=key_int(query,&body,"max_us").map(|v|v as u32);
        let cont=key_str(query,&body,"continuous").map(|v|parse_bool(v.as_str())).unwrap_or(false);
        match servo::servo_attach(pin as u32, min, max, cont) {
            Ok(c) => { let mut b=heapless::String::<256>::new(); let _=write!(b,r#"{{"status":"attached","pin":{},"angle":{},"pulse_us":{},"min_us":{},"max_us":{},"continuous":{}}}"#,c.pin,c.angle,c.pulse_us,c.min_us,c.max_us,c.continuous); send_json(request,response_writer,StatusCode::OK,b.as_str()).await }
            Err(_) => send_json(request,response_writer,StatusCode::CONFLICT,r#"{"error":"pin conflict"}"#).await,
        }
    }
}
impl ServoWrite {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query=request.parts.query();
        let body=read_body_str(&mut request).await;
        let pin=key_int(query,&body,"pin").unwrap_or(-1);
        let angle=key_int(query,&body,"angle").unwrap_or(0);
        if pin<0 { return send_json(request,response_writer,StatusCode::BAD_REQUEST,r#"{"error":"bad args"}"#).await; }
        match servo::servo_write_angle(pin as u32, angle as i32) {
            Ok(_) => { let mut b=heapless::String::<64>::new(); let _=write!(b,r#"{{"status":"ok","pin":{},"angle":{}}}"#,pin,angle); send_json(request,response_writer,StatusCode::OK,b.as_str()).await }
            Err(_) => send_json(request,response_writer,StatusCode::NOT_FOUND,r#"{"error":"not attached"}"#).await,
        }
    }
}
impl ServoWriteUs {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query=request.parts.query();
        let body=read_body_str(&mut request).await;
        let pin=key_int(query,&body,"pin").unwrap_or(-1);
        let pulse=key_int(query,&body,"pulse_us").unwrap_or(-1);
        if pin<0 || pulse<0 { return send_json(request,response_writer,StatusCode::BAD_REQUEST,r#"{"error":"bad args"}"#).await; }
        match servo::servo_write_us(pin as u32, pulse as u32) {
            Ok(_) => { let mut b=heapless::String::<64>::new(); let _=write!(b,r#"{{"status":"ok","pin":{},"pulse_us":{}}}"#,pin,pulse); send_json(request,response_writer,StatusCode::OK,b.as_str()).await }
            Err(_) => send_json(request,response_writer,StatusCode::NOT_FOUND,r#"{"error":"not attached"}"#).await,
        }
    }
}
impl ServoDetach {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query=request.parts.query();
        let body=read_body_str(&mut request).await;
        let pin=key_int(query,&body,"pin").unwrap_or(-1);
        if pin<0 { return send_json(request,response_writer,StatusCode::BAD_REQUEST,r#"{"error":"bad args"}"#).await; }
        let _=servo::detach(pin as u32);
        let mut b=heapless::String::<64>::new(); let _=write!(b,r#"{{"status":"detached","pin":{}}}"#,pin);
        send_json(request,response_writer,StatusCode::OK,b.as_str()).await
    }
}
impl PwmList {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let chans=servo::pwm_list();
        let mut b=heapless::String::<512>::new();
        let _=b.push_str(r#"{"status":"ok","channels":["#);
        let mut first=true;
        for c in chans.iter() {
            if !first { let _=b.push(','); } first=false;
            let _=write!(b,r#"{{"pin":{},"freq":{},"pulse_us":{},"active":true}}"#,c.pin,c.freq,c.pulse_us);
        }
        let _=b.push_str("]}");
        send_json(request,response_writer,StatusCode::OK,b.as_str()).await
    }
}
impl PwmClaim {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query=request.parts.query();
        let body=read_body_str(&mut request).await;
        let pin=key_int(query,&body,"pin").unwrap_or(-1);
        if pin<0 { return send_json(request,response_writer,StatusCode::BAD_REQUEST,r#"{"error":"bad args"}"#).await; }
        match servo::pwm_claim(pin as u32) {
            Ok((_,h)) => { let mut b=heapless::String::<64>::new(); let _=write!(b,r#"{{"status":"claimed","pin":{},"handle":{}}}"#,pin,h); send_json(request,response_writer,StatusCode::OK,b.as_str()).await }
            Err(_) => send_json(request,response_writer,StatusCode::CONFLICT,r#"{"error":"pin conflict"}"#).await,
        }
    }
}
impl PwmSet {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query=request.parts.query();
        let body=read_body_str(&mut request).await;
        let pin=key_int(query,&body,"pin").unwrap_or(-1);
        let freq=key_int(query,&body,"freq").unwrap_or(-1);
        let pulse=key_int(query,&body,"pulse_us").unwrap_or(-1);
        if pin<0||freq<1||freq>100000||pulse<0 { return send_json(request,response_writer,StatusCode::BAD_REQUEST,r#"{"error":"bad args"}"#).await; }
        match servo::pwm_set(pin as u32, freq as u32, pulse as u32) {
            Ok(_) => { let mut b=heapless::String::<64>::new(); let _=write!(b,r#"{{"status":"ok","pin":{}}}"#,pin); send_json(request,response_writer,StatusCode::OK,b.as_str()).await }
            Err(_) => send_json(request,response_writer,StatusCode::INTERNAL_SERVER_ERROR,r#"{"error":"set failed"}"#).await,
        }
    }
}
impl PwmRelease {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query=request.parts.query();
        let body=read_body_str(&mut request).await;
        let pin=key_int(query,&body,"pin").unwrap_or(-1);
        let handle=key_int(query,&body,"handle").unwrap_or(-1);
        if handle>=0 { let _=pin_owner::release(pin_owner::SlotId(handle as u32)); }
        if pin>=0 { let _=servo::detach(pin as u32); }
        let mut b=heapless::String::<64>::new(); let _=b.push_str(r#"{"status":"released"}"#);
        send_json(request,response_writer,StatusCode::OK,b.as_str()).await
    }
}

// ============ logic analyzer handlers ============
impl LogicAnalyzerArm {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query=request.parts.query();
        let body=read_body_str(&mut request).await;
        let freq=key_int(query,&body,"freq").map(|v|v as u32).unwrap_or(1_000_000);
        let length=key_int(query,&body,"length").map(|v|v as u32).unwrap_or(1024);
        let mut channels: heapless::Vec<u8,32> = heapless::Vec::new();
        if let Some(q)=query_val(query,"channels") {
            for t in q.as_str().split(',') { if let Ok(n)=t.parse::<u8>() { let _=channels.push(n); } }
        } else {
            for n in 0u8..4u8 { let _=channels.push(n); }
        }
        let mut trig = crate::analyzer::TriggerConfig::default();
        if let Some(k)=key_str(query,&body,"trigger") {
            if k.as_str().eq_ignore_ascii_case("rising") || k.as_str()=="rising" { trig.r#type = crate::analyzer::TriggerType::RisingEdge; }
        }
        if let Some(t)=key_int(query,&body,"trigger_type") { if t==1 { trig.r#type = crate::analyzer::TriggerType::RisingEdge; } if t==2 { trig.r#type = crate::analyzer::TriggerType::FallingEdge; } }
        let cfg = crate::analyzer::CaptureConfig { freq, length, channels, trigger: trig };
        match crate::analyzer::arm(cfg) {
            Ok(()) => {
                let mut b=heapless::String::<64>::new();
                let _=b.push_str(r#"{"status":"armed"}"#);
                send_json(request,response_writer,StatusCode::OK,b.as_str()).await
            }
            Err(_) => send_json(request,response_writer,StatusCode::CONFLICT,r#"{"error":"already capturing or pin conflict"}"#).await,
        }
    }
}
impl LogicAnalyzerStart {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = crate::analyzer::start();
        embassy_time::Timer::after(embassy_time::Duration::from_millis(20)).await;
        let st = crate::analyzer::get_status();
        let mut b=heapless::String::<128>::new();
        let _=core::fmt::Write::write_fmt(&mut b, format_args!(r#"{{"state":{},"pre_bytes":{},"post_bytes":{},"sample_count":{}}}"#, st.state as u8, st.pre_bytes, st.post_bytes, st.sample_count));
        send_json(request,response_writer,StatusCode::OK,b.as_str()).await
    }
}
impl LogicAnalyzerStop {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        crate::analyzer::stop();
        let st = crate::analyzer::get_status();
        let mut b=heapless::String::<128>::new();
        let _=core::fmt::Write::write_fmt(&mut b, format_args!(r#"{{"state":{},"pre_bytes":{},"post_bytes":{},"sample_count":{}}}"#, st.state as u8, st.pre_bytes, st.post_bytes, st.sample_count));
        send_json(request,response_writer,StatusCode::OK,b.as_str()).await
    }
}
impl LogicAnalyzerStatus {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let st = crate::analyzer::get_status();
        let mut b=heapless::String::<256>::new();
        let _=core::fmt::Write::write_fmt(&mut b, format_args!(r#"{{"state":{},"freq":{},"length":{},"pre_bytes":{},"post_bytes":{},"sample_count":{},"is_capturing":{}}}"#, st.state as u8, st.freq, st.length, st.pre_bytes, st.post_bytes, st.sample_count, st.is_capturing));
        send_json(request,response_writer,StatusCode::OK,b.as_str()).await
    }
}
impl LogicAnalyzerConfig {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let mut b=heapless::String::<64>::new();
        let _=b.push_str(r#"{"status":"ok"}"#);
        send_json(request,response_writer,StatusCode::OK,b.as_str()).await
    }
}
impl LogicAnalyzerData {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let st = crate::analyzer::get_status();
        let first = crate::analyzer::bank_word(0);
        let mut bankbuf = [0u8; 32];
        let n = ((st.sample_count as usize)*4).min(32);
        let mut i = 0usize;
        while i < n {
            let w = crate::analyzer::bank_word(i/4);
            bankbuf[i] = ((w >> ((i % 4)*8)) & 0xFF) as u8;
            i += 1;
        }
        let i3c = crate::decoder::i3c_decode(&bankbuf, 0, 1);
        let mut b=heapless::String::<256>::new();
        let _=core::fmt::Write::write_fmt(&mut b, format_args!(r#"{{"state":{},"pre_bytes":{},"post_bytes":{},"sample_count":{},"first_sample":{},"i3c_frames":{}, "i3c_bytes":{}}}"#, st.state as u8 , st.pre_bytes, st.pre_bytes, st.sample_count, first, i3c.frame_count, i3c.byte_count));
        send_json(request,response_writer,StatusCode::OK,b.as_str()).await
    }
}

impl LogicAnalyzerVcd {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let mut vcd = heapless::String::<2048>::new();
        crate::analyzer::generate_vcd(&mut vcd);
        let finalizer = request.body_connection.finalize().await?;
        (StatusCode::OK, ("Content-Type", "text/plain"), vcd.as_str()).write_to(finalizer, response_writer).await
    }
}

impl LogicAnalyzerDecodersConfigure {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let body = read_body_str(&mut request).await;
        let s = body.as_str();
        let bus = key_str(None, s, "bus").unwrap_or_default();
        let pin_a = key_u32(None, s, "pin_a").unwrap_or(0) as u8;
        let pin_b = key_u32(None, s, "pin_b").unwrap_or(0) as u8;
        let baud = key_u32(None, s, "baud").unwrap_or(115_200);
        let kind = match bus.as_str() {
            "uart" => 0u8,
            "i2c" => 1u8,
            "spi" => 2u8,
            "can" => 3u8,
            _ => 0u8,
        };
        if let Ok(mut dec) = crate::analyzer::ACTIVE_DECODER.try_lock() {
            dec.enabled = true;
            dec.kind = kind;
            dec.pin_a = pin_a;
            dec.pin_b = pin_b;
            dec.baud = baud;
        }
        send_json(request, response_writer, StatusCode::OK, r#"{"status":"configured","decoders_active":true}"#).await
    }
}

impl LogicAnalyzerTriggersConfigure {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let body = read_body_str(&mut request).await;
        let s = body.as_str();
        let rule_id = key_u32(None, s, "rule_id").unwrap_or(0) as u8;
        let bus_str = key_str(None, s, "bus").unwrap_or_default();
        let bus = match bus_str.as_str() {
            "uart" => agentla_analyzer::decoders::event::ProtocolKind::Uart,
            "spi" => agentla_analyzer::decoders::event::ProtocolKind::Spi,
            "can" => agentla_analyzer::decoders::event::ProtocolKind::Can,
            _ => agentla_analyzer::decoders::event::ProtocolKind::I2c,
        };
        let match_addr = key_u32(None, s, "addr").unwrap_or(0) as u16;
        let match_id = key_u32(None, s, "id").unwrap_or(0);
        let match_ack = if s.contains(r#""ack":false"#) { Some(false) } else if s.contains(r#""ack":true"#) { Some(true) } else { None };
        let freeze = !s.contains(r#""freeze":false"#);
        let alert = !s.contains(r#""alert":false"#);

        let rule = agentla_analyzer::decoders::matrix::HardwareTriggerRule {
            rule_id,
            enabled: true,
            bus,
            match_addr,
            match_addr_mask: if match_addr != 0 { 0x7F } else { 0 },
            match_ack,
            match_id,
            match_id_mask: if match_id != 0 { 0x7FF } else { 0 },
            match_data_byte0: None,
            match_error: s.contains(r#""error":true"#),
            action_freeze_capture: freeze,
            action_assert_pin: None,
            action_halt_swd: s.contains(r#""halt_swd":true"#),
            action_ws_alert: alert,
        };

        if let Ok(mut matrix) = crate::analyzer::TRIGGER_MATRIX.try_lock() {
            let _ = matrix.install_rule(rule);
        }
        send_json(request, response_writer, StatusCode::OK, r#"{"status":"configured","rule_installed":true}"#).await
    }
}

impl ProgrammerTarget {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query = request.parts.query();
        let body = read_body_str(&mut request).await;
        let port_str = key_str(query, &body, "port");
        let result = match port_str.as_ref().map(|s| s.as_str()) {
            Some("ch32v203") => crate::programmer::detect_target_port_global(crate::programmer::TargetPort::Ch32v203),
            Some("ch32v003") => crate::programmer::detect_target_port_global(crate::programmer::TargetPort::Ch32v003),
            Some("bluepill") => crate::programmer::detect_target_port_global(crate::programmer::TargetPort::BluePill),
            _ => crate::programmer::detect_target_global(),
        };
        match result {
            Ok(target) => {
                let mut resp = heapless::String::<256>::new();
                let _ = write!(
                    resp,
                    r#"{{"status":"ok","detected":true,"family":"{:?}","name":"{}","idcode":"0x{:08X}","flash_base":"0x{:08X}","flash_size":{},"ram_base":"0x{:08X}","ram_size":{}}}"#,
                    target.family, target.name, target.idcode, target.flash_base, target.flash_size, target.ram_base, target.ram_size
                );
                send_json(request, response_writer, StatusCode::OK, resp.as_str()).await
            }
            Err(e) => {
                let mut resp = heapless::String::<192>::new();
                if port_str.as_ref().map(|s| s.as_str()) == Some("ch32v203") {
                    let (dmstatus, partid) = crate::programmer::debug_probe_ch32v203_global();
                    let _ = write!(
                        resp,
                        r#"{{"status":"error","detected":false,"port":"ch32v203","error":"{:?}","dmstatus":"0x{:08X}","partid":"0x{:08X}"}}"#,
                        e, dmstatus, partid
                    );
                } else {
                    let (dmstatus, partid, abstractcs, hartinfo) = crate::programmer::debug_probe_ch32v003_global();
                    let _ = write!(
                        resp,
                        r#"{{"status":"error","detected":false,"port":"ch32v003","error":"{:?}","dmstatus":"0x{:08X}","partid":"0x{:08X}","abstractcs":"0x{:08X}","hartinfo":"0x{:08X}"}}"#,
                        e, dmstatus, partid, abstractcs, hartinfo
                    );
                }
                send_json(request, response_writer, StatusCode::OK, resp.as_str()).await
            }
        }
    }
}

impl MethodHandlerService<(), ()> for ProgrammerTarget {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error=R::Error>>(
        &self, _state: &(), (): (), _method: &str, request: Request<'_, R>, response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        ProgrammerTarget::run(request, response_writer).await
    }
}

impl ProgrammerReset {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        let mut buf = [0u8; 16];
        let _ = crate::programmer::read_flash_global(0x08000000, &mut buf);
        send_json(request, response_writer, StatusCode::OK, r#"{"status":"ok","reset_complete":true}"#).await
    }
}

impl ProgrammerResetCatch {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query = request.parts.query();
        let body = read_body_str(&mut request).await;
        let port_str = key_str(query, &body, "port");
        let port = match port_str.as_ref().map(|s| s.as_str()) {
            Some("ch32v203") => crate::programmer::TargetPort::Ch32v203,
            Some("ch32v003") => crate::programmer::TargetPort::Ch32v003,
            _ => crate::programmer::TargetPort::BluePill,
        };
        match crate::programmer::connect_under_reset_port_global(port) {
            Ok(pc) => {
                let mut resp = heapless::String::<128>::new();
                let _ = write!(resp, r#"{{"status":"ok","halted":true,"pc":{},"pc_hex":"0x{:08X}"}}"#, pc, pc);
                send_json(request, response_writer, StatusCode::OK, resp.as_str()).await
            }
            Err(e) => {
                let mut resp = heapless::String::<128>::new();
                let _ = write!(resp, r#"{{"status":"error","error":"{:?}"}}"#, e);
                send_json(request, response_writer, StatusCode::INTERNAL_SERVER_ERROR, resp.as_str()).await
            }
        }
    }
}

impl ProgrammerRescueStm32 {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        match crate::programmer::rescue_stm32_global() {
            Ok(()) => {
                send_json(
                    request,
                    response_writer,
                    StatusCode::OK,
                    r#"{"status":"ok","unprotected":true,"mass_erased":true,"message":"STM32 Readout Unprotect and mass-erase complete"}"#,
                ).await
            }
            Err(crate::programmer::bootloader::BootloaderError::PinConflict) => {
                send_json(
                    request,
                    response_writer,
                    StatusCode::CONFLICT,
                    r#"{"status":"error","error":"Pin conflict with active analyzer or peripheral driver"}"#,
                ).await
            }
            Err(e) => {
                let mut resp = heapless::String::<128>::new();
                let _ = write!(resp, r#"{{"status":"error","error":"{:?}"}}"#, e);
                send_json(
                    request,
                    response_writer,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    resp.as_str(),
                ).await
            }
        }
    }
}

impl ProgrammerRescueNrf {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        match crate::programmer::rescue_nrf_global() {
            Ok(()) => {
                send_json(
                    request,
                    response_writer,
                    StatusCode::OK,
                    r#"{"status":"ok","unprotected":true,"mass_erased":true,"message":"Nordic nRF CTRL-AP ERASEALL complete"}"#,
                ).await
            }
            Err(e) => {
                let mut resp = heapless::String::<128>::new();
                let _ = write!(resp, r#"{{"status":"error","error":"{:?}"}}"#, e);
                send_json(
                    request,
                    response_writer,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    resp.as_str(),
                ).await
            }
        }
    }
}

impl ProgrammerErase {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        send_json(request, response_writer, StatusCode::OK, r#"{"status":"ok","erased":true,"mode":"mass"}"#).await
    }
}

impl ProgrammerRead {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query = request.parts.query();
        let body = read_body_str(&mut request).await;
        let port_str = key_str(query, &body, "port");
        let port = match port_str.as_ref().map(|s| s.as_str()) {
            Some("ch32v203") => crate::programmer::TargetPort::Ch32v203,
            Some("ch32v003") => crate::programmer::TargetPort::Ch32v003,
            _ => crate::programmer::TargetPort::BluePill,
        };
        let addr = key_u32(query, &body, "addr").unwrap_or(0x08000000);

        let mut buf = [0u8; 16];
        let res = crate::programmer::read_flash_port_global(port, addr, &mut buf);
        match res {
            Ok(_) => {
                let mut hex = heapless::String::<64>::new();
                for &b in &buf {
                    let _ = write!(hex, "{:02X}", b);
                }
                let mut resp = heapless::String::<160>::new();
                let _ = write!(resp, r#"{{"status":"ok","port":"{:?}","addr":{},"len":16,"data_hex":"{}"}}"#, port, addr, hex.as_str());
                send_json(request, response_writer, StatusCode::OK, resp.as_str()).await
            }
            Err(e) => {
                let mut resp = heapless::String::<128>::new();
                let _ = write!(resp, r#"{{"status":"error","port":"{:?}","error":"{:?}"}}"#, port, e);
                send_json(request, response_writer, StatusCode::INTERNAL_SERVER_ERROR, resp.as_str()).await
            }
        }
    }
}

impl ProgrammerRttStatus {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        let mut buf = heapless::String::<256>::new();
        crate::programmer::rtt::rtt_status_json(&mut buf);
        send_json(request, response_writer, StatusCode::OK, buf.as_str()).await
    }
}

impl ProgrammerRttStream {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        let mut chunk = [0u8; 256];
        match crate::programmer::rtt::rtt_drain_global(&mut chunk) {
            Ok(n) => {
                let mut resp = heapless::String::<512>::new();
                let _ = write!(resp, r#"{{"status":"ok","bytes_read":{},"data":""#, n);
                for &b in &chunk[..n] {
                    match b {
                        b'"' => { let _ = resp.push_str("\\\""); }
                        b'\\' => { let _ = resp.push_str("\\\\"); }
                        b'\n' => { let _ = resp.push_str("\\n"); }
                        b'\r' => { let _ = resp.push_str("\\r"); }
                        b'\t' => { let _ = resp.push_str("\\t"); }
                        0x20..=0x7E => { let _ = resp.push(b as char); }
                        _ => { let _ = write!(resp, "\\u{:04x}", b); }
                    }
                }
                let _ = resp.push_str(r#""}"#);
                send_json(request, response_writer, StatusCode::OK, resp.as_str()).await
            }
            Err(_) => {
                send_json(request, response_writer, StatusCode::OK, r#"{"status":"error","message":"ERR RTT NOT FOUND"}"#).await
            }
        }
    }
}

impl BridgeState {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let body = crate::bridge::state_json();
        send_json(request, response_writer, StatusCode::OK, body.as_str()).await
    }
}
// placeholder wrappers to fit macro (off returns ()): convert

impl BridgeI2cOn {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        match crate::bridge::i2c_on() {
            Ok(()) => { let mut b=heapless::String::<64>::new(); let _=b.push_str(r#"{"status":"ok"}"#); send_json(request,response_writer,StatusCode::OK,b.as_str()).await }
            Err(e) => { let mut b=heapless::String::<64>::new(); let _=core::write!(b,r#"{{"error":"{:?}"}}"#,e); send_json(request,response_writer,StatusCode::CONFLICT,b.as_str()).await }
        }
    }
}
impl BridgeI2cOff {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        crate::bridge::i2c_off();
        let mut b=heapless::String::<64>::new(); let _=b.push_str(r#"{"status":"ok"}"#);
        send_json(request,response_writer,StatusCode::OK,b.as_str()).await
    }
}
impl BridgeCanOn {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        match crate::bridge::can_on() {
            Ok(()) => { let mut b=heapless::String::<64>::new(); let _=b.push_str(r#"{"status":"ok"}"#); send_json(request,response_writer,StatusCode::OK,b.as_str()).await }
            Err(e) => { let mut b=heapless::String::<64>::new(); let _=core::write!(b,r#"{{"error":"{:?}"}}"#,e); send_json(request,response_writer,StatusCode::CONFLICT,b.as_str()).await }
        }
    }
}
impl BridgeCanOff {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        crate::bridge::can_off();
        let mut b=heapless::String::<64>::new(); let _=b.push_str(r#"{"status":"ok"}"#);
        send_json(request,response_writer,StatusCode::OK,b.as_str()).await
    }
}
impl BridgeCan {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let active = crate::bridge::can_gsusb::adapter_can_is_active();
        let bitrate = crate::bridge::can_gsusb::adapter_can_get_bitrate();
        let termination = crate::bridge::can_gsusb::adapter_can_get_termination();
        let tx = crate::bridge::can_gsusb::adapter_can_get_tx_frames();
        let rx = crate::bridge::can_gsusb::adapter_can_get_rx_frames();
        let mut b = heapless::String::<160>::new();
        let _ = core::write!(
            b,
            r#"{{"active":{},"bitrate":{},"termination":{},"tx_frames":{},"rx_frames":{}}}"#,
            active, bitrate, termination, tx, rx
        );
        send_json(request, response_writer, StatusCode::OK, b.as_str()).await
    }
}
impl BridgeCanTermination {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let body = read_body_str(&mut request).await;
        let enabled = body.contains("true") || body.contains("1");
        crate::bridge::can_gsusb::adapter_can_set_termination(enabled);
        let mut resp = heapless::String::<64>::new();
        let _ = core::write!(resp, r#"{{"status":"ok","termination":{}}}"#, enabled);
        send_json(request, response_writer, StatusCode::OK, resp.as_str()).await
    }
}
impl BridgeLedOn {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        match crate::bridge::led_on() {
            Ok(()) => { let mut b=heapless::String::<64>::new(); let _=b.push_str(r#"{"status":"ok"}"#); send_json(request,response_writer,StatusCode::OK,b.as_str()).await }
            Err(e) => { let mut b=heapless::String::<64>::new(); let _=core::write!(b,r#"{{"error":"{:?}"}}"#,e); send_json(request,response_writer,StatusCode::CONFLICT,b.as_str()).await }
        }
    }
}
impl BridgeLedOff {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        crate::bridge::led_off();
        let mut b=heapless::String::<64>::new(); let _=b.push_str(r#"{"status":"ok"}"#);
        send_json(request,response_writer,StatusCode::OK,b.as_str()).await
    }
}
impl BridgeI3cOn {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        match crate::bridge::i3c_on() {
            Ok(()) => { let mut b=heapless::String::<64>::new(); let _=b.push_str(r#"{"status":"ok"}"#); send_json(request,response_writer,StatusCode::OK,b.as_str()).await }
            Err(e) => { let mut b=heapless::String::<64>::new(); let _=core::write!(b,r#"{{"error":"{:?}"}}"#,e); send_json(request,response_writer,StatusCode::CONFLICT,b.as_str()).await }
        }
    }
}
impl BridgeI3cOff {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        crate::bridge::i3c_off();
        let mut b=heapless::String::<64>::new(); let _=b.push_str(r#"{"status":"ok"}"#);
        send_json(request,response_writer,StatusCode::OK,b.as_str()).await
    }
}

impl BridgeI2cScan {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let found = crate::bridge::i2c::hw_i2c_scan(16, 17);
        let mut b = heapless::String::<256>::new();
        let _ = write!(b, r#"{{"status":"ok","count":{},"addresses":["#, found.len());
        for (i, &addr) in found.iter().enumerate() {
            if i > 0 { let _ = b.push(','); }
            let _ = write!(b, "{}", addr);
        }
        let _ = b.push_str("]}");
        send_json(request, response_writer, StatusCode::OK, b.as_str()).await
    }
}

impl BridgeI2cRead {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query = request.parts.query();
        let body = read_body_str(&mut request).await;
        let addr = key_int(query, &body, "addr").unwrap_or(0x3C) as u8;
        let reg = key_int(query, &body, "reg").unwrap_or(0) as u8;
        let len = (key_int(query, &body, "len").unwrap_or(1) as usize).min(32);
        let mut buf = [0u8; 32];
        let ok = crate::bridge::i2c::hw_i2c_read(16, 17, addr, reg, &mut buf[..len]);
        let mut hex = heapless::String::<64>::new();
        if ok {
            for &byte in &buf[..len] {
                let _ = write!(hex, "{:02X}", byte);
            }
        }
        let mut resp = heapless::String::<128>::new();
        let _ = write!(resp, r#"{{"status":"{}","addr":{},"reg":{},"len":{},"data_hex":"{}"}}"#,
            if ok { "ok" } else { "nack" }, addr, reg, len, hex.as_str());
        send_json(request, response_writer, StatusCode::OK, resp.as_str()).await
    }
}

impl BridgeI2cWrite {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query = request.parts.query();
        let body = read_body_str(&mut request).await;
        let addr = key_int(query, &body, "addr").unwrap_or(0x3C) as u8;
        let reg = key_int(query, &body, "reg").unwrap_or(0) as u8;
        let val = key_int(query, &body, "val").unwrap_or(0) as u8;
        let ok = crate::bridge::i2c::hw_i2c_write(16, 17, addr, reg, &[val]);
        let mut resp = heapless::String::<64>::new();
        let _ = write!(resp, r#"{{"status":"{}","bytes_written":{}}}"#, if ok { "ok" } else { "nack" }, if ok { 1 } else { 0 });
        send_json(request, response_writer, StatusCode::OK, resp.as_str()).await
    }
}

impl BridgeLedTransmit {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        let colors = [
            (255, 0, 0),
            (255, 128, 0),
            (255, 255, 0),
            (0, 255, 0),
            (0, 255, 255),
            (0, 0, 255),
            (128, 0, 255),
            (255, 255, 255),
        ];
        let n = crate::bridge::ws2812::transmit_colors(&colors).unwrap_or(0);
        let mut resp = heapless::String::<64>::new();
        let _ = write!(resp, r#"{{"status":"ok","transmitted_pixels":{}}}"#, n);
        send_json(request, response_writer, StatusCode::OK, resp.as_str()).await
    }
}

fn hex_nibble(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

impl BridgeSpiOn {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        match crate::bridge::spi_on() {
            Ok(()) => { let mut b = heapless::String::<64>::new(); let _ = b.push_str(r#"{"status":"ok"}"#); send_json(request, response_writer, StatusCode::OK, b.as_str()).await }
            Err(e) => { let mut b = heapless::String::<64>::new(); let _ = write!(b, r#"{{"error":"{:?}"}}"#, e); send_json(request, response_writer, StatusCode::CONFLICT, b.as_str()).await }
        }
    }
}

impl BridgeSpiOff {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        crate::bridge::spi_off();
        let mut b = heapless::String::<64>::new(); let _ = b.push_str(r#"{"status":"ok"}"#);
        send_json(request, response_writer, StatusCode::OK, b.as_str()).await
    }
}

impl BridgeSpiTransfer {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query = request.parts.query();
        let body = read_body_str(&mut request).await;
        let mode = (key_int(query, &body, "mode").unwrap_or(0) as u8).min(3);
        let speed_hz = (key_int(query, &body, "speed_hz").unwrap_or(1_000_000) as u32).clamp(1000, 10_000_000);
        let read_len = (key_int(query, &body, "read_len").unwrap_or(0) as usize).min(64);

        let mut tx_buf = [0u8; 64];
        let mut tx_len = 0usize;
        if let Some(hex_str) = key_str(query, &body, "data_hex") {
            let bytes = hex_str.as_bytes();
            let mut i = 0;
            while i + 1 < bytes.len() && tx_len < tx_buf.len() {
                if let (Some(h), Some(l)) = (hex_nibble(bytes[i]), hex_nibble(bytes[i+1])) {
                    tx_buf[tx_len] = (h << 4) | l;
                    tx_len += 1;
                }
                i += 2;
            }
        } else if let Some(raw_str) = key_str(query, &body, "data") {
            let b = raw_str.as_bytes();
            let l = b.len().min(tx_buf.len());
            tx_buf[..l].copy_from_slice(&b[..l]);
            tx_len = l;
        }

        let total_len = tx_len.max(read_len).max(1).min(64);
        let mut rx_buf = [0u8; 64];
        let count = crate::bridge::spi::spi_transfer_default(mode, speed_hz, &tx_buf[..total_len], &mut rx_buf[..total_len]);

        let mut rx_hex = heapless::String::<130>::new();
        for &byte in &rx_buf[..count] {
            let _ = write!(rx_hex, "{:02X}", byte);
        }

        let mut resp = heapless::String::<256>::new();
        let _ = write!(resp, r#"{{"status":"ok","mode":{},"speed_hz":{},"rx_hex":"{}"}}"#, mode, speed_hz, rx_hex.as_str());
        send_json(request, response_writer, StatusCode::OK, resp.as_str()).await
    }
}

impl BridgeUartOn {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        match crate::bridge::uart_on() {
            Ok(()) => { let mut b = heapless::String::<64>::new(); let _ = b.push_str(r#"{"status":"ok"}"#); send_json(request, response_writer, StatusCode::OK, b.as_str()).await }
            Err(e) => { let mut b = heapless::String::<64>::new(); let _ = write!(b, r#"{{"error":"{:?}"}}"#, e); send_json(request, response_writer, StatusCode::CONFLICT, b.as_str()).await }
        }
    }
}

impl BridgeUartOff {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        crate::bridge::uart_off();
        let mut b = heapless::String::<64>::new(); let _ = b.push_str(r#"{"status":"ok"}"#);
        send_json(request, response_writer, StatusCode::OK, b.as_str()).await
    }
}

impl BridgeUartWrite {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query = request.parts.query();
        let body = read_body_str(&mut request).await;
        let baud = (key_int(query, &body, "baud").unwrap_or(115_200) as u32).clamp(1200, 921_600);
        let data_str = key_str(query, &body, "data").unwrap_or_default();
        let bytes = data_str.as_bytes();
        let sent = if !bytes.is_empty() {
            crate::bridge::uart::uart_write_bytes(crate::bridge::uart::PIN_UART_TX_DEFAULT, baud, bytes)
        } else {
            0
        };
        let mut resp = heapless::String::<128>::new();
        let _ = write!(resp, r#"{{"status":"ok","baud":{},"bytes_sent":{}}}"#, baud, sent);
        send_json(request, response_writer, StatusCode::OK, resp.as_str()).await
    }
}

impl BridgeUartRead {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query = request.parts.query();
        let body = read_body_str(&mut request).await;
        let baud = (key_int(query, &body, "baud").unwrap_or(115_200) as u32).clamp(1200, 921_600);
        let len = (key_int(query, &body, "len").unwrap_or(64) as usize).clamp(1, 64);
        let timeout_ms = (key_int(query, &body, "timeout_ms").unwrap_or(5) as u32).clamp(1, 1000);

        let mut buf = [0u8; 64];
        let n = crate::bridge::uart::uart_read_blocking(crate::bridge::uart::PIN_UART_RX_DEFAULT, baud, &mut buf[..len], timeout_ms);

        let mut data_escaped = heapless::String::<128>::new();
        for &b in &buf[..n] {
            if b.is_ascii_graphic() || b == b' ' {
                let _ = data_escaped.push(b as char);
            } else {
                let _ = write!(data_escaped, "\\x{:02X}", b);
            }
        }

        let mut resp = heapless::String::<256>::new();
        let _ = write!(resp, r#"{{"status":"ok","bytes_read":{},"data":"{}"}}"#, n, data_escaped.as_str());
        send_json(request, response_writer, StatusCode::OK, resp.as_str()).await
    }
}

pub struct McpRpc;
impl MethodHandlerService<(), ()> for McpRpc {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error=R::Error>>(
        &self, _state: &(), (): (), _method: &str, mut request: Request<'_, R>, response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        let body = read_body_str(&mut request).await;
        let text = crate::mcp::handle_request(body.as_bytes());
        if text.is_empty() {
            let finalizer = request.body_connection.finalize().await?;
            // notifications return 204
            return StatusCode::NO_CONTENT.write_to(finalizer, response_writer).await;
        }
        send_json(request, response_writer, StatusCode::OK, text.as_str()).await
    }
}
pub struct McpHealth;
impl MethodHandlerService<(), ()> for McpHealth {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error=R::Error>>(
        &self, _state: &(), (): (), _method: &str, mut request: Request<'_, R>, response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        let _ = read_body_str(&mut request).await;
        send_json(request, response_writer, StatusCode::OK, r#"{"status":"healthy","tools":14}"#).await
    }
}

pub struct ThemesEndpoint;
impl MethodHandlerService<(), ()> for ThemesEndpoint {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        (): (),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        if method != "GET" {
            return send_json(request, response_writer, StatusCode::METHOD_NOT_ALLOWED, r#"{"error":"method_not_allowed"}"#).await;
        }
        let _ = read_body_str(&mut request).await;
        send_json(
            request,
            response_writer,
            StatusCode::OK,
            r#"[{"name":"forest","colorScheme":"dark","prefersDark":true,"isDefault":true},{"name":"emerald","colorScheme":"light","prefersDark":false,"isDefault":true},{"name":"caramellatte","colorScheme":"light","prefersDark":false,"isDefault":false}]"#,
        )
        .await
    }
}

pub struct SettingsEndpoint;

struct UiSettingsState {
    theme_mode: heapless::String<64>,
    default_light_theme: heapless::String<64>,
    default_dark_theme: heapless::String<64>,
    default_home_route: heapless::String<64>,
    active_profile: heapless::String<64>,
}

static UI_SETTINGS: embassy_sync::blocking_mutex::Mutex<
    embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex,
    core::cell::RefCell<UiSettingsState>,
> = embassy_sync::blocking_mutex::Mutex::new(core::cell::RefCell::new(UiSettingsState {
    theme_mode: heapless::String::new(),
    default_light_theme: heapless::String::new(),
    default_dark_theme: heapless::String::new(),
    default_home_route: heapless::String::new(),
    active_profile: heapless::String::new(),
}));

pub fn get_default_home_route() -> heapless::String<64> {
    UI_SETTINGS.lock(|cell| {
        let s = cell.borrow();
        if s.default_home_route.is_empty() || s.default_home_route == "/lab" || s.default_home_route == "/apps" {
            let mut r = heapless::String::new();
            let _ = r.push_str("/home");
            r
        } else {
            let mut r = s.default_home_route.clone();
            if r.len() > 1 && r.ends_with('/') {
                let _ = r.pop();
            }
            r
        }
    })
}

pub fn init_settings_from_fs() {
    let _ = crate::storage::with_user_fs(|fs| {
        let Ok(path) = littlefs2::path::PathBuf::try_from("/system/settings.json") else { return };
        if let Ok(data) = fs.read::<512>(&path) {
            if let Ok(body) = core::str::from_utf8(&data) {
                UI_SETTINGS.lock(|cell| {
                    let mut s = cell.borrow_mut();
                    if let Some(m) = key_str(None, body, "theme_mode") {
                        s.theme_mode = m;
                    }
                    if let Some(l) = key_str(None, body, "default_light_theme") {
                        s.default_light_theme = l;
                    }
                    if let Some(d) = key_str(None, body, "default_dark_theme") {
                        s.default_dark_theme = d;
                    }
                    if let Some(h) = key_str(None, body, "default_home_route") {
                        s.default_home_route = h;
                    }
                    if let Some(p) = key_str(None, body, "active_profile") {
                        s.active_profile = p;
                    }
                });
            }
        }
    });
}

fn save_settings_to_fs() {
    let mut body = heapless::String::<256>::new();
    UI_SETTINGS.lock(|cell| {
        let s = cell.borrow();
        let theme_mode = if s.theme_mode.is_empty() { "system" } else { s.theme_mode.as_str() };
        let light_theme = if s.default_light_theme.is_empty() { "emerald" } else { s.default_light_theme.as_str() };
        let dark_theme = if s.default_dark_theme.is_empty() { "forest" } else { s.default_dark_theme.as_str() };
        let home_route = if s.default_home_route.is_empty() { "/home" } else { s.default_home_route.as_str() };
        let profile = if s.active_profile.is_empty() { "Safe State" } else { s.active_profile.as_str() };
        let _ = write!(
            body,
            r#"{{"theme_mode":"{}","default_light_theme":"{}","default_dark_theme":"{}","default_home_route":"{}","active_profile":"{}"}}"#,
            theme_mode, light_theme, dark_theme, home_route, profile
        );
    });

    let _ = crate::storage::with_user_fs(|fs| {
        let Ok(dir) = littlefs2::path::PathBuf::try_from("/system") else { return };
        let _ = fs.create_dir_all(&dir);
        let Ok(path) = littlefs2::path::PathBuf::try_from("/system/settings.json") else { return };
        let _ = fs.write(&path, body.as_bytes());
    });
}

impl MethodHandlerService<(), ()> for SettingsEndpoint {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        (): (),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        if method == "GET" {
            let _ = read_body_str(&mut request).await;
            let mut body = heapless::String::<256>::new();
            UI_SETTINGS.lock(|cell| {
                let s = cell.borrow();
                let theme_mode = if s.theme_mode.is_empty() { "system" } else { s.theme_mode.as_str() };
                let light_theme = if s.default_light_theme.is_empty() { "emerald" } else { s.default_light_theme.as_str() };
                let dark_theme = if s.default_dark_theme.is_empty() { "forest" } else { s.default_dark_theme.as_str() };
                let home_route = if s.default_home_route.is_empty() { "/home" } else { s.default_home_route.as_str() };
                let profile = if s.active_profile.is_empty() { "Safe State" } else { s.active_profile.as_str() };
                let _ = write!(
                    body,
                    r#"{{"theme_mode":"{}","default_light_theme":"{}","default_dark_theme":"{}","default_home_route":"{}","active_profile":"{}"}}"#,
                    theme_mode, light_theme, dark_theme, home_route, profile
                );
            });
            send_json(request, response_writer, StatusCode::OK, body.as_str()).await
        } else if method == "POST" || method == "PUT" {
            let body = read_body_str(&mut request).await;
            UI_SETTINGS.lock(|cell| {
                let mut s = cell.borrow_mut();
                if let Some(m) = key_str(None, &body, "theme_mode") {
                    s.theme_mode = m;
                }
                if let Some(l) = key_str(None, &body, "default_light_theme") {
                    s.default_light_theme = l;
                }
                if let Some(d) = key_str(None, &body, "default_dark_theme") {
                    s.default_dark_theme = d;
                }
                if let Some(h) = key_str(None, &body, "default_home_route") {
                    s.default_home_route = h;
                }
                if let Some(p) = key_str(None, &body, "active_profile") {
                    s.active_profile = p;
                }
            });
            save_settings_to_fs();
            send_json(request, response_writer, StatusCode::OK, r#"{"status":"ok"}"#).await
        } else {
            send_json(request, response_writer, StatusCode::METHOD_NOT_ALLOWED, r#"{"error":"method_not_allowed"}"#).await
        }
    }
}

// ============ Mixed-Signal Instrument Suite Handlers ============

impl ScopeData {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        match crate::instruments::oscilloscope::capture_frame() {
            Ok(frame) => {
                let m = &frame.metrics;
                let mut b = heapless::String::<1024>::new();
                let _ = core::fmt::Write::write_fmt(
                    &mut b,
                    format_args!(
                        r#"{{"status":"ok","sample_rate":{},"ch_mask":{},"vpp_mv":[{},{},{},{}],"vrms_mv":[{},{},{},{}],"freq_hz":[{},{},{},{}],"duty":[{},{},{},{}],"samples_len":{}}}"#,
                        frame.sample_rate, frame.ch_mask,
                        m[0].vpp_mv, m[1].vpp_mv, m[2].vpp_mv, m[3].vpp_mv,
                        m[0].vrms_mv, m[1].vrms_mv, m[2].vrms_mv, m[3].vrms_mv,
                        m[0].freq_hz, m[1].freq_hz, m[2].freq_hz, m[3].freq_hz,
                        m[0].duty_percent, m[1].duty_percent, m[2].duty_percent, m[3].duty_percent,
                        frame.sample_count
                    ),
                );
                send_json(request, response_writer, StatusCode::OK, b.as_str()).await
            }
            Err(e) => {
                let mut b = heapless::String::<128>::new();
                let _ = core::fmt::Write::write_fmt(&mut b, format_args!(r#"{{"error":"{}"}}"#, e));
                send_json(request, response_writer, StatusCode::INTERNAL_SERVER_ERROR, b.as_str()).await
            }
        }
    }
}

impl ScopeStart {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query = request.parts.query();
        let body = read_body_str(&mut request).await;
        let sample_rate = key_int(query, &body, "sample_rate").map(|v| v as u32).unwrap_or(125_000);
        let ch_mask = key_int(query, &body, "ch_mask").map(|v| v as u8).unwrap_or(0x0F);
        let trigger_ch = key_int(query, &body, "trigger_ch").map(|v| v as u8).unwrap_or(0);
        let threshold_mv = key_int(query, &body, "threshold_mv").map(|v| v as u16).unwrap_or(1650);

        let cfg = crate::instruments::oscilloscope::ScopeConfig {
            sample_rate,
            ch_mask,
            trigger_ch,
            trigger_type: crate::instruments::oscilloscope::TriggerType::Rising,
            threshold_mv,
        };

        match crate::instruments::oscilloscope::start(cfg) {
            Ok(()) => send_json(request, response_writer, StatusCode::OK, r#"{"status":"ok","running":true}"#).await,
            Err(e) => {
                let mut b = heapless::String::<128>::new();
                let _ = core::fmt::Write::write_fmt(&mut b, format_args!(r#"{{"error":"{}"}}"#, e));
                send_json(request, response_writer, StatusCode::CONFLICT, b.as_str()).await
            }
        }
    }
}

impl ScopeStop {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        crate::instruments::oscilloscope::stop();
        send_json(request, response_writer, StatusCode::OK, r#"{"status":"stopped","running":false}"#).await
    }
}

impl GeneratorStart {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query = request.parts.query();
        let body = read_body_str(&mut request).await;
        let pin = key_int(query, &body, "pin").map(|v| v as u32).unwrap_or(12);
        let freq_hz = key_int(query, &body, "freq").or_else(|| key_int(query, &body, "frequency")).map(|v| v as u32).unwrap_or(1000);
        let duty_percent = key_int(query, &body, "duty").or_else(|| key_int(query, &body, "duty_cycle")).map(|v| v as u8).unwrap_or(50);
        let burst_count = key_int(query, &body, "burst").or_else(|| key_int(query, &body, "count")).map(|v| v as u32).unwrap_or(0);

        let cfg = crate::instruments::generator::GeneratorConfig {
            pin,
            freq_hz,
            duty_percent,
            burst_count,
        };

        match crate::instruments::generator::start(cfg) {
            Ok(()) => {
                let mut b = heapless::String::<192>::new();
                let _ = core::fmt::Write::write_fmt(
                    &mut b,
                    format_args!(
                        r#"{{"status":"ok","pin":{},"freq":{},"duty":{},"burst":{}}}"#,
                        pin, freq_hz, duty_percent, burst_count
                    ),
                );
                send_json(request, response_writer, StatusCode::OK, b.as_str()).await
            }
            Err(e) => {
                let mut b = heapless::String::<128>::new();
                let _ = core::fmt::Write::write_fmt(&mut b, format_args!(r#"{{"error":"{}"}}"#, e));
                send_json(request, response_writer, StatusCode::CONFLICT, b.as_str()).await
            }
        }
    }
}

impl GeneratorStop {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        crate::instruments::generator::stop();
        send_json(request, response_writer, StatusCode::OK, r#"{"status":"stopped","active":false}"#).await
    }
}

impl GeneratorStatus {
    async fn run<R, W>(request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let st = crate::instruments::generator::get_status();
        let mut b = heapless::String::<192>::new();
        let _ = core::fmt::Write::write_fmt(
            &mut b,
            format_args!(
                r#"{{"status":"ok","active":{},"pin":{},"freq":{},"duty":{},"burst":{},"running":{}}}"#,
                st.active, st.pin, st.freq_hz, st.duty_percent, st.burst_count, st.running
            ),
        );
        send_json(request, response_writer, StatusCode::OK, b.as_str()).await
    }
}

impl LedPattern {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query = request.parts.query();
        let body = read_body_str(&mut request).await;
        let pin = key_int(query, &body, "pin").map(|v| v as u32).unwrap_or(15);
        let pattern_str = key_str(query, &body, "pattern").unwrap_or_else(|| {
            let mut s = heapless::String::new();
            let _ = s.push_str("rainbow");
            s
        });
        let count = key_int(query, &body, "count").map(|v| v as usize).unwrap_or(8);

        match crate::instruments::ws2812::drive_pattern(pin, pattern_str.as_str(), count) {
            Ok(()) => {
                let mut b = heapless::String::<160>::new();
                let _ = core::fmt::Write::write_fmt(
                    &mut b,
                    format_args!(
                        r#"{{"status":"ok","pin":{},"pattern":"{}","count":{}}}"#,
                        pin, pattern_str.as_str(), count
                    ),
                );
                send_json(request, response_writer, StatusCode::OK, b.as_str()).await
            }
            Err(e) => {
                let mut b = heapless::String::<128>::new();
                let _ = core::fmt::Write::write_fmt(&mut b, format_args!(r#"{{"error":"{}"}}"#, e));
                send_json(request, response_writer, StatusCode::BAD_REQUEST, b.as_str()).await
            }
        }
    }
}

impl LedAnalyze {
    async fn run<R, W>(request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query = request.parts.query();
        let pin = query_val(query, "pin").and_then(|v| v.parse::<u32>().ok()).unwrap_or(15);

        match crate::instruments::ws2812::analyze_timing(pin) {
            Ok(rep) => {
                let mut b = heapless::String::<256>::new();
                let _ = core::fmt::Write::write_fmt(
                    &mut b,
                    format_args!(
                        r#"{{"status":"ok","pin":{},"t0h_ns":{},"t1h_ns":{},"t0l_ns":{},"t1l_ns":{},"reset_us":{},"result":"{}","tolerances":{{"t0h_min":200,"t0h_max":500,"t1h_min":550,"t1h_max":850}}}}"#,
                        rep.pin, rep.t0h_ns, rep.t1h_ns, rep.t0l_ns, rep.t1l_ns, rep.reset_us,
                        if rep.passed { "PASS" } else { "FAIL" }
                    ),
                );
                send_json(request, response_writer, StatusCode::OK, b.as_str()).await
            }
            Err(e) => {
                let mut b = heapless::String::<128>::new();
                let _ = core::fmt::Write::write_fmt(&mut b, format_args!(r#"{{"error":"{}"}}"#, e));
                send_json(request, response_writer, StatusCode::INTERNAL_SERVER_ERROR, b.as_str()).await
            }
        }
    }
}

impl GlitchArm {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query = request.parts.query();
        let body = read_body_str(&mut request).await;
        let glitch_pin = key_int(query, &body, "pin").map(|v| v as u32).unwrap_or(20);
        let trigger_pin = key_int(query, &body, "trig_pin").or_else(|| key_int(query, &body, "trigger_pin")).map(|v| v as u32).unwrap_or(14);
        let delay_ns = key_int(query, &body, "delay_ns").map(|v| v as u32).unwrap_or(1250);
        let width_ns = key_int(query, &body, "width_ns").map(|v| v as u32).unwrap_or(33);
        let edge_str = key_str(query, &body, "edge");
        let edge = if let Some(e) = edge_str {
            if e.as_str().eq_ignore_ascii_case("falling") {
                crate::instruments::glitch::TriggerEdge::Falling
            } else {
                crate::instruments::glitch::TriggerEdge::Rising
            }
        } else {
            crate::instruments::glitch::TriggerEdge::Rising
        };

        let cfg = crate::instruments::glitch::GlitchConfig {
            glitch_pin,
            trigger_pin,
            edge,
            delay_ns,
            width_ns,
            polarity: crate::instruments::glitch::GlitchPolarity::ActiveLow,
        };

        match crate::instruments::glitch::arm(cfg) {
            Ok(()) => {
                let mut b = heapless::String::<192>::new();
                let _ = core::fmt::Write::write_fmt(
                    &mut b,
                    format_args!(
                        r#"{{"status":"armed","pin":{},"trig_pin":{},"delay_ns":{},"width_ns":{}}}"#,
                        glitch_pin, trigger_pin, delay_ns, width_ns
                    ),
                );
                send_json(request, response_writer, StatusCode::OK, b.as_str()).await
            }
            Err(e) => {
                let mut b = heapless::String::<128>::new();
                let _ = core::fmt::Write::write_fmt(&mut b, format_args!(r#"{{"error":"{}"}}"#, e));
                send_json(request, response_writer, StatusCode::CONFLICT, b.as_str()).await
            }
        }
    }
}

impl GlitchDisarm {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        crate::instruments::glitch::disarm();
        send_json(request, response_writer, StatusCode::OK, r#"{"status":"disarmed","armed":false}"#).await
    }
}

impl GlitchStatus {
    async fn run<R, W>(request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let st = crate::instruments::glitch::get_status();
        let mut b = heapless::String::<256>::new();
        let _ = core::fmt::Write::write_fmt(
            &mut b,
            format_args!(
                r#"{{"status":"ok","armed":{},"pin":{},"trig_pin":{},"delay_ns":{},"width_ns":{},"triggered":{},"trigger_count":{}}}"#,
                st.armed, st.glitch_pin, st.trigger_pin, st.delay_ns, st.width_ns, st.triggered, st.trigger_count
            ),
        );
        send_json(request, response_writer, StatusCode::OK, b.as_str()).await
    }
}

pub struct FirmwareStatus;
pub struct FirmwareRollback;
pub struct FirmwareOtaStream;

impl FirmwareStatus {
    async fn run<R, W>(request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let (bt, _) = crate::boot::get_boot_state();
        let cur = crate::boot::current_executing_slot();
        let mut b = heapless::String::<256>::new();
        let cand = match bt.get_candidate_slot() {
            Some(agentla_boot_proto::SlotId::SlotA) => "\"SlotA\"",
            Some(agentla_boot_proto::SlotId::SlotB) => "\"SlotB\"",
            None => "null",
        };
        let _ = core::fmt::Write::write_fmt(
            &mut b,
            format_args!(
                r#"{{"active_slot":"{:?}","current_slot":"{:?}","candidate_slot":{},"state":"{:?}","boot_attempts":{},"slot_a_size":{},"slot_b_size":{},"firmware":"AgentLA v1.0.0-rust"}}"#,
                bt.get_active_slot(), cur, cand, bt.get_state(), bt.boot_attempts, bt.slot_a_size, bt.slot_b_size
            ),
        );
        send_json(request, response_writer, StatusCode::OK, b.as_str()).await
    }
}

impl FirmwareRollback {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        let cur = crate::boot::current_executing_slot();
        let target = cur.other();
        let target_xip = target.xip_address();
        let sp = unsafe { *(target_xip as *const u32) };
        let pc = unsafe { *((target_xip + 4) as *const u32) };
        let valid = (sp >= 0x2000_0000 && sp <= 0x2008_2000) && (pc >= target_xip && pc < target_xip + 1200 * 1024);
        if !valid {
            return send_json(request, response_writer, StatusCode::CONFLICT, r#"{"error":"alternate_slot_empty_or_corrupt"}"#).await;
        }
        if crate::boot::ota_rollback().is_err() {
            return send_json(request, response_writer, StatusCode::INTERNAL_SERVER_ERROR, r#"{"error":"rollback_failed"}"#).await;
        }
        let _ = send_json(request, response_writer, StatusCode::OK, r#"{"status":"rollback_initiated","rebooting":true}"#).await;
        embassy_time::Timer::after(embassy_time::Duration::from_millis(100)).await;
        cortex_m::peripheral::SCB::sys_reset()
    }
}

impl SystemBootsel {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        defmt::warn!("Rebooting into USB BOOTSEL mode via ROM call...");
        let res = send_json(request, response_writer, StatusCode::OK, r#"{"status":"bootsel_initiated"}"#).await;
        embassy_time::Timer::after(embassy_time::Duration::from_millis(50)).await;
        embassy_rp::rom_data::reset_to_usb_boot(0, 0);
        res
    }
}

impl SystemReboot {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        defmt::warn!("Rebooting core via SCB sys_reset...");
        let _ = send_json(request, response_writer, StatusCode::OK, r#"{"status":"rebooting"}"#).await;
        embassy_time::Timer::after(embassy_time::Duration::from_millis(50)).await;
        cortex_m::peripheral::SCB::sys_reset()
    }
}

impl SystemPinsReleaseAll {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let _ = read_body_str(&mut request).await;
        crate::pin_owner::quiesce_and_reset();
        send_json(request, response_writer, StatusCode::OK, r#"{"status":"all_pins_released"}"#).await
    }
}

impl SystemPinsToggle {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let body = read_body_str(&mut request).await;
        let pin = key_u32(request.parts.query(), body.as_str(), "pin").unwrap_or(255);
        let state = key_str(request.parts.query(), body.as_str(), "state").unwrap_or_else(|| heapless::String::try_from("input").unwrap());

        if pin >= 30 {
            return send_json(request, response_writer, StatusCode::BAD_REQUEST, r#"{"error":"invalid_pin"}"#).await;
        }

        match state.as_str() {
            "high" => {
                rp_pac::PADS_BANK0.gpio(pin as usize).modify(|w| {
                    w.set_iso(false);
                    w.set_ie(true);
                    w.set_od(false);
                });
                rp_pac::SIO.gpio_out(0).value_set().write_value(1u32 << pin);
                rp_pac::SIO.gpio_oe(0).value_set().write_value(1u32 << pin);
                rp_pac::IO_BANK0.gpio(pin as usize).ctrl().write(|w| {
                    w.set_funcsel(5);
                });
            }
            "low" => {
                rp_pac::PADS_BANK0.gpio(pin as usize).modify(|w| {
                    w.set_iso(false);
                    w.set_ie(true);
                    w.set_od(false);
                });
                rp_pac::SIO.gpio_out(0).value_clr().write_value(1u32 << pin);
                rp_pac::SIO.gpio_oe(0).value_set().write_value(1u32 << pin);
                rp_pac::IO_BANK0.gpio(pin as usize).ctrl().write(|w| {
                    w.set_funcsel(5);
                });
            }
            _ => {
                crate::pin_owner::quiesce_pin(pin);
            }
        }

        let mut resp = heapless::String::<128>::new();
        let _ = core::fmt::Write::write_fmt(&mut resp, format_args!(r#"{{"status":"ok","pin":{},"state":"{}"}}"#, pin, state.as_str()));
        send_json(request, response_writer, StatusCode::OK, resp.as_str()).await
    }
}

impl SystemFsFormat {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let body = read_body_str(&mut request).await;
        let part = key_str(request.parts.query(), body.as_str(), "partition").unwrap_or_else(|| heapless::String::try_from("user").unwrap());
        match crate::storage::erase_and_format_partition(part.as_str()) {
            Ok(()) => {
                let mut resp = heapless::String::<128>::new();
                let _ = core::fmt::Write::write_fmt(&mut resp, format_args!(r#"{{"status":"formatted","partition":"{}"}}"#, part.as_str()));
                send_json(request, response_writer, StatusCode::OK, resp.as_str()).await
            }
            Err(_) => {
                send_json(request, response_writer, StatusCode::INTERNAL_SERVER_ERROR, r#"{"error":"format_failed"}"#).await
            }
        }
    }
}

impl picoserve::routing::MethodHandlerService<(), ()> for FirmwareOtaStream {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        (): (),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        if method != "POST" {
            return send_json(
                request,
                response_writer,
                StatusCode::METHOD_NOT_ALLOWED,
                r#"{"error":"method_not_allowed"}"#,
            )
            .await;
        }

        let mut reader = request
            .body_connection
            .body()
            .reader()
            .with_different_timeout(embassy_time::Duration::from_secs(60));

        // 1. Read 64-byte FirmwareHeader
        let mut hdr_bytes = [0u8; agentla_boot_proto::FirmwareHeader::SIZE];
        let mut hdr_read = 0;
        while hdr_read < hdr_bytes.len() {
            match reader.read(&mut hdr_bytes[hdr_read..]).await {
                Ok(0) => break,
                Ok(n) => hdr_read += n,
                Err(_) => {
                    return send_json(
                        request,
                        response_writer,
                        StatusCode::BAD_REQUEST,
                        r#"{"error":"stream_read_error"}"#,
                    )
                    .await;
                }
            }
        }

        if hdr_read < hdr_bytes.len() {
            return send_json(
                request,
                response_writer,
                StatusCode::BAD_REQUEST,
                r#"{"error":"incomplete_header"}"#,
            )
            .await;
        }

        let hdr = match agentla_boot_proto::FirmwareHeader::from_bytes(&hdr_bytes) {
            Some(h) => h,
            None => {
                return send_json(
                    request,
                    response_writer,
                    StatusCode::BAD_REQUEST,
                    r#"{"error":"invalid_header_or_crc"}"#,
                )
                .await;
            }
        };

        if !hdr.matches_board(b"PICO2") && !hdr.matches_board(b"BAKUPROB") {
            return send_json(
                request,
                response_writer,
                StatusCode::BAD_REQUEST,
                r#"{"error":"board_mismatch"}"#,
            )
            .await;
        }

        if hdr.payload_size as usize > crate::storage::SLOT_SIZE_BYTES {
            return send_json(
                request,
                response_writer,
                StatusCode::PAYLOAD_TOO_LARGE,
                r#"{"error":"payload_too_large"}"#,
            )
            .await;
        }

        let cur_slot = crate::boot::current_executing_slot();
        let target_slot = cur_slot.other();
        let slot_base = target_slot.flash_offset();

        // 2. Stream payload chunks, erase 4 KB sectors on demand, write in 256-byte pages
        let mut page_buf = [0xFFu8; 256];
        let mut page_idx = 0;
        let mut written_bytes = 0u32;
        let mut crc_val = 0xFFFF_FFFFu32;

        let mut chunk_buf = [0u8; 512];
        loop {
            let n = match reader.read(&mut chunk_buf).await {
                Ok(0) => break,
                Ok(n) => n,
                Err(_) => {
                    return send_json(
                        request,
                        response_writer,
                        StatusCode::BAD_REQUEST,
                        r#"{"error":"payload_read_error"}"#,
                    )
                    .await;
                }
            };

            for &b in &chunk_buf[..n] {
                // Erase 4 KB sector right before writing its first byte
                if written_bytes % 4096 == 0 && page_idx == 0 {
                    let sec_start = slot_base + written_bytes;
                    if crate::storage::flash_slot_erase(sec_start, sec_start + 4096).is_err() {
                        return send_json(
                            request,
                            response_writer,
                            StatusCode::INTERNAL_SERVER_ERROR,
                            r#"{"error":"flash_erase_failed"}"#,
                        )
                        .await;
                    }
                }

                // CRC update
                crc_val ^= b as u32;
                for _ in 0..8 {
                    if crc_val & 1 != 0 {
                        crc_val = (crc_val >> 1) ^ 0xEDB8_8320;
                    } else {
                        crc_val >>= 1;
                    }
                }

                page_buf[page_idx] = b;
                page_idx += 1;
                if page_idx == 256 {
                    let page_offset = slot_base + written_bytes;
                    if crate::storage::flash_slot_write(page_offset, &page_buf).is_err() {
                        return send_json(
                            request,
                            response_writer,
                            StatusCode::INTERNAL_SERVER_ERROR,
                            r#"{"error":"flash_write_failed"}"#,
                        )
                        .await;
                    }
                    written_bytes += 256;
                    page_buf.fill(0xFF);
                    page_idx = 0;
                }
            }
        }

        // Write residual partial page
        if page_idx > 0 {
            let page_offset = slot_base + written_bytes;
            if crate::storage::flash_slot_write(page_offset, &page_buf).is_err() {
                return send_json(
                    request,
                    response_writer,
                    StatusCode::INTERNAL_SERVER_ERROR,
                    r#"{"error":"flash_write_residual_failed"}"#,
                )
                .await;
            }
            written_bytes += page_idx as u32;
        }

        let computed_crc = !crc_val;
        if written_bytes != hdr.payload_size || computed_crc != hdr.payload_crc {
            return send_json(
                request,
                response_writer,
                StatusCode::BAD_REQUEST,
                r#"{"error":"payload_crc_mismatch"}"#,
            )
            .await;
        }

        // 4. Update Boot Table to stage candidate slot
        if crate::boot::stage_candidate(target_slot, hdr.payload_size, hdr.payload_crc).is_err() {
            return send_json(
                request,
                response_writer,
                StatusCode::INTERNAL_SERVER_ERROR,
                r#"{"error":"boot_table_update_failed"}"#,
            )
            .await;
        }

        let mut resp = heapless::String::<128>::new();
        let _ = core::fmt::Write::write_fmt(
            &mut resp,
            format_args!(
                r#"{{"status":"staged","slot":"{:?}","bytes":{},"crc":"0x{:08X}","rebooting":true}}"#,
                target_slot, written_bytes, computed_crc
            ),
        );

        let res = send_json(request, response_writer, StatusCode::OK, resp.as_str()).await;

        embassy_time::Timer::after(embassy_time::Duration::from_millis(100)).await;
        cortex_m::peripheral::SCB::sys_reset();

        res
    }
}

// ---------------------------------------------------------------------------
// In-Browser Web Serial Terminal WebSocket Handler: /api/v1/terminal/ws
// ---------------------------------------------------------------------------

pub struct TerminalWebSocket;

struct TerminalWsCallback {
    tx_pin: u32,
    _rx_pin: u32,
    baud: u32,
    slot: pin_owner::SlotId,
}

impl picoserve::response::ws::WebSocketCallback for TerminalWsCallback {
    async fn run<R: picoserve::io::Read, W: picoserve::io::Write<Error = R::Error>>(
        self,
        mut rx: picoserve::response::ws::SocketRx<R>,
        mut tx: picoserve::response::ws::SocketTx<W>,
    ) -> Result<(), W::Error> {
        // Drain existing 2 KiB backlog first so client sees boot messages
        let mut backlog_buf = [0u8; 256];
        loop {
            let n = crate::bridge::uart::drain_rx_bytes(&mut backlog_buf);
            if n == 0 {
                break;
            }
            let _ = tx.send_binary(&backlog_buf[..n]).await;
        }
        let _ = tx.flush().await;

        let mut msg_buf = [0u8; 512];
        let mut uart_rx_buf = [0u8; 128];

        loop {
            match rx.next_message(&mut msg_buf, crate::bridge::uart::wait_rx_data()).await {
                Ok(picoserve::futures::Either::First(Ok(picoserve::response::ws::Message::Text(s)))) => {
                    crate::bridge::uart::uart_write_bytes(self.tx_pin, self.baud, s.as_bytes());
                }
                Ok(picoserve::futures::Either::First(Ok(picoserve::response::ws::Message::Binary(b)))) => {
                    crate::bridge::uart::uart_write_bytes(self.tx_pin, self.baud, b);
                }
                Ok(picoserve::futures::Either::Second(())) => {
                    // Incoming UART RX byte(s) arrived in ring buffer
                    let n = crate::bridge::uart::drain_rx_bytes(&mut uart_rx_buf);
                    if n > 0 {
                        let _ = tx.send_binary(&uart_rx_buf[..n]).await;
                        let _ = tx.flush().await;
                    }
                }
                _ => break, // Connection dropped or closed
            }
        }

        let _ = pin_owner::release(self.slot);
        Ok(())
    }
}

impl picoserve::routing::MethodHandlerService<()> for TerminalWebSocket {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        if method != "GET" {
            return send_json(request, response_writer, StatusCode::METHOD_NOT_ALLOWED, r#"{"error":"method_not_allowed"}"#).await;
        }

        let query = request.parts.query();
        let baud = key_u32(query, "", "baud").unwrap_or(115_200).clamp(1200, 921_600);
        let tx_pin = key_u32(query, "", "tx").unwrap_or(crate::bridge::uart::PIN_UART_TX_DEFAULT);
        let rx_pin = key_u32(query, "", "rx").unwrap_or(crate::bridge::uart::PIN_UART_RX_DEFAULT);

        let pins = [tx_pin, rx_pin];
        let claim_res = pin_owner::claim(pin_owner::Driver::Adapter, &pins, pin_owner::CAP_UART);
        let slot = match claim_res {
            Ok(s) => s,
            Err(_) => {
                return send_json(request, response_writer, StatusCode::CONFLICT, r#"{"error":"pins_busy"}"#).await;
            }
        };

        crate::bridge::uart::init_uart_pins(tx_pin, rx_pin);

        use picoserve::extract::FromRequest;
        let upgrade = match picoserve::response::ws::WebSocketUpgrade::from_request(
            &(),
            request.parts,
            request.body_connection.body(),
        ).await {
            Ok(u) => u,
            Err(e) => {
                let _ = pin_owner::release(slot);
                return e.write_to(request.body_connection.finalize().await?, response_writer).await;
            }
        };

        let callback = TerminalWsCallback { tx_pin, _rx_pin: rx_pin, baud, slot };
        upgrade.on_upgrade(callback).write_to(request.body_connection.finalize().await?, response_writer).await
    }
}

// ---------------------------------------------------------------------------
// In-Browser CAN Bus Analyzer WebSocket Handler: /api/v1/can/ws
// ---------------------------------------------------------------------------

pub struct CanWebSocket;

fn parse_can_json(s: &str) -> Option<crate::bridge::can_gsusb::CanWsFrame> {
    let id_val = key_u32(None, s, "id")?;
    let ext_val = s.contains(r#""ext":true"#) || s.contains(r#""ext": true"#);
    let rtr_val = s.contains(r#""rtr":true"#) || s.contains(r#""rtr": true"#);
    let dlc_val = key_u32(None, s, "dlc").unwrap_or(8).min(8) as u8;

    let mut data = [0u8; 8];
    if let Some(data_str) = key_str(None, s, "data") {
        let bytes_str = data_str.as_str();
        let mut idx = 0;
        let mut i = 0;
        while i + 1 < bytes_str.len() && idx < 8 {
            if let Ok(byte) = u8::from_str_radix(&bytes_str[i..i + 2], 16) {
                data[idx] = byte;
                idx += 1;
            }
            i += 2;
        }
    }
    Some(crate::bridge::can_gsusb::CanWsFrame {
        id: id_val,
        ext: ext_val,
        rtr: rtr_val,
        dlc: dlc_val,
        data,
        ts_us: (embassy_time::Instant::now().as_micros() & 0xFFFF_FFFF) as u32,
    })
}

struct CanWsCallback;

impl picoserve::response::ws::WebSocketCallback for CanWsCallback {
    async fn run<R: picoserve::io::Read, W: picoserve::io::Write<Error = R::Error>>(
        self,
        mut rx: picoserve::response::ws::SocketRx<R>,
        mut tx: picoserve::response::ws::SocketTx<W>,
    ) -> Result<(), W::Error> {
        let mut msg_buf = [0u8; 256];
        let mut json_buf = heapless::String::<256>::new();

        loop {
            match rx.next_message(&mut msg_buf, crate::bridge::can_gsusb::wait_can_rx()).await {
                Ok(picoserve::futures::Either::First(Ok(picoserve::response::ws::Message::Text(s)))) => {
                    if let Some(frame) = parse_can_json(s) {
                        crate::bridge::can_gsusb::transmit_can_ws_frame(&frame);
                    }
                }
                Ok(picoserve::futures::Either::First(Ok(picoserve::response::ws::Message::Binary(b)))) => {
                    if let Ok(s) = core::str::from_utf8(b) {
                        if let Some(frame) = parse_can_json(s) {
                            crate::bridge::can_gsusb::transmit_can_ws_frame(&frame);
                        }
                    }
                }
                Ok(picoserve::futures::Either::Second(())) => {
                    while let Some(frame) = crate::bridge::can_gsusb::pop_can_rx_frame() {
                        json_buf.clear();
                        let mut hex_data = heapless::String::<17>::new();
                        let dlc = (frame.dlc as usize).min(8);
                        for b in &frame.data[..dlc] {
                            let _ = core::write!(hex_data, "{:02X}", b);
                        }
                        let _ = core::write!(
                            json_buf,
                            r#"{{"id":{},"ext":{},"rtr":{},"dlc":{},"data":"{}","ts":{}}}"#,
                            frame.id,
                            frame.ext,
                            frame.rtr,
                            frame.dlc,
                            hex_data.as_str(),
                            frame.ts_us
                        );
                        let _ = tx.send_text(json_buf.as_str()).await;
                    }
                    let _ = tx.flush().await;
                }
                _ => break,
            }
        }
        Ok(())
    }
}

impl picoserve::routing::MethodHandlerService<()> for CanWebSocket {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        if method != "GET" {
            return send_json(
                request,
                response_writer,
                StatusCode::METHOD_NOT_ALLOWED,
                r#"{"error":"method_not_allowed"}"#,
            )
            .await;
        }

        crate::bridge::can_gsusb::adapter_can_start();

        use picoserve::extract::FromRequest;
        let upgrade = match picoserve::response::ws::WebSocketUpgrade::from_request(
            &(),
            request.parts,
            request.body_connection.body(),
        )
        .await
        {
            Ok(u) => u,
            Err(e) => {
                return e
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        upgrade
            .on_upgrade(CanWsCallback)
            .write_to(request.body_connection.finalize().await?, response_writer)
            .await
    }
}

pub struct LedWebSocket;

struct LedWsCallback;

impl picoserve::response::ws::WebSocketCallback for LedWsCallback {
    async fn run<R: picoserve::io::Read, W: picoserve::io::Write<Error = R::Error>>(
        self,
        mut rx: picoserve::response::ws::SocketRx<R>,
        mut tx: picoserve::response::ws::SocketTx<W>,
    ) -> Result<(), W::Error> {
        let mut msg_buf = [0u8; 1536];
        let mut claimed_slot: Option<crate::pin_owner::SlotId> = None;
        let mut current_data_pin = 22u32;
        let mut current_clk_pin: Option<u32> = None;
        let mut is_clocked = false;
        let mut reset_us = 65u32;

        if let Ok(slot) = crate::pin_owner::claim(crate::pin_owner::Driver::Ws2812, &[current_data_pin], crate::pin_owner::CAP_PIO) {
            claimed_slot = Some(slot);
        }

        loop {
            match rx.next_message(&mut msg_buf, core::future::pending::<()>()).await {
                Ok(picoserve::futures::Either::First(Ok(picoserve::response::ws::Message::Text(s)))) => {
                    if s.contains(r#""cmd":"config""#) || s.contains(r#""cmd": "config""#) {
                        let req_pin = key_u32(None, s, "data_pin").unwrap_or(current_data_pin);
                        let req_clk = key_u32(None, s, "clk_pin");
                        let req_proto = key_str(None, s, "protocol");
                        let req_reset = key_u32(None, s, "reset_us").unwrap_or(65);
                        reset_us = req_reset;

                        let wants_clocked = req_clk.is_some() || req_proto.as_ref().map(|p| p.contains("apa102") || p.contains("dotstar") || p.contains("ws2801") || p.contains("sk9822")).unwrap_or(false);

                        if let Some(slot) = claimed_slot.take() {
                            let _ = crate::pin_owner::release(slot);
                        }

                        let claim_res = if wants_clocked {
                            if let Some(clk) = req_clk {
                                current_clk_pin = Some(clk);
                                is_clocked = true;
                                crate::pin_owner::claim(crate::pin_owner::Driver::Ws2812, &[req_pin, clk], crate::pin_owner::CAP_PIO)
                            } else {
                                is_clocked = false;
                                current_clk_pin = None;
                                crate::pin_owner::claim(crate::pin_owner::Driver::Ws2812, &[req_pin], crate::pin_owner::CAP_PIO)
                            }
                        } else {
                            is_clocked = false;
                            current_clk_pin = None;
                            crate::pin_owner::claim(crate::pin_owner::Driver::Ws2812, &[req_pin], crate::pin_owner::CAP_PIO)
                        };

                        match claim_res {
                            Ok(slot) => {
                                claimed_slot = Some(slot);
                                current_data_pin = req_pin;
                                let mut ack = heapless::String::<128>::new();
                                let _ = core::write!(ack, r#"{{"status":"ok","data_pin":{},"clk_pin":{:?},"clocked":{}}}"#, current_data_pin, current_clk_pin, is_clocked);
                                let _ = tx.send_text(ack.as_str()).await;
                                let _ = tx.flush().await;
                            }
                            Err(e) => {
                                let mut err = heapless::String::<128>::new();
                                let _ = core::write!(err, r#"{{"status":"error","message":"pin conflict: {:?}"}}"#, e);
                                let _ = tx.send_text(err.as_str()).await;
                                let _ = tx.flush().await;
                            }
                        }
                    }
                }
                Ok(picoserve::futures::Either::First(Ok(picoserve::response::ws::Message::Binary(bytes)))) => {
                    if is_clocked {
                        if let Some(clk) = current_clk_pin {
                            let _ = crate::bridge::ws2812::transmit_clocked_bytes(current_data_pin, clk, bytes);
                        }
                    } else {
                        let _ = crate::bridge::ws2812::transmit_raw_bytes_pin(current_data_pin, bytes, reset_us);
                    }
                    let _ = tx.send_binary(&[0x06]).await;
                    let _ = tx.flush().await;
                }
                _ => break,
            }
        }

        if let Some(slot) = claimed_slot.take() {
            let _ = crate::pin_owner::release(slot);
        }
        Ok(())
    }
}

impl picoserve::routing::MethodHandlerService<()> for LedWebSocket {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        if method != "GET" {
            return send_json(
                request,
                response_writer,
                StatusCode::METHOD_NOT_ALLOWED,
                r#"{"error":"method_not_allowed"}"#,
            )
            .await;
        }

        use picoserve::extract::FromRequest;
        let upgrade = match picoserve::response::ws::WebSocketUpgrade::from_request(
            &(),
            request.parts,
            request.body_connection.body(),
        )
        .await
        {
            Ok(u) => u,
            Err(e) => {
                return e
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        upgrade
            .on_upgrade(LedWsCallback)
            .write_to(request.body_connection.finalize().await?, response_writer)
            .await
    }
}

pub struct SystemTelemetry;

impl picoserve::routing::MethodHandlerService<()> for SystemTelemetry {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        method: &str,
        request: Request<'_, R>,
        response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        if method != "GET" {
            return send_json(request, response_writer, StatusCode::METHOD_NOT_ALLOWED, r#"{"error":"method_not_allowed"}"#).await;
        }
        let mut buf = heapless::String::<2048>::new();
        crate::telemetry::format_telemetry_json(&mut buf);
        send_json(request, response_writer, StatusCode::OK, buf.as_str()).await
    }
}

pub struct SystemTelemetryStream;

pub struct SystemClockHandler;

impl picoserve::routing::MethodHandlerService<()> for SystemClockHandler {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        if method == "GET" {
            let cur_mhz = crate::benchmark::get_sys_clk_hz() / 1_000_000;
            let mut buf = heapless::String::<128>::new();
            let _ = core::fmt::Write::write_fmt(
                &mut buf,
                format_args!(r#"{{"current_mhz":{},"available_mhz":[150,300,500]}}"#, cur_mhz),
            );
            return send_json(request, response_writer, StatusCode::OK, buf.as_str()).await;
        }

        if method == "POST" {
            let body = read_body_str(&mut request).await;
            let mhz = key_u32(request.parts.query(), body.as_str(), "mhz").unwrap_or(0);
            if mhz != 150 && mhz != 300 && mhz != 500 {
                return send_json(
                    request,
                    response_writer,
                    StatusCode::BAD_REQUEST,
                    r#"{"error":"invalid_mhz","available_mhz":[150,300,500]}"#,
                ).await;
            }

            let mut resp = heapless::String::<128>::new();
            let _ = core::fmt::Write::write_fmt(
                &mut resp,
                format_args!(r#"{{"status":"rebooting","mhz":{}}}"#, mhz),
            );
            let finalizer = request.body_connection.finalize().await?;
            let _sent = (
                StatusCode::OK,
                [("Content-Type", "application/json"), ("Connection", "close")],
                resp.as_str(),
            )
                .write_to(finalizer, response_writer)
                .await;

            // Allow the CDC-NCM network stack to transmit response and close TCP connection
            embassy_time::Timer::after(embassy_time::Duration::from_millis(150)).await;

            // 1. Arm WATCHDOG.scratch4 for immediate warm reset recognition
            rp_pac::WATCHDOG.scratch4().write_value(0xC10C_0000 | (mhz & 0xFFFF));

            // 2. Temporarily downclock in RAM to safe 150 MHz so RP2350 Bootrom flash erase/write is 100% stable
            let _ = crate::benchmark::apply_system_clock_in_ram(crate::benchmark::PROFILE_150);

            // 3. Commit to BootTable in flash so it persists across hard power cycles
            let (mut table, _) = crate::boot::get_boot_state();
            table.set_boot_clock_mhz(mhz);
            let _ = crate::boot::commit_boot_table(&mut table);

            defmt::info!("Clock change committed: {} MHz. Resetting chip...", mhz);

            embassy_time::Timer::after(embassy_time::Duration::from_millis(50)).await;
            cortex_m::peripheral::SCB::sys_reset();
            return _sent;
        }

        send_json(request, response_writer, StatusCode::METHOD_NOT_ALLOWED, r#"{"error":"method_not_allowed"}"#).await
    }
}

pub struct SystemInfoHandler;

impl picoserve::routing::MethodHandlerService<()> for SystemInfoHandler {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        _method: &str,
        request: Request<'_, R>,
        response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        let cur_mhz = crate::benchmark::get_sys_clk_hz() / 1_000_000;
        let mut buf = heapless::String::<256>::new();
        let _ = core::fmt::Write::write_fmt(
            &mut buf,
            format_args!(
                r#"{{"architecture":"ARM Cortex-M33","chip":"RP2350A","clock_mhz":{},"cores":2,"flash_bytes":{},"sram_bytes":532480,"littlefs_offset":"0x{:08x}","firmware":"AgentLA v1.0.0-rust","status":"healthy"}}"#,
                cur_mhz,
                crate::storage::FLASH_SIZE_BYTES,
                crate::storage::SYSTEM_FS_OFFSET,
            ),
        );
        send_json(request, response_writer, StatusCode::OK, buf.as_str()).await
    }
}

pub struct SystemPartitionsHandler;

impl picoserve::routing::MethodHandlerService<()> for SystemPartitionsHandler {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        _method: &str,
        request: Request<'_, R>,
        response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        let current_slot = crate::boot::current_executing_slot();
        let is_a = current_slot == agentla_boot_proto::SlotId::SlotA;
        let slot_a_role = if is_a { "Active Firmware [Running]" } else { "Alternate Firmware [Standby]" };
        let slot_b_role = if !is_a { "Active Firmware [Running]" } else { "Alternate Firmware [Standby]" };

        let mut buf = heapless::String::<1024>::new();
        let _ = core::fmt::Write::write_fmt(
            &mut buf,
            format_args!(
                r#"{{"flash_total_bytes":{},"partitions":[{{"name":"BOOT_TABLE","label":"BOOT","offset":{},"size":{},"type":"ROM","role":"Bootloader Switcher","active":false}},{{"name":"FW_SLOT_A","label":"SLOT_A","offset":{},"size":{},"type":"XIP_FLASH","role":"{}","active":{}}},{{"name":"FW_SLOT_B","label":"SLOT_B","offset":{},"size":{},"type":"XIP_FLASH","role":"{}","active":{}}},{{"name":"SYSTEM_FS","label":"SYSTEM_FS","offset":{},"size":{},"type":"LITTLEFS2","role":"WebUI & System Bundle","active":false}},{{"name":"USER_FS","label":"USER_FS","offset":{},"size":{},"type":"LITTLEFS2","role":"User Apps, Profiles & Data","active":false}},{{"name":"RESERVED_END","label":"END","offset":{},"size":{},"type":"ROM","role":"Reserved Trail","active":false}}]}}"#,
                crate::storage::FLASH_SIZE_BYTES,
                crate::storage::BOOT_TABLE_OFFSET,
                crate::storage::BOOT_TABLE_SIZE_BYTES,
                crate::storage::SLOT_A_OFFSET,
                crate::storage::SLOT_SIZE_BYTES,
                slot_a_role,
                is_a,
                crate::storage::SLOT_B_OFFSET,
                crate::storage::SLOT_SIZE_BYTES,
                slot_b_role,
                !is_a,
                crate::storage::SYSTEM_FS_OFFSET,
                crate::storage::SYSTEM_FS_SIZE_BYTES,
                crate::storage::USER_FS_OFFSET,
                crate::storage::USER_FS_SIZE_BYTES,
                crate::storage::USER_FS_END,
                crate::storage::FLASH_RESERVED_TRAIL_BYTES,
            ),
        );
        send_json(request, response_writer, StatusCode::OK, buf.as_str()).await
    }
}

impl picoserve::routing::MethodHandlerService<()> for SystemTelemetryStream {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        if method != "GET" {
            return send_json(request, response_writer, StatusCode::METHOD_NOT_ALLOWED, r#"{"error":"method_not_allowed"}"#).await;
        }
        let stream = picoserve::response::sse::EventStream(crate::telemetry::TelemetryEventSource);
        let finalizer = request.body_connection.finalize().await?;
        stream.write_to(finalizer, response_writer).await
    }
}

pub struct AnalyzerStream;

impl picoserve::routing::MethodHandlerService<()> for AnalyzerStream {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        if method != "GET" {
            return send_json(request, response_writer, StatusCode::METHOD_NOT_ALLOWED, r#"{"error":"method_not_allowed"}"#).await;
        }
        let _ = crate::analyzer::start_continuous_stream(8, agentla_analyzer::FLAG_FORMAT_BIT_DELTA, 1_000_000);
        let mut chunk_buf = [0u8; 1024];
        let mut n = crate::analyzer::read_stream_chunk(&mut chunk_buf);
        if n == 0 {
            crate::analyzer::STREAM_SIGNAL.wait().await;
            n = crate::analyzer::read_stream_chunk(&mut chunk_buf);
        }
        let finalizer = request.body_connection.finalize().await?;
        let res = (
            StatusCode::OK,
            ("Content-Type", "application/octet-stream"),
            &chunk_buf[..n],
        )
            .write_to(finalizer, response_writer)
            .await;
        crate::analyzer::stop_continuous_stream();
        res
    }
}

struct DecodedStreamWsCallback;

impl picoserve::response::ws::WebSocketCallback for DecodedStreamWsCallback {
    async fn run<R: picoserve::io::Read, W: picoserve::io::Write<Error = R::Error>>(
        self,
        mut rx: picoserve::response::ws::SocketRx<R>,
        mut tx: picoserve::response::ws::SocketTx<W>,
    ) -> Result<(), W::Error> {
        let mut msg_buf = [0u8; 128];
        let mut event_buf = [0u8; 256];

        loop {
            match rx.next_message(&mut msg_buf, crate::analyzer::DECODED_SIGNAL.wait()).await {
                Ok(picoserve::futures::Either::First(_)) => {}
                Ok(picoserve::futures::Either::Second(())) => {
                    let n = crate::analyzer::DECODED_RING.pop_bytes(&mut event_buf);
                    if n > 0 {
                        if let Ok(s) = core::str::from_utf8(&event_buf[..n]) {
                            let _ = tx.send_text(s).await;
                        }
                    }
                    let _ = tx.flush().await;
                }
                _ => break,
            }
        }
        Ok(())
    }
}

impl picoserve::routing::MethodHandlerService<()> for DecodedStreamWebSocket {
    async fn call_method_handler_service<R: Read, W: ResponseWriter<Error = R::Error>>(
        &self,
        _state: &(),
        _path_parameters: (),
        method: &str,
        mut request: Request<'_, R>,
        response_writer: W,
    ) -> Result<picoserve::ResponseSent, W::Error> {
        if method != "GET" {
            return send_json(request, response_writer, StatusCode::METHOD_NOT_ALLOWED, r#"{"error":"method_not_allowed"}"#).await;
        }

        use picoserve::extract::FromRequest;
        let upgrade = match picoserve::response::ws::WebSocketUpgrade::from_request(
            &(),
            request.parts,
            request.body_connection.body(),
        )
        .await
        {
            Ok(u) => u,
            Err(e) => {
                return e
                    .write_to(request.body_connection.finalize().await?, response_writer)
                    .await;
            }
        };

        upgrade
            .on_upgrade(DecodedStreamWsCallback)
            .write_to(request.body_connection.finalize().await?, response_writer)
            .await
    }
}
