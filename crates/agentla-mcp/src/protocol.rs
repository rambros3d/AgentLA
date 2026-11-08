//! JSON-RPC 2.0 types and framing for MCP.

/// JSON-RPC 2.0 Request ID.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Id<'a> {
    Number(i64),
    String(&'a str),
    Null,
}

impl<'a> Id<'a> {
    pub fn serialize<W: core::fmt::Write>(&self, w: &mut W) -> core::fmt::Result {
        match self {
            Id::Number(n) => write!(w, "{}", n),
            Id::String(s) => write!(w, "\"{}\"", s),
            Id::Null => write!(w, "null"),
        }
    }
}

impl<'a> core::fmt::Display for Id<'a> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        self.serialize(f)
    }
}

/// JSON-RPC 2.0 Error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct JsonRpcError<'a> {
    pub code: i32,
    pub message: &'a str,
    pub data: Option<&'a str>,
}

impl<'a> JsonRpcError<'a> {
    pub const PARSE_ERROR: Self = Self {
        code: -32700,
        message: "parse error",
        data: None,
    };
    pub const INVALID_REQUEST: Self = Self {
        code: -32600,
        message: "invalid request",
        data: None,
    };
    pub const METHOD_NOT_FOUND: Self = Self {
        code: -32601,
        message: "method not found",
        data: None,
    };
    pub const INVALID_PARAMS: Self = Self {
        code: -32602,
        message: "invalid params",
        data: None,
    };
    pub const INTERNAL_ERROR: Self = Self {
        code: -32603,
        message: "internal error",
        data: None,
    };

    pub const fn new(code: i32, message: &'a str) -> Self {
        Self {
            code,
            message,
            data: None,
        }
    }

    pub fn serialize<W: core::fmt::Write>(&self, id: Option<Id<'_>>, w: &mut W) -> core::fmt::Result {
        let id_val = id.unwrap_or(Id::Null);
        write!(
            w,
            r#"{{"jsonrpc":"2.0","id":{},"error":{{"code":{},"message":"{}"}}}}"#,
            id_val, self.code, self.message
        )
    }
}

/// Parsed JSON-RPC 2.0 Request.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonRpcRequest<'a> {
    pub id: Option<Id<'a>>,
    pub method: &'a str,
    pub params: Option<&'a str>,
}

impl<'a> JsonRpcRequest<'a> {
    pub fn parse(body: &'a str) -> Result<Self, JsonRpcError<'static>> {
        let method = extract_string_arg(body, "method")
            .ok_or(JsonRpcError::INVALID_REQUEST)?;
        let id = extract_id(body);
        let params = extract_params_slice(body);

        Ok(Self { id, method, params })
    }
}

/// Extract method from JSON-RPC body, falling back to substring scan if necessary.
pub fn extract_method(body: &str) -> &'static str {
    if let Some(m) = extract_string_arg(body, "method") {
        match m {
            "initialize" => return "initialize",
            "notifications/initialized" => return "notifications/initialized",
            "ping" => return "ping",
            "tools/list" => return "tools/list",
            "tools/call" => return "tools/call",
            "resources/list" => return "resources/list",
            "resources/read" => return "resources/read",
            _ => return "unknown",
        }
    }

    let candidates = [
        "initialize",
        "notifications/initialized",
        "ping",
        "tools/list",
        "tools/call",
        "resources/list",
        "resources/read",
    ];
    for c in candidates.iter() {
        if body.contains(c) {
            return *c;
        }
    }
    "unknown"
}

/// Extract ID from JSON-RPC body.
pub fn extract_id<'a>(body: &'a str) -> Option<Id<'a>> {
    let mut cursor = body;
    loop {
        let idx = cursor.find("id")?;
        let before = &cursor[..idx];
        let after = &cursor[idx + 2..];

        // Ensure "id" is a standalone JSON key surrounded by quotes
        if before.ends_with('"') && after.starts_with('"') {
            let tail = &after[1..].trim_start();
            if tail.starts_with(':') {
                let val_tail = tail[1..].trim_start();
                if val_tail.starts_with("null") {
                    return Some(Id::Null);
                } else if val_tail.starts_with('"') {
                    let end = val_tail[1..].find('"')?;
                    return Some(Id::String(&val_tail[1..1 + end]));
                } else {
                    let end = val_tail
                        .find(|c: char| c == ',' || c == '}' || c == ']' || c.is_whitespace())
                        .unwrap_or(val_tail.len());
                    let raw = val_tail[..end].trim();
                    if let Ok(n) = raw.parse::<i64>() {
                        return Some(Id::Number(n));
                    } else if !raw.is_empty() {
                        return Some(Id::String(raw));
                    }
                }
            }
        }
        cursor = after;
    }
}

/// Extract tool name from tools/call body.
pub fn extract_tool_name(body: &str) -> Option<heapless::String<32>> {
    let val = extract_string_arg(body, "name")?;
    let mut o = heapless::String::<32>::new();
    let _ = o.push_str(val);
    Some(o)
}

/// Extract string argument from JSON body by key.
pub fn extract_string_arg<'a>(body: &'a str, arg: &str) -> Option<&'a str> {
    let key = arg.trim_matches('"');
    let mut cursor = body;
    loop {
        let idx = cursor.find(key)?;
        let before = &cursor[..idx];
        let after = &cursor[idx + key.len()..];

        if before.ends_with('"') && after.starts_with('"') {
            let tail = &after[1..].trim_start();
            if tail.starts_with(':') {
                let val_tail = tail[1..].trim_start();
                if val_tail.starts_with('"') {
                    let end = val_tail[1..].find('"')?;
                    return Some(&val_tail[1..1 + end]);
                }
            }
        }
        cursor = after;
    }
}

/// Extract u32 argument from JSON body by key.
pub fn extract_u32_arg(body: &str, arg: &str) -> Option<u32> {
    let key = arg.trim_matches('"');
    let mut cursor = body;
    loop {
        let idx = cursor.find(key)?;
        let before = &cursor[..idx];
        let after = &cursor[idx + key.len()..];

        if before.ends_with('"') && after.starts_with('"') {
            let tail = &after[1..].trim_start();
            if tail.starts_with(':') {
                let val_tail = tail[1..].trim_start();
                let mut num: u32 = 0;
                let mut found = false;

                if val_tail.starts_with("0x") || val_tail.starts_with("0X") {
                    let hex_tail = &val_tail[2..];
                    for c in hex_tail.chars() {
                        if let Some(digit) = c.to_digit(16) {
                            num = num.saturating_mul(16).saturating_add(digit);
                            found = true;
                        } else if found {
                            break;
                        }
                    }
                } else {
                    for c in val_tail.chars() {
                        if c.is_ascii_digit() {
                            num = num.saturating_mul(10).saturating_add(c as u32 - '0' as u32);
                            found = true;
                        } else if found {
                            break;
                        }
                    }
                }

                if found {
                    return Some(num);
                }
            }
        }
        cursor = after;
    }
}

/// Extract bool argument from JSON body by key.
pub fn extract_bool_arg(body: &str, arg: &str) -> Option<bool> {
    let key = arg.trim_matches('"');
    let mut cursor = body;
    loop {
        let idx = cursor.find(key)?;
        let before = &cursor[..idx];
        let after = &cursor[idx + key.len()..];

        if before.ends_with('"') && after.starts_with('"') {
            let tail = &after[1..].trim_start();
            if tail.starts_with(':') {
                let val_tail = tail[1..].trim_start();
                if val_tail.starts_with("true") {
                    return Some(true);
                } else if val_tail.starts_with("false") {
                    return Some(false);
                }
            }
        }
        cursor = after;
    }
}

/// Extract raw slice of params object.
fn extract_params_slice(body: &str) -> Option<&str> {
    let key = "params";
    let mut cursor = body;
    loop {
        let idx = cursor.find(key)?;
        let before = &cursor[..idx];
        let after = &cursor[idx + key.len()..];

        if before.ends_with('"') && after.starts_with('"') {
            let tail = &after[1..].trim_start();
            if tail.starts_with(':') {
                let val_tail = tail[1..].trim_start();
                if val_tail.starts_with('{') {
                    // Match balanced braces
                    let mut depth = 0;
                    for (i, c) in val_tail.char_indices() {
                        if c == '{' {
                            depth += 1;
                        } else if c == '}' {
                            depth -= 1;
                            if depth == 0 {
                                return Some(&val_tail[..i + 1]);
                            }
                        }
                    }
                } else {
                    let end = val_tail
                        .find(|c: char| c == ',' || c == '}')
                        .unwrap_or(val_tail.len());
                    return Some(val_tail[..end].trim());
                }
            }
        }
        cursor = after;
    }
}

/// Escape a string for JSON output into a `core::fmt::Write`.
pub fn escape_json_write<W: core::fmt::Write>(src: &str, w: &mut W) -> core::fmt::Result {
    for c in src.chars() {
        match c {
            '"' => w.write_str("\\\"")?,
            '\\' => w.write_str("\\\\")?,
            '\n' => w.write_str("\\n")?,
            '\r' => w.write_str("\\r")?,
            '\t' => w.write_str("\\t")?,
            other => w.write_char(other)?,
        }
    }
    Ok(())
}

/// Escape string for JSON output into a `heapless::String`.
pub fn escape_json<const N: usize>(src: &str, out: &mut heapless::String<N>) {
    let _ = escape_json_write(src, out);
}
