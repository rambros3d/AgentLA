"""
AgentLA Native USB-to-I3C Adapter Client.

Provides direct binary framing communication over USB Bulk endpoints
to AgentLA's hardware MIPI I3C SDR master running on RP2350 PIO2.
"""

import struct
import time
from typing import List, Optional, Tuple

try:
    import usb.core
    import usb.util
    HAS_PYUSB = True
except ImportError:
    HAS_PYUSB = False

I3C_BULK_MAGIC = 0x3C
I3C_HEADER_SIZE = 8

# Command Opcodes
I3C_OP_SET_BUS_SPEED = 0x01
I3C_OP_TARGET_RESET  = 0x02
I3C_OP_RSTDAA        = 0x03
I3C_OP_ENTDAA        = 0x04
I3C_OP_CCC_BROADCAST = 0x05
I3C_OP_CCC_DIRECT    = 0x06
I3C_OP_PRIVATE_WRITE = 0x07
I3C_OP_PRIVATE_READ  = 0x08
I3C_OP_PRIVATE_WRITE_READ = 0x09
I3C_OP_SET_PINS      = 0x0A
I3C_OP_I2C_SCAN      = 0x0B
I3C_OP_IBI_CONFIG    = 0x10

# Flags
I3C_FLAG_REPEATED_START = 1 << 0
I3C_FLAG_EMIT_STOP      = 1 << 1
I3C_FLAG_OPEN_DRAIN     = 1 << 2

# Event Opcodes
I3C_EVT_IBI_NOTIFICATION = 0x80

# Status Codes
I3C_STATUS_OK          = 0x00
I3C_STATUS_IBI         = 0x01
I3C_STATUS_NAK_ARBHDR  = 0x02
I3C_STATUS_NAK_ADDR    = 0x03
I3C_STATUS_PARITY_ERR  = 0x04
I3C_STATUS_TIMEOUT     = 0x05
I3C_STATUS_INVALID_PARAM = 0x06

STATUS_NAMES = {
    I3C_STATUS_OK: "OK",
    I3C_STATUS_IBI: "IBI_PENDING",
    I3C_STATUS_NAK_ARBHDR: "ERR_NAK_ARBITRATION_HEADER",
    I3C_STATUS_NAK_ADDR: "ERR_NAK_ADDRESS",
    I3C_STATUS_PARITY_ERR: "ERR_PARITY",
    I3C_STATUS_TIMEOUT: "ERR_TIMEOUT",
    I3C_STATUS_INVALID_PARAM: "ERR_INVALID_PARAM",
}

class I3cException(Exception):
    """Exception raised for I3C communication errors."""
    def __init__(self, status: int, message: str = ""):
        self.status = status
        name = STATUS_NAMES.get(status, f"UNKNOWN_STATUS_{status}")
        full_msg = f"I3C Error: {name}" + (f" ({message})" if message else "")
        super().__init__(full_msg)

class AgentLaI3c:
    """AgentLA High-Speed Binary I3C Adapter Client."""

    def __init__(self, vid: int = 0x1209, pid: int = 0x414D):
        self.vid = vid
        self.pid = pid
        self.dev = None
        self.ep_out = None
        self.ep_in = None
        self.scl_pin = 10
        self.sda_pin = 11
        self._seq = 0

    def connect(self) -> bool:
        """Find AgentLA USB device and claim the I3C bulk interface."""
        if not HAS_PYUSB:
            raise ImportError("pyusb is required. Install via 'pip install pyusb'.")

        self.dev = usb.core.find(idVendor=self.vid, idProduct=self.pid)
        if self.dev is None:
            return False

        # Search for I3C interface: Class 0xFF, Subclass 0x3C, Protocol 0x01
        cfg = self.dev.get_active_configuration()
        i3c_interface = None

        for intf in cfg:
            if intf.bInterfaceClass == 0xFF and intf.bInterfaceSubClass == 0x3C:
                i3c_interface = intf
                break

        # Fallback: find interface with string matching I3C or use interface index 8
        if i3c_interface is None:
            for intf in cfg:
                try:
                    s = usb.util.get_string(self.dev, intf.iInterface)
                    if s and "I3C" in s:
                        i3c_interface = intf
                        break
                except Exception:
                    pass

        if i3c_interface is None:
            # Fallback to last vendor interface
            vendor_intfs = [i for i in cfg if i.bInterfaceClass == 0xFF]
            if vendor_intfs:
                i3c_interface = vendor_intfs[-1]

        if i3c_interface is None:
            return False

        intf_num = i3c_interface.bInterfaceNumber

        # Detach kernel driver if active
        try:
            if self.dev.is_kernel_driver_active(intf_num):
                self.dev.detach_kernel_driver(intf_num)
        except (NotImplementedError, usb.core.USBError):
            pass

        usb.util.claim_interface(self.dev, intf_num)

        for ep in i3c_interface:
            if usb.util.endpoint_direction(ep.bEndpointAddress) == usb.util.ENDPOINT_OUT:
                self.ep_out = ep
            else:
                self.ep_in = ep

        return self.ep_out is not None and self.ep_in is not None

    def close(self):
        """Release claimed USB interface."""
        if self.dev:
            usb.util.dispose_resources(self.dev)
            self.dev = None

    def _next_seq(self) -> int:
        self._seq = (self._seq + 1) & 0xFF
        return self._seq

    def _transceive(self, opcode: int, addr: int = 0, payload: bytes = b"", flags: Optional[int] = None, timeout_ms: int = 1000, req_payload_len: Optional[int] = None, status: int = 0) -> Tuple[int, int, bytes]:
        """Send a binary I3C frame and await response."""
        if self.ep_out is None or self.ep_in is None:
            raise ConnectionError("AgentLaI3c is not connected.")

        seq = self._next_seq()
        if flags is None:
            flags = 0x02 | 0x08 # EMIT_STOP | PUSH_PULL
        plen = len(payload) if req_payload_len is None else req_payload_len
        header = struct.pack("<BBBBHBB", I3C_BULK_MAGIC, opcode, flags, seq, plen, addr, status & 0xFF)
        packet = header + payload

        self.ep_out.write(packet, timeout=timeout_ms)

        resp = self.ep_in.read(256, timeout=timeout_ms)
        if len(resp) < I3C_HEADER_SIZE:
            raise I3cException(I3C_STATUS_TIMEOUT, "Short response header")

        magic, resp_op, resp_flags, resp_seq, resp_len, resp_addr, status = struct.unpack("<BBBBHBB", resp[:I3C_HEADER_SIZE])
        if magic != I3C_BULK_MAGIC:
            raise I3cException(I3C_STATUS_INVALID_PARAM, f"Bad response magic: {magic:#x}")

        resp_payload = bytes(resp[I3C_HEADER_SIZE:I3C_HEADER_SIZE + resp_len])
        return status, resp_addr, resp_payload

    def set_bus_speed(self, freq_khz: int = 1000):
        """Set I3C SCL clock rate in kHz (e.g. 1000 for 1 MHz, 12500 for 12.5 MHz)."""
        payload = struct.pack("<I", freq_khz)
        st, _, _ = self._transceive(I3C_OP_SET_BUS_SPEED, payload=payload)
        if st != I3C_STATUS_OK:
            raise I3cException(st, f"Failed to set clock speed to {freq_khz} kHz")

    def set_pins(self, scl: int, sda: int):
        """Set unified I2C/I3C SCL and SDA pin assignments on AgentLA."""
        payload = bytes([scl & 0xFF, sda & 0xFF])
        st, _, _ = self._transceive(I3C_OP_SET_PINS, payload=payload)
        if st != I3C_STATUS_OK:
            raise I3cException(st, f"Failed to set pins to SCL={scl}, SDA={sda}")
        self.scl_pin = scl
        self.sda_pin = sda

    def is_inverted(self) -> bool:
        """Return True if SCL and SDA are inverted (SCL=11, SDA=10)."""
        return self.scl_pin == 11 and self.sda_pin == 10

    def set_inverted(self, inverted: bool):
        """Set unified pin inversion across both I2C and I3C."""
        if inverted:
            self.set_pins(11, 10)
        else:
            self.set_pins(10, 11)
        return self.scl_pin, self.sda_pin

    def swap_pins(self):
        """Invert active SCL and SDA pin assignments."""
        new_scl = self.sda_pin
        new_sda = self.scl_pin
        self.set_pins(new_scl, new_sda)
        return self.scl_pin, self.sda_pin

    def auto_detect_target(self, target_addrs: Optional[list] = None) -> Tuple[bool, int, int]:
        """Autonomously probe for target on bus. If not found, invert pins and probe again.
        Returns (found: bool, scl_pin: int, sda_pin: int).
        """
        targets = target_addrs if target_addrs is not None else [0x40]
        for orientation in [False, True]:
            self.set_inverted(orientation)
            time.sleep(0.02)
            try:
                found = self.i2c_scan()
                for t in targets:
                    if t in found:
                        return True, self.scl_pin, self.sda_pin
            except Exception:
                pass
            # Try ENTDAA
            try:
                for t in targets:
                    dev = self.entdaa(t)
                    if dev is not None:
                        return True, self.scl_pin, self.sda_pin
            except Exception:
                pass
        # Reset to non-inverted if nothing detected
        self.set_inverted(False)
        return False, self.scl_pin, self.sda_pin

    def target_reset(self):
        """Emit I3C target reset sequence (9 SCL pulses with SDA low)."""
        st, _, _ = self._transceive(I3C_OP_TARGET_RESET)
        if st != I3C_STATUS_OK:
            raise I3cException(st, "Target reset failed")

    def rstdaa(self):
        """Broadcast Reset Dynamic Address Assignment (RSTDAA) CCC."""
        st, _, _ = self._transceive(I3C_OP_RSTDAA)
        if st != I3C_STATUS_OK:
            raise I3cException(st, "RSTDAA failed")

    def entdaa(self, proposed_addr: int = 0x08) -> Optional[dict]:
        """
        Execute Enter Dynamic Address Assignment (ENTDAA).
        Returns device info dict if a target responded, or None if bus NAKed (no targets).
        """
        st, assigned, payload = self._transceive(I3C_OP_ENTDAA, addr=proposed_addr)
        if st == I3C_STATUS_NAK_ARBHDR:
            return None # 0x7E NAKed: no more targets
        if st != I3C_STATUS_OK:
            raise I3cException(st, "ENTDAA failed")

        if len(payload) >= 8:
            pid = payload[:6]
            bcr = payload[6]
            dcr = payload[7]
            return {
                "dynamic_address": assigned,
                "provisional_id": pid.hex(),
                "bcr": bcr,
                "dcr": dcr,
            }
        return {"dynamic_address": assigned}

    def private_write(self, target_addr: int, data: bytes):
        """Perform a push-pull private SDR data write to target dynamic address."""
        st, _, _ = self._transceive(I3C_OP_PRIVATE_WRITE, addr=target_addr, payload=data)
        if st != I3C_STATUS_OK:
            raise I3cException(st, f"Private write to address {target_addr:#x} failed")

    def private_read(self, target_addr: int, length: int) -> bytes:
        """Perform a push-pull private SDR data read from target dynamic address."""
        st, _, data = self._transceive(I3C_OP_PRIVATE_READ, addr=target_addr, payload=b"", req_payload_len=length)
        if st != I3C_STATUS_OK:
            raise I3cException(st, f"Private read from address {target_addr:#x} failed")
        return data

    def private_write_read(self, target_addr: int, data: bytes, read_len: int = 64) -> bytes:
        """Perform a write followed by repeated read in a single atomic transaction."""
        rx_len = max(1, min(read_len, 128))
        st, _, rx = self._transceive(
            I3C_OP_PRIVATE_WRITE_READ, addr=target_addr, payload=data, status=rx_len
        )
        if st != I3C_STATUS_OK:
            raise I3cException(st, f"Private write-read to address {target_addr:#x} failed")
        return rx[:read_len]

    def ccc_broadcast(self, ccc_code: int, data: bytes = b""):
        """Broadcast a Common Command Code (CCC)."""
        st, _, _ = self._transceive(I3C_OP_CCC_BROADCAST, addr=ccc_code, payload=data)
        if st != I3C_STATUS_OK:
            raise I3cException(st, f"Broadcast CCC {ccc_code:#x} failed")

    def i2c_scan(self) -> list:
        """Scan I2C bus across addresses 0x08..0x77 and return list of responding addresses."""
        st, _, payload = self._transceive(I3C_OP_I2C_SCAN)
        if st != I3C_STATUS_OK:
            raise I3cException(st, "I2C scan failed")
        return list(payload)

    def i2c_write(self, target_addr: int, data: bytes):
        """Perform an open-drain standard I2C write to target address."""
        st, _, _ = self._transceive(I3C_OP_PRIVATE_WRITE, addr=target_addr, flags=I3C_FLAG_OPEN_DRAIN, payload=data)
        if st != I3C_STATUS_OK:
            raise I3cException(st, f"I2C write to address {target_addr:#x} failed")

    def i2c_read(self, target_addr: int, length: int) -> bytes:
        """Perform an open-drain standard I2C read from target address."""
        st, _, data = self._transceive(
            I3C_OP_PRIVATE_READ, addr=target_addr, flags=I3C_FLAG_OPEN_DRAIN, payload=b"", req_payload_len=length
        )
        if st != I3C_STATUS_OK:
            raise I3cException(st, f"I2C read from address {target_addr:#x} failed")
        return data

    def i2c_transfer(self, target_addr: int, tx_data: bytes, read_len: int = 64) -> bytes:
        """Perform an open-drain I2C write followed by read."""
        rx_len = max(1, min(read_len, 128))
        st, _, rx = self._transceive(
            I3C_OP_PRIVATE_WRITE_READ,
            addr=target_addr,
            flags=I3C_FLAG_OPEN_DRAIN,
            payload=tx_data,
            status=rx_len,
        )
        if st != I3C_STATUS_OK:
            raise I3cException(st, f"I2C transfer to address {target_addr:#x} failed")
        return rx[:read_len]
