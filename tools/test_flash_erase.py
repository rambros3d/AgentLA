import time

PROBE_PATH = "/dev/hidraw5"

def test_erase():
    f = open(PROBE_PATH, "r+b", buffering=0)

    def send_cmd(cmd_bytes):
        payload = bytes([0x00]) + cmd_bytes
        payload = payload + bytes(65 - len(payload))
        f.write(payload[:65])
        return f.read(64)

    def transfer(transfers):
        payload = bytearray([0x05, 0x00, len(transfers)])
        for r_b, d in transfers:
            payload.append(r_b)
            if d is not None:
                payload.extend(d.to_bytes(4, "little"))
        resp = send_cmd(bytes(payload))
        if resp[0] != 0x05 or resp[2] != 1:
            raise RuntimeError(f"DAP_Transfer failed: count={resp[1]} ack=0x{resp[2]:02X}")
        return resp

    def read_mem(addr):
        transfer([(0x05, addr)])
        resp = transfer([(0x0F, None)])
        resp = transfer([(0x0E, None)])
        return int.from_bytes(resp[3:7], "little")

    def write_mem(addr, val):
        transfer([(0x05, addr), (0x0D, val)])

    print("[*] Connecting under reset...")
    send_cmd(bytes([0x11, 0xE8, 0x03, 0x00, 0x00])) # 1MHz
    send_cmd(bytes([0x10, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00])) # Assert nRST
    time.sleep(0.005)
    send_cmd(bytes([0x02, 0x01])) # Connect SWD
    transfer([(0x02, None)]) # Read DPIDR
    transfer([(0x00, 0x1E), (0x04, 0x50000000)]) # Power up debug
    send_cmd(bytes([0x10, 0x80, 0x80, 0x00, 0x00, 0x00, 0x00])) # Release nRST
    transfer([(0x00, 0x1E), (0x04, 0x50000000)])
    transfer([(0x08, 0x01000000), (0x01, 0x23000012)]) # AP 1, Bank 0, Secure CSW

    transfer([(0x05, 0xE000EDFC), (0x0D, 0x01000001)]) # DEMCR VC_CORERESET
    transfer([(0x05, 0xE000EDF0), (0x0D, 0xA05F0003)]) # DHCSR HALT

    dhcsr = read_mem(0xE000EDF0)
    print(f"[*] Halted: DHCSR=0x{dhcsr:08X}")

    base = 0x50022000
    cr = read_mem(base + 0x28)
    if (cr & 1) != 0:
        print("[*] Unlocking flash...")
        write_mem(base + 0x04, 0x45670123)
        write_mem(base + 0x04, 0xCDEF89AB)
        cr = read_mem(base + 0x28)
        assert (cr & 1) == 0, f"Flash still locked: 0x{cr:08X}"

    # Clear CCR error flags
    write_mem(base + 0x30, 0x00FE0000 | 0x00010000)

    # Erase sector 0: SER=1 (bit 2), START=1 (bit 5), SNB=0 (bits 6..12)
    # (1 << 2) | (1 << 5) = 0x24
    print("[*] Starting Sector 0 erase...")
    t0 = time.time()
    write_mem(base + 0x28, 0x00000024)

    # Wait for BSY (bit 0) == 0
    while True:
        sr = read_mem(base + 0x20)
        if (sr & 1) == 0:
            break
        if time.time() - t0 > 5.0:
            raise TimeoutError("Erase timed out")

    duration = time.time() - t0
    # Clear SER in CR
    write_mem(base + 0x28, 0x00000000)
    print(f"[*] Sector 0 erased in {duration:.3f}s, SR=0x{sr:08X}")

    # Check first words of sector 0 (0x08000000)
    w0 = read_mem(0x08000000)
    w1 = read_mem(0x08000004)
    print(f"[*] Readback 0x08000000: 0x{w0:08X} 0x{w1:08X} (Erased = 0xFFFFFFFF)")
    assert w0 == 0xFFFFFFFF and w1 == 0xFFFFFFFF, "Sector 0 was not erased!"
    print("[*] SECTOR 0 ERASE VERIFIED!")

    f.close()

if __name__ == "__main__":
    test_erase()
