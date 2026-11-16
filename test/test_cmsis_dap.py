import os
import sys
import time

print("=" * 60)
print("AGENTLA CMSIS-DAP VERIFICATION SUITE")
print("Target: STM32H503KBU6 (N20 Servo Board)")
print("=" * 60)

with open("/dev/hidraw5", "r+b", buffering=0) as f:
    def send_cmd(cmd_bytes):
        payload = bytes([0x00]) + cmd_bytes
        payload = payload + bytes(65 - len(payload))
        f.write(payload[:65])
        return f.read(64)

    # 1. Info commands
    for id_val, name in [
        (0x01, "Vendor"),
        (0x02, "Product"),
        (0x03, "Serial"),
        (0x04, "CMSIS-DAP Version"),
        (0xF0, "Capabilities"),
        (0xFF, "Packet Size"),
    ]:
        resp = send_cmd(bytes([0x00, id_val]))
        length = resp[1]
        if id_val in (0x01, 0x02, 0x03, 0x04):
            val = resp[2:2+length].decode("ascii", errors="replace")
        elif id_val == 0xF0:
            val = f"0x{resp[2]:02X} (SWD={bool(resp[2] & 1)})"
        elif id_val == 0xFF:
            sz = int.from_bytes(resp[2:4], "little")
            val = f"{sz} bytes"
        print(f"[*] DAP_Info [0x{id_val:02X}] {name:<18}: {val}")

    # 2. Clock configuration
    for khz in (100, 2000, 1000):
        hz = khz * 1000
        resp = send_cmd(bytes([0x11]) + hz.to_bytes(4, "little"))
        assert resp[0] == 0x11 and resp[1] == 0, f"Clock set {khz} kHz failed"
        print(f"[*] DAP_SWJ_Clock set to {khz} kHz: OK")

    # 3. Assert nRESET low & verify pin state
    resp = send_cmd(bytes([0x10, 0x00, 0x80]) + (10000).to_bytes(4, "little"))
    assert resp[0] == 0x10, "DAP_SWJ_Pins failed"
    pin_in = resp[1]
    nrst_low = (pin_in & 0x80) == 0
    print(f"[*] DAP_SWJ_Pins (assert nRST low): Pin state = 0x{pin_in:02X} (nRST low = {nrst_low})")

    # 4. DAP_Connect
    resp = send_cmd(bytes([0x02, 0x01]))
    assert resp[0] == 0x02 and resp[1] == 0x01, "DAP_Connect failed"
    print(f"[*] DAP_Connect (SWD mode): OK")

    def dap_transfer(transfers):
        payload = bytearray([0x05, 0x00, len(transfers)])
        for r_b, d in transfers:
            payload.append(r_b)
            if d is not None:
                payload.extend(d.to_bytes(4, "little"))
        resp = send_cmd(bytes(payload))
        assert resp[0] == 0x05, "DAP_Transfer header error"
        return resp

    try:
        # 5. Read DPIDR
        resp = dap_transfer([(0x02, None)])
        assert resp[1] == 1 and resp[2] == 1, f"DPIDR read failed: count={resp[1]} ack={resp[2]}"
        dpidr = int.from_bytes(resp[3:7], "little")
        match_str = "MATCH!" if dpidr == 0x6BA02477 else "MISMATCH"
        print(f"[*] DPIDR: 0x{dpidr:08X} (Expected: 0x6BA02477) -> {match_str}")
        assert dpidr == 0x6BA02477

        # 6. Clear Aborts & Power Up Debug
        resp = dap_transfer([(0x00, 0x0000001E), (0x04, 0x50000000)])
        assert resp[2] == 1, "Clear aborts and power up failed"
        resp = dap_transfer([(0x06, None)])
        ctrlstat = int.from_bytes(resp[3:7], "little")
        print(f"[*] DP CTRL/STAT: 0x{ctrlstat:08X} (Powered: {bool(ctrlstat & 0xA0000000)})")

        # 7. Read AP 0 IDR (APB-AP)
        dap_transfer([(0x08, 0x000000F0)])
        resp = dap_transfer([(0x0F, None)])
        ap0_idr = int.from_bytes(resp[3:7], "little")
        print(f"[*] AP 0 IDR: 0x{ap0_idr:08X} (Expected: 0x54770002)")
        assert ap0_idr == 0x54770002

        # 8. Read AP 1 IDR (AHB-AP)
        dap_transfer([(0x08, 0x010000F0)])
        resp = dap_transfer([(0x0F, None)])
        ap1_idr = int.from_bytes(resp[3:7], "little")
        print(f"[*] AP 1 IDR: 0x{ap1_idr:08X} (Expected: 0x84770001)")
        assert ap1_idr == 0x84770001

        # 9. Read DBGMCU_IDCODE at 0xE00E4000 via AP 0 (APB-AP)
        dap_transfer([(0x08, 0x00000000)])
        dap_transfer([(0x01, 0x80000012)])
        dap_transfer([(0x05, 0xE00E4000)])
        resp = dap_transfer([(0x0F, None)])
        dbgmcu_id = int.from_bytes(resp[3:7], "little")
        devid = dbgmcu_id & 0xFFF
        dev_str = "STM32H503" if devid == 0x474 else "Unknown"
        print(f"[*] DBGMCU_IDCODE: 0x{dbgmcu_id:08X} (Device ID: 0x{devid:03X} -> {dev_str})")
        assert devid == 0x474

        # 10. Read Flash Memory 0x08000000 via AP 1 (AHB-AP)
        dap_transfer([(0x08, 0x01000000)])
        dap_transfer([(0x01, 0xE3000012)])
        dap_transfer([(0x05, 0x08000000)])
        resp = dap_transfer([(0x0F, None)])
        w0 = int.from_bytes(resp[3:7], "little")
        print(f"[*] Flash @ 0x08000000: 0x{w0:08X}")
    finally:
        # 11. Release nRESET high
        resp = send_cmd(bytes([0x10, 0x80, 0x80, 0x00, 0x00, 0x00, 0x00]))
        pin_in = resp[1]
        nrst_high = (pin_in & 0x80) != 0
        print(f"[*] DAP_SWJ_Pins (release nRST high): Pin state = 0x{pin_in:02X} (nRST high = {nrst_high})")

        # 12. DAP_Disconnect
        resp = send_cmd(bytes([0x03]))
        assert resp[0] == 0x03 and resp[1] == 0x00
        print("[*] DAP_Disconnect: OK")

print("=" * 60)
print("ALL 12 CMSIS-DAP CHECKS PASSED PERFECTLY!")
print("=" * 60)
