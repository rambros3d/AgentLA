# BakuProbe Custom Hardware & Feature Specification

**BakuProbe** is the reference custom workstation PCB for AgentLA, engineered as an all-in-one embedded hardware bring-up, power diagnostics, and agentic debugging instrument based on the **Raspberry Pi RP2350B (QFN-80 package)**.

> **Hardware Planning Note**: This document defines the planned features, power rails, high-power DC stages, and IC selection for the custom PCB. Final GPIO pin assignments will be mapped and locked during schematic capture and layout.

---

## 1. Core Architecture & Compute

- **MCU**: Raspberry Pi RP2350B (Dual ARM Cortex-M33 / Hazard3 RISC-V @ 150 MHz nominal, up to 250+ MHz).
- **Package**: QFN-80 (48 GPIOs, 8 analog ADC channels).
- **On-Chip SRAM**: 520 KB flat SRAM (10 interleaved banks, zero cache jitter).
- **Flash Storage**: **Winbond W25Q80EWUXIE** (8 Mbit / 1 MB, 1.8V SPI NOR Flash, USON-8 2x3mm).
- **Precision Reference**: Dedicated **TI REF3030** ($3.000\,\text{V} \pm 0.2\%$, $35\,\text{ppm}/^\circ\text{C}$) for noise-immune ADC conversion.

---

## 2. Power Architecture & Voltage Regulation

BakuProbe features two distinct power domains:
1. **Instrument & Target MCU Domain**: Powered entirely via USB-C ($5\,\text{V}$), with dedicated 3.3V and 1.8V LDOs.
2. **Auxiliary High-Power DC Domain**: A separate $24\,\text{V} / 20\,\text{A}$ power path with high-side switching and telemetry for testing power electronics, motor drivers, and actuators.

```
[ USB-C 5V VBUS ] ──> [ USBLC6-2SC6 ESD ] ──> [ PTC Polyfuse ] ──┬──> 5.0V Select Rail
                                                                 ├──> [ 3.3V LDO ] ──> 3.3V Select Rail
                                                                 └──> [ 1.8V LDO ] ──> 1.8V Select Rail
                                                                             │
                    ┌────────────────────────────────────────────────────────┘
                    ▼
          [ 3-Way Selector Switch ] (1.8V / 3.3V / 5.0V Manual Selection)
                    │
                    ▼
       [ High-Side P-FET Gate ] (Firmware-controlled TARGET_PWR_EN)
                    │
                    ▼
       [ High-Side Current Sense ] (TI INA180 Shunt Monitor ──> ADC Channel)
                    │
                    ├──────> [ Voltage Divider (10:1) ] ───────> ADC Channel (Auto-Detect 1.8V/3.3V/5V)
                    ▼
        Target MCU VTARGET Supply (Programmer & Bus Pods)


─── AUXILIARY HIGH-POWER DOMAIN (Motor Drivers / Power Electronics) ───────────────
[ DC Barrel Jack (up to 24V) ] ──> [ Reverse Polarity Protection & TVS ]
                                                   │
                                                   ▼
                                    [ High-Side Switch (20A) ] (Firmware controllable)
                                                   │
                                                   ▼
                                    [ High-Side Current Sense (20A) ] ──> ADC Channel
                                                   │
                                                   ├──────> [ 24V Voltage Divider ] ──> ADC Channel
                                                   ▼
                                    [ High-Current Screw Terminal Output ]
```

### 2.1 Target MCU Power Rail & Telemetry
- **Voltage Selection**: Manual 3-way hardware selector switch (or jumper) choosing between **1.8V**, **3.3V**, or **5.0V**.
- **Automated Rail Auto-Detection**: A precision 10:1 resistor divider feeds an RP2350 ADC channel, allowing AgentLA to automatically identify which voltage rail the user has selected (1.8V, 3.3V, or 5.0V) and calibrate logic level thresholds accordingly.
- **High-Side Target Power Switch**: Firmware-controlled P-channel MOSFET (`TARGET_PWR_EN`) enabling software-driven power cycling, hard resets, and brownout tests.
- **High-Side Target Current Sense**: Dedicated **TI INA180A2** high-side current monitor ($50\,\text{V/V}$) across a low-resistance shunt to measure MCU current draw from microamps to hundreds of milliamps (detecting Sleep, Active, and hard lockups).

### 2.2 Auxiliary High-Power DC Path ($24\,\text{V} / 20\,\text{A}$)
- **Input**: Heavy-duty DC Barrel Jack ($5.5\text{mm} \times 2.1/2.5\text{mm}$) rated for up to $24\,\text{V}$.
- **Output**: Robust 2-pin / 3-pin high-current pluggable screw terminal.
- **High-Side Load Switch**: Rated for **20 A continuous**, controllable by firmware (e.g. smart high-side power switch or low-$R_{\text{DS(on)}}$ high-current MOSFET with charge pump / gate driver).
- **High-Power Telemetry**:
  - **Voltage Sense**: Precision resistor divider scaled for $0 - 28\,\text{V}$ input range.
  - **Current Sense**: High-side low-milliohm shunt ($1\,\text{m}\Omega - 2\,\text{m}\Omega$) with dedicated current sense amplifier (e.g. TI INA240 / INA180) measuring up to **20 A**.
- **Target Use Cases**: Testing small motor drivers (BLDC / Stepper / Brushed DC), solenoids, relays, high-power LED arrays, and DC-DC converters under automated AI agent control.

---

## 3. Five Dedicated Functional Hardware Pods

### Pod 1: Universal Target Programmer & Debugger
* **Connector**: Standard 10-pin 1.27mm / 2.54mm shrouded ARM Cortex Debug Header.
* **Level Translation**: Dedicated **TI TXS0108E** bi-directional level shifter ($1.2\,\text{V} - 5.0\,\text{V}$), referenced dynamically to `VTARGET`.
* **Planned Signals**:
  - `SWD`: `SWDCLK` (clock via PIO), `SWDIO` (bi-directional data via PIO).
  - `JTAG`: `TCK`, `TMS`, `TDI`, `TDO`, `nTRST` (full hardware JTAG boundary scan/debug).
  - `1-Wire RISC-V`: `PROG_SDI` (single-wire bi-directional debug for WCH CH32V003/CH32X035).
  - `Recovery & Reset`: `PROG_NRESET` (open-drain reset driver), `PROG_BOOT0` (push-pull ST/WCH bootloader drive).
  - `ISP Bootloader UART`: Dedicated `PROG_UART_TX` and `PROG_UART_RX` for ST AN3155 and factory serial flashing.

### Pod 2: Industrial Differential Transceivers
* **Connectors**: Two 3-pin 3.5mm pluggable screw terminals with TVS diode clamps.
* **CAN-FD Bus**:
  - **Transceiver**: **SIT1044QTK** (Silicon Core Technology high-speed CAN-FD transceiver, 5 Mbps, compact package, 3.3V/5V logic compatible with standby mode).
  - **Switchable Termination**: Solid-state switch / MOSFET controlling a precision $120\,\Omega$ bus termination resistor.
  - Native Linux `SocketCAN` (`gs_usb`) over USB bulk endpoints.
* **RS-485 / Modbus / DMX-512**:
  - **Transceiver**: **TI SN75176BDQ** (standard differential bus transceiver, SOIC-8).
  - Direction control (`DE` / `~RE`) via hardware UART or GPIO.

### Pod 3: Host Debug & Bus Master Pod
* **Connectors**: 2x5 2.54mm shrouded header + **JST-SH 1.0mm 4-pin Qwiic / STEMMA QT** connector.
* **Level Shifting**: Dedicated **TI TXS0108E** ($1.2\,\text{V} - 5.0\,\text{V}$) referenced to selected bus VCC.
* **Planned Signals**:
  - **I2C / I3C**: Dedicated `SDA` and `SCL` lines with $2.2\,\text{k}\Omega$ pull-ups.
  - **SPI**: Dedicated `SCK`, `MOSI`, `MISO`, and `CS`.
  - **Auxiliary UART**: Independent `TX` and `RX` for external communication.

### Pod 4: 16-Channel High-Speed Logic Analyzer & Pattern Generator
* **Connector**: 2x10 2.54mm shrouded header with interleaved ground return pins.
* **Capture Engine**: 16 contiguous RP2350 pins routed to PIO0 for single-instruction ingestion (`in pins, 16`) up to **100–150 MS/s**.
* **Signal Conditioning**:
  - In-line $47\,\Omega - 100\,\Omega$ damping series resistors located close to RP2350 inputs.
  - Low-capacitance ESD protection diode arrays (e.g., ST USBLC6-4SC6).
* **Multi-Personality**: Any channel dynamically switchable between logic capture, hardware PWM generation, or arbitrary pulse output.

### Pod 5: 8-Channel Analog Oscilloscope & Power Diagnostics Front-End
* **Connectors**: Dual edge-mount BNC / SMA (CH1/CH2), dual SMA (CH3/CH4), pin header (CH5–CH8).
* **Analog Front-End (AFE)**:
  - **Programmable Gain Amplifier**: TI PGA113 (SPI controlled, gains 1, 2, 5, 10, 20, 50, 100, 200).
  - **Attenuator Stage**: High-impedance $1\,\text{M}\Omega \parallel 20\,\text{pF}$ input attenuator with switchable 1x / 10x range ($\pm 3\,\text{V} / \pm 30\,\text{V}$) on CH1 & CH2.
  - **Overvoltage Protection**: Dual Schottky clamp diodes (BAT54S) on every analog channel.
  - **Telemetry Channels**:
    - `SCOPE_CH1`, `SCOPE_CH2`: Attenuated/buffered scope inputs (BNC/SMA).
    - `SCOPE_CH3`, `SCOPE_CH4`: Direct $0-3\,\text{V}$ high-speed scope inputs (SMA).
    - `AUX_ADC`: Auxiliary general-purpose analog inputs.
    - `TARGET_CURRENT`: High-side current monitor for target MCU rail (INA180).
    - `TARGET_VOLTAGE`: Resistor divider sensing manual 1.8V / 3.3V / 5.0V target selection.
    - `AUX_DC_VOLTAGE`: Resistor divider sensing Auxiliary 0–24V DC input rail.
    - `AUX_DC_CURRENT`: High-side current sense for Auxiliary 20A DC output rail.

---

## 4. Bill of Materials (BOM) & Key ICs Checklist

| Function | Part Number | Package | Key Specifications |
| :--- | :--- | :--- | :--- |
| **Main MCU** | Raspberry Pi RP2350B | QFN-80 | Dual Cortex-M33 / Hazard3 @ 150MHz, 48 GPIOs, 8 ADCs |
| **SPI Flash** | Winbond W25Q80EWUXIE | USON-8 (2x3mm) | 8 Mbit (1 MB), 1.8V SPI NOR Flash, 104 MHz |
| **Digital 3.3V LDO** | TI TLV75533PDBVR | SOT-23-5 | 3.3V, 500 mA, low dropout (238 mV) |
| **Digital 1.8V LDO** | TI TLV75518PDBVR | SOT-23-5 | 1.8V, 500 mA low-dropout regulator |
| **Analog 3.3V LDO** | TI TPS7A2033PDBVR | SOT-23-5 | 3.3V, 300 mA, ultra-low noise ($12\,\mu\text{V}_{\text{RMS}}$), 95 dB PSRR |
| **Voltage Reference** | TI REF3030AIDBZT | SOT-23-3 | 3.000V precision $\pm 0.2\%$, $35\,\text{ppm}/^\circ\text{C}$ drift |
| **Level Shifters (x2)** | TI TXS0108EPWR | TSSOP-20 | 8-channel bi-directional auto-direction ($1.2\text{V} - 5.0\text{V}$) |
| **CAN Transceiver** | SIT1044QTK | DFN3x3-8 / SOP-8 | High-speed CAN-FD (5 Mbps), 3.3V/5V logic, standby mode |
| **RS-485 Transceiver**| TI SN75176BDQ | SOIC-8 | Differential bus transceiver (standard half-duplex) |
| **Target Current Sense**| TI INA180A2IDBVR | SOT-23-5 | High-side current monitor ($50\,\text{V/V}$ gain, 350 kHz BW) |
| **20A DC Current Sense**| TI INA240A1 / INA180 | TSSOP-8 / SOT-23 | High-side bi/unidirectional current sense amplifier for 20A shunt |
| **High-Current Switch**| High-side P-FET / Smart Switch | PowerPAK / D2PAK | 30V, >30A rated low-$R_{\text{DS(on)}}$ high-side switch (e.g. BTS500xx or P-FET) |
| **Scope PGA** | TI PGA113AIDGSR | MSOP-10 | SPI programmable gain amp ($G = 1..200$, 4.5 MHz BW) |
| **USB ESD Protection** | ST USBLC6-2SC6 | SOT-23-6 | Ultra-low capacitance ($3.5\,\text{pF}$) ESD protection |
| **Screw Terminals** | Pluggable 3.5mm / 5.0mm | THT | Terminals for CAN, RS-485, and heavy-duty terminal for 24V/20A DC out |
| **Qwiic Connector** | JST-SH 1.0mm 4-pin | SMD R/A | STEMMA QT / Qwiic standard I2C connection |

---

## 5. PCB Layout & Signal Integrity Guidelines

1. **Recommended Layer Stackup (4-Layer)**:
   - **Layer 1 (Top Signal)**: High-speed digital traces (USB D+/D- 90Ω diff pairs, SWD, SPI, PIO logic analyzer inputs).
   - **Layer 2 (Ground Plane)**: Continuous solid GND copper plane across the entire board.
   - **Layer 3 (Power Planes)**: Split copper pours for $+5.0\,\text{V}$, $+3.3\,\text{V}_{\text{DIG}}$, $+1.8\,\text{V}_{\text{DIG}}$, $+3.3\,\text{V}_{\text{ANA}}$ ($AVDD$), and high-current $24\,\text{V}$ DC bus.
   - **Layer 4 (Bottom Signal & Passives)**: Low-speed routing, transceivers, decoupling caps, and high-power DC traces.
2. **High-Current (20A) Auxiliary DC Path**:
   - Keep the $24\,\text{V} / 20\,\text{A}$ trace runs wide (using heavy top/bottom copper pours, min 4–6 mm with 2 oz copper or unmasked solder-tinned pours) between the DC barrel jack, high-side switch, shunt resistor, and screw terminal.
   - Place kelvin sense traces across the current shunt resistor to eliminate trace resistance error in the current measurement.
3. **Analog Ground & Noise Isolation**:
   - Keep digital switching traces far away from the analog front-end (Pod 5).
   - Filter $AVDD$ through a ferrite bead before feeding the TPS7A2033. Tie analog AGND to system GND at a single star point directly under the RP2350 thermal pad.
4. **Logic Analyzer Header Interleaving**:
   - The 2x10 shrouded header must alternate signals with ground pins (Row A: Signal, Row B: Ground) so every probe lead has an immediate adjacent return path, minimizing ground bounce at 150 MS/s.
5. **Pin Allocation Phase**:
   - Once schematic capture is initiated and component placement is settled, physical GPIO pins will be assigned, mapped into [`board.json`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/boards/bakuprobe/board.json), and linked to [`src/lib.rs`](file:///home/sun/Filelink/Firmwares/AgentLA-rust/boards/bakuprobe/src/lib.rs).
