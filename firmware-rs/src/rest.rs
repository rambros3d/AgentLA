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
    let mut buf=[0u8;256];
    let mut out=heapless::String::<512>::new();
    loop {
        match request.body_connection.body().reader().read(&mut buf).await {
            Ok(0) => break,
            Ok(n) => { if let Ok(s)=core::str::from_utf8(&buf[..n]) { let _=out.push_str(s);} }
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
pub struct BridgeState;
pub struct BridgeI2cOn;
pub struct BridgeI2cOff;
pub struct BridgeCanOn;
pub struct BridgeCanOff;
pub struct BridgeLedOn;
pub struct BridgeLedOff;
pub struct BridgeI3cOn;
pub struct BridgeI3cOff;

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

service!(ProfilesList, "GET"); service!(ProfilesApply, "POST"); service!(ProfilesBoot, "POST"); service!(ProfilesSave, "POST"); service!(ProfilesDelete, "POST"); service!(PinsClaim, "POST"); service!(PinsRelease, "POST"); service!(ServoList, "GET"); service!(ServoAttach, "POST"); service!(ServoWrite, "POST"); service!(ServoWriteUs, "POST"); service!(ServoDetach, "POST"); service!(PwmList, "GET"); service!(PwmClaim, "POST"); service!(PwmSet, "POST"); service!(PwmRelease, "POST"); service!(LogicAnalyzerArm, "POST"); service!(LogicAnalyzerStart, "POST"); service!(LogicAnalyzerStop, "POST"); service!(LogicAnalyzerStatus, "GET"); service!(LogicAnalyzerConfig, "POST"); service!(LogicAnalyzerData, "GET");
service!(BridgeState, "GET"); service!(BridgeI2cOn, "POST"); service!(BridgeI2cOff, "POST"); service!(BridgeCanOn, "POST"); service!(BridgeCanOff, "POST"); service!(BridgeLedOn, "POST"); service!(BridgeLedOff, "POST"); service!(BridgeI3cOn, "POST"); service!(BridgeI3cOff, "POST");

// ============ implementations ============
impl ProfilesList {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let mut body = heapless::String::<2048>::new();
        let _ = write!(body, r#"{{"active_slot":{},"active_name":"{}","slots":["#, profile::active_slot(), profile::active_name().as_str());
        let mut first=true;
        for slot in 0..profile::SLOT_COUNT {
            if !first { let _=body.push(','); } first=false;
            match profile::read_slot(slot) {
                profile::SlotRead::SafeState(_) => { let _ = write!(body, r#"{{"id":0,"name":"Safe State (Hi-Z)","in_use":true,"is_boot":false,"read_only":true}}"#); }
                profile::SlotRead::InUse(h) => {
                    let _=write!(body, r#"{{"id":{},"name":"{}","in_use":true,"is_boot":{},"read_only":{}}}"#, slot, h.name.as_str(), (h.flags & profile::FLAG_BOOT_DEFAULT)!=0, (h.flags & profile::FLAG_READ_ONLY)!=0);
                }
                profile::SlotRead::Empty => { let _=write!(body, r#"{{"id":{},"name":"empty","in_use":false,"is_boot":false,"read_only":false}}"#, slot); }
                profile::SlotRead::Corrupt => { let _=write!(body, r#"{{"id":{},"name":"corrupt","in_use":false,"is_boot":false,"read_only":false}}"#, slot); }
            }
        }
        let _=body.push_str("]}");
        send_json(request, response_writer, StatusCode::OK, body.as_str()).await
    }
}
impl ProfilesApply {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query = request.parts.query();
        let body = read_body_str(&mut request).await;
        let slot = key_int(query, &body, "slot");
        let Some(slot)=slot else { return send_json(request, response_writer, StatusCode::BAD_REQUEST, r#"{"error":"Invalid profile slot"}"#).await; };
        if slot<1 || (slot as u32)>=profile::SLOT_COUNT as u32 { return send_json(request, response_writer, StatusCode::BAD_REQUEST, r#"{"error":"Invalid profile slot"}"#).await; }
        match profile::read_slot(slot as u32) {
            profile::SlotRead::Empty => { return send_json(request, response_writer, StatusCode::NOT_FOUND, r#"{"error":"slot not in use"}"#).await; }
            profile::SlotRead::Corrupt => { return send_json(request, response_writer, StatusCode::BAD_REQUEST, r#"{"error":"Invalid profile slot"}"#).await; }
            _ => {}
        }
        match profile::apply_slot(slot as u32).await {
            Ok(a) => {
                let mut b=heapless::String::<128>::new();
                let _=write!(b, r#"{{"status":"applied","slot":{},"name":"{}"}}"#, a.slot, a.name.as_str());
                send_json(request, response_writer, StatusCode::OK, b.as_str()).await
            }
            Err(_) => send_json(request, response_writer, StatusCode::INTERNAL_SERVER_ERROR, r#"{"error":"apply failed"}"#).await,
        }
    }
}
impl ProfilesBoot {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query=request.parts.query();
        let body=read_body_str(&mut request).await;
        let slot = key_int(query,&body,"slot").unwrap_or(0);
        if slot<0 || (slot as i32)>=profile::SLOT_COUNT as i32 { return send_json(request,response_writer,StatusCode::BAD_REQUEST,r#"{"error":"Invalid profile slot"}"#).await; }
        match profile::set_boot_default(slot as u32) {
            Ok(()) => { let mut b=heapless::String::<64>::new(); let _=write!(b,r#"{{"status":"boot_default_set","slot":{}}}"#,slot); send_json(request,response_writer,StatusCode::OK,b.as_str()).await }
            Err(_) => send_json(request,response_writer,StatusCode::BAD_REQUEST,r#"{"error":"Invalid profile slot"}"#).await,
        }
    }
}
impl ProfilesSave {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query=request.parts.query();
        let body=read_body_str(&mut request).await;
        let slot=key_int(query,&body,"slot").unwrap_or(-1);
        if slot<1 || slot as u32>=profile::SLOT_COUNT { return send_json(request,response_writer,StatusCode::BAD_REQUEST,r#"{"error":"Invalid profile slot"}"#).await; }
        if let profile::SlotRead::InUse(h)=profile::read_slot(slot as u32){ if h.flags & profile::FLAG_READ_ONLY!=0 { return send_json(request,response_writer,StatusCode::BAD_REQUEST,r#"{"error":"slot read only"}"#).await } }
        let mut name_tmp = heapless::String::<64>::new(); let _ = name_tmp.push_str("Custom Profile"); let name = key_str(query,&body,"name").unwrap_or(name_tmp);
        let is_boot=key_str(query,&body,"is_boot").map(|v|parse_bool(v.as_str())).unwrap_or(false);
        match profile::save_profile(slot as u32, name.as_str(), is_boot, profile::METADATA_PAYLOAD) {
            Ok(()) => { let mut b=heapless::String::<128>::new(); let _=write!(b,r#"{{"status":"saved","slot":{},"name":"{}"}}"#,slot,name.as_str()); send_json(request,response_writer,StatusCode::OK,b.as_str()).await }
            Err(_) => send_json(request,response_writer,StatusCode::INTERNAL_SERVER_ERROR,r#"{"error":"save failed"}"#).await,
        }
    }
}
impl ProfilesDelete {
    async fn run<R, W>(mut request: Request<'_, R>, response_writer: W) -> Result<picoserve::ResponseSent, W::Error>
    where R: Read, W: ResponseWriter<Error=R::Error> {
        let query=request.parts.query();
        let body=read_body_str(&mut request).await;
        let slot=key_int(query,&body,"slot").unwrap_or(-1);
        if slot<1 || slot as u32>=profile::SLOT_COUNT { return send_json(request,response_writer,StatusCode::BAD_REQUEST,r#"{"error":"slot read only"}"#).await; }
        let r=profile::delete_profile(slot as u32);
        if r.is_ok() {
            let mut b=heapless::String::<64>::new(); let _=write!(b,r#"{{"status":"deleted","slot":{}}}"#,slot); send_json(request,response_writer,StatusCode::OK,b.as_str()).await
        } else {
            send_json(request,response_writer,StatusCode::NOT_FOUND,r#"{"error":"slot not in use"}"#).await
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
        let _=core::fmt::Write::write_fmt(&mut b, format_args!(r#"{{"state":{},"pre_bytes":{},"post_bytes":{},"sample_count":{},"first_sample":{},"i3c_frames":{}, "i3c_bytes":{}}}"#, st.state as u8 , st.pre_bytes, st.post_bytes, st.sample_count, first, i3c.frame_count, i3c.byte_count));
        send_json(request,response_writer,StatusCode::OK,b.as_str()).await
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
        send_json(request, response_writer, StatusCode::OK, r#"{"status":"healthy","tools":6}"#).await
    }
}
