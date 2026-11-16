import time

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
    return send_cmd(f, bytes(payload))

def main():
    with open("/dev/hidraw5", "r+b", buffering=0) as f:
        send_cmd(f, bytes([0x11]) + (1000 * 1000).to_bytes(4, "little"))
        
        # 1. Assert nRESET low
        send_cmd(f, bytes([0x10, 0x00, 0x80]) + (10000).to_bytes(4, "little"))
        send_cmd(f, bytes([0x02, 0x01])) # Connect SWD
        
        # 2. Read DPIDR
        r = dap_transfer(f, [(0x02, None)])
        dpidr = int.from_bytes(r[3:7], "little")
        print(f"DPIDR under reset: 0x{dpidr:08X} (ack={r[2]})")
        
        # 3. Power up debug
        dap_transfer(f, [(0x00, 0x0000001E), (0x04, 0x50000000)])
        
        # 4. Check AP 0 (APB-AP) vs AP 1 (AHB-AP) while in reset
        for ap in (0, 1):
            dap_transfer(f, [(0x08, ap << 24)])
            r = dap_transfer(f, [(0x03, None)]) # Read CSW
            csw = int.from_bytes(r[3:7], "little")
            dap_transfer(f, [(0x08, (ap << 24) | 0xF0)])
            r_id = dap_transfer(f, [(0x0F, None)])
            ap_idr = int.from_bytes(r_id[3:7], "little")
            print(f"AP {ap}: IDR=0x{ap_idr:08X} CSW=0x{csw:08X}")

        # 5. On AP 0, enable Core Debug (DHCSR) and Reset Vector Catch (DEMCR)
        # In STM32H5, AP 0 (APB-AP) gives access to core debug
        dap_transfer(f, [(0x08, 0x00000000)])
        dap_transfer(f, [(0x01, 0x80000012)])
        dap_transfer(f, [(0x05, 0xE000EDFC)]) # DEMCR
        r = dap_transfer(f, [(0x0D, 0x01000001)]) # VC_CORERESET | TRCENA
        print(f"DEMCR write ack: {r[2]}")
        dap_transfer(f, [(0x05, 0xE000EDF0)]) # DHCSR
        r = dap_transfer(f, [(0x0D, 0xA05F0003)]) # C_DEBUGEN | C_HALT
        print(f"DHCSR write ack: {r[2]}")

        # 6. Release nRESET high
        send_cmd(f, bytes([0x10, 0x80, 0x80, 0x00, 0x00, 0x00, 0x00]))
        time.sleep(0.02) # Wait 20ms

        # 7. Check if core is halted!
        dap_transfer(f, [(0x08, 0x00000000)])
        dap_transfer(f, [(0x05, 0xE000EDF0)])
        dap_transfer(f, [(0x0F, None)])
        r = dap_transfer(f, [(0x0E, None)])
        dhcsr = int.from_bytes(r[3:7], "little")
        print(f"DHCSR after release: 0x{dhcsr:08X} (ack={r[2]}, S_HALT={bool(dhcsr & (1 << 17))})")

        # 8. Check AP 1 CSW after release
        dap_transfer(f, [(0x08, 0x01000000)])
        r = dap_transfer(f, [(0x03, None)])
        csw1 = int.from_bytes(r[3:7], "little")
        print(f"AP 1 CSW after release: 0x{csw1:08X} (DeviceEn={bool(csw1 & 0x40)})")

        send_cmd(f, bytes([0x03]))

if __name__ == "__main__":
    main()
