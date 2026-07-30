"""Sigrok integration for AgentLA Python SDK."""

import os
import struct
from typing import Optional

from google.protobuf import descriptor_pb2, descriptor_pool, message_factory

try:
    from google.protobuf import descriptor_pb2, descriptor_pool, message_factory, message
    PROTOBUF_AVAILABLE = True
except ImportError:
    PROTOBUF_AVAILABLE = False


_SESSION_CLASS = None
_DEVICE_CLASS = None
_CHANNEL_CLASS = None
_LOGIC_CLASS = None


def _get_sr_messages():
    global _SESSION_CLASS, _DEVICE_CLASS, _CHANNEL_CLASS, _LOGIC_CLASS
    if _SESSION_CLASS is not None:
        return _SESSION_CLASS, _DEVICE_CLASS, _CHANNEL_CLASS, _LOGIC_CLASS

    if not PROTOBUF_AVAILABLE:
        raise ImportError(
            "protobuf package required for Sigrok export. "
            "Install with: pip install protobuf"
        )

    fd = descriptor_pb2.FileDescriptorProto()
    fd.name = "session.proto"
    fd.syntax = "proto2"
    fd.package = "sigrok"

    msg_session = fd.message_type.add()
    msg_session.name = "Session"
    f = msg_session.field.add()
    f.name = "devices"
    f.number = 1
    f.type = descriptor_pb2.FieldDescriptorProto.TYPE_MESSAGE
    f.label = descriptor_pb2.FieldDescriptorProto.LABEL_REPEATED
    f.type_name = ".sigrok.Device"

    msg_device = fd.message_type.add()
    msg_device.name = "Device"
    for name, num, ftype in [
        ("vendor", 1, descriptor_pb2.FieldDescriptorProto.TYPE_STRING),
        ("model", 2, descriptor_pb2.FieldDescriptorProto.TYPE_STRING),
        ("version", 3, descriptor_pb2.FieldDescriptorProto.TYPE_STRING),
        ("serial", 4, descriptor_pb2.FieldDescriptorProto.TYPE_STRING),
        ("configuration", 5, descriptor_pb2.FieldDescriptorProto.TYPE_STRING),
        ("device_class", 6, descriptor_pb2.FieldDescriptorProto.TYPE_STRING),
        ("sample_rate", 7, descriptor_pb2.FieldDescriptorProto.TYPE_UINT64),
        ("driver", 8, descriptor_pb2.FieldDescriptorProto.TYPE_STRING),
    ]:
        f = msg_device.field.add()
        f.name = name
        f.number = num
        f.type = ftype
        f.label = descriptor_pb2.FieldDescriptorProto.LABEL_OPTIONAL
    f = msg_device.field.add()
    f.name = "channels"
    f.number = 9
    f.type = descriptor_pb2.FieldDescriptorProto.TYPE_MESSAGE
    f.label = descriptor_pb2.FieldDescriptorProto.LABEL_REPEATED
    f.type_name = ".sigrok.Channel"

    msg_channel = fd.message_type.add()
    msg_channel.name = "Channel"
    for name, num in [
        ("number", 1), ("name", 2), ("logic", 3), ("analog", 4),
        ("type", 5), ("mapping", 6),
    ]:
        f = msg_channel.field.add()
        f.name = name
        f.number = num
        if name in ("number", "type", "mapping"):
            f.type = descriptor_pb2.FieldDescriptorProto.TYPE_UINT32
        elif name in ("logic", "analog"):
            f.type = descriptor_pb2.FieldDescriptorProto.TYPE_BOOL
        else:
            f.type = descriptor_pb2.FieldDescriptorProto.TYPE_STRING
        f.label = descriptor_pb2.FieldDescriptorProto.LABEL_OPTIONAL

    msg_logic = fd.message_type.add()
    msg_logic.name = "Logic"
    for name, num in [("start_sample", 1), ("length", 2), ("data", 3)]:
        f = msg_logic.field.add()
        f.name = name
        f.number = num
        if name == "data":
            f.type = descriptor_pb2.FieldDescriptorProto.TYPE_BYTES
        else:
            f.type = descriptor_pb2.FieldDescriptorProto.TYPE_UINT64
        f.label = descriptor_pb2.FieldDescriptorProto.LABEL_OPTIONAL

    pool = descriptor_pool.Default()
    pool.Add(fd)
    _SESSION_CLASS = message_factory.GetMessageClass(
        pool.FindMessageTypeByName("sigrok.Session")
    )
    _DEVICE_CLASS = message_factory.GetMessageClass(
        pool.FindMessageTypeByName("sigrok.Device")
    )
    _CHANNEL_CLASS = message_factory.GetMessageClass(
        pool.FindMessageTypeByName("sigrok.Channel")
    )
    _LOGIC_CLASS = message_factory.GetMessageClass(
        pool.FindMessageTypeByName("sigrok.Logic")
    )
    return _SESSION_CLASS, _DEVICE_CLASS, _CHANNEL_CLASS, _LOGIC_CLASS


def export_sigrok(capture, filename: str) -> str:
    """Export a RawCapture to a Sigrok session directory.

    Creates a ``.sr`` directory containing protobuf-encoded metadata
    and logic data that PulseView can open natively.

    Args:
        capture: RawCapture instance with sample data
        filename: Output filename (directory or ``.sr`` path)

    Returns:
        Path to the created ``.sr`` directory

    Raises:
        ValueError: If filename is empty or invalid
        TypeError: If capture is missing required attributes
    """
    if not filename or not filename.strip():
        raise ValueError("Invalid Sigrok session filename: empty string")

    sr_dir = filename
    if not sr_dir.endswith(".sr"):
        sr_dir = sr_dir + ".sr"

    os.makedirs(sr_dir, exist_ok=True)

    session_msg, device_msg, channel_msg, logic_msg = _get_sr_messages()

    session = session_msg()
    device = device_msg()
    device.model = "AGENTLA"
    device.vendor = "AgentLA"
    device.version = "v1.0"
    device.device_class = "logic-analyzer"
    device.sample_rate = capture.sample_rate if capture.sample_rate else 0
    device.driver = "agentla"

    num_channels = capture.num_channels if capture.num_channels else 8
    for i in range(num_channels):
        ch = channel_msg()
        ch.number = i
        ch.name = f"CH{i}"
        ch.logic = True
        ch.analog = False
        ch.type = 0
        ch.mapping = i
        device.channels.append(ch)

    session.devices.add().CopyFrom(device)

    metadata_path = os.path.join(sr_dir, "metadata")
    with open(metadata_path, "wb") as f:
        f.write(session.SerializeToString())

    logic_data = _pack_logic_data(capture)
    logic = logic_msg()
    logic.start_sample = 0
    logic.length = capture.num_samples if capture.num_samples else len(logic_data)
    logic.data = logic_data

    logic_path = os.path.join(sr_dir, "logic")
    with open(logic_path, "wb") as f:
        f.write(logic.SerializeToString())

    return sr_dir


def _pack_logic_data(capture) -> bytes:
    if hasattr(capture, 'data') and capture.data:
        return bytes(capture.data)
    return b''


class SigrokDecoderError(Exception):
    """Raised when SigrokDecoder encounters an error."""
    pass


class SigrokDecoder:
    """Wraps libsigrokdecode for host-side protocol decoding.

    Requires the ``sigrokdecode`` Python package and ``libsigrokdecode``
    C library. Install with ``pip install agentla[sigrok]`` after
    ``sudo apt install libsigrokdecode-dev`` (or equivalent).
    """

    def __init__(self, protocol: str):
        try:
            import sigrokdecode as srds
        except ImportError:
            raise SigrokDecoderError(
                "sigrokdecode package required for SigrokDecoder. "
                "Install with: pip install agentla[sigrok]"
            )
        try:
            self._decoder = srds.Decoder(protocol)
        except Exception as e:
            raise SigrokDecoderError(
                f"Protocol '{protocol}' not available in Sigrok: {e}"
            )
        self.protocol = protocol

    def decode(self, raw_data: bytes, sample_rate: int, **kwargs) -> list:
        """Decode raw capture data using Sigrok protocol decoders.

        Args:
            raw_data: Raw sample bytes from AgentLA capture
            sample_rate: Sample rate in Hz
            **kwargs: Protocol-specific parameters (e.g. sda=0, scl=1 for I2C)

        Returns:
            List of decoded events as dicts with 'type', 'timestamp', 'data' fields
        """
        try:
            self._decoder.start(sample_rate, kwargs)
            for i, byte in enumerate(raw_data):
                self._decoder.put(i, {'width': 1}, byte)
            self._decoder.end()
            return self._format_events()
        except Exception as e:
            raise SigrokDecoderError(f"Decoding failed: {e}") from e

    def _format_events(self) -> list:
        events = []
        try:
            for ann in self._decoder.ann:
                if not ann:
                    continue
                for text in ann:
                    events.append({
                        'protocol': self.protocol,
                        'text': text,
                    })
        except Exception:
            pass
        return events
