# BakuProbe Hardware Specification & Pinout

**Target MCU**: Raspberry Pi RP2350B (QFN-80)  
**Board Profile ID**: `bakuprobe`  
**Baseline Profile**: `pico2` (RP2350A, QFN-60)

---

## 1. Executive Summary & Design Highlights

**BakuProbe** is an advanced open-hardware multi-instrument workstation probe based on the **RP2350B** (QFN-80 package). It provides a complete, simultaneous embedded toolset:
- **Zero Pin-Sharing**: 48 GPIOs and 8 ADCs partitioned into 5 independent physical zones.
- **Bi-Directional Auto-Sensing Level Shifting ($1.2\,\text{V} - 5.0\,\text{V}$)** on the Programmer and Bus Master pods via TI TXS0108E / TXB0108, referenced dynamically to target VCC (`VTARGET`).
- **Robust Power Architecture**: Direct USB-C $5\,\text{V}$ VBUS with USBLC6-2SC6 ESD protection and resettable PTC fuse, plus wide-input DC Barrel Jack ($7.0\,\text{V} - 24.0\,\text{V}$) with synchronous buck step-down for high-current target boards.
- **High-Speed Deterministic Capture**: 520 KB on-chip high-speed SRAM paired with 12 microcoded PIO state machines for zero-jitter, cycle-exact logic analyzer capture up to 150 MS/s.
- **Plug-and-Play Sensor Ecosystem**: JST-SH 1.0mm 4-pin **Qwiic / STEMMA QT** connector tied to the Zone 3 I2C bus.

---

## 2. Power Architecture & Voltage Regulation

```
[ USB-C Port (5V VBUS) ] ──> [ USBLC6-2SC6 ESD ] ──> [ PTC Polyfuse ] ──────────┐
                                                                                 ├──> [ Ideal Diode OR-ing ]
[ DC Barrel Jack 5.5x2.1 ] ──> [ P-FET Rev-Pol ] ──> [ SMBJ24A TVS ] ──> [ Buck ]┘             │
  (7.0V - 24.0V Aux Power)                               (5.0V 3A TPS54302)                     ▼
                                                                                      Internal 5.0V Rail
                                         ┌──────────────────────────────────┬───────────────────┴──────────────────┐
                                         ▼                                  ▼                                      ▼
                       [ TI TLV75533 3.3V 500mA LDO ]     [ TI TPS7A2033 3.3V Ultra-Low-Noise LDO ]            [ Programmable Target VCC ]
                                         │                                  │                                           (1.2V - 5.0V)
                                         ▼                                  ▼                                                      │
                       RP2350B Digital Core & Flash               RP2350B ADC & Analog Scope AFE                         High-Side INA180 Shunt
                                                                            │                                                      │
                                                          [ TI REF3030 3.000V Precision VREF ]                                     ▼
                                                                                                                      Target Board Power
```

1. **Power Inputs**:
   - **USB-C 5V VBUS**: Standard USB 2.0 Type-C input with dual 5.1kΩ CC pulldown resistors, ST USBLC6-2SC6 high-speed ESD protection, and a resettable PTC polyfuse.
   - **DC Barrel Jack**: Standard 5.5mm x 2.1mm / 2.5mm jack accepting $7\,\text{V} - 24\,\text{V}$ from bench power supplies, protected with P-FET reverse-polarity protection and an SMBJ24A TVS surge suppressor.
2. **System Step-Down & LDO Regulation**:
   - **Main 5.0V Rail**: Fed directly from USB-C VBUS or from the DC Jack via a TI TPS54302 synchronous buck regulator with ideal diode OR-ing.
   - **Digital 3.3V Rail**: TI TLV75533 low-dropout regulator (500mA).
   - **Analog 3.3V Rail (AVDD)**: Dedicated TI TPS7A2033 ultra-low-noise ($12\,\mu\text{V}_{\text{RMS}}$) LDO.
   - **Precision ADC Reference**: TI REF3030 ($3.000\,\text{V} \pm 0.2\%$, $35\,\text{ppm}/^\circ\text{C}$).
3. **Target Power Sourcing & Measurement**:
   - Firmware-controlled high-side P-FET switch (`TARGET_PWR_EN`).
   - High-side current measurement via **TI INA180A2** ($50\,\text{V/V}$, $50\,\text{m}\Omega$ shunt) mapped to ADC6 (`GP46`).
   - Target voltage divider ($10:1$) mapped to ADC7 (`GP47`) sensing $0.8\,\text{V} - 6.0\,\text{V}$ target health.

---

## 3. Five Dedicated Functional Zones

### Zone 1: Target Programmer Header (GPIO 0–9)
*Connector: 2x5 1.27mm / 2.54mm Shrouded Header (ARM Cortex-M 10-pin compatible) with TI TXS0108E $1.2\,\text{V} - 5.0\,\text{V}$ auto-sensing level shifter.*

| Pin | Function | Peripheral | Direction | Target Description |
|---|---|---|---|---|
| **GP0** | `PROG_TRST` | SIO | OUT | JTAG TRST (optional test reset) |
| **GP1** | `PROG_TDO` | SIO | IN | JTAG TDO (serial data out) |
| **GP2** | `PROG_BOOT0` | SIO (Push-Pull) | OUT | Target BOOT0 recovery drive |
| **GP3** | `PROG_TDI` | SIO | OUT | JTAG TDI (serial data in) |
| **GP4** | `PROG_SWDCLK` | PIO1 SM0 / SIO | OUT | SWD Clock / JTAG TCK |
| **GP5** | `PROG_SWDIO` | PIO1 SM0 / SIO | BI | SWD Bi-directional Data / JTAG TMS |
| **GP6** | `PROG_NRESET` | SIO (Open-Drain) | OUT | Active-low hardware target reset |
| **GP7** | `PROG_SDI` | PIO1 SM1 / SIO | BI | WCH 1-Wire SDI / RISC-V SWIO |
| **GP8** | `PROG_UART_TX` | UART1 TX (F1) | OUT | Target UART bootloader host TX |
| **GP9** | `PROG_UART_RX` | UART1 RX (F1) | IN | Target UART bootloader host RX |

---

### Zone 2: Industrial Differential Transceivers (GPIO 10–15)
*Connectors: Two 3-pin 3.5mm Screw Terminals with integrated ESD Protection.*

#### CAN-FD Transceiver (Texas Instruments TCAN334 / Microchip MCP2542FD)
- Native Linux `SocketCAN` support over USB bulk endpoints (`gs_usb` protocol).
- Dedicated 120 Ω switchable bus termination via low-noise solid-state relay / MOSFET.

| Pin | Net Name | Function | Direction | Notes |
|---|---|---|---|---|
| **GP10** | `CAN_TX` | SIO / PIO | Output | Transmit to TCAN334 TXD |
| **GP11** | `CAN_RX` | SIO / PIO | Input | Receive from TCAN334 RXD |
| **GP12** | `CAN_TERM_EN` | SIO | Output | Active-high $120\,\Omega$ termination relay |

#### RS-485 / Modbus / DMX-512 Transceiver (TI SN65HVD3082E / MaxLinear SP3485)
- Half-duplex differential transceiver with auto-direction DE/~RE control.

| Pin | Net Name | Function | Direction | Notes |
|---|---|---|---|---|
| **GP13** | `RS485_TX` | UART0 TX (F1) | Output | Driver input (DI) |
| **GP14** | `RS485_RX` | UART0 RX (F1) | Input | Receiver output (RO) |
| **GP15** | `RS485_DE` | SIO | Output | Driver/Receiver Enable (DE/~RE) |

---

### Zone 3: Dedicated Bus Master Pod & Quick-Connect (GPIO 16–23)
*Connectors: 2x5 2.54mm Shrouded Header + 4-pin JST-SH 1.0mm Qwiic / STEMMA QT Connector. TI TXS0108E / TXB0108 $1.2\,\text{V}-5.0\,\text{V}$ level shifting.*

| Pin | Net Name | Function | Direction | Description |
|---|---|---|---|---|
| **GP16** | `BUS_I2C_SDA` | I2C0 SDA / PIO2 I3C | Bi-dir | Dedicated I2C / I3C SDA line (Qwiic SDA, 2.2kΩ pull-up) |
| **GP17** | `BUS_I2C_SCL` | I2C0 SCL / PIO2 I3C | Bi-dir | Dedicated I2C / I3C SCL line (Qwiic SCL, 2.2kΩ pull-up) |
| **GP18** | `BUS_SPI_SCK` | SPI0 SCK (F0) | Output | Dedicated SPI Serial Clock |
| **GP19** | `BUS_SPI_MOSI` | SPI0 TX (F0) | Output | Master Out Slave In |
| **GP20** | `BUS_SPI_MISO` | SPI0 RX (F0) | Input | Master In Slave Out |
| **GP21** | `BUS_SPI_CS` | SIO | Output | Dedicated SPI Chip Select |
| **GP22** | `BUS_UART_TX` | UART1 TX (F10) | Output | Dedicated Aux UART TX |
| **GP23** | `BUS_UART_RX` | UART1 RX (F10) | Input | Dedicated Aux UART RX |

---

### Zone 4: 16-Channel Logic Analyzer & Pattern Generator (GPIO 24–39)
*Connector: 2x10 2.54mm Shrouded Header with interleaved GND ground shield pins.*

- Contiguous 16-pin range matching PIO0 `IN BASE 24` (`in pins, 16`).
- Max capture rate: **100 MS/s standard (150 MS/s turbo)** with DMA streaming into internal 520 KB high-speed SRAM.
- Any channel dynamically switchable to PWM generator or arbitrary pulse pattern generator (`OUTCLAIM`/`OUTSET`).

| Channel | GPIO | Pin Header Pin | Function |
|---|---|---|---|
| **LA_CH0** | GP24 | Pin 1 | PIO0 Logic In / PWM4 A |
| **LA_CH1** | GP25 | Pin 3 | PIO0 Logic In / PWM4 B |
| **LA_CH2** | GP26 | Pin 5 | PIO0 Logic In / PWM5 A |
| **LA_CH3** | GP27 | Pin 7 | PIO0 Logic In / PWM5 B |
| **LA_CH4** | GP28 | Pin 9 | PIO0 Logic In / PWM6 A |
| **LA_CH5** | GP29 | Pin 11 | PIO0 Logic In / PWM6 B |
| **LA_CH6** | GP30 | Pin 13 | PIO0 Logic In / PWM7 A |
| **LA_CH7** | GP31 | Pin 15 | PIO0 Logic In / PWM7 B |
| **LA_CH8** | GP32 | Pin 2 | PIO0 Logic In / PWM0 A |
| **LA_CH9** | GP33 | Pin 4 | PIO0 Logic In / PWM0 B |
| **LA_CH10**| GP34 | Pin 6 | PIO0 Logic In / PWM1 A |
| **LA_CH11**| GP35 | Pin 8 | PIO0 Logic In / PWM1 B |
| **LA_CH12**| GP36 | Pin 10 | PIO0 Logic In / PWM2 A |
| **LA_CH13**| GP37 | Pin 12 | PIO0 Logic In / PWM2 B |
| **LA_CH14**| GP38 | Pin 14 | PIO0 Logic In / PWM3 A |
| **LA_CH15**| GP39 | Pin 16 | PIO0 Logic In / PWM3 B |

*(Ground return pins: 17, 18, 19, 20).*

---

### Zone 5: 8-Channel Analog Oscilloscope Front-End (GPIO 40–47, ADC0–7)
*Connectors: Dual edge-mount BNC (CH1/CH2), dual SMA (CH3/CH4), pin header (CH5–CH7).*

- Reference Voltage: External **TI REF3030** precision $3.000\,\text{V} \pm 0.2\%$ voltage reference.
- High-impedance $1\,\text{M}\Omega \parallel 20\,\text{pF}$ input attenuator stage with switchable 1x / 10x range on CH1 & CH2 ($\pm 3\,\text{V} / \pm 30\,\text{V}$).
- Dual Schottky clamp diodes (BAT54S) on every channel for ESD & overvoltage protection.

| Channel | GPIO | ADC Input | Connector | Stage Details |
|---|---|---|---|---|
| **SCOPE_CH1** | GP40 | ADC0 | BNC / SMA | $1\,\text{M}\Omega$ input buffer, 1x/10x attenuator ($\pm 3\,\text{V} / \pm 30\,\text{V}$) |
| **SCOPE_CH2** | GP41 | ADC1 | BNC / SMA | $1\,\text{M}\Omega$ input buffer, 1x/10x attenuator ($\pm 3\,\text{V} / \pm 30\,\text{V}$) |
| **SCOPE_CH3** | GP42 | ADC2 | SMA | Direct $0-3\,\text{V}$ high-speed analog scope |
| **SCOPE_CH4** | GP43 | ADC3 | SMA | Direct $0-3\,\text{V}$ high-speed analog scope |
| **AUX_ADC5**  | GP44 | ADC4 | Pin Header | Auxiliary analog input / general sensor |
| **AUX_ADC6**  | GP45 | ADC5 | Pin Header | Auxiliary analog input / general sensor |
| **CURRENT_SENSE** | GP46 | ADC6 | Internal | High-side current monitor (TI INA180A2, 50V/V, $50\,\text{m}\Omega$ shunt) |
| **VTARGET_SENSE** | GP47 | ADC7 | Internal | Target VCC voltage divider (10:1, senses $0.8\,\text{V} - 6.0\,\text{V}$) |

---

## 4. Software & Firmware Configuration

The hardware profile is declaratively represented in [`boards/bakuprobe/board.json`](file:///run/media/sun/halo/AgentLA/boards/bakuprobe/board.json) and exposed to the C firmware via [`firmware/src/boards/board_bakuprobe.h`](file:///run/media/sun/halo/AgentLA/firmware/src/boards/board_bakuprobe.h).

To compile the firmware for the **BakuProbe** PCB:
```bash
# In CMake build settings (agentla_build_settings.cmake):
cmake -B build-work -DBAKUPROBE_BOARD=ON
make -C build-work -j4 AgentLA
```
