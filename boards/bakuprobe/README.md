# BakuProbe Custom Hardware & Feature Specification

**BakuProbe** is the reference custom workstation PCB for AgentLA, engineered as an all-in-one embedded hardware bring-up, power diagnostics, and agentic debugging instrument based on the **Raspberry Pi RP2350B (QFN-80 package)**.

---

## 1. Core Architecture & Compute

- **MCU**: Raspberry Pi RP2350B (Dual ARM Cortex-M33 / Hazard3 RISC-V @ 150 MHz nominal, up to 250+ MHz).
- **Package**: QFN-80 (48 GPIOs, 8 analog ADC channels).
- **On-Chip SRAM**: 520 KB flat SRAM (10 interleaved banks, zero cache jitter).
- **Flash Storage**: **Winbond W25Q80EWUXIE** (8 Mbit / 1 MB, 1.8V SPI NOR Flash, USON-8 2x3mm) connected directly to RP2350 `QSPI_IOVDD` (1.8V rail).
- **Precision Reference**: Dedicated **TI REF3030** ($3.000\,\text{V} \pm 0.2\%$, $35\,\text{ppm}/^\circ\text{C}$) for noise-immune ADC conversion.
- **Hierarchical Silicon Partitioning**:
  1. **Pins 00 .. 11 (Power & Instrumentation)**: Power telemetry, 20A motor PWM, auto-bypass flag, internal Power ADC (`SPI0`), shift register bus, emergency stop, and NeoPixels.
  2. **Pins 12 .. 18 (Programmer)**: Pod 1 Universal Target Debugger (SWD, JTAG, 1-Wire, UART0).
  3. **Pins 19 .. 31 (Adapter)**: Pod 2 Industrial Transceivers (RS-485, CAN-FD) + Pod 3 Host Bus Master (12.5 MHz I3C/I2C + SPI1).
  4. **Pins 32 .. 47 (Acquisition)**: Pod 4 8-Channel Logic Analyzer (`32..39`) + Pod 5 8-Channel MSO Oscilloscope (`40..47`).
- **Visual UX Indication**: **Chained Micro-RGB NeoPixels (SK6805-EC15 / WS2812B-1515)** placed adjacent to every physical header pin for dynamic role, voltage, and status lighting.
- **Hardware Emergency Stop**: Front-panel tactile pushbutton (`EMERGENCY_BTN`) for instant hardware trip into safe mode.

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
       [ High-Side P-FET Gate ] (Shift Reg Bit 24: TARGET_PWR_EN)
                    │
                    ▼
       [ High-Side Current Sense ] (TI INA180 Shunt Monitor ──> ADS1118 ADC Ch 1)
                    │
                    ├──────> [ Voltage Divider (10:1) ] ───────> ADS1118 ADC Ch 0
                    ▼
        Target MCU VTARGET Supply (Programmer & Bus Pods)


─── AUXILIARY HIGH-POWER DOMAIN (Motor Drivers / Power Electronics) ───────────────
[ DC Barrel Jack (up to 24V) ] ──> [ Reverse Polarity Protection & TVS ]
                                                   │
                                                   ▼
                                    [ High-Side Switch (20A) ] (RP2350 DC24V_PWM)
                                                   │
                                                   ▼
                                    [ High-Side Current Sense (20A) ] ──> ADS1118 ADC Ch 3
                                                   │
                                                   ├──────> [ 24V Resistor Divider ] ──> ADS1118 ADC Ch 2
                                                   ▼
                                    [ High-Current Screw Terminal Output ]
```

### 2.1 Target MCU Power Rail & Wide-Range Dynamic Telemetry
- **Voltage Selection**: Manual 3-way hardware selector switch choosing between **1.8V**, **3.3V**, or **5.0V**.
- **Automated Rail Auto-Detection**: A precision 10:1 resistor divider feeds TI ADS1118 Channel 0, allowing AgentLA firmware to automatically identify the target voltage rail and configure logic thresholds.
- **High-Side Target Power Switch**: Firmware-controlled P-channel MOSFET (`TARGET_PWR_EN`) enabling software-driven power cycling, hard resets, and brownout tests.
- **Ultra-Wide Dynamic Range Current Sense ($1\,\mu\text{A}$ to $500\,\text{mA}$)**:
  - **Hardware Auto-Bypass Architecture (PPK2-Lite Style)**:
    - **Dual Shunt Resistors**: $R_{\text{low}} = 0.05\,\Omega$ for Active range ($5\,\text{mA} - 500\,\text{mA}$) in series with $R_{\text{high}} = 10\,\Omega$ for Sleep range ($1\,\mu\text{A} - 5\,\text{mA}$).
    - **Auto-Bypass P-FET (SI2301 / DMP3099)**: Connected across $R_{\text{high}}$ to bypass it during active currents.
    - **Fast Hardware Comparator (TLV3691 / SGM8743)**: Automatically trips in $< 50\,\text{ns}$ when voltage drop exceeds $50\,\text{mV}$, driving the P-FET gate without firmware latency or software overhead.
    - **Sub-Nanosecond Schottky Diode Clamp (BAT54)**: In parallel across $R_{\text{high}}$, clamping maximum rail drop to $< 0.3\,\text{V}$ during sudden target wakeups to completely eliminate brownout resets.
    - **Telemetry Interface**: ADS1118 reads the INA180 current sense output, while direct RP2350 GPIO pin `GPIO 0` (`TARGET_RANGE_FLAG`) reads the comparator state to apply active calibration multipliers.

### 2.2 Auxiliary High-Power DC Path ($24\,\text{V} / 20\,\text{A}$) with PWM Modulation
- **Input**: Heavy-duty DC Barrel Jack ($5.5\text{mm} \times 2.1/2.5\text{mm}$) rated for up to $24\,\text{V}$.
- **Output**: Robust high-current pluggable screw terminal.
- **PWM-Modulated High-Side Switch**:
  - High-efficiency N-channel MOSFET ($40\text{V} / 60\text{V}$, $R_{\text{DS(on)}} < 3\,\text{m}\Omega$) driven by a dedicated high-side gate driver (e.g. TI LM5106 / MIC5014).
  - Driven by RP2350 hardware PWM pin `GPIO 1` (`DC24V_PWM`, up to **20 kHz**), allowing BakuProbe to act as a programmable DC motor speed controller, electronic load, and step-load transient tester.
- **Inductive Flyback Clamping**: Fast Schottky flyback diode (V10150 / SS54) and TVS suppressor across the output screw terminal to safely absorb inductive kickbacks from motor windings and solenoids.
- **High-Power Telemetry**: TI ADS1118 Channels 2 and 3 measure 24V rail voltage (divider) and 20A motor current (TI INA240 across $2\,\text{m}\Omega$ shunt).

---

## 3. Dedicated Functional Hardware Pods

### Pod 1: Universal Target Programmer & Debugger (Dual-Rail 74LVC Architecture)
* **Connector**: Standard 10-pin 1.27mm / 2.54mm shrouded ARM Cortex Debug Header.
* **Level Translation**: **Two SN74LVC4T245** dual-supply bus transceivers ($1.65\,\text{V} - 5.5\,\text{V}$):
  - **Zero Voltage Tracking Restrictions**: $V_{\text{CCA}}$ and $V_{\text{CCB}}$ operate completely independently ($V_{\text{CCA}} > V_{\text{CCB}}$, $V_{\text{CCA}} < V_{\text{CCB}}$, or $V_{\text{CCA}} = V_{\text{CCB}}$).
  - **Crisp Drive Strength**: $\pm 24\,\text{mA}$ push-pull drive delivering razor-sharp square waves up to **50+ MHz** over long ribbon cables.
* **SWDIO Bidirectional Architecture (CMSIS-DAP Split-Resistor)**:
  - `SWDIO_TX` drives target pin through a $100\,\Omega$ damping resistor.
  - `SWDIO_RX` listens directly at target pin.
  - **Instantaneous 0 ns Hardware Turnaround**: When switching from write to read, the probe puts `SWDIO_TX` in high-Z and reads `SWDIO_RX` immediately without software delay.
* **Pin Mapping (7 Direct RP2350 GPIOs on `GPIO 12..18`)**:
  - `GPIO 12`: `PROG_UART_TX` (Native hardware UART0 TX for ISP bootloader & serial monitor).
  - `GPIO 13`: `PROG_UART_RX` (Native hardware UART0 RX).
  - `GPIO 14`: `PROG_SWDCLK_TCK` (ARM SWD Clock / JTAG Clock via PIO).
  - `GPIO 15`: `PROG_SWDIO_TMS_SDI` (ARM SWD Data / JTAG Mode / WCH 1-Wire RISC-V debug data via PIO).
  - `GPIO 16`: `PROG_TDI` (JTAG Test Data In).
  - `GPIO 17`: `PROG_TDO` (JTAG Test Data Out / SWO Trace).
  - `GPIO 18`: `PROG_NRESET` (Target hardware reset driver).
* **Static Controls (Shift Register 4)**:
  - `PROG_OE` (Bit 25): Enables transceivers once target rail is verified.
  - `PROG_BOOT0` (Bit 30): Target bootloader entry pin drive.
  - `PROG_nTRST` (Bit 31): Optional JTAG TAP reset line.

### Pod 2: Industrial Differential Transceivers (Adapter Part 1)
* **Connectors**: Two 3.5mm pluggable screw terminals with TVS diode clamps.
* **CAN-FD Bus**:
  - **Transceiver**: **SIT1044QTK** (5 Mbps high-speed CAN-FD, 3.3V/5V logic, standby mode).
  - `GPIO 19`: `CAN_TX` (PIO CAN controller engine).
  - `GPIO 22`: `CAN_RX`.
  - Shift Reg Bit 27 (`CAN_TERM_EN`): Switchable $120\,\Omega$ termination relay/MOSFET.
  - Shift Reg Bit 28 (`CAN_STBY`): Low-power transceiver sleep control.
* **RS-485 / Modbus / DMX-512**:
  - **Transceiver**: **TI SN75176BDQ** (standard differential transceiver, SOIC-8).
  - `GPIO 20`: `RS485_TX` (Native hardware UART1 TX).
  - `GPIO 21`: `RS485_RX` (Native hardware UART1 RX).
  - Shift Reg Bit 29 (`RS485_DE`): Driver enable / receiver enable control.

### Pod 3: External Host Bus Master (Adapter Part 2 - 100% Dedicated External Bus)
* **Design Rule**: **Zero internal components are attached to this bus.** This port is 100% dedicated to external target DUT communication, completely eliminating probe-induced bus collisions.
* **Connectors**: 2x5 2.54mm shrouded header + **JST-SH 1.0mm 4-pin Qwiic / STEMMA QT** connector.
* **Level Translation Architecture (Specialized Split Domain)**:
  - **I²C / I3C Bus (`SDA`, `SCL`)**:
    - **TI TCA39306** (8-pin VSSOP / SOT-23-8): Officially certified for **12.5 MHz MIPI I3C v1.1.1** as well as legacy I²C ($100\text{k}, 400\text{k}, 1\text{M}$ Fast-Plus).
    - Bilateral low-$R_{\text{on}}$ ($3.5\,\Omega$) pass-FET switch supporting dynamic push-pull and open-drain transitions across $0.9\,\text{V} - 5.5\,\text{V}$ target domains with $< 1\,\text{ns}$ propagation delay.
  - **SPI Bus (`SCK`, `MOSI`, `CS`, `MISO`)**:
    - **SN74LVC4T245** (TSSOP-16 dual-supply transceiver): Bank 1 drives `SCK`, `MOSI`, `CS` push-pull up to **50+ MHz** into $1.65\,\text{V} - 5.5\,\text{V}$ targets; Bank 2 buffers incoming `MISO`.
* **Pin Mapping (6 Direct RP2350 GPIOs on `GPIO 26..31`)**:
  - `GPIO 26`: `I2C1_SDA` (Native hardware I2C1 Data / PIO I3C Data with $2.2\,\text{k}\Omega$ pull-up).
  - `GPIO 27`: `I2C1_SCL` (Native hardware I2C1 Clock / PIO I3C Clock up to 12.5 MHz).
  - `GPIO 28`: `BUS_SPI1_RX` (`MISO`, native hardware `SPI1`).
  - `GPIO 29`: `BUS_SPI1_CS` (Native hardware `SPI1` Chip Select).
  - `GPIO 30`: `BUS_SPI1_SCK` (Native hardware `SPI1` Clock).
  - `GPIO 31`: `BUS_SPI1_TX` (`MOSI`, native hardware `SPI1`).
* **Static Control (Shift Register 4)**:
  - `BUS_OE` (Bit 26): Level shifter output enable for safe hot-plugging.

---

## 4. The High-Speed Acquisition Block (`GPIO 32 .. 47`, Last 16 Pins)

All 16 high-speed instrumentation channels are placed consecutively on the **highest pins of the RP2350B**, sitting adjacent on the silicon die and directly in the **`GPIOBASE = 16` window of `PIO2`**:

```
┌────────────────────────────────────────────────────────────────────────────────────────┐
│               RP2350B UPPER INSTRUMENTATION BLOCK (GPIO 32 .. 47)                      │
├──────────────────────────────────────────┬─────────────────────────────────────────────┤
│   POD 4: 8-CHANNEL LOGIC ANALYZER / TX   │      POD 5: 8-CHANNEL MSO OSCILLOSCOPE      │
│            (GPIO 32 .. 39)               │               (GPIO 40 .. 47)               │
│                                          │                                             │
│ • 8x 74LVC1T45 Bidirectional Shifters    │ • 2x SGM8054 / TP1564 Quad Rail-to-Rail     │
│ • 1.65V - 5.5V Independent Translation   │ • 4x WCH CH444G 4:1 Range Switches          │
│ • Single-Cycle 150 MS/s PIO2 Capture     │ • 12-Bit SAR ADC (ADC0..ADC7)               │
│ • 150 MS/s Hardware Pattern Generation   │ • 150 MS/s PIO2 Digital Capture             │
│ • Shrouded 2x8 Pin Header (Row A Signal) │ • Shrouded 2x8 Pin Header (Row A Signal)    │
│ • Interleaved GND Pins (Row B Ground)    │ • Interleaved GND Pins (Row B Ground)       │
└──────────────────────────────────────────┴─────────────────────────────────────────────┘
```

### Pod 4: 8-Channel High-Speed Logic Analyzer & Pattern Generator (`GPIO 32 .. 39`)
* **Connector**: 2x8 2.54mm shrouded header with interleaved ground return pins.
* **Architecture**: 8 discrete single-gate dual-supply transceivers (**8$\times$ 74LVC1T45** in tiny SC-70-6 packages).
* **Direct RP2350 Pins (`GPIO 32..39`)**:
  - `GPIO 32`: `LA_CH0`
  - `GPIO 33`: `LA_CH1`
  - `GPIO 34`: `LA_CH2`
  - `GPIO 35`: `LA_CH3`
  - `GPIO 36`: `LA_CH4`
  - `GPIO 37`: `LA_CH5`
  - `GPIO 38`: `LA_CH6`
  - `GPIO 39`: `LA_CH7`
* **Independent Channel Direction**: Controlled by Shift Register 3 (`LA_DIR[0..7]`):
  - `0` (Input Mode): Port B is pure CMOS input ($C_{\text{in}} \approx 3\,\text{pF}, I_{\text{leakage}} < 1\,\mu\text{A}$). Passive logic sniffing at **150 MS/s** via PIO2.
  - `1` (Output Mode): Port B drives target with $\pm 24\,\text{mA}$ push-pull drive at `VTARGET` ($1.65\,\text{V} - 5.5\,\text{V}$). Pattern generator at **150 MS/s** via PIO2/PWM.
* **Multi-Role Capability**: Any pin dynamically acts as a hardware trigger in/out or 100+ MHz PWM frequency counter.

### Pod 5: 8-Channel Mixed-Signal Oscilloscope (MSO) (`GPIO 40 .. 47`)
* **Connector**: 2x8 2.54mm shrouded pin header with interleaved GND pins.
* **Dual-Personality Channels (MSO 0..7)**: Direct op-amp outputs into RP2350 `GPIO 40..47` (acting as BOTH 12-bit SAR ADC and 150 MS/s digital PIO2 inputs).
* **Discrete High-Speed Op-Amp Buffers**:
  - Two Quad Rail-to-Rail Op-Amps (**SGMICRO SGM8054** / **3PEAK TP1564**, $250\,\text{MHz}$ GBW, $130\,\text{V}/\mu\text{s}$ slew rate).
  - Input impedance: True **$1\,\text{M}\Omega$** ($900\,\text{k}\Omega / 100\,\text{k}\Omega$ divider), safely absorbing continuous overvoltage faults up to **$50\,\text{V}$ DC**.
* **Independent 4-Range Gain Switching (Per Channel)**:
  - Four **WCH CH444G** (Dual 4:1, $5\,\Omega$ low-$R_{\text{on}}$, 550 MHz) analog switches controlled by Shift Registers 1 & 2:
    - **$1\times$ Gain ($0.0\,\text{V} - 25.0\,\text{V}$)**: 24V industrial & motor signals.
    - **$2\times$ Gain ($0.0\,\text{V} - 12.5\,\text{V}$)**: 12V automotive & battery signals.
    - **$5\times$ Gain ($0.0\,\text{V} - 5.0\,\text{V}$)**: 5V TTL/CMOS & sensor signals.
    - **$10\times$ Gain ($0.0\,\text{V} - 2.5\,\text{V}$)**: 1.8V / 2.5V low-voltage MCU logic.

---

## 5. Master RP2350B 48-Pin Allocation Netlist

| Physical GPIO | BakuProbe Net Name | Hardware Peripheral | Functional Domain | Description |
| :---: | :--- | :--- | :--- | :--- |
| **`GPIO 0`** | `TARGET_RANGE_FLAG`| SIO Input | **1. Power & Instrumentation** | High-speed comparator input for target dynamic auto-bypass |
| **`GPIO 1`** | `DC24V_PWM` | PWM0B | **1. Power & Instrumentation** | 20 kHz PWM gate drive for 24V/20A motor power stage |
| **`GPIO 2`** | `SR_LATCH` | SIO Output | **1. Power & Instrumentation** | Atomic latch pulse for the 32-bit shift register chain |
| **`GPIO 3`** | `SR_CLK` | PIO0 | **1. Power & Instrumentation** | Dedicated Shift Register Clock (74HC595 SRCLK) |
| **`GPIO 4`** | `SR_DATA` | PIO0 | **1. Power & Instrumentation** | Dedicated Shift Register Serial Data (74HC595 SER) |
| **`GPIO 5`** | `EMERGENCY_BTN` | SIO Input (IRQ) | **1. Power & Instrumentation** | Front-panel Emergency Stop / Safe Mode pushbutton |
| **`GPIO 6`** | `NEOPIXEL_DATA` | PIO0 | **1. Power & Instrumentation** | Serial 800 kHz RGB drive for Header Pin NeoPixels |
| **`GPIO 7`** | `SPARE_GPIO7` | SIO / Spare | **1. Power & Instrumentation** | **Unassigned Pure Spare GPIO** (Routed to test pad) |
| **`GPIO 8`** | `INT_SPI0_RX` | SPI0_RX | **1. Power & Instrumentation** | Dedicated Internal SPI MISO from TI ADS1118 DOUT/DRDY |
| **`GPIO 9`** | `ADS1118_CS` | SPI0_CSn | **1. Power & Instrumentation** | Dedicated Internal SPI Chip Select for TI ADS1118 ADC |
| **`GPIO 10`** | `INT_SPI0_SCK` | SPI0_SCK | **1. Power & Instrumentation** | Dedicated Internal SPI Clock for TI ADS1118 ADC |
| **`GPIO 11`** | `INT_SPI0_TX` | SPI0_TX | **1. Power & Instrumentation** | Dedicated Internal SPI MOSI for TI ADS1118 ADC |
| **`GPIO 12`** | `PROG_UART_TX` | UART0_TX | **2. Programmer** | Pod 1 Target Serial ISP / Debug UART TX |
| **`GPIO 13`** | `PROG_UART_RX` | UART0_RX | **2. Programmer** | Pod 1 Target Serial ISP / Debug UART RX |
| **`GPIO 14`** | `PROG_SWDCLK_TCK`| PIO1 / SIO | **2. Programmer** | Pod 1 SWD Clock / JTAG Clock |
| **`GPIO 15`** | `PROG_SWDIO_TMS_SDI`| PIO1 / SIO | **2. Programmer** | Pod 1 SWD Data / JTAG TMS / WCH 1-Wire SDI |
| **`GPIO 16`** | `PROG_TDI` | PIO1 / SIO | **2. Programmer** | Pod 1 JTAG Test Data In |
| **`GPIO 17`** | `PROG_TDO` | PIO1 / SIO | **2. Programmer** | Pod 1 JTAG Test Data Out / SWO Trace |
| **`GPIO 18`** | `PROG_NRESET` | SIO | **2. Programmer** | Pod 1 Target Hardware Reset Output (Open-Drain) |
| **`GPIO 19`** | `CAN_TX` | PIO1 / SIO | **3. Adapter (Industrial)** | Pod 2 CAN-FD Transceiver Transmit Data |
| **`GPIO 20`** | `RS485_TX` | UART1_TX | **3. Adapter (Industrial)** | Pod 2 RS-485 Hardware UART1 TX |
| **`GPIO 21`** | `RS485_RX` | UART1_RX | **3. Adapter (Industrial)** | Pod 2 RS-485 Hardware UART1 RX |
| **`GPIO 22`** | `CAN_RX` | PIO1 / SIO | **3. Adapter (Industrial)** | Pod 2 CAN-FD Transceiver Receive Data |
| **`GPIO 23`** | `SPARE_GPIO23` | SIO / Spare | **3. Adapter** | **Unassigned Pure Spare GPIO** (Routed to test pad) |
| **`GPIO 24`** | `SPARE_GPIO24` | SIO / Spare | **3. Adapter** | **Unassigned Pure Spare GPIO** (Routed to test pad) |
| **`GPIO 25`** | `SPARE_GPIO25` | SIO / Spare | **3. Adapter** | **Unassigned Pure Spare GPIO** (Routed to test pad) |
| **`GPIO 26`** | `I2C1_SDA` | I2C1_SDA / PIO1 | **3. Adapter (Bus Master)** | Pod 3 Host Bus Master I2C / I3C Data |
| **`GPIO 27`** | `I2C1_SCL` | I2C1_SCL / PIO1 | **3. Adapter (Bus Master)** | Pod 3 Host Bus Master I2C / I3C Clock (up to 12.5 MHz) |
| **`GPIO 28`** | `BUS_SPI1_RX` | SPI1_RX | **3. Adapter (Bus Master)** | Pod 3 Dedicated External SPI MISO |
| **`GPIO 29`** | `BUS_SPI1_CS` | SPI1_CSn | **3. Adapter (Bus Master)** | Pod 3 Dedicated External SPI Chip Select |
| **`GPIO 30`** | `BUS_SPI1_SCK` | SPI1_SCK | **3. Adapter (Bus Master)** | Pod 3 Dedicated External SPI Bus Clock |
| **`GPIO 31`** | `BUS_SPI1_TX` | SPI1_TX | **3. Adapter (Bus Master)** | Pod 3 Dedicated External SPI MOSI |
| **`GPIO 32`** | `LA_CH0` | PIO2 | **4. Acquisition (LA)** | Pod 4 Logic Analyzer / Pattern Generator Channel 0 (150 MS/s) |
| **`GPIO 33`** | `LA_CH1` | PIO2 | **4. Acquisition (LA)** | Pod 4 Logic Analyzer / Pattern Generator Channel 1 (150 MS/s) |
| **`GPIO 34`** | `LA_CH2` | PIO2 | **4. Acquisition (LA)** | Pod 4 Logic Analyzer / Pattern Generator Channel 2 (150 MS/s) |
| **`GPIO 35`** | `LA_CH3` | PIO2 | **4. Acquisition (LA)** | Pod 4 Logic Analyzer / Pattern Generator Channel 3 (150 MS/s) |
| **`GPIO 36`** | `LA_CH4` | PIO2 | **4. Acquisition (LA)** | Pod 4 Logic Analyzer / Pattern Generator Channel 4 (150 MS/s) |
| **`GPIO 37`** | `LA_CH5` | PIO2 | **4. Acquisition (LA)** | Pod 4 Logic Analyzer / Pattern Generator Channel 5 (150 MS/s) |
| **`GPIO 38`** | `LA_CH6` | PIO2 | **4. Acquisition (LA)** | Pod 4 Logic Analyzer / Pattern Generator Channel 6 (150 MS/s) |
| **`GPIO 39`** | `LA_CH7` | PIO2 | **4. Acquisition (LA)** | Pod 4 Logic Analyzer / Pattern Generator Channel 7 (150 MS/s) |
| **`GPIO 40`** | `MSO_CH0` | ADC0 / PIO2 | **4. Acquisition (MSO)** | Pod 5 Mixed-Signal Channel 0 (12-bit ADC / 150 MS/s Logic) |
| **`GPIO 41`** | `MSO_CH1` | ADC1 / PIO2 | **4. Acquisition (MSO)** | Pod 5 Mixed-Signal Channel 1 (12-bit ADC / 150 MS/s Logic) |
| **`GPIO 42`** | `MSO_CH2` | ADC2 / PIO2 | **4. Acquisition (MSO)** | Pod 5 Mixed-Signal Channel 2 (12-bit ADC / 150 MS/s Logic) |
| **`GPIO 43`** | `MSO_CH3` | ADC3 / PIO2 | **4. Acquisition (MSO)** | Pod 5 Mixed-Signal Channel 3 (12-bit ADC / 150 MS/s Logic) |
| **`GPIO 44`** | `MSO_CH4` | ADC4 / PIO2 | **4. Acquisition (MSO)** | Pod 5 Mixed-Signal Channel 4 (12-bit ADC / 150 MS/s Logic) |
| **`GPIO 45`** | `MSO_CH5` | ADC5 / PIO2 | **4. Acquisition (MSO)** | Pod 5 Mixed-Signal Channel 5 (12-bit ADC / 150 MS/s Logic) |
| **`GPIO 46`** | `MSO_CH6` | ADC6 / PIO2 | **4. Acquisition (MSO)** | Pod 5 Mixed-Signal Channel 6 (12-bit ADC / 150 MS/s Logic) |
| **`GPIO 47`** | `MSO_CH7` | ADC7 / PIO2 | **4. Acquisition (MSO)** | Pod 5 Mixed-Signal Channel 7 (12-bit ADC / 150 MS/s Logic) |

---

## 6. PIO Architecture & Partitioning

The RP2350 contains **3 independent PIO blocks (`PIO0`, `PIO1`, `PIO2`)** with **12 State Machines** and 96 instruction slots:

| PIO Block | `GPIOBASE` Window | State Machines & Functions Assigned | Notes |
| :---: | :---: | :--- | :--- |
| **`PIO0`** | **`0`** (`GPIO 0..31`) | • **SM0**: Header Pin Micro-RGB NeoPixel String (`GPIO 6`)<br>• **SM1**: Dedicated 32-bit Shift Register Driver (`GPIO 3, 4`) | Background DMA visual/config drivers |
| **`PIO1`** | **`0`** (`GPIO 0..31`) | • **SM0**: Pod 1 Target SWD/JTAG Bit Clocking (`GPIO 14..17`)<br>• **SM1, SM2**: Pod 2 CAN-FD Physical Layer Transmit & Receive (`GPIO 19, 22`)<br>• **SM3**: Pod 3 MIPI I3C 12.5 MHz Protocol Engine (`GPIO 26, 27`) | Fast communication & protocol engines |
| **`PIO2`** | **`16`** (`GPIO 16..47`) | • **SM0**: Pod 4 Logic Analyzer Capture (`in pins, 8` @ 150 MS/s on `GPIO 32..39`)<br>• **SM1**: Pod 4 Pattern Generator (`out pins, 8` @ 150 MS/s on `GPIO 32..39`)<br>• **SM2**: Pod 5 MSO Digital Mode Capture (`in pins, 8` @ 150 MS/s on `GPIO 40..47`) | **Full Acquisition Engine**: Manages all 16 channels with cross-triggering! |

---

## 7. 32-Bit Configuration Shift Register Map

Four cascaded 74HC595 shift registers shift 32 bits out over dedicated PIO pins in **$1.28\,\mu\text{s}$**:

```
[ RP2350 PIO0 ] ──> [ Shift Reg 1 ] ──> [ Shift Reg 2 ] ──> [ Shift Reg 3 ] ──> [ Shift Reg 4 ]
                    (MSO Ch 0..3)       (MSO Ch 4..7)       (LA_DIR 0..7)       (System Enables)
```

| Byte | Bit Offset | Signal Name | Function & Logic State |
| :---: | :---: | :--- | :--- |
| **Byte 1** | `Bit 0..1` | `MSO_GAIN0[1:0]` | MSO Channel 0 Gain (00=25V, 01=12.5V, 10=5V, 11=2.5V) |
| | `Bit 2..3` | `MSO_GAIN1[1:0]` | MSO Channel 1 Gain |
| | `Bit 4..5` | `MSO_GAIN2[1:0]` | MSO Channel 2 Gain |
| | `Bit 6..7` | `MSO_GAIN3[1:0]` | MSO Channel 3 Gain |
| **Byte 2** | `Bit 8..9` | `MSO_GAIN4[1:0]` | MSO Channel 4 Gain |
| | `Bit 10..11` | `MSO_GAIN5[1:0]` | MSO Channel 5 Gain |
| | `Bit 12..13` | `MSO_GAIN6[1:0]` | MSO Channel 6 Gain |
| | `Bit 14..15` | `MSO_GAIN7[1:0]` | MSO Channel 7 Gain |
| **Byte 3** | `Bit 16` | `LA_DIR0` | Pod 4 Ch 0 Direction (0 = Input Sniffer, 1 = Output Pattern Drive) |
| | `Bit 17` | `LA_DIR1` | Pod 4 Ch 1 Direction |
| | `Bit 18` | `LA_DIR2` | Pod 4 Ch 2 Direction |
| | `Bit 19` | `LA_DIR3` | Pod 4 Ch 3 Direction |
| | `Bit 20` | `LA_DIR4` | Pod 4 Ch 4 Direction |
| | `Bit 21` | `LA_DIR5` | Pod 4 Ch 5 Direction |
| | `Bit 22` | `LA_DIR6` | Pod 4 Ch 6 Direction |
| | `Bit 23` | `LA_DIR7` | Pod 4 Ch 7 Direction |
| **Byte 4** | `Bit 24` | `TARGET_PWR_EN` | Target high-side P-FET power switch (1 = Power target, 0 = Off) |
| | `Bit 25` | `PROG_OE` | Programmer Level Shifter Output Enable (1 = Active, 0 = Safe Hi-Z) |
| | `Bit 26` | `BUS_OE` | Pod 3 Host Bus Level Shifter Output Enable (1 = Active, 0 = Safe Hi-Z) |
| | `Bit 27` | `CAN_TERM_EN` | CAN-FD $120\,\Omega$ bus termination relay (1 = Terminated, 0 = Open) |
| | `Bit 28` | `CAN_STBY` | SIT1044 CAN transceiver low-power standby mode (1 = Standby, 0 = Active) |
| | `Bit 29` | `RS485_DE` | SN75176 RS-485 Driver Enable (1 = Drive bus, 0 = Listen) |
| | `Bit 30` | `PROG_BOOT0` | Target MCU hardware boot mode drive |
| | `Bit 31` | `PROG_nTRST` | Target JTAG TAP reset line |

---

## 8. Bill of Materials (BOM) & Key ICs Checklist

| Function | Part Number | Package | Qty | Key Specifications |
| :--- | :--- | :--- | :---: | :--- |
| **Main MCU** | Raspberry Pi RP2350B | QFN-80 | 1 | Dual Cortex-M33 / Hazard3 @ 150MHz, 48 GPIOs, 8 ADCs |
| **SPI Flash** | Winbond W25Q80EWUXIE | USON-8 (2x3mm) | 1 | 8 Mbit (1 MB), 1.8V SPI NOR Flash, 104 MHz (on `QSPI_IOVDD`) |
| **Digital 3.3V LDO** | MKSemi AMS1117-3.3V | SOT-89 | 1 | 3.3V, up to 1A, robust thermal pad |
| **Digital 1.8V LDO** | MKSemi AMS1117-1.8V | SOT-89 | 1 | 1.8V, up to 1A, robust thermal pad |
| **Analog Power Filter** | Ferrite Bead (BLM18PG121) | 0603 | 1 | Passive LC filter from 3.3V to ADC_AVDD ($100\,\text{nF} + 2.2\,\mu\text{F}$) |
| **Voltage Reference** | TI REF3030AIDBZT | SOT-23-3 | 1 | 3.000V precision $\pm 0.2\%$, $35\,\text{ppm}/^\circ\text{C}$ drift |
| **Pod 1/3 Bus Transceivers**| TI / Nexperia SN74LVC4T245 | TSSOP-16 | 3 | 4-channel dual-supply transceivers ($1.65\text{V} - 5.5\text{V}$, 380 Mbps, 2 for Pod 1, 1 for Pod 3 SPI) |
| **Pod 3 I2C/I3C Level Shifter**| TI TCA39306DCUR | VSSOP-8 | 1 | **12.5 MHz MIPI I3C v1.1.1** & legacy I2C ($0.9\text{V} - 5.5\text{V}$, $R_{\text{on}}=3.5\,\Omega$) |
| **LA Bidirectional Shifters**| TI / Nexperia 74LVC1T45 | SC-70-6 / SOT-363 | 8 | Single-bit dual-supply transceivers ($1.65\text{V} - 5.5\text{V}$, 150 MS/s) on `GPIO 32..39` |
| **CAN Transceiver** | SIT1044QTK | DFN3x3-8 / SOP-8 | 1 | High-speed CAN-FD (5 Mbps), 3.3V/5V logic, standby mode |
| **RS-485 Transceiver**| TI SN75176BDQ | SOIC-8 | 1 | Differential bus transceiver (standard half-duplex) |
| **Target Current Sense Amp**| TI INA180A2IDBVR | SOT-23-5 | 1 | High-side current monitor ($G = 50\,\text{V/V}$, 350 kHz BW) |
| **Auto-Bypass Comparator** | TI TLV3691 / SGM8743 | SC-70-5 / SOT-23 | 1 | Nano-power fast comparator for hardware auto-ranging (<50 ns) |
| **Auto-Bypass P-FET** | Vishay Si2301 / DMP3099 | SOT-23 | 1 | Low $R_{\text{DS(on)}}$ (<30 mΩ) P-channel MOSFET |
| **Brownout Clamp Diode** | Diodes Inc BAT54 | SOT-23 | 1 | Ultra-fast Schottky clamp diode (<1 ns forward turn-on) |
| **20A DC Current Sense**| TI INA240A1 | TSSOP-8 | 1 | High-side current sense amplifier with enhanced PWM rejection for 20A shunt |
| **20A PWM High-Side Switch**| N-FET + TI LM5106 / MIC5014 | DPAK / PowerPAK | 1 | 40V/60V low-$R_{\text{DS(on)}}$ (<3 mΩ) N-FET with high-speed gate driver (up to 20 kHz PWM) |
| **20A Flyback Clamp Diode**| V10150 / SS54 | TO-277 / SMC | 1 | Fast high-current Schottky clamp diode across 24V screw terminal |
| **Power Telemetry ADC** | TI ADS1118IRUGR | VSSOP-10 / X2QFN | 1 | 16-bit 4-channel SPI $\Delta\Sigma$ ADC with precision internal VREF on dedicated SPI0 (`GPIO 8..11`) |
| **MSO Quad Op-Amps** | SGMICRO SGM8054 / TP1564 | TSSOP-14 | 2 | 250 MHz GBW, $130\,\text{V}/\mu\text{s}$ slew rate, rail-to-rail I/O (8 channels total on `GPIO 40..47`) |
| **MSO 4:1 Switches** | WCH CH444G / 74LVC4052 | TSSOP-16 / SOP-16 | 4 | Dual 4:1 analog switch ($R_{\text{on}} \approx 5\,\Omega$, 550 MHz BW) |
| **Config Shift Registers**| 74HC595 / 74LVC595 | TSSOP-16 | 4 | 32-bit cascaded dedicated PIO shift register chain |
| **Header Micro-RGB LEDs** | SK6805-EC15 / WS2812B-1515 | 1515 SMD | ~40 | $1.5\text{mm} \times 1.5\text{mm}$ addressable RGB NeoPixels for header pin lighting |
| **USB ESD Protection** | UMW USBLC6-2SC6 | SOT-23-6 | 1 | Ultra-low capacitance ($3.5\,\text{pF}$) ESD protection for USB-C |
| **Logic ESD Protection** | UMW USBLC6-4SC6 | SOT-23-6 | 2 | 4-channel ultra-low capacitance ESD clamp for logic analyzer pins |
| **Screw Terminals** | Pluggable 3.5mm / 5.0mm | THT | 3 | Terminals for CAN, RS-485, and heavy-duty 24V/20A DC out |
| **Qwiic Connector** | JST-SH 1.0mm 4-pin | SMD R/A | 1 | STEMMA QT / Qwiic standard I2C connection |
| **Tactile Buttons** | 3x4mm / 6x6mm SMD | SMD | 3 | Pushbuttons for BOOTSEL, RESET, and EMERGENCY_BTN |

---

## 9. PCB Layout & Signal Integrity Guidelines

1. **Recommended Layer Stackup (4-Layer)**:
   - **Layer 1 (Top Signal)**: High-speed digital traces (USB D+/D- 90Ω diff pairs, SWD, SPI, PIO logic analyzer inputs).
   - **Layer 2 (Ground Plane)**: Continuous solid GND copper plane across the entire board.
   - **Layer 3 (Power Planes)**: Split copper pours for $+5.0\,\text{V}$, $+3.3\,\text{V}_{\text{DIG}}$, $+1.8\,\text{V}_{\text{DIG}}$, $+3.3\,\text{V}_{\text{ANA}}$ ($AVDD$), and high-current $24\,\text{V}$ DC bus.
   - **Layer 4 (Bottom Signal & Passives)**: Low-speed routing, transceivers, decoupling caps, and high-power DC traces.
2. **The 16-Channel Acquisition Edge**:
   - Route all 16 lines (`GPIO 32..47`) directly out to the board edge in a neat, contiguous row. Pod 4 (Logic Analyzer, pins 32–39) and Pod 5 (MSO, pins 40–47) sit side-by-side on the PCB with identical trace lengths and ground plane shielding.
3. **High-Current (20A) Auxiliary DC Path**:
   - Keep the $24\,\text{V} / 20\,\text{A}$ trace runs wide (using heavy top/bottom copper pours, min 4–6 mm with 2 oz copper or unmasked solder-tinned pours) between the DC barrel jack, high-side switch, shunt resistor, and screw terminal.
   - Place kelvin sense traces across the current shunt resistor to eliminate trace resistance error in the current measurement.
4. **Analog Ground & Noise Isolation**:
   - Keep digital switching traces far away from the analog front-end (Pod 5).
   - Filter $AVDD$ through a ferrite bead before feeding the RP2350 analog rail. Tie analog AGND to system GND at a single star point directly under the RP2350 thermal pad.
5. **Header Micro-LED Power Branch**:
   - Power the 40 NeoPixels from the 5V VBUS rail through a 100µF bulk capacitor and small ferrite bead to prevent LED PWM switching current from modulating the 3.3V sensitive digital or analog rails.
