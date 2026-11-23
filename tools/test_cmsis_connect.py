import time

PROBE_PATH = "/dev/hidraw5"

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
    # Pipelined AP read: read RDBUFF
    resp = transfer([(0x0E, None)])
    return int.from_bytes(resp[3:7], "little")

def write_mem(addr, val):
    transfer([(0x05, addr), (0x0D, val)])

print("[*] Testing Connect-Under-Reset via CMSIS-DAP...")
# Clock = 1000 kHz
send_cmd(bytes([0x11, 0xE8, 0x03, 0x00, 0x00]))

# 1. Assert nRST low
send_cmd(bytes([0x10, 0x00, 0x80, 0x00, 0x00, 0x00, 0x00]))
time.sleep(0.005)

# 2. Connect SWD while reset asserted
send_cmd(bytes([0x02, 0x01]))

# Read DPIDR
resp = transfer([(0x02, None)])
dpidr = int.from_bytes(resp[3:7], "little")
print(f"[*] Under-Reset DPIDR: 0x{dpidr:08X}")

# Power up debug
transfer([(0x00, 0x1E), (0x04, 0x50000000)])
resp = transfer([(0x06, None)])
ctrl = int.from_bytes(resp[3:7], "little")
print(f"[*] Under-Reset CTRL/STAT: 0x{ctrl:08X}")

# 3. Release nRST
send_cmd(bytes([0x10, 0x80, 0x80, 0x00, 0x00, 0x00, 0x00]))

# 4. Clear abort and power up debug again
transfer([(0x00, 0x1E), (0x04, 0x50000000)])

# 5. Select AP 1 (AHB-AP), Bank 0, CSW = 0x23000012 (Secure privileged 32-bit)
transfer([(0x08, 0x01000000), (0x01, 0x23000012)])

# 6. Halt CPU via DHCSR immediately
transfer([(0x05, 0xE000EDFC), (0x0D, 0x01000001)]) # DEMCR TRCENA | VC_CORERESET
transfer([(0x05, 0xE000EDF0), (0x0D, 0xA05F0003)]) # DHCSR DBGKEY | DEBUGEN | HALT

# Un-gate DBGMCU in STOP/STANDBY
try:
    write_mem(0xE0044004, 0x00000006)
except Exception as e:
    print(f"[*] DBGMCU write error: {e}")

dhcsr = read_mem(0xE000EDF0)
print(f"[*] DHCSR: 0x{dhcsr:08X} (Core Halted: {bool(dhcsr & 0x00020000)})")

f.close()
