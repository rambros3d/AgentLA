# hosted-marketing-splash Specification

## Purpose
TBD - created by archiving change hosted-webui-landing-demo. Update Purpose after archive.
## Requirements
### Requirement: Marketing and Product Hero Presentation
The hosted landing page SHALL display an interactive, responsive hero section using pure DaisyUI `.hero`, `.hero-content`, and `.card` components showcasing AgentLA as an RP2350 high-speed multi-tool bench engine with live feature summaries, pinout details, and action prompts matching the on-device firmware styling.

#### Scenario: Viewing the hero landing page
- **WHEN** a user visits the hosted portal root (`/`)
- **THEN** the landing page renders a DaisyUI hero banner, feature cards with 1px `base-300` borders, and primary action buttons for "Launch Live Demo" and "Connect to Device" using DaisyUI `.btn .btn-primary`

### Requirement: Subnet Device Discovery and Direct Launch
The hosted landing page SHALL provide an automated and manual discovery control to detect whether a physical AgentLA probe is online on the local network (`192.168.7.1` or `agentla.local`) and provide a 1-click launch link.

#### Scenario: Checking local device reachability
- **WHEN** user clicks "Search for Connected AgentLA" on the landing page
- **THEN** the page probes the default IP endpoint (`http://192.168.7.1/ok.json`) and displays an active status badge with a direct link if reachable, or troubleshooting instructions if offline

### Requirement: WebSerial Hardware Link
The landing page SHALL support direct WebSerial USB connection on compatible HTTPS browsers to bridge serial commands directly from the hosted client to a USB-connected AgentLA.

#### Scenario: Initiating WebSerial connection
- **WHEN** user clicks "Connect via USB (WebSerial)" in a supported browser
- **THEN** the browser triggers `navigator.serial.requestPort()` filtering for Raspberry Pi RP2350 vendor ID (`0x2E8A`) and establishes a serial stream

