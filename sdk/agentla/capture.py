"""Capture data classes for AgentLA SDK."""

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
    
    def export_sigrok(self, filename: str):
        """
        Export capture to Sigrok session format.
        
        Args:
            filename: Output filename (.sr)
        
        Note:
            This is a stub implementation. Full Sigrok export
            requires additional file format handling.
        """
        # TODO: Implement Sigrok session file export
        raise NotImplementedError("Sigrok export not yet implemented")
    
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
