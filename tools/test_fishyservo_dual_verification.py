#!/usr/bin/env python3
"""
FishyServo Dual SWD + I2C Closed-Loop Position Control Verification
- Telemetry & RTT: Read via SWD (WCH-Link A1A48F062680)
- Control & Multibus: Controlled via AgentLA REST I2C (http://10.10.10.10)
"""

import sys
import time
import math
import struct
import urllib.request
import json
from pyocd.core.helpers import ConnectHelper

PROBE_ID = "A1A48F062680"
TARGET_TYPE = "stm32h503kbux"
BASE_URL = "http://10.10.10.10"
I2C_ADDR = 0x40

# Multibus register offsets
REG_TARGET = 0x01
REG_ENABLE = 0x04
REG_ANGLE = 0x09


def i2c_post(path, payload):
    req = urllib.request.Request(
        f"{BASE_URL}{path}",
        data=json.dumps(payload).encode(),
        headers={"Content-Type": "application/json"},
        method="POST"
    )
    for _ in range(5):
        try:
            with urllib.request.urlopen(req, timeout=5) as r:
                return json.loads(r.read().decode())
        except Exception:
            time.sleep(0.02)
    return {}


def i2c_write_reg(reg, val_bytes):
    data_hex = f"{reg:02x}" + val_bytes.hex()
    return i2c_post("/api/v1/bus/i2c/transfer", {
        "addr": I2C_ADDR,
        "data_hex": data_hex,
        "read_len": 0
    })


def i2c_read_reg(reg, n_bytes):
    data_hex = f"{reg:02x}"
    res = i2c_post("/api/v1/bus/i2c/transfer", {
        "addr": I2C_ADDR,
        "data_hex": data_hex,
        "read_len": n_bytes
    })
    rx_hex = res.get("rx_hex", "")
    if len(rx_hex) >= n_bytes * 2:
        return bytes.fromhex(rx_hex[:n_bytes * 2])
    return None


def i2c_set_target(rad):
    val_bytes = struct.pack("<f", rad)
    i2c_write_reg(REG_TARGET, val_bytes)


def i2c_set_enable(enabled):
    val_bytes = bytes([1 if enabled else 0])
    i2c_write_reg(REG_ENABLE, val_bytes)


def i2c_get_angle():
    b = i2c_read_reg(REG_ANGLE, 4)
    if b and len(b) == 4:
        return struct.unpack("<f", b)[0]
    return None


def main():
    print("=================================================================")
    print("    FISHYSERVO DUAL SWD + I2C CLOSED-LOOP POSITION VERIFICATION")
    print("    - Telemetry Probe: WCH-Link (SWD, ID: " + PROBE_ID + ")")
    print("    - Control Bus: AgentLA REST I2C (RP2350 @ 10.10.10.10)")
    print("=================================================================")

    # 1. Connect to SWD session
    print("\nConnecting to SWD session via pyocd...")
    session = ConnectHelper.session_with_chosen_probe(
        unique_id=PROBE_ID,
        target_override=TARGET_TYPE,
        options={"connect_mode": "attach", "frequency": 500000}
    )
    session.open()
    target = session.board.target
    if target.is_halted():
        target.resume()
    print("SWD session established.")

    # Locate RTT Control Block
    ram = bytes(target.read_memory_block8(0x20000000, 0x8000))
    idx = ram.find(b"SEGGER RTT\0")
    if idx == -1:
        print("ERROR: SEGGER RTT header not found in RAM")
        session.close()
        sys.exit(1)

    rtt_cb = 0x20000000 + idx
    up_hdr = target.read_memory_block32(rtt_cb + 0x18, 6)
    up_buf_ptr, up_buf_size = up_hdr[1], up_hdr[2]
    print(f"RTT Control Block found at 0x{rtt_cb:08X} (Up buffer: 0x{up_buf_ptr:08X}, size {up_buf_size})")

    def drain_rtt():
        hdr = target.read_memory_block32(rtt_cb + 0x18, 6)
        wr_off, rd_off = hdr[3], hdr[4]
        if wr_off == rd_off:
            return ""
        out = bytearray()
        while rd_off != wr_off:
            out.append(target.read8(up_buf_ptr + rd_off))
            rd_off = (rd_off + 1) % up_buf_size
        target.write32(rtt_cb + 0x18 + 16, rd_off)
        return out.decode("utf-8", errors="replace")

    drain_rtt()
    time.sleep(0.1)

    # 2. Check I2C communication
    init_ang = i2c_get_angle()
    print(f"\nInitial State: I2C Angle = {init_ang:.4f} rad ({init_ang*180.0/math.pi:+.2f} deg)")

    # 3. Enable Motor over I2C
    print("\n[Action] Enabling motor over I2C...")
    i2c_set_enable(True)
    time.sleep(0.2)

    def test_step(target_rad, label, timeout_s=3.0):
        target_deg = target_rad * 180.0 / math.pi
        print(f"\n---> [STEP] Command Target: {target_rad:+.2f} rad ({target_deg:+5.1f} deg) ({label})")
        i2c_set_target(target_rad)

        start = time.time()
        settled = False
        while time.time() - start < timeout_s:
            time.sleep(0.2)
            ang = i2c_get_angle()
            rtt_txt = drain_rtt().strip()
            rtt_last = rtt_txt.splitlines()[-1] if rtt_txt else ""
            if ang is not None:
                deg = ang * 180.0 / math.pi
                err = ang - target_rad
                print(f"  t={time.time()-start:4.2f}s | I2C Angle={ang:+.2f} rad ({deg:+5.1f} deg, err={err:+.3f}) | RTT: {rtt_last}")
                if abs(err) < 0.08:
                    settled = True
                    break

        if settled:
            print(f"  ==> SUCCESS: Settled at target within {time.time()-start:.2f}s!")
            return True
        else:
            print(f"  ==> WARNING: Did not settle within {timeout_s}s.")
            return False

    try:
        # Step 1: Target = +0.35 rad (+20.0 deg)
        ok1 = test_step(0.35, "Positive deflection (+20.0 deg)")
        time.sleep(0.3)

        # Step 2: Target = -0.35 rad (-20.0 deg)
        ok2 = test_step(-0.35, "Negative deflection (-20.0 deg)")
        time.sleep(0.3)

        # Step 3: Target = +0.50 rad (+28.6 deg)
        ok3 = test_step(0.50, "Larger positive deflection (+28.6 deg)")
        time.sleep(0.3)

        # Step 4: Target = 0.00 rad (Return to Center)
        ok4 = test_step(0.00, "Return to Center (0.0 deg)")
        time.sleep(0.3)

        # 4. Disable Motor
        print("\n[Action] Disabling motor over I2C...")
        i2c_set_enable(False)
        time.sleep(0.2)
        final_ang = i2c_get_angle()
        print(f"Final Angle: {final_ang:.4f} rad ({final_ang*180.0/math.pi:+.2f} deg)")

        print("\n=================================================================")
        if all([ok1, ok2, ok3, ok4]):
            print("   ALL TESTS PASSED: CLOSED-LOOP POSITION CONTROL VERIFIED!")
        else:
            print("   CLOSED-LOOP VERIFICATION COMPLETED WITH WARNINGS.")
        print("=================================================================")

    finally:
        session.close()


if __name__ == "__main__":
    main()
