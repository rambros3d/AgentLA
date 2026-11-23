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
I3C_OP_IBI_CONFIG    = 0x10

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

    def _transceive(self, opcode: int, addr: int = 0, payload: bytes = b"", timeout_ms: int = 1000) -> Tuple[int, int, bytes]:
        """Send a binary I3C frame and await response."""
        if self.ep_out is None or self.ep_in is None:
            raise ConnectionError("AgentLaI3c is not connected.")

        seq = self._next_seq()
        flags = 0x02 | 0x08 # EMIT_STOP | PUSH_PULL
        header = struct.pack("<BBBBHBB", I3C_BULK_MAGIC, opcode, flags, seq, len(payload), addr, 0)
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
        header_payload = b"\x00" * length
        st, _, data = self._transceive(I3C_OP_PRIVATE_READ, addr=target_addr, payload=header_payload)
        if st != I3C_STATUS_OK:
            raise I3cException(st, f"Private read from address {target_addr:#x} failed")
        return data

    def ccc_broadcast(self, ccc_code: int, data: bytes = b""):
        """Broadcast a Common Command Code (CCC)."""
        st, _, _ = self._transceive(I3C_OP_CCC_BROADCAST, addr=ccc_code, payload=data)
        if st != I3C_STATUS_OK:
            raise I3cException(st, f"Broadcast CCC {ccc_code:#x} failed")
