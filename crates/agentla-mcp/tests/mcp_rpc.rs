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
    assert!(res.contains(r#""capabilities":{"tools":{},"resources":{},"prompts":{}}"#));
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
    assert!(res.contains(r#""uri":"agentla://skills/mcu-autonomous-testing""#));
    assert!(res.contains(r#""uri":"agentla://skills/dev""#));
    assert!(res.contains(r#""uri":"agentla://skills/debug""#));
    assert!(res.contains(r#""uri":"agentla://skills/programmer""#));
    assert!(res.contains(r#""uri":"agentla://skills/analyzer""#));
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

    let req_skill = br#"{"jsonrpc":"2.0","id":9,"method":"resources/read","params":{"uri":"agentla://skills/mcu-autonomous-testing"}}"#;
    let res_skill = server.handle_request(req_skill);
    assert!(res_skill.contains(r#""contents":[{"uri":"agentla://skills/mcu-autonomous-testing""#));
    assert!(res_skill.contains(r#"Universal MCU Autonomous Testing Playbook"#));

    // Verify reading on-probe skills suite resources
    let req_dev = br#"{"jsonrpc":"2.0","id":11,"method":"resources/read","params":{"uri":"agentla://skills/dev"}}"#;
    let res_dev = server.handle_request(req_dev);
    assert!(res_dev.contains(r#""contents":[{"uri":"agentla://skills/dev""#));
    assert!(res_dev.contains(r#"Closed-Loop Developer Flow"#));

    let req_dbg = br#"{"jsonrpc":"2.0","id":12,"method":"resources/read","params":{"uri":"agentla://skills/debug"}}"#;
    let res_dbg = server.handle_request(req_dbg);
    assert!(res_dbg.contains(r#""contents":[{"uri":"agentla://skills/debug""#));
    assert!(res_dbg.contains(r#"Diagnostic & Triage Flow"#));

    let req_prog = br#"{"jsonrpc":"2.0","id":13,"method":"resources/read","params":{"uri":"agentla://skills/programmer"}}"#;
    let res_prog = server.handle_request(req_prog);
    assert!(res_prog.contains(r#""contents":[{"uri":"agentla://skills/programmer""#));
    assert!(res_prog.contains(r#"Programming & Rescue Flow"#));

    let req_ana = br#"{"jsonrpc":"2.0","id":14,"method":"resources/read","params":{"uri":"agentla://skills/analyzer"}}"#;
    let res_ana = server.handle_request(req_ana);
    assert!(res_ana.contains(r#""contents":[{"uri":"agentla://skills/analyzer""#));
    assert!(res_ana.contains(r#"Signal Capture & Analysis Flow"#));

    let req_unknown = br#"{"jsonrpc":"2.0","id":10,"method":"resources/read","params":{"uri":"agentla://unknown"}}"#;
    let res_unknown = server.handle_request(req_unknown);
    assert!(res_unknown.contains(r#""code":-32602"#));
}

#[test]
fn test_prompts_list() {
    let server = McpServer::new();
    let req = br#"{"jsonrpc":"2.0","id":20,"method":"prompts/list"}"#;
    let res = server.handle_request(req);

    assert!(res.contains(r#""jsonrpc":"2.0""#));
    assert!(res.contains(r#""id":20"#));
    assert!(res.contains(r#""prompts":["#));
    assert!(res.contains(r#""name":"dev_workflow""#));
    assert!(res.contains(r#""name":"debug_target""#));
    assert!(res.contains(r#""name":"program_target""#));
    assert!(res.contains(r#""name":"analyze_bus""#));
}

#[test]
fn test_prompts_get() {
    let server = McpServer::new();

    // 1. dev_workflow with custom target parameter
    let req_dev = br#"{"jsonrpc":"2.0","id":21,"method":"prompts/get","params":{"name":"dev_workflow","arguments":{"target":"STM32F401"}}}"#;
    let res_dev = server.handle_request(req_dev);
    assert!(res_dev.contains(r#""id":21"#));
    assert!(res_dev.contains(r#"Autonomous firmware development for STM32F401"#));
    assert!(res_dev.contains(r#"8-phase closed-loop playbook"#));

    // 2. dev_workflow default parameter
    let req_dev_def = br#"{"jsonrpc":"2.0","id":22,"method":"prompts/get","params":{"name":"dev_workflow"}}"#;
    let res_dev_def = server.handle_request(req_dev_def);
    assert!(res_dev_def.contains(r#"Autonomous firmware development for target MCU"#));

    // 3. debug_target
    let req_dbg = br#"{"jsonrpc":"2.0","id":23,"method":"prompts/get","params":{"name":"debug_target","arguments":{"target":"bluepill"}}}"#;
    let res_dbg = server.handle_request(req_dbg);
    assert!(res_dbg.contains(r#""id":23"#));
    assert!(res_dbg.contains(r#"Hardware triage for bluepill"#));
    assert!(res_dbg.contains(r#"CFSR/HFSR/MMFAR/BFAR fault register decode"#));

    // 4. program_target
    let req_prog = br#"{"jsonrpc":"2.0","id":24,"method":"prompts/get","params":{"name":"program_target","arguments":{"target":"RP2040"}}}"#;
    let res_prog = server.handle_request(req_prog);
    assert!(res_prog.contains(r#""id":24"#));
    assert!(res_prog.contains(r#"Target programming for RP2040"#));
    assert!(res_prog.contains(r#"sector-aligned programmer_flash"#));

    // 5. analyze_bus
    let req_ana = br#"{"jsonrpc":"2.0","id":25,"method":"prompts/get","params":{"name":"analyze_bus","arguments":{"protocol":"can"}}}"#;
    let res_ana = server.handle_request(req_ana);
    assert!(res_ana.contains(r#""id":25"#));
    assert!(res_ana.contains(r#"Protocol analysis for can"#));
    assert!(res_ana.contains(r#"120-ohm CAN termination relay"#));
}

#[test]
fn test_prompts_get_errors() {
    let server = McpServer::new();

    // Missing name
    let req_missing = br#"{"jsonrpc":"2.0","id":30,"method":"prompts/get","params":{}}"#;
    let res_missing = server.handle_request(req_missing);
    assert!(res_missing.contains(r#""code":-32602"#));
    assert!(res_missing.contains(r#"missing name parameter"#));

    // Unknown prompt
    let req_unknown = br#"{"jsonrpc":"2.0","id":31,"method":"prompts/get","params":{"name":"nonexistent"}}"#;
    let res_unknown = server.handle_request(req_unknown);
    assert!(res_unknown.contains(r#""code":-32602"#));
    assert!(res_unknown.contains(r#"prompt not found"#));
}

#[test]
fn test_no_std_prompt_and_resource_serialization() {
    use agentla_mcp::{Prompt, PromptArgument, Resource};

    let arg = PromptArgument::new("target", Some("Target chip"), false);
    let mut buf = heapless::String::<128>::new();
    arg.serialize(&mut buf).unwrap();
    assert_eq!(buf.as_str(), r#"{"name":"target","description":"Target chip","required":false}"#);

    static ARGS: [PromptArgument; 1] = [PromptArgument::new("target", None, true)];
    let prompt = Prompt::new("test_prompt", Some("Test desc"), &ARGS);
    let mut pbuf = heapless::String::<256>::new();
    prompt.serialize(&mut pbuf).unwrap();
    assert_eq!(pbuf.as_str(), r#"{"name":"test_prompt","description":"Test desc","arguments":[{"name":"target","required":true}]}"#);

    let res = Resource::new("agentla://test", "Test", Some("Test description"), "text/plain");
    let mut rbuf = heapless::String::<256>::new();
    res.serialize(&mut rbuf).unwrap();
    assert_eq!(rbuf.as_str(), r#"{"uri":"agentla://test","name":"Test","description":"Test description","mimeType":"text/plain"}"#);
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
