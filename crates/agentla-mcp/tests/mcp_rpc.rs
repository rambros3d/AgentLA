use core::fmt::Write as _;
use agentla_mcp::{
    extract_string_arg, extract_tool_name, extract_u32_arg, McpServer,
};

#[test]
fn test_initialize() {
    let server = McpServer::new();
    let req = br#"{"jsonrpc":"2.0","id":1,"method":"initialize","params":{}}"#;
    let res = server.handle_request(req);

    assert!(res.contains(r#""jsonrpc":"2.0""#));
    assert!(res.contains(r#""id":1"#));
    assert!(res.contains(r#""protocolVersion":"2025-03-26""#));
    assert!(res.contains(r#""serverInfo":{"name":"AgentLA RP2350","version":"rust-1.0"}"#));
    assert!(res.contains(r#""capabilities":{"tools":{},"resources":{}}"#));
}

#[test]
fn test_notifications_initialized() {
    let server = McpServer::new();
    let req = br#"{"jsonrpc":"2.0","method":"notifications/initialized"}"#;
    let res = server.handle_request(req);
    assert_eq!(res.as_str(), "");
}

#[test]
fn test_ping() {
    let server = McpServer::new();
    let req = br#"{"jsonrpc":"2.0","id":"ping-99","method":"ping"}"#;
    let res = server.handle_request(req);

    assert!(res.contains(r#""jsonrpc":"2.0""#));
    assert!(res.contains(r#""id":"ping-99""#));
    assert!(res.contains(r#""result":{}"#));
}

#[test]
fn test_tools_list() {
    let server = McpServer::new();
    let req = br#"{"jsonrpc":"2.0","id":10,"method":"tools/list"}"#;
    let res = server.handle_request(req);

    assert!(res.contains(r#""jsonrpc":"2.0""#));
    assert!(res.contains(r#""id":10"#));
    // Verify static schemas for composite tools per spec
    assert!(res.contains(r#""name":"inspect_firmware""#));
    assert!(res.contains(r#""name":"sniff_bus""#));
    assert!(res.contains(r#""name":"target_diagnose""#));

    // Verify input schemas are embedded
    assert!(res.contains(r#""address":{"type":"integer""#));
    assert!(res.contains(r#""protocol":{"type":"string","enum":["i2c","spi"]"#));
    assert!(res.contains(r#""port":{"type":"string""#));
}

#[test]
fn test_tools_call_success() {
    let handler = |name: &str, _body: &str, out: &mut heapless::String<1536>| -> bool {
        match name {
            "status" => {
                let _ = write!(out, "clk=150MHz tools=17 active_profile=0");
                false
            }
            _ => {
                let _ = write!(out, "not found");
                true
            }
        }
    };
    let server = McpServer::with_handler(handler);
    let req = br#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"status","arguments":{}}}"#;
    let res = server.handle_request(req);

    assert!(res.contains(r#""jsonrpc":"2.0""#));
    assert!(res.contains(r#""id":3"#));
    assert!(res.contains(r#""content":[{"type":"text","text":"clk=150MHz tools=17 active_profile=0"}]"#));
    assert!(res.contains(r#""isError":false"#));
}

#[test]
fn test_tools_call_error() {
    let handler = |name: &str, _body: &str, out: &mut heapless::String<1536>| -> bool {
        let _ = write!(out, "error executing {}", name);
        true
    };
    let server = McpServer::with_handler(handler);
    let req = br#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"fail_tool"}}"#;
    let res = server.handle_request(req);

    assert!(res.contains(r#""isError":true"#));
    assert!(res.contains(r#"error executing fail_tool"#));
}

#[test]
fn test_tools_call_escaping() {
    let handler = |_name: &str, _body: &str, out: &mut heapless::String<1536>| -> bool {
        let _ = write!(out, "line 1\nline 2 with \"quotes\" and \\backslash");
        false
    };
    let server = McpServer::with_handler(handler);
    let req = br#"{"jsonrpc":"2.0","id":5,"method":"tools/call","params":{"name":"escaped"}}"#;
    let res = server.handle_request(req);

    assert!(res.contains(r#"line 1\nline 2 with \"quotes\" and \\backslash"#));
}

#[test]
fn test_resources_list() {
    let server = McpServer::new();
    let req = br#"{"jsonrpc":"2.0","id":6,"method":"resources/list"}"#;
    let res = server.handle_request(req);

    assert!(res.contains(r#""resources":["#));
    assert!(res.contains(r#""uri":"agentla://info""#));
    assert!(res.contains(r#""uri":"agentla://pinout""#));
}

#[test]
fn test_resources_read() {
    let server = McpServer::new();
    let req = br#"{"jsonrpc":"2.0","id":7,"method":"resources/read","params":{"uri":"agentla://info"}}"#;
    let res = server.handle_request(req);

    assert!(res.contains(r#""contents":[{"uri":"agentla://info""#));
    assert!(res.contains(r#""status\":\"ready\""#));

    let req_pin = br#"{"jsonrpc":"2.0","id":8,"method":"resources/read","params":{"uri":"agentla://pinout"}}"#;
    let res_pin = server.handle_request(req_pin);
    assert!(res_pin.contains(r#""contents":[{"uri":"agentla://pinout""#));
    assert!(res_pin.contains(r#"GPIO0-GPIO9"#));

    let req_unknown = br#"{"jsonrpc":"2.0","id":9,"method":"resources/read","params":{"uri":"agentla://unknown"}}"#;
    let res_unknown = server.handle_request(req_unknown);
    assert!(res_unknown.contains(r#""code":-32602"#));
}

#[test]
fn test_invalid_method() {
    let server = McpServer::new();
    let req = br#"{"jsonrpc":"2.0","id":42,"method":"unknown/method"}"#;
    let res = server.handle_request(req);

    assert!(res.contains(r#""error":{"code":-32601,"message":"method not found"}"#));
    assert!(res.contains(r#""id":42"#));
}

#[test]
fn test_id_variants() {
    let server = McpServer::new();

    // Numeric ID
    let res = server.handle_request(br#"{"jsonrpc":"2.0","id":1234,"method":"ping"}"#);
    assert!(res.contains(r#""id":1234"#));

    // String ID
    let res = server.handle_request(br#"{"jsonrpc":"2.0","id":"uuid-abc-123","method":"ping"}"#);
    assert!(res.contains(r#""id":"uuid-abc-123""#));

    // Null ID
    let res = server.handle_request(br#"{"jsonrpc":"2.0","id":null,"method":"ping"}"#);
    assert!(res.contains(r#""id":null"#));
}

#[test]
fn test_extractors() {
    let body = r#"{"name": "sniff_bus", "arguments": {"protocol": "i2c", "duration_ms": 500, "address": 0x08000000}}"#;

    assert_eq!(extract_tool_name(body).unwrap().as_str(), "sniff_bus");
    assert_eq!(extract_string_arg(body, "protocol"), Some("i2c"));
    assert_eq!(extract_u32_arg(body, "duration_ms"), Some(500));
    assert_eq!(extract_u32_arg(body, "address"), Some(0x08000000));
}
