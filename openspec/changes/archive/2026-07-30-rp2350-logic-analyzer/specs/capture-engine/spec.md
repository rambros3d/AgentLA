## ADDED Requirements

### Requirement: Configurable sample rate capture
The capture engine SHALL support configurable sample rates from 100 kHz to 100 MHz via a `rate` parameter in the CONFIGURE command.

#### Scenario: Set 10 MHz sample rate
- **WHEN** device receives `CONFIGURE channels=0,1,2,3 rate=10000000`
- **THEN** device responds with `OK configured 4 channels at 10000000 Hz`

#### Scenario: Invalid sample rate
- **WHEN** device receives `CONFIGURE rate=500000000`
- **THEN** device responds with `ERR invalid sample rate, max 100000000`

### Requirement: Multi-channel parallel capture
The capture engine SHALL sample all configured channels simultaneously in a single capture cycle.

#### Scenario: Capture 4 channels
- **WHEN** device receives `CONFIGURE channels=0,1,2,3 rate=1000000` followed by `CAPTURE samples=1000`
- **THEN** device captures all 4 channels in parallel at 1 MHz and returns 1000 samples per channel

#### Scenario: Maximum 32 channels
- **WHEN** device receives `CONFIGURE channels=0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15,16,17,18,19,20,21,22,23,24,25,26,27,28,29,30,31`
- **THEN** device responds with `OK configured 32 channels`

#### Scenario: Exceed channel limit
- **WHEN** device receives `CONFIGURE channels=0,1,2,3,4,5,6,7,8` (9 channels on 8-channel base)
- **THEN** device responds with `ERR max 8 channels (expand to 32 with EXPAND command)`

### Requirement: Edge trigger
The capture engine SHALL support triggering on rising or falling edges on any configured channel.

#### Scenario: Rising edge trigger on pin 0
- **WHEN** device receives `CONFIGURE trigger=RISING pin=0`
- **THEN** device captures samples before and after the first rising edge on pin 0

#### Scenario: Falling edge trigger
- **WHEN** device receives `CONFIGURE trigger=FALLING pin=2`
- **THEN** device captures samples before and after the first falling edge on pin 2

#### Scenario: No trigger
- **WHEN** device receives `CAPTURE samples=1000` without trigger configured
- **THEN** device captures 1000 samples immediately without waiting for an edge

### Requirement: Pattern trigger
The capture engine SHALL support triggering on a specific bit pattern across configured channels.

#### Scenario: Pattern trigger
- **WHEN** device receives `CONFIGURE trigger=PATTERN pattern=0b10101010 channels=0,1,2,3,4,5,6,7`
- **THEN** device triggers when the 8-bit pattern matches on the specified channels

### Requirement: Pre/post trigger buffer
The capture engine SHALL allocate configurable pre-trigger and post-trigger sample buffers.

#### Scenario: 50% pre-trigger
- **WHEN** device receives `CAPTURE samples=1000 pretrigger=50`
- **THEN** device captures 500 samples before trigger and 500 samples after trigger

#### Scenario: Default pre-trigger
- **WHEN** device receives `CAPTURE samples=1000` without pretrigger parameter
- **THEN** device uses 50% pre-trigger by default

### Requirement: DMA ping-pong buffering
The capture engine SHALL use DMA ping-pong buffering to achieve continuous capture without gaps.

#### Scenario: Continuous capture at 10 MHz
- **WHEN** capture is in progress at 10 MHz with 8 channels
- **THEN** DMA alternates between two 128 KB SRAM buffers with no sample loss during transfer

### Requirement: Maximum capture time
The capture engine SHALL enforce maximum capture time limits based on sample rate and buffer size.

#### Scenario: 10 MHz capture time limit
- **WHEN** device receives `CAPTURE samples=200000000` at 10 MHz rate (20 seconds)
- **THEN** device responds with `ERR exceeds max capture time, max 12.8s at 10MHz` and does not start capture

### Requirement: Capture status reporting
The capture engine SHALL report capture status including sample count and elapsed time.

#### Scenario: Successful capture completion
- **WHEN** capture completes successfully
- **THEN** device responds with `DONE samples=4096 time=0.0004096`
