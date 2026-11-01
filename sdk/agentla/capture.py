"""Capture data classes for AgentLA SDK."""

import os
from dataclasses import dataclass
from typing import List, Optional


@dataclass
class RawCapture:
    """Raw capture data from device."""

    data: bytes
    sample_rate: int
    num_channels: int
    duration: float

    @property
    def num_samples(self) -> int:
        """Number of samples in capture."""
        bytes_per_sample = (self.num_channels + 7) // 8
        return len(self.data) // bytes_per_sample if bytes_per_sample > 0 else 0

    def export_sigrok(self, filename: str) -> str:
        """Export capture to Sigrok session directory (.sr format).

        Creates a ``.sr`` directory containing protobuf-encoded metadata
        and logic data that PulseView can open natively.

        Args:
            filename: Output filename for the ``.sr`` session directory

        Returns:
            Path to the created ``.sr`` directory

        Raises:
            ValueError: If filename is empty or invalid
        """
        from .sigrok import export_sigrok
        return export_sigrok(self, filename)

    def export_vcd(self, filename: str) -> str:
        """Export capture to Value Change Dump (.vcd) format."""
        return export_vcd(self, filename)

    def __len__(self) -> int:
        return len(self.data)

    def __getitem__(self, index):
        return self.data[index]


def export_vcd(capture: RawCapture, filename: str) -> str:
    """Export RawCapture to Value Change Dump (.vcd) format."""
    if not filename or not filename.strip():
        raise ValueError("Invalid VCD filename: empty string")
    if not filename.endswith('.vcd'):
        filename = filename + '.vcd'

    num_channels = capture.num_channels
    bytes_per_sample = (num_channels + 7) // 8
    num_samples = capture.num_samples
    dt_ns = int(round(1_000_000_000 / capture.sample_rate)) if capture.sample_rate > 0 else 100

    symbols = [chr(33 + ch) for ch in range(min(num_channels, 90))]

    lines = [
        "$date 2026-11-02 $end",
        "$version AgentLA Decoupled Capture Engine $end",
        "$timescale 1ns $end",
        "$scope module logic $end"
    ]
    for ch in range(len(symbols)):
        lines.append(f"$var wire 1 {symbols[ch]} D{ch} $end")
    lines.append("$upscope $end")
    lines.append("$enddefinitions $end")
    lines.append("#0")
    lines.append("$dumpvars")

    prev_state = None
    data = capture.data

    for s_idx in range(num_samples):
        offset = s_idx * bytes_per_sample
        val = 0
        for b in range(bytes_per_sample):
            val |= data[offset + b] << (b * 8)

        if s_idx == 0:
            for ch in range(len(symbols)):
                bit = (val >> ch) & 1
                lines.append(f"{bit}{symbols[ch]}")
            lines.append("$end")
            prev_state = val
        else:
            if val != prev_state:
                time_ns = s_idx * dt_ns
                lines.append(f"#{time_ns}")
                for ch in range(len(symbols)):
                    prev_bit = (prev_state >> ch) & 1
                    curr_bit = (val >> ch) & 1
                    if curr_bit != prev_bit:
                        lines.append(f"{curr_bit}{symbols[ch]}")
                prev_state = val

    os.makedirs(os.path.dirname(os.path.abspath(filename)), exist_ok=True)
    with open(filename, 'w') as f:
        f.write("\n".join(lines) + "\n")

    return filename


def unpack_rle_packets(rle_data: bytes, num_channels: int = 16, sample_rate: int = 10_000_000) -> RawCapture:
    """Unpack 32-bit [RunLength:16 | PinState:16] RLE words into a RawCapture object.

    Args:
        rle_data: Raw byte string containing packed 32-bit words
        num_channels: Number of digital channels (default: 16)
        sample_rate: Sample frequency in Hz (default: 10 MS/s)

    Returns:
        RawCapture with fully reconstructed uncompressed sample data
    """
    import struct
    if not rle_data:
        return RawCapture(data=b'', sample_rate=sample_rate, num_channels=num_channels, duration=0.0)

    num_words = len(rle_data) // 4
    out_bytes = bytearray()
    bytes_per_sample = 2 if num_channels > 8 else 1

    for i in range(num_words):
        word = struct.unpack_from('<I', rle_data, i * 4)[0]
        pin_state = word & 0xFFFF
        run_length = (word >> 16) & 0xFFFF
        if run_length == 0:
            continue

        if bytes_per_sample == 1:
            out_bytes.extend(bytes([pin_state & 0xFF]) * run_length)
        else:
            sample_val = struct.pack('<H', pin_state)
            out_bytes.extend(sample_val * run_length)

    total_samples = len(out_bytes) // bytes_per_sample
    duration = total_samples / sample_rate if sample_rate > 0 else 0.0

    return RawCapture(
        data=bytes(out_bytes),
        sample_rate=sample_rate,
        num_channels=num_channels,
        duration=duration
    )


@dataclass
class CapturedEvent:
    """Base class for decoded protocol events."""

    timestamp: float
    type: str
    text: str
