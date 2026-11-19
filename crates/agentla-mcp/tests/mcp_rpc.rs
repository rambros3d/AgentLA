use core::fmt::Write as _;
use agentla_mcp::{
    extract_i32_arg, extract_string_arg, extract_tool_name, extract_u32_arg, McpServer,
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

    // Verify newly added MCP tools
    assert!(res.contains(r#""name":"app_list""#));
    assert!(res.contains(r#""name":"app_install""#));
    assert!(res.contains(r#""name":"power_control""#));
    assert!(res.contains(r#""name":"power_cycle""#));
    assert!(res.contains(r#""name":"emergency_safe_state""#));
    assert!(res.contains(r#""name":"profile_list""#));
    assert!(res.contains(r#""name":"profile_apply""#));
    assert!(res.contains(r#""name":"swo_read""#));

    // Verify servo tools
    assert!(res.contains(r#""name":"servo_attach""#));
    assert!(res.contains(r#""name":"servo_write_angle""#));
    assert!(res.contains(r#""name":"servo_write_us""#));
    assert!(res.contains(r#""name":"servo_detach""#));

    // Verify neopixel tools
    assert!(res.contains(r#""name":"neopixel_fill""#));
    assert!(res.contains(r#""name":"neopixel_set""#));
    assert!(res.contains(r#""name":"neopixel_clear""#));
    assert!(res.contains(r#""name":"neopixel_pattern""#));
    assert!(res.contains(r#""name":"neopixel_analyze""#));
    assert!(res.contains(r#""name":"neopixel_transmit""#));
    assert!(res.contains(r#""name":"sensor_scan""#));
    assert!(res.contains(r#""name":"sensor_read_reg""#));
    assert!(res.contains(r#""name":"sensor_write_reg""#));
    assert!(res.contains(r#""name":"sensor_poll""#));

    // Verify app_uninstall is strictly omitted from tools list
    assert!(!res.contains(r#""name":"app_uninstall""#));
    assert!(!res.contains(r#""name":"app_delete""#));

    // Verify BareFOC tools are advertised
    assert!(res.contains(r#""name":"barefoc_status""#));
    assert!(res.contains(r#""name":"barefoc_target""#));
    assert!(res.contains(r#""name":"barefoc_read""#));
    assert!(res.contains(r#""name":"barefoc_write""#));
    assert!(res.contains(r#""name":"barefoc_command""#));
    assert!(res.contains(r#""name":"barefoc_mode""#));
    assert!(res.contains(r#""name":"barefoc_calibrate""#));
    assert!(res.contains(r#""name":"barefoc_emergency""#));

    // Verify the authoritative tool count equals the compiled catalog length
    assert_eq!(agentla_mcp::ALL_TOOLS.len(), 56);
}

#[test]
fn test_barefoc_tools_advertised_and_catalog_counts() {
    let server = McpServer::new();

    let tools_req = br#"{"jsonrpc":"2.0","id":100,"method":"tools/list"}"#;
    let tools_res = server.handle_request(tools_req);
    for name in [
        "barefoc_status",
        "barefoc_target",
        "barefoc_read",
        "barefoc_write",
        "barefoc_command",
        "barefoc_mode",
        "barefoc_calibrate",
        "barefoc_emergency",
    ] {
        let needle = format!(r#""name":"{}""#, name);
        assert!(tools_res.contains(&needle), "tools/list missing {}", name);
    }
    assert_eq!(agentla_mcp::ALL_TOOLS.len(), 56);

    let res_req = br#"{"jsonrpc":"2.0","id":101,"method":"resources/list"}"#;
    let res_res = server.handle_request(res_req);
    assert!(res_res.contains(r#""uri":"agentla://skills/barefoc""#));
    assert_eq!(agentla_mcp::DEFAULT_RESOURCES.len(), 8);

    let prompt_req = br#"{"jsonrpc":"2.0","id":102,"method":"prompts/list"}"#;
    let prompt_res = server.handle_request(prompt_req);
    assert!(prompt_res.contains(r#""name":"barefoc_debug""#));
    assert_eq!(agentla_mcp::DEFAULT_PROMPTS.len(), 5);
}

#[test]
fn test_app_uninstall_omission_returns_32601() {
    let server = McpServer::new();

    // 1. Calling tool app_uninstall via tools/call returns -32601
    let req_tool = br#"{"jsonrpc":"2.0","id":50,"method":"tools/call","params":{"name":"app_uninstall"}}"#;
    let res_tool = server.handle_request(req_tool);
    assert!(res_tool.contains(r#""code":-32601"#));
    assert!(res_tool.contains(r#"tool not found"#));

    // 2. Calling method app_uninstall directly returns -32601
    let req_method = br#"{"jsonrpc":"2.0","id":51,"method":"app_uninstall"}"#;
    let res_method = server.handle_request(req_method);
    assert!(res_method.contains(r#""code":-32601"#));
    assert!(res_method.contains(r#"method not found"#));
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
    assert!(res.contains(r#""uri":"agentla://skills/barefoc""#));
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

    let req_barefoc = br#"{"jsonrpc":"2.0","id":15,"method":"resources/read","params":{"uri":"agentla://skills/barefoc"}}"#;
    let res_barefoc = server.handle_request(req_barefoc);
    assert!(res_barefoc.contains(r#""uri":"agentla://skills/barefoc""#));
    assert!(res_barefoc.contains("BareFOC Agentic Debug Playbook"));
    assert!(res_barefoc.contains("Safety First"));
    assert!(res_barefoc.contains("Safe-State Teardown"));

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
    assert!(res.contains(r#""name":"barefoc_debug""#));
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

    // 6. barefoc_debug with explicit node
    let req_bf = br#"{"jsonrpc":"2.0","id":26,"method":"prompts/get","params":{"name":"barefoc_debug","arguments":{"node":"3"}}}"#;
    let res_bf = server.handle_request(req_bf);
    assert!(res_bf.contains(r#""id":26"#));
    assert!(res_bf.contains("target node 3"));
    assert!(res_bf.contains("SAFETY FIRST"));
    assert!(res_bf.contains("agentla://skills/barefoc"));

    // 7. barefoc_debug default node
    let req_bf_def = br#"{"jsonrpc":"2.0","id":27,"method":"prompts/get","params":{"name":"barefoc_debug"}}"#;
    let res_bf_def = server.handle_request(req_bf_def);
    assert!(res_bf_def.contains("target node 1"));
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
    let body = r#"{"name": "sniff_bus", "arguments": {"protocol": "i2c", "duration_ms": 500, "address": 0x08000000, "angle": -45}}"#;

    assert_eq!(extract_tool_name(body).unwrap().as_str(), "sniff_bus");
    assert_eq!(extract_string_arg(body, "protocol"), Some("i2c"));
    assert_eq!(extract_u32_arg(body, "duration_ms"), Some(500));
    assert_eq!(extract_u32_arg(body, "address"), Some(0x08000000));
    assert_eq!(extract_i32_arg(body, "angle"), Some(-45));
    assert_eq!(extract_i32_arg(r#"{"angle": 90}"#, "angle"), Some(90));
    assert_eq!(extract_i32_arg(r#"{"angle": 0}"#, "angle"), Some(0));
}

#[test]
fn test_servo_tools_dispatch() {
    let handler = |name: &str, body: &str, out: &mut heapless::String<1536>| -> bool {
        match name {
            "servo_attach" => {
                let pin = extract_u32_arg(body, "pin").unwrap_or(0);
                let _ = write!(out, r#"{{"status":"ok","pin":{},"continuous":false,"active":true}}"#, pin);
                false
            }
            "servo_write_angle" => {
                let pin = extract_u32_arg(body, "pin").unwrap_or(0);
                let angle = extract_i32_arg(body, "angle").unwrap_or(0);
                let _ = write!(out, r#"{{"status":"ok","pin":{},"angle":{},"pulse_us":1500,"active":true}}"#, pin, angle);
                false
            }
            "servo_write_us" => {
                let pin = extract_u32_arg(body, "pin").unwrap_or(0);
                let pulse_us = extract_u32_arg(body, "pulse_us").unwrap_or(0);
                let _ = write!(out, r#"{{"status":"ok","pin":{},"pulse_us":{},"active":true}}"#, pin, pulse_us);
                false
            }
            "servo_detach" => {
                let pin = extract_u32_arg(body, "pin").unwrap_or(0);
                let _ = write!(out, r#"{{"status":"ok","pin":{},"detached":true}}"#, pin);
                false
            }
            _ => {
                let _ = out.push_str(r#"{"status":"error","error":"unknown"}"#);
                true
            }
        }
    };

    let server = McpServer::with_handler(handler);

    // Test servo_attach
    let req = br#"{"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":"servo_attach","arguments":{"pin":16,"min_us":544,"max_us":2400,"continuous":false}}}"#;
    let res = server.handle_request(req);
    assert!(res.contains(r#""isError":false"#));
    assert!(res.contains(r#"\"status\":\"ok\""#));
    assert!(res.contains(r#"\"pin\":16"#));

    // Test servo_write_angle
    let req = br#"{"jsonrpc":"2.0","id":2,"method":"tools/call","params":{"name":"servo_write_angle","arguments":{"pin":16,"angle":90}}}"#;
    let res = server.handle_request(req);
    assert!(res.contains(r#""isError":false"#));
    assert!(res.contains(r#"\"angle\":90"#));

    // Test servo_write_us
    let req = br#"{"jsonrpc":"2.0","id":3,"method":"tools/call","params":{"name":"servo_write_us","arguments":{"pin":16,"pulse_us":1500}}}"#;
    let res = server.handle_request(req);
    assert!(res.contains(r#""isError":false"#));
    assert!(res.contains(r#"\"pulse_us\":1500"#));

    // Test servo_detach
    let req = br#"{"jsonrpc":"2.0","id":4,"method":"tools/call","params":{"name":"servo_detach","arguments":{"pin":16}}}"#;
    let res = server.handle_request(req);
    assert!(res.contains(r#""isError":false"#));
    assert!(res.contains(r#"\"detached\":true"#));
}

#[test]
fn test_neopixel_tools_dispatch() {
    let handler = |name: &str, body: &str, out: &mut heapless::String<1536>| -> bool {
        match name {
            "neopixel_fill" => {
                let pin = extract_u32_arg(body, "pin").unwrap_or(22);
                let count = extract_u32_arg(body, "count").unwrap_or(16);
                let color = extract_string_arg(body, "color").unwrap_or("#FFFFFF");
                let _ = write!(out, r#"{{"status":"ok","pin":{},"count":{},"color":"{}"}}"#, pin, count, color);
                false
            }
            "neopixel_set" => {
                let pin = extract_u32_arg(body, "pin").unwrap_or(22);
                let index = extract_u32_arg(body, "index").unwrap_or(0);
                let color = extract_string_arg(body, "color").unwrap_or("#FFFFFF");
                let _ = write!(out, r#"{{"status":"ok","pin":{},"index":{},"color":"{}"}}"#, pin, index, color);
                false
            }
            "neopixel_clear" => {
                let pin = extract_u32_arg(body, "pin").unwrap_or(22);
                let count = extract_u32_arg(body, "count").unwrap_or(64);
                let _ = write!(out, r#"{{"status":"ok","pin":{},"count":{},"cleared":true}}"#, pin, count);
                false
            }
            "neopixel_pattern" => {
                let pin = extract_u32_arg(body, "pin").unwrap_or(22);
                let pattern = extract_string_arg(body, "pattern").unwrap_or("rainbow");
                let count = extract_u32_arg(body, "count").unwrap_or(16);
                let speed = extract_u32_arg(body, "speed").unwrap_or(50);
                let _ = write!(out, r#"{{"status":"ok","pin":{},"pattern":"{}","count":{},"speed":{}}}"#, pin, pattern, count, speed);
                false
            }
            "neopixel_analyze" => {
                let pin = extract_u32_arg(body, "pin").unwrap_or(22);
                let _ = write!(out, r#"{{"status":"ok","pin":{},"t0h_ns":350,"t1h_ns":700,"t0l_ns":800,"t1l_ns":600,"reset_us":65,"passed":true}}"#, pin);
                false
            }
            "neopixel_transmit" | "ws2812_transmit" => {
                let pin = extract_u32_arg(body, "pin").unwrap_or(22);
                let _ = write!(out, r#"{{"status":"ok","pin":{},"pixels_updated":3}}"#, pin);
                false
            }
            _ => {
                let _ = out.push_str(r#"{"status":"error","error":"unknown"}"#);
                true
            }
        }
    };

    let server = McpServer::with_handler(handler);

    // Test neopixel_fill
    let req = br##"{"jsonrpc":"2.0","id":10,"method":"tools/call","params":{"name":"neopixel_fill","arguments":{"pin":22,"color":"#FF0000","count":8}}}"##;
    let res = server.handle_request(req);
    assert!(res.contains(r#""isError":false"#));
    assert!(res.contains(r#"\"status\":\"ok\""#));
    assert!(res.contains(r#"\"pin\":22"#));
    assert!(res.contains(r#"\"count\":8"#));

    // Test neopixel_set
    let req = br##"{"jsonrpc":"2.0","id":11,"method":"tools/call","params":{"name":"neopixel_set","arguments":{"pin":22,"index":3,"color":"#00FF00"}}}"##;
    let res = server.handle_request(req);
    assert!(res.contains(r#""isError":false"#));
    assert!(res.contains(r#"\"index\":3"#));

    // Test neopixel_clear
    let req = br##"{"jsonrpc":"2.0","id":12,"method":"tools/call","params":{"name":"neopixel_clear","arguments":{"pin":22,"count":16}}}"##;
    let res = server.handle_request(req);
    assert!(res.contains(r#""isError":false"#));
    assert!(res.contains(r#"\"cleared\":true"#));

    // Test neopixel_pattern
    let req = br##"{"jsonrpc":"2.0","id":13,"method":"tools/call","params":{"name":"neopixel_pattern","arguments":{"pin":22,"pattern":"rainbow","count":16,"speed":80}}}"##;
    let res = server.handle_request(req);
    assert!(res.contains(r#""isError":false"#));
    assert!(res.contains(r#"\"pattern\":\"rainbow\""#));

    // Test neopixel_analyze
    let req = br##"{"jsonrpc":"2.0","id":14,"method":"tools/call","params":{"name":"neopixel_analyze","arguments":{"pin":22}}}"##;
    let res = server.handle_request(req);
    assert!(res.contains(r#""isError":false"#));
    assert!(res.contains(r#"\"passed\":true"#));

    // Test neopixel_transmit
    let req = br##"{"jsonrpc":"2.0","id":15,"method":"tools/call","params":{"name":"neopixel_transmit","arguments":{"pin":22,"colors":["#FF0000","#00FF00","#0000FF"]}}}"##;
    let res = server.handle_request(req);
    assert!(res.contains(r#""isError":false"#));
    assert!(res.contains(r#"\"pixels_updated\":3"#));
}

#[test]
fn test_sensor_tools_dispatch() {
    let handler = |name: &str, _body: &str, out: &mut heapless::String<1536>| -> bool {
        match name {
            "sensor_scan" => {
                let _ = write!(out, r#"{{"status":"ok","devices":[{{"addr":104,"hex":"0x68","name":"MPU6050"}}]}}"#);
                false
            }
            "sensor_read_reg" => {
                let _ = write!(out, r#"{{"status":"ok","addr":104,"reg":117,"bytes":["0x68"]}}"#);
                false
            }
            "sensor_write_reg" => {
                let _ = write!(out, r#"{{"status":"ok","addr":104,"reg":107,"written":1}}"#);
                false
            }
            "sensor_poll" => {
                let _ = write!(out, r#"{{"status":"ok","addr":104,"profile":"mpu6050","telemetry":{{"accel_x_g":0.02,"accel_y_g":-0.01,"accel_z_g":0.99}}}}"#);
                false
            }
            _ => {
                let _ = write!(out, "unknown");
                true
            }
        }
    };
    let server = McpServer::with_handler(handler);

    // Test sensor_scan
    let req = br##"{"jsonrpc":"2.0","id":20,"method":"tools/call","params":{"name":"sensor_scan","arguments":{"bus":"i2c"}}}"##;
    let res = server.handle_request(req);
    assert!(res.contains(r#""isError":false"#));
    assert!(res.contains(r#"\"status\":\"ok\""#));
    assert!(res.contains(r#"\"MPU6050\""#));

    // Test sensor_read_reg
    let req = br##"{"jsonrpc":"2.0","id":21,"method":"tools/call","params":{"name":"sensor_read_reg","arguments":{"addr":104,"reg":117,"len":1}}}"##;
    let res = server.handle_request(req);
    assert!(res.contains(r#""isError":false"#));
    assert!(res.contains(r#"\"0x68\""#));

    // Test sensor_write_reg
    let req = br##"{"jsonrpc":"2.0","id":22,"method":"tools/call","params":{"name":"sensor_write_reg","arguments":{"addr":104,"reg":107,"data":"0x00"}}}"##;
    let res = server.handle_request(req);
    assert!(res.contains(r#""isError":false"#));
    assert!(res.contains(r#"\"written\":1"#));

    // Test sensor_poll
    let req = br##"{"jsonrpc":"2.0","id":23,"method":"tools/call","params":{"name":"sensor_poll","arguments":{"addr":104,"profile":"mpu6050"}}}"##;
    let res = server.handle_request(req);
    assert!(res.contains(r#""isError":false"#));
    assert!(res.contains(r#"\"accel_z_g\":0.99"#));
}

#[test]
fn test_barefoc_tools_dispatch() {
    let handler = |name: &str, body: &str, out: &mut heapless::String<1536>| -> bool {
        match name {
            "barefoc_status" => {
                let _ = write!(out, r#"{{"status":"ok","vtarget_mv":3300,"active_profile":0,"safe_state":true}}"#);
                false
            }
            "barefoc_target" => {
                let node = extract_u32_arg(body, "node").unwrap_or(1);
                let _ = write!(out, r#"{{"status":"ok","node":{},"addr":{}}}"#, node, 0x40 + node);
                false
            }
            "barefoc_read" => {
                let reg = extract_u32_arg(body, "reg").unwrap_or(0);
                let _ = write!(out, r#"{{"status":"ok","reg":{},"hex":"AB","bytes":[171]}}"#, reg);
                false
            }
            "barefoc_write" => {
                let reg = extract_u32_arg(body, "reg").unwrap_or(0);
                let _ = write!(out, r#"{{"status":"ok","reg":{},"written":1}}"#, reg);
                false
            }
            "barefoc_command" => {
                let cmd = extract_string_arg(body, "command").unwrap_or("");
                let _ = write!(out, r#"{{"status":"ok","command":"{}","response":"M1"}}"#, cmd);
                false
            }
            "barefoc_mode" => {
                let mode = extract_string_arg(body, "mode").unwrap_or("idle");
                let _ = write!(out, r#"{{"status":"ok","mode":"{}","enabled":false}}"#, mode);
                false
            }
            "barefoc_calibrate" => {
                let kind = extract_string_arg(body, "kind").unwrap_or("motor");
                let _ = write!(out, r#"{{"status":"ok","kind":"{}","state":"in_progress"}}"#, kind);
                false
            }
            "barefoc_emergency" => {
                let _ = out.push_str(r#"{"status":"ok","safe_state":true,"power_enabled":false}"#);
                false
            }
            _ => true,
        }
    };
    let server = McpServer::with_handler(handler);

    let cases: [(&str, &str); 8] = [
        ("barefoc_status", r#"{"name":"barefoc_status","arguments":{}}"#),
        ("barefoc_target", r#"{"name":"barefoc_target","arguments":{"node":2}}"#),
        ("barefoc_read", r#"{"name":"barefoc_read","arguments":{"reg":16,"len":1}}"#),
        ("barefoc_write", r#"{"name":"barefoc_write","arguments":{"reg":16,"data":"0xAB"}}"#),
        ("barefoc_command", r#"{"name":"barefoc_command","arguments":{"command":"M1"}}"#),
        ("barefoc_mode", r#"{"name":"barefoc_mode","arguments":{"mode":"velocity","enabled":false}}"#),
        ("barefoc_calibrate", r#"{"name":"barefoc_calibrate","arguments":{"kind":"encoder"}}"#),
        ("barefoc_emergency", r#"{"name":"barefoc_emergency","arguments":{}}"#),
    ];
    for (i, (name, params)) in cases.iter().enumerate() {
        let req = format!(
            r#"{{"jsonrpc":"2.0","id":{},"method":"tools/call","params":{}}}"#,
            i + 40,
            params
        );
        let res = server.handle_request(req.as_bytes());
        assert!(res.contains(r#""isError":false"#), "{} failed: {}", name, res);
        assert!(res.contains(r#"\"status\":\"ok\""#), "{} missing ok: {}", name, res);
    }
}
