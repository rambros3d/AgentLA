"""Exception classes for AgentLA SDK."""


class DeviceError(Exception):
    """Base exception for device errors."""
    pass


class DeviceNotFoundError(DeviceError):
    """Raised when no AgentLA device is found on any serial port."""
    pass


class TimeoutError(DeviceError):
    """Raised when device operation times out."""
    pass


class InvalidResponseError(DeviceError):
    """Raised when device returns an unexpected response."""
    pass


class CaptureError(DeviceError):
    """Raised when capture fails."""
    pass
