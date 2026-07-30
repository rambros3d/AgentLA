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

    def __len__(self) -> int:
        return len(self.data)

    def __getitem__(self, index):
        return self.data[index]


@dataclass
class CapturedEvent:
    """Base class for decoded protocol events."""

    timestamp: float
    type: str
    text: str
