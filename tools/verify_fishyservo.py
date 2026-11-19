#!/usr/bin/env python3
"""
FishyServo Closed-Loop Position Control Verification Script
Debugger: AgentLA CMSIS-DAP Probe (Pi Pico 2 RP2350, serial: C000DDC77247F500414D0001)
Target: FishyServo N20 (STM32H503KBU6 Cortex-M33)
Sensor: TI TMAG5273 Magnetic Angle Sensor on I2C1 (PB6/PB7 @ 0x35)
"""

import sys
import time
import math
import struct
from pyocd.core.helpers import ConnectHelper

PROBE_ID = "C000DDC77247F500414D0001"
TARGET_TYPE = "stm32h503kbux"

RTT_CB_ADDR = 0x20002350

UP_BUF_ADDR = 0x20000055
UP_BUF_SIZE = 2048
DOWN_BUF_ADDR = 0x20000855
DOWN_BUF_SIZE = 128

UP_WR_ADDR = RTT_CB_ADDR + 7 * 4
UP_RD_ADDR = RTT_CB_ADDR + 8 * 4
DOWN_WR_ADDR = RTT_CB_ADDR + 13 * 4
DOWN_RD_ADDR = RTT_CB_ADDR + 14 * 4


class FishyServoController:
    def __init__(self):
        print(f"Connecting to AgentLA CMSIS-DAP Probe ({PROBE_ID})...")
        self.session = ConnectHelper.session_with_chosen_probe(
            unique_id=PROBE_ID,
            target_override=TARGET_TYPE,
            resume_on_disconnect=False,
            options={"auto_unlock": False}
        )
        self.session.open()
        self.target = self.session.board.target
        print("Connected to STM32H503 via AgentLA.")

    def close(self):
        try:
            self.session.close()
        except Exception:
            pass

    def send_cmd(self, cmd_str):
        cmd_bytes = cmd_str.encode("ascii") + b"\n"
        wr = self.target.read32(DOWN_WR_ADDR)
        
        # We must use aligned 32-bit writes for STM32H5 AHB-AP
        aligned_base = (DOWN_BUF_ADDR + wr) & ~3
        offset_in_word = (DOWN_BUF_ADDR + wr) - aligned_base
        n_words = (offset_in_word + len(cmd_bytes) + 3) // 4
        
        words = list(self.target.read_memory_block32(aligned_base, n_words))
        buf = bytearray()
        for w in words:
            buf.extend(struct.pack("<I", w))
            
        for i, b in enumerate(cmd_bytes):
            buf[offset_in_word + i] = b
            
        new_words = []
        for i in range(n_words):
            new_words.append(struct.unpack("<I", buf[i * 4:(i + 1) * 4])[0])
            
        self.target.write_memory_block32(aligned_base, new_words)
        new_wr = (wr + len(cmd_bytes)) % DOWN_BUF_SIZE
        self.target.write32(DOWN_WR_ADDR, new_wr)

    def flush_up_buffer(self):
        wr = self.target.read32(UP_WR_ADDR)
        self.target.write32(UP_RD_ADDR, wr)

    def read_telemetry_lines(self):
        wr = self.target.read32(UP_WR_ADDR)
        rd = self.target.read32(UP_RD_ADDR)
        
        if wr == rd:
            return []
            
        if wr > rd:
            count = wr - rd
            read_start = rd
        else:
            # Wrapped
            count = wr
            read_start = 0
            
        aligned_base = (UP_BUF_ADDR + read_start) & ~3
        offset_in_word = (UP_BUF_ADDR + read_start) - aligned_base
        n_words = (offset_in_word + count + 3) // 4
        
        words = self.target.read_memory_block32(aligned_base, n_words)
        raw = bytearray()
        for w in words:
            raw.extend(struct.pack("<I", w))
            
        self.target.write32(UP_RD_ADDR, wr)
        
        text = raw[offset_in_word:offset_in_word + count].decode("ascii", errors="replace")
        lines = []
        for line in text.splitlines():
            line = line.strip()
            if line:
                lines.append(line)
        return lines

    def parse_ang(self, line):
        # Format: [ANG] tgt=0.00 (0.0 deg) | ang=0.01 (0.4 deg) | vel=-0.2 rad/s | I=9.6 mA | en=1
        if not (line.startswith("[ANG]") and "| en=" in line):
            return None
        try:
            parts = line.split("|")
            tgt = float(parts[0].split("tgt=")[1].split()[0])
            ang = float(parts[1].split("ang=")[1].split()[0])
            vel = float(parts[2].split("vel=")[1].split()[0])
            curr = float(parts[3].split("I=")[1].split()[0])
            en = int(parts[4].split("en=")[1].split()[0])
            return {
                "tgt": tgt,
                "ang": ang,
                "vel": vel,
                "current_ma": curr,
                "enabled": en,
                "raw": line
            }
        except Exception:
            return None


def run_position_step(servo, target_rad, name, timeout_s=4.0):
    target_deg = target_rad * 180.0 / math.pi
    print(f"\n---> [STEP] Command Target: {target_rad:+.2f} rad ({target_deg:+5.1f} deg) ({name})")
    servo.send_cmd(f"t {target_rad:.3f}")
    
    start = time.time()
    settled = False
    samples = []
    
    while time.time() - start < timeout_s:
        time.sleep(0.1)
        lines = servo.read_telemetry_lines()
        for line in lines:
            sample = servo.parse_ang(line)
            if sample:
                samples.append(sample)
                elapsed = time.time() - start
                deg = sample["ang"] * 180.0 / math.pi
                err = sample["ang"] - target_rad
                print(
                    f"  t={elapsed:4.2f}s | tgt={sample['tgt']:+.2f} rad | "
                    f"ang={sample['ang']:+.2f} rad ({deg:+5.1f} deg) | "
                    f"err={err:+.3f} rad | vel={sample['vel']:+5.1f} rad/s | "
                    f"I={sample['current_ma']:5.1f} mA | en={sample['enabled']}"
                )
                
                # Settle condition: error < 0.05 rad (~2.8 deg) and low velocity
                if abs(err) < 0.05 and abs(sample["vel"]) < 1.0:
                    settled = True
                    break
        if settled:
            break
            
    if settled:
        print(f"  ==> SUCCESS: Settled at {target_rad:+.2f} rad within {time.time()-start:.2f}s!")
        return True
    else:
        print(f"  ==> WARNING: Did not reach settle criteria within {timeout_s}s.")
        return False


def main():
    print("=================================================================")
    print("   FISHYSERVO CLOSED-LOOP POSITION MODE VERIFICATION SUITE")
    print("   Controlled via AgentLA CMSIS-DAP Probe on Pi Pico 2 (RP2350)")
    print("   Target: STM32H503KBU6 | Sensor: TI TMAG5273 (I2C1 PB6/PB7)")
    print("=================================================================")

    servo = FishyServoController()
    try:
        # Flush any existing buffer
        servo.flush_up_buffer()
        time.sleep(0.1)
        
        # 1. Read initial state
        initial_lines = servo.read_telemetry_lines()
        print("\n[Step 0] Initial target telemetry:")
        for l in initial_lines[-4:]:
            print("  ", l)
            
        # 2. Enable Motor
        print("\n[Step 1] Enabling Motor (command: 'e 1')...")
        servo.send_cmd("e 1")
        time.sleep(0.3)
        lines = servo.read_telemetry_lines()
        for l in lines[-3:]:
            print("  ", l)
            
        # 3. Step 1: Target = +0.35 rad (+20.0 deg)
        ok1 = run_position_step(servo, 0.35, "Positive deflection (+20.0 deg)")
        time.sleep(0.5)
        
        # 4. Step 2: Target = -0.35 rad (-20.0 deg)
        ok2 = run_position_step(servo, -0.35, "Negative deflection (-20.0 deg)")
        time.sleep(0.5)
        
        # 5. Step 3: Target = 0.00 rad (Center 0.0 deg)
        ok3 = run_position_step(servo, 0.00, "Return to Center (0.0 deg)")
        time.sleep(0.5)
        
        # 6. Step 4: Small Step = +0.15 rad (+8.6 deg)
        ok4 = run_position_step(servo, 0.15, "Small step (+8.6 deg)")
        time.sleep(0.5)
        
        # 7. Step 5: Small Step = -0.15 rad (-8.6 deg)
        ok5 = run_position_step(servo, -0.15, "Small step (-8.6 deg)")
        time.sleep(0.5)
        
        # 8. Step 6: Return to Center = 0.00 rad
        ok6 = run_position_step(servo, 0.00, "Return to Center (0.0 deg)")
        time.sleep(0.5)
        
        # 9. Disable Motor
        print("\n[Step 7] Disabling Motor (command: 'e 0')...")
        servo.send_cmd("e 0")
        time.sleep(0.3)
        final_lines = servo.read_telemetry_lines()
        for l in final_lines[-3:]:
            print("  ", l)
            
        print("\n=================================================================")
        if all([ok1, ok2, ok3, ok4, ok5, ok6]):
            print("   ALL TESTS PASSED: CLOSED-LOOP POSITION CONTROL VERIFIED!")
        else:
            print("   TEST COMPLETED WITH WARNINGS.")
        print("=================================================================")

    finally:
        servo.close()


if __name__ == "__main__":
    main()
