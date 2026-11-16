import time
import struct

def send_cmd(f, cmd_bytes):
    payload = bytes([0x00]) + bytes(cmd_bytes)
    payload = payload + bytes(65 - len(payload))
    f.write(payload[:65])
    return f.read(64)

def dap_transfer(f, transfers):
    payload = bytearray([0x05, 0x00, len(transfers)])
    for r_b, d in transfers:
        payload.append(r_b)
        if d is not None:
            payload.extend(d.to_bytes(4, "little"))
    resp = send_cmd(f, bytes(payload))
    assert resp[0] == 0x05, f"DAP_Transfer header error: {resp.hex()}"
    return resp

def init_dap(f):
    send_cmd(f, bytes([0x11]) + (1000 * 1000).to_bytes(4, "little"))
    # Hold nRST low
    send_cmd(f, bytes([0x10, 0x00, 0x80]) + (10000).to_bytes(4, "little"))
    send_cmd(f, bytes([0x02, 0x01]))

def release_dap(f):
    send_cmd(f, bytes([0x10, 0x80, 0x80, 0x00, 0x00, 0x00, 0x00]))
    send_cmd(f, bytes([0x03]))

def mem_write32(f, addr, val):
    dap_transfer(f, [(0x08, 0x01000000)]) # Select AP 1
    dap_transfer(f, [(0x01, 0xE3000012)]) # CSW 32-bit privileged debug
    dap_transfer(f, [(0x05, addr)])       # TAR
    dap_transfer(f, [(0x0D, val)])        # DRW

def mem_read32(f, addr):
    dap_transfer(f, [(0x08, 0x01000000)]) # Select AP 1
    dap_transfer(f, [(0x01, 0xE3000012)]) # CSW 32-bit privileged debug
    dap_transfer(f, [(0x05, addr)])       # TAR
    dap_transfer(f, [(0x0F, None)])       # Dummy read
    resp = dap_transfer(f, [(0x0F, None)])
    return int.from_bytes(resp[3:7], "little")

def main():
    with open("/dev/hidraw5", "r+b", buffering=0) as f:
        try:
            init_dap(f)
            resp = dap_transfer(f, [(0x02, None)])
            dpidr = int.from_bytes(resp[3:7], "little")
            print(f"DPIDR: 0x{dpidr:08X}")

            # Clear aborts and power up debug + system
            dap_transfer(f, [(0x00, 0x1E), (0x04, 0x50000000)])
            resp = dap_transfer(f, [(0x06, None)])
            print(f"DP CTRL/STAT: 0x{int.from_bytes(resp[3:7], 'little'):08X}")

            # Now release nRST so peripherals and bus matrix power up
            send_cmd(f, bytes([0x10, 0x80, 0x80, 0x00, 0x00, 0x00, 0x00]))
            time.sleep(0.01)

            # Halt the core: write DHCSR = 0xA05F0003
            mem_write32(f, 0xE000EDF0, 0xA05F0003)
            dhcsr = mem_read32(f, 0xE000EDF0)
            print(f"DHCSR: 0x{dhcsr:08X} (Halted: {bool(dhcsr & (1 << 17))})")

            test_vals = [0xDEADBEEF, 0x12345678, 0xCAFEBABE, 0xFEEDFACE]
            for i, val in enumerate(test_vals):
                mem_write32(f, 0x20000000 + i * 4, val)

            read_vals = []
            for i in range(len(test_vals)):
                read_vals.append(mem_read32(f, 0x20000000 + i * 4))

            print("Wrote to RAM:", [f"0x{w:08X}" for w in test_vals])
            print("Read from RAM:", [f"0x{w:08X}" for w in read_vals])
            assert test_vals == read_vals, "RAM mismatch!"
            print("RAM READ/WRITE SUCCEEDED!")

            # Also check FLASH registers:
            flash_cr = mem_read32(f, 0x40022028)
            flash_sr = mem_read32(f, 0x40022020)
            print(f"FLASH_CR: 0x{flash_cr:08X}, FLASH_SR: 0x{flash_sr:08X}")
        finally:
            release_dap(f)

if __name__ == "__main__":
    main()
