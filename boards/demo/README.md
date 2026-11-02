# AgentLA Web Demo Board Package (`demo`)

This board definition is the single source of truth for the AgentLA hosted in-browser web demo, showcasing the full flagship feature set with all options enabled:

- **MCU**: RP2350B (Dual ARM Cortex-M33 / Hazard3 RISC-V @ 150MHz)
- **Package**: QFN-80 (48 GPIOs available)
- **Flash**: 16MB QSPI Flash
- **PSRAM**: 8MB QSPI PSRAM (APMemory APS6404L)
- **Power**: Dual-Input: USB-C (5V up to 3A) + DC Barrel Jack (7V-24V with TI TPS54302 Buck)
- **Analog Reference**: TI REF3030 (3.000V precision ±0.2%)
- **Target Sensing**: TI INA180 current monitor (GP46/ADC6) + 10:1 VTARGET divider (GP47/ADC7)

## Zones & Transceivers (All Enabled in Demo)
- **Zone 1**: Target Programmer Header (GP0-GP9) with TI TXS0108E 1.2V-5.0V Level Translation
- **Zone 2**: Industrial Differential Transceivers:
  - CAN-FD: TCAN334 (GP10 TX, GP11 RX) + GP12 120Ω relay (Active Onboard Mode)
  - RS-485: SN65HVD3082 (GP13 TX, GP14 RX, GP15 DE) (Active Onboard Mode)
- **Zone 3**: Dedicated Bus Master Pod & Qwiic/STEMMA QT (GP16-GP23) with level translation
- **Zone 4**: 16-Channel Logic Analyzer (GP24-GP39, 150 MHz DMA sampling)
- **Zone 5**: 8-Channel Oscilloscope & Power Sense Front-End (GP40-GP47)

## WebUI Assets
- `board.json`: Declarative pinout, capabilities, power architecture, and zones
- `logo.svg`: Board branding icon
- `favicon.svg`: Web favicon
- `pinout.svg`: Interactive multi-zone vector diagram with DOM element IDs matching `pin-gpX` and `zone-X`
