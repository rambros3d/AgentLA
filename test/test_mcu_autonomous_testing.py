#!/usr/bin/env python3
"""
Unit test suite for Universal MCU Autonomous Testing Skill & Harness.
Validates:
- Canonical Skill location at SKILLS/mcu-autonomous-testing/SKILL.md (not .agent/skills or .agents/skills)
- Skill specification adherence (YAML frontmatter, 5-phase protocol, safety rules)
- Automated LittleFS packaging in tools/pack_webui.py
- McpRpcClient and run_closed_loop_test execution in tools/mcu_test_runner.py
- TAP 13 and Markdown summary report formatting
- CoreSight SCB register HardFault root-cause triage (CFSR, HFSR, BFAR)
- MCP server resource registration (agentla://skills/mcu-autonomous-testing)
"""

import json
import os
import sys
from pathlib import Path
import pytest

REPO_ROOT = Path(__file__).resolve().parent.parent
sys.path.insert(0, str(REPO_ROOT / "tools"))

from mcu_test_runner import McpRpcClient, TestReport, run_closed_loop_test


def test_skill_canonical_path_and_no_legacy_dirs():
    """Ensure all 4 canonical skills exist under SKILLS/ and obsolete mcu-autonomous-testing is removed."""
    skills_dir = REPO_ROOT / "SKILLS"
    for name in ["agentla-dev", "agentla-debug", "agentla-programmer", "agentla-analyzer"]:
        skill_file = skills_dir / name / "SKILL.md"
        assert skill_file.is_file(), f"Expected canonical skill at {skill_file}"

    # Verify retired mcu-autonomous-testing does not exist
    deprecated_skill = skills_dir / "mcu-autonomous-testing"
    assert not deprecated_skill.exists(), f"Deprecated skill must be removed: {deprecated_skill}"

    # Verify .agent/skills and .agents/skills do not contain rogue duplicate directories
    rogue1 = REPO_ROOT / ".agent" / "skills" / "mcu-autonomous-testing"
    rogue2 = REPO_ROOT / ".agents" / "skills" / "mcu-autonomous-testing"
    assert not rogue1.exists(), f"Rogue skill directory must not exist: {rogue1}"
    assert not rogue2.exists(), f"Rogue skill directory must not exist: {rogue2}"


def test_skill_document_content():
    """Verify that all 4 skills adhere to the canonical AgentLA skill specification structure."""
    skills_dir = REPO_ROOT / "SKILLS"

    # 1. agentla-dev
    dev_content = (skills_dir / "agentla-dev" / "SKILL.md").read_text(encoding="utf-8")
    assert "name: agentla-dev" in dev_content
    assert "tools_required: [\"agentla-mcp\"]" in dev_content
    assert "Required Hardware Connections" in dev_content
    assert "logic_capture_trigger" in dev_content
    assert "TAP 13" in dev_content

    # 2. agentla-debug
    debug_content = (skills_dir / "agentla-debug" / "SKILL.md").read_text(encoding="utf-8")
    assert "name: agentla-debug" in debug_content
    assert "CFSR" in debug_content
    assert "HFSR" in debug_content
    assert "DHCSR" in debug_content
    assert "target_diagnose" in debug_content

    # 3. agentla-programmer
    prog_content = (skills_dir / "agentla-programmer" / "SKILL.md").read_text(encoding="utf-8")
    assert "name: agentla-programmer" in prog_content
    assert "catalog_lookup" in prog_content
    assert "programmer_flash" in prog_content
    assert "programmer_unbrick" in prog_content

    # 4. agentla-analyzer
    ana_content = (skills_dir / "agentla-analyzer" / "SKILL.md").read_text(encoding="utf-8")
    assert "name: agentla-analyzer" in ana_content
    assert "pins_claim" in ana_content
    assert "pins_release" in ana_content
    assert "mcp_sniff_bus" in ana_content


def test_pack_webui_includes_skill():
    """Verify pack_webui discovers and compresses the 4 skills suite into LittleFS."""
    from pack_webui import pack_webui, PARTITION_SIZE

    build_dir = REPO_ROOT / "build"
    img_path = build_dir / "test_webui.img"
    uf2_path = build_dir / "test_webui.uf2"
    tar_path = build_dir / "test_webui.tar"

    result = pack_webui(
        web_dir=REPO_ROOT / "webui",
        output_img=img_path,
        output_uf2=uf2_path,
        output_tar=tar_path,
    )

    for skill in ["agentla-dev", "agentla-debug", "agentla-programmer", "agentla-analyzer"]:
        assert f"skills/{skill}/SKILL.md" in result["asset_names"], f"Missing {skill} in packed assets"

    assert "skills/mcu-autonomous-testing/SKILL.md" not in result["asset_names"], "Deprecated skill should not be packed"

    headroom_pct = result["headroom_bytes"] / PARTITION_SIZE * 100
    assert headroom_pct > 30.0, f"Headroom {headroom_pct:.1f}% should be > 30%"

    # Clean up test files
    for p in (img_path, uf2_path, tar_path):
        if p.exists():
            p.unlink()


def test_mcu_test_runner_passing_flow():
    """Test the complete closed-loop execution flow on a passing target."""
    call_log = []

    def mock_passing_handler(name: str, args: dict):
        call_log.append(name)
        if name == "status":
            return False, {"vtarget_mv": 3310, "overcurrent": False}
        elif name == "target_diagnose":
            return False, {"pins": {"swclk": "ok", "swdio": "ok"}}
        elif name == "probe_target":
            return False, {"name": "STM32F103C8T6"}
        elif name == "inspect_firmware":
            return False, {"stack_pointer_valid": True, "thumb_mode_valid": True}
        elif name == "programmer_flash":
            return False, {"status": "ok"}
        elif name == "programmer_reset":
            return False, {"status": "ok"}
        elif name == "uart_command":
            return False, {"response": "help: system ready\r\n"}
        return False, {}

    client = McpRpcClient(mock_handler=mock_passing_handler)
    report = run_closed_loop_test(
        client,
        firmware_hex="0001020304050607",
        flash_address=0x08000000,
        expected_uart_token="help",
        uart_baud=115200,
    )

    # All phases passed
    assert all(c["ok"] for c in report.cases)
    assert len(report.cases) >= 5

    # TAP 13 formatting
    tap = report.to_tap13()
    assert "TAP version 13" in tap
    assert "ok 1 - Phase 1: Electrical Health" in tap
    assert "ok 2 - Phase 1: Pin Continuity" in tap
    assert "not ok" not in tap

    # Markdown table formatting
    md = report.to_markdown_table()
    assert "| Phase | Subsystem | Result | Verdict | Details |" in md
    assert "**PASS**" in md
    assert "**FAIL**" not in md


def test_mcu_test_runner_failing_flow_with_hardfault_triage():
    """Test failing execution flow triggering Phase 5 HardFault triage synthesis."""
    call_log = []

    def mock_failing_handler(name: str, args: dict):
        call_log.append(name)
        if name == "status":
            return False, {"vtarget_mv": 3300, "overcurrent": False}
        elif name == "target_diagnose":
            if "uart_command" in call_log:
                # Phase 5 triage call
                return False, {
                    "pins": {"swclk": "ok", "swdio": "ok"},
                    "root_cause": "UsageFault_DIVBYZERO: Division by zero at PC=0x08000214 (CFSR=0x02000000)",
                }
            return False, {"pins": {"swclk": "ok", "swdio": "ok"}}
        elif name == "probe_target":
            return False, {"name": "STM32F103C8T6"}
        elif name == "inspect_firmware":
            return False, {"stack_pointer_valid": True, "thumb_mode_valid": True}
        elif name == "programmer_flash":
            return False, {"status": "ok"}
        elif name == "programmer_reset":
            return False, {"status": "ok"}
        elif name == "uart_command":
            return True, {"response": ""}  # Stimulus timed out / failed
        return False, {}

    client = McpRpcClient(mock_handler=mock_failing_handler)
    report = run_closed_loop_test(
        client,
        firmware_hex="0001020304050607",
        flash_address=0x08000000,
        expected_uart_token="help",
        uart_baud=115200,
    )

    # Stimulus case should fail with diagnostic
    failed_cases = [c for c in report.cases if not c["ok"]]
    assert len(failed_cases) == 1
    stim_fail = failed_cases[0]
    assert stim_fail["phase"] == "Phase 4"
    assert "UsageFault_DIVBYZERO" in stim_fail["diag"]

    # TAP 13 formatting reflects failure and diagnostic
    tap = report.to_tap13()
    assert "not ok" in tap
    assert "UsageFault_DIVBYZERO" in tap

    # Markdown table formatting reflects FAIL
    md = report.to_markdown_table()
    assert "**FAIL**" in md
    assert "UsageFault_DIVBYZERO" in md
