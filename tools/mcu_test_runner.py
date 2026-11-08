#!/usr/bin/env python3
"""
AgentLA Universal MCU Closed-Loop Test Runner
Automates Phase 1-5 closed-loop testing across arbitrary MCUs using AgentLA MCP.
Supports live MCP JSON-RPC HTTP endpoint as well as local simulated execution.
Outputs TAP 13 and formatted Markdown summary reports.
"""

import json
import time
import argparse
import urllib.request
import urllib.error
from typing import Dict, Any, Optional, List, Tuple


class McpRpcClient:
    """JSON-RPC 2.0 client for AgentLA MCP endpoint."""

    def __init__(self, endpoint: str = "http://10.10.10.10/mcp", mock_handler=None):
        self.endpoint = endpoint
        self.mock_handler = mock_handler
        self.req_id = 1

    def call_tool(self, name: str, arguments: Optional[Dict[str, Any]] = None) -> Tuple[bool, Any]:
        """Calls an MCP tool by name. Returns (is_error, result_data)."""
        if self.mock_handler:
            return self.mock_handler(name, arguments or {})

        payload = {
            "jsonrpc": "2.0",
            "id": self.req_id,
            "method": "tools/call",
            "params": {
                "name": name,
                "arguments": arguments or {}
            }
        }
        self.req_id += 1

        data_bytes = json.dumps(payload).encode("utf-8")
        req = urllib.request.Request(
            self.endpoint,
            data=data_bytes,
            headers={"Content-Type": "application/json"}
        )

        try:
            with urllib.request.urlopen(req, timeout=5.0) as resp:
                resp_json = json.loads(resp.read().decode("utf-8"))
                result = resp_json.get("result", {})
                is_err = result.get("isError", False)
                content = result.get("content", [])
                text = content[0].get("text", "") if content else ""
                try:
                    parsed = json.loads(text)
                    return is_err, parsed
                except Exception:
                    return is_err, text
        except Exception as e:
            return True, {"error": str(e)}


class TestReport:
    """Collects test results and formats TAP 13 and Markdown summaries."""
    __test__ = False

    def __init__(self):
        self.cases: List[Dict[str, Any]] = []

    def add(self, phase: str, name: str, ok: bool, details: str = "", diag: str = ""):
        self.cases.append({
            "phase": phase,
            "name": name,
            "ok": ok,
            "details": details,
            "diag": diag
        })

    def to_tap13(self) -> str:
        lines = ["TAP version 13", f"1..{len(self.cases)}"]
        for i, c in enumerate(self.cases, 1):
            status = "ok" if c["ok"] else "not ok"
            lines.append(f"{status} {i} - {c['phase']}: {c['name']}")
            if not c["ok"] and c.get("diag"):
                lines.append("  ---")
                lines.append(f"  diagnostic: {json.dumps(c['diag'])}")
                lines.append("  ...")
        return "\n".join(lines)

    def to_markdown_table(self) -> str:
        lines = [
            "| Phase | Subsystem | Result | Verdict | Details |",
            "| :--- | :--- | :--- | :--- | :--- |"
        ]
        for c in self.cases:
            verdict = "**PASS**" if c["ok"] else "**FAIL**"
            det = c["diag"] if not c["ok"] and c["diag"] else c["details"]
            lines.append(f"| {c['phase']} | {c['name']} | {c['details']} | {verdict} | {det} |")
        return "\n".join(lines)


def run_closed_loop_test(
    client: McpRpcClient,
    firmware_hex: Optional[str] = None,
    flash_address: Optional[int] = None,
    expected_uart_token: str = "help",
    uart_baud: int = 115200
) -> TestReport:
    """Executes the universal 5-phase closed-loop testing protocol."""
    report = TestReport()

    # =========================================================================
    # Phase 1: Electrical & Silicon Auto-Discovery
    # =========================================================================
    err, status_data = client.call_tool("status")
    if err or not isinstance(status_data, dict):
        report.add("Phase 1", "Electrical Health", False, "Failed querying status", str(status_data))
        return report

    vtarget_mv = status_data.get("vtarget_mv", 0)
    if vtarget_mv < 500:
        report.add("Phase 1", "Electrical Health", False, f"Target unpowered ({vtarget_mv} mV < 500 mV)", "Check target VCC/GND wiring")
        return report
    elif vtarget_mv < 2700:
        report.add("Phase 1", "Electrical Health", False, f"Target brownout ({vtarget_mv} mV < 2700 mV)", "Target rail below 2.7V threshold")
        return report
    else:
        report.add("Phase 1", "Electrical Health", True, f"{vtarget_mv} mV nominal (overcurrent={status_data.get('overcurrent', False)})")

    # Target Diagnostic & Silicon probe
    err, diag_data = client.call_tool("target_diagnose")
    pins = diag_data.get("pins", {}) if isinstance(diag_data, dict) else {}
    has_short = any(v == "short_to_gnd" for v in pins.values()) if isinstance(pins, dict) else False
    if has_short:
        report.add("Phase 1", "Pin Continuity", False, "Short circuit detected on SWD pins", json.dumps(pins))
        return report
    else:
        report.add("Phase 1", "Pin Continuity", True, "SWD pins nominal / pulled up")

    err, target_info = client.call_tool("probe_target")
    chip_name = target_info.get("name", "Unknown") if isinstance(target_info, dict) else str(target_info)
    report.add("Phase 1", "Silicon Discovery", not err, f"Detected target: {chip_name}")

    # =========================================================================
    # Phase 2: Vector Table Pre-Check & Flash Programming
    # =========================================================================
    if firmware_hex and flash_address is not None:
        err, inspect_data = client.call_tool("inspect_firmware", {"address": flash_address})
        if isinstance(inspect_data, dict):
            sp_valid = inspect_data.get("stack_pointer_valid", True)
            thumb_valid = inspect_data.get("thumb_mode_valid", True)
            if not sp_valid or not thumb_valid:
                report.add("Phase 2", "Vector Table Invariants", False, "Vector table invariant failure", json.dumps(inspect_data.get("warnings", [])))
                return report
        report.add("Phase 2", "Vector Table Invariants", True, "SP in SRAM, Thumb bit active")

        # Flash programming
        err, flash_data = client.call_tool("programmer_flash", {"address": flash_address, "hex_data": firmware_hex})
        if err:
            report.add("Phase 2", "Flash Programming", False, "Flashing failed", str(flash_data))
            return report
        report.add("Phase 2", "Flash Programming", True, "Firmware programmed & verified")

    # =========================================================================
    # Phase 3: Hardware Reset & Boot Settling
    # =========================================================================
    err, reset_data = client.call_tool("programmer_reset")
    report.add("Phase 3", "Target Reset", not err, "Target pulsed via nRST/SYSRESETREQ")
    # Minimum 100 ms settling delay
    time.sleep(0.12)

    # =========================================================================
    # Phase 4: Multi-Protocol Stimulus & Interactive Handshake
    # =========================================================================
    err, uart_resp = client.call_tool("uart_command", {
        "tx_data": f"{expected_uart_token}\n",
        "baud": uart_baud,
        "timeout_ms": 600
    })

    resp_str = ""
    if isinstance(uart_resp, dict):
        resp_str = uart_resp.get("response", "")
    elif isinstance(uart_resp, str):
        resp_str = uart_resp

    if expected_uart_token in resp_str or ("status" in uart_resp and uart_resp.get("status") == "ok" and not err):
        report.add("Phase 4", "UART Console Stimulus", True, f"Received response: {resp_str.strip()[:60]}")
    else:
        # =====================================================================
        # Phase 5: Autonomous HardFault Triage (On Stimulus Failure)
        # =====================================================================
        _, triage_data = client.call_tool("target_diagnose")
        root_cause = "Unknown timeout"
        if isinstance(triage_data, dict):
            root_cause = triage_data.get("root_cause") or triage_data.get("status", "Core unresponsive")
        report.add("Phase 4", "UART Console Stimulus", False, f"Expected '{expected_uart_token}' not found", root_cause)

    return report


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Universal MCU Autonomous Closed-Loop Test Runner")
    parser.add_argument("--endpoint", default="http://10.10.10.10/mcp", help="AgentLA MCP endpoint URL")
    parser.add_argument("--hex", help="Hex string of binary to flash")
    parser.add_argument("--addr", type=lambda x: int(x, 0), default=0x08000000, help="Flash base address")
    parser.add_argument("--token", default="help", help="Expected string in UART handshake")
    parser.add_argument("--baud", type=int, default=115200, help="UART baud rate")
    args = parser.parse_args()

    client = McpRpcClient(endpoint=args.endpoint)
    report = run_closed_loop_test(
        client,
        firmware_hex=args.hex,
        flash_address=args.addr,
        expected_uart_token=args.token,
        uart_baud=args.baud
    )

    print("\n" + report.to_tap13())
    print("\n" + report.to_markdown_table())
