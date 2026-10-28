## ADDED Requirements

### Requirement: Raw capture mode streaming
The raw capture mode SHALL stream pin state data to host for use with Sigrok/PulseView or custom parsers.

#### Scenario: Raw mode activation
- **WHEN** host sends `CAPTURE samples=4096 mode=RAW\r\n`
- **THEN** device streams hex-encoded pin states over USB serial at the configured sample rate

#### Scenario: Raw mode data format
- **WHEN** raw capture is in progress
- **THEN** each byte represents one sample of all configured channels packed into bits (bit 0 = channel 0, bit 1 = channel 1, etc.)

#### Scenario: Raw mode sample rate preservation
- **WHEN** raw capture completes
- **THEN** all 4096 samples represent equally-spaced time points at the configured sample rate

### Requirement: Sigrok session compatibility
The raw capture mode output SHALL be compatible with Sigrok's session file format when exported.

#### Scenario: Export to .sr file
- **WHEN** user calls `capture.export_sigrok("session.sr")`
- **THEN** SDK writes a valid Sigrok session file that PulseView can open

#### Scenario: Sigrok metadata
- **WHEN** Sigrok session is created
- **THEN** file includes sample rate, channel count, and probe names as metadata

### Requirement: Raw capture Python object
The Python SDK SHALL return raw capture data in a `RawCapture` object with numpy array access.

#### Scenario: Access raw samples
- **WHEN** user calls `capture = device.capture_raw(samples=4096)`
- **THEN** `capture.data` is a numpy array of shape (4096, num_channels) with dtype uint8

#### Scenario: Raw capture metadata
- **WHEN** raw capture is complete
- **THEN** `capture.sample_rate`, `capture.num_channels`, and `capture.duration` are accessible

### Requirement: Sigrok decoder integration on host
The raw capture mode SHALL support passing data to Sigrok decoders running on the host PC.

#### Scenario: Sigrok I2C decode on host
- **WHEN** user opens PulseView and loads an I2C decoder on channels 0,1
- **THEN** PulseView displays decoded I2C transactions overlaid on waveforms

#### Scenario: Custom Python decoder
- **WHEN** user writes a Python script using `agentla.SigrokDecoder("I2C", sda=0, scl=1)`
- **THEN** SDK provides raw data in a format compatible with libsigrokdecode

### Requirement: Multi-channel raw bit packing
The raw capture mode SHALL pack multiple channels into bytes efficiently.

#### Scenario: 8 channels packed into 1 byte
- **WHEN** 8 channels are configured
- **THEN** each sample is 1 byte with channel 0 in bit 0 through channel 7 in bit 7

#### Scenario: 16 channels packed into 2 bytes
- **WHEN** 16 channels are configured
- **THEN** each sample is 2 bytes with channels 0-7 in byte 0 and channels 8-15 in byte 1

#### Scenario: 32 channels packed into 4 bytes
- **WHEN** 32 channels are configured
- **THEN** each sample is 4 bytes

### Requirement: Raw capture with trigger
The raw capture mode SHALL respect trigger configuration and include pre-trigger data.

#### Scenario: Raw capture with pre-trigger
- **WHEN** trigger is configured and capture has 50% pre-trigger
- **THEN** raw output includes 2048 samples before trigger and 2048 samples after trigger
