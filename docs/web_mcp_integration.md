# WebGUI-MCP Agent — Validation & Performance Notes

## 6.1 End-to-end validation flow
1. Flash AgentLA.uf2 to RP2350
2. Verify USB enumeration (CDC+HID+network)
3. Browse to agentla.lab / 192.168.7.1 (expects web UI)
4. Test MCP: GET /mcp?method=list
5. Test UI actions (Flasher, Analyzer, Debugger, GPIO)
6. Expected behavior: JSON responses, UI shows status

## 6.2 Performance sanity
- Memory budget: 512KB SRAM; capture buffer reduced to 64*3KB to fit
- Network lwIP core memory: 8KB (MEM_SIZE), 16 PBUFs
- Additional web asset size: 81KB embedded
- Keep code compile sizes under 250KB flash

## 6.3 Documentation
Change reference `webgui-mcp-agent` under openspec/changes:
- proposal.md, design.md, specs/network-access/spec.md, tasks.md all aligned with implementation
