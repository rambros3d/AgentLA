#!/usr/bin/env python3
"""
Flash STM32H503 over AgentLA CMSIS-DAP interface.
Unlocks Flash, erases requested sectors, programs quad-words (128-bit), and resets target.
"""

import sys
import time
from pathlib import Path

# Paths
BOOTLOADER_BIN = Path("/home/sun/Filelink/Firmwares/bareBoot/artifacts/bareboot-stm32h503-i3c-i2c.bin")
PROBE_PATH = "/dev/hidraw5"


class CmsisDapSwd:
    def __init__(self, dev_path=PROBE_PATH):
        self.f = open(dev_path, "r+b", buffering=0)

    def close(self):
        self.f.close()

    def send_cmd(self, cmd_bytes):
        payload = bytes([0x00]) + cmd_bytes
        payload = payload + bytes(65 - len(payload))
        self.f.write(payload[:65])
        return self.f.read(64)

    def transfer(self, transfers, retries=5):
        payload = bytearray([0x05, 0x00, len(transfers)])
        for r_b, d in transfers:
            payload.append(r_b)
            if d is not None:
                payload.extend(d.to_bytes(4, "little"))
        for attempt in range(retries):
            resp = self.send_cmd(bytes(payload))
            if resp[0] == 0x05 and resp[2] == 1:
                return resp
            # Clear sticky errors on DP ABORT
            self.send_cmd(bytes([0x05, 0x00, 0x01, 0x00, 0x1E, 0x00, 0x00, 0x00]))
            time.sleep(0.002)
        raise RuntimeError(f"DAP_Transfer failed: count={resp[1]} ack=0x{resp[2]:02X}")

    def init_target(self):
        # Clock = 100 kHz
        self.send_cmd(bytes([0x11, 0xA0, 0x86, 0x01, 0x00]))
        self.send_cmd(bytes([0x04, 0x08, 0x88, 0x13, 0x00, 0x00])) # idle=8, wait=5000
        self.send_cmd(bytes([0x02, 0x01]))

        # Pulse nRST low 20ms, then release high
        self.send_cmd(bytes([0x10, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00]))
        time.sleep(0.02)
        self.send_cmd(bytes([0x10, 0x80, 0x80, 0x00, 0x00, 0x00, 0x00]))
        time.sleep(0.01)

        # Dormant-to-SWD wake sequence
        seq = [0xFF, 0xFF]
        seq += list(0x19BC0EA2.to_bytes(4, "little"))
        seq += list(0xE3DDAFE9.to_bytes(4, "little"))
        seq += list(0x86852D95.to_bytes(4, "little"))
        seq += list(0x6209F392.to_bytes(4, "little"))
        seq += [0xA0, 0x01]
        seq += [0xFF] * 8
        seq += [0x00]
        self.send_cmd(bytes([0x12, 228]) + bytes(seq))

        # Read DPIDR (mandatory first transfer)
        resp = self.transfer([(0x02, None)])
        dpidr = int.from_bytes(resp[3:7], "little")
        print(f"[*] Target DPIDR: 0x{dpidr:08X}")
        assert dpidr in (0x6BA02477, 0xDBE02477), f"Unexpected DPIDR: 0x{dpidr:08X}"

        # Clear aborts and request debug power
        self.transfer([(0x00, 0x1E), (0x04, 0x50000000)])
        resp = self.transfer([(0x06, None)])
        ctrl = int.from_bytes(resp[3:7], "little")
        print(f"[*] DP CTRL/STAT: 0x{ctrl:08X}")
        assert (ctrl & 0xA0000000) == 0xA0000000, "Debug power-up not acknowledged"

        # Select AP 1 (AHB-AP), Bank 0
        self.transfer([(0x08, 0x01000000)])
        # Set CSW = 0x22000012 (32-bit privileged non-secure access, auto-inc)
        self.transfer([(0x01, 0x22000012)])

        # Halt CPU via DHCSR
        self.transfer([(0x05, 0xE000EDF0), (0x0D, 0xA05F0003)])
        resp = self.transfer([(0x05, 0xE000EDF0), (0x0F, None)])
        dhcsr = int.from_bytes(resp[3:7], "little")
        print(f"[*] DHCSR: 0x{dhcsr:08X} (Core Halted: {bool(dhcsr & 0x00020000)})")

    def read_mem32(self, addr):
        resp = self.transfer([(0x05, addr), (0x0F, None)])
        return int.from_bytes(resp[3:7], "little")

    def write_mem32(self, addr, val):
        self.transfer([(0x05, addr), (0x0D, val)])

    def unlock_flash(self):
        cr = self.read_mem32(0x40022028)
        if (cr & 1) != 0:
            print("[*] Unlocking Non-Secure Flash...")
            self.write_mem32(0x40022004, 0x45670123)
            self.write_mem32(0x40022004, 0xCDEF89AB)
            cr_after = self.read_mem32(0x40022028)
            assert (cr_after & 1) == 0, f"Flash unlock failed: CR=0x{cr_after:08X}"
            print("[*] Flash unlocked successfully!")
        else:
            print("[*] Flash already unlocked.")

    def erase_sector(self, sector_num):
        print(f"[*] Erasing Sector {sector_num}...")
        self.unlock_flash()
        self.write_mem32(0x40022030, 0x001F0000) # Clear CCR flags
        
        # SER=1, START=1, SNB=sector_num
        cr_val = (1 << 2) | (1 << 5) | ((sector_num & 0x7F) << 6)
        self.write_mem32(0x40022028, cr_val)

        t0 = time.time()
        while True:
            sr = self.read_mem32(0x40022020)
            if (sr & 0x0B) == 0:
                break
            if time.time() - t0 > 3.0:
                raise TimeoutError("Sector erase timed out")
        self.write_mem32(0x40022028, 0) # Clear SER
        if (sr & 0x00BE0000) != 0:
            raise RuntimeError(f"Sector erase error: SR=0x{sr:08X}")
        print(f"[*] Sector {sector_num} erased in {time.time() - t0:.3f}s")

    def program_flash(self, base_addr, data):
        self.unlock_flash()
        self.write_mem32(0x40022030, 0x001F0000) # Clear CCR

        # Set PG = 1 in FLASH_NSCR
        self.write_mem32(0x40022028, 1 << 1)

        total_bytes = len(data)
        # Pad to 16-byte boundary (128-bit quad-words)
        padded_data = bytearray(data)
        if len(padded_data) % 16 != 0:
            padded_data.extend(b"\xFF" * (16 - (len(padded_data) % 16)))

        t0 = time.time()
        offset = 0
        while offset < len(padded_data):
            chunk_addr = base_addr + offset
            w0 = int.from_bytes(padded_data[offset:offset+4], "little")
            w1 = int.from_bytes(padded_data[offset+4:offset+8], "little")
            w2 = int.from_bytes(padded_data[offset+8:offset+12], "little")
            w3 = int.from_bytes(padded_data[offset+12:offset+16], "little")

            # Write 4 words (128 bits)
            self.write_mem32(chunk_addr + 0, w0)
            self.write_mem32(chunk_addr + 4, w1)
            self.write_mem32(chunk_addr + 8, w2)
            self.write_mem32(chunk_addr + 12, w3)

            # Wait BSY == 0
            for _ in range(1000):
                sr = self.read_mem32(0x40022020)
                if (sr & 0x0B) == 0:
                    break
            if (sr & 0x00BE0000) != 0:
                self.write_mem32(0x40022030, 0x001F0000)
                raise RuntimeError(f"Flash write error at 0x{chunk_addr:08X}: SR=0x{sr:08X}")

            offset += 16
            if offset % 256 == 0 or offset >= len(padded_data):
                pct = min(100, (offset * 100) // len(padded_data))
                print(f"  > Programming: {offset}/{len(padded_data)} bytes ({pct}%)", end="\r")

        self.write_mem32(0x40022028, 0) # Clear PG
        duration = time.time() - t0
        print(f"\n[*] Programmed {total_bytes} bytes in {duration:.3f}s ({(total_bytes/duration)/1024:.2f} KB/s)")

    def verify_flash(self, base_addr, data):
        print(f"[*] Verifying {len(data)} bytes at 0x{base_addr:08X}...")
        offset = 0
        while offset < len(data):
            read_len = min(4, len(data) - offset)
            val = self.read_mem32(base_addr + offset)
            expected = int.from_bytes(data[offset:offset+read_len].ljust(4, b"\xFF"), "little")
            if (val & ((1 << (read_len * 8)) - 1)) != (expected & ((1 << (read_len * 8)) - 1)):
                raise ValueError(f"Verify mismatch at 0x{base_addr + offset:08X}: read 0x{val:08X}, expected 0x{expected:08X}")
            offset += 4
        print("[*] Verification PASSED (100% bit-exact match)!")

    def reset_and_run(self):
        print("[*] Resetting target into application...")
        # Release nRST
        self.send_cmd(bytes([0x10, 0x80, 0x80, 0x00, 0x00, 0x00, 0x00]))
        # System Reset via AIRCR
        try:
            self.transfer([(0x05, 0xE000ED0C), (0x0D, 0x05FA0004)]) # AIRCR SYSRESETREQ
        except Exception:
            pass
        self.send_cmd(bytes([0x03])) # Disconnect


def main():
    bin_path = Path(sys.argv[1]) if len(sys.argv) > 1 else BOOTLOADER_BIN
    base_addr = int(sys.argv[2], 0) if len(sys.argv) > 2 else 0x08000000

    print("=" * 60)
    print("FLASHING STM32H503 VIA AGENTLA CMSIS-DAP")
    print(f"Binary: {bin_path} ({bin_path.stat().st_size} bytes)")
    print(f"Base Address: 0x{base_addr:08X}")
    print("=" * 60)

    data = bin_path.read_bytes()
    dap = CmsisDapSwd()
    try:
        dap.init_target()
        
        # Erase Sector 0 (0x08000000..0x08002000, 8KB)
        dap.erase_sector(0)

        # Program Flash
        dap.program_flash(base_addr, data)

        # Verify Flash
        dap.verify_flash(base_addr, data)

        # Reset & Run
        dap.reset_and_run()
        print("=" * 60)
        print("BOOTLOADER FLASH COMPLETED SUCCESSFULLY!")
        print("=" * 60)
    finally:
        dap.close()


if __name__ == "__main__":
    main()
