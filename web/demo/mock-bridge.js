/**
 * AgentLA In-Browser Hardware Simulator & WebSerial Bridge
 * Intercepts embedded fetch endpoints (/mcp, /events, ok.json)
 * Emulates RP2350 multi-tool hardware, 30-pin dynamic registry,
 * synthetic waveform captures, and optional WebSerial USB connection.
 */

(function() {
    console.log("%c[AgentLA Demo]%c In-browser hardware simulator loaded", "color: #00e5ff; font-weight: bold;", "color: inherit;");

    // Virtual RP2350 State Machine
    const simState = {
        sysClockMhz: 150,
        coreVoltage: 1.10,
        probeState: 'SIM_IDLE',
        activeClaims: new Map(), // handle -> { pins: [], func: 0, observer: 0 }
        claimCounter: 100,
        i2cRegisters: {
            0x58: 0x58, // BMP280 chip ID
            0x68: 0x71, // MPU6050 WHO_AM_I
            0x50: 0x42  // 24C02 EEPROM byte
        }
    };

    // Synthetic multi-channel waveform generator for demo
    window.generateSyntheticWaveforms = function(numSamples) {
        const buf = new Uint8Array(numSamples);
        let cs = 1; // Channel 3: Active low CS
        let sck = 0; // Channel 0: Clock
        let mosi = 0; // Channel 1: Master Out
        let miso = 0; // Channel 2: Master In
        let uartTx = 1; // Channel 4: Idle high UART TX

        for (let i = 0; i < numSamples; i++) {
            // SPI packet burst every 256 samples
            const packetPhase = i % 256;
            if (packetPhase >= 32 && packetPhase < 160) {
                cs = 0; // Assert CS
                sck = (Math.floor(i / 4) % 2); // 4-sample half-period
                const bitIdx = Math.floor((packetPhase - 32) / 8) % 8;
                // Transmit simulated byte 0x9F (10011111) on MOSI, receive 0xEF (11101111) on MISO
                mosi = ((0x9F >> (7 - bitIdx)) & 1);
                miso = ((0xEF >> (7 - bitIdx)) & 1);
            } else {
                cs = 1; // De-assert CS
                sck = 0;
                mosi = 0;
                miso = 0;
            }

            // UART byte transmission every 512 samples at 9600 baud equivalent
            const uartPhase = i % 512;
            if (uartPhase >= 50 && uartPhase < 50 + 10 * 16) {
                const bit = Math.floor((uartPhase - 50) / 16);
                if (bit === 0) uartTx = 0; // Start bit
                else if (bit >= 1 && bit <= 8) {
                    const charCode = 0x41; // 'A'
                    uartTx = (charCode >> (bit - 1)) & 1;
                } else {
                    uartTx = 1; // Stop bit
                }
            } else {
                uartTx = 1; // Idle
            }

            // High-speed digital counters for channels 5..9
            const ch5 = (Math.floor(i / 8) % 2);
            const ch6 = (Math.floor(i / 16) % 2);
            const ch7 = (Math.floor(i / 32) % 2);
            const ch8 = (Math.floor(i / 64) % 2);
            const ch9 = (Math.floor(i / 128) % 2);

            buf[i] = (sck << 0) |
                     (mosi << 1) |
                     (miso << 2) |
                     (cs << 3) |
                     (uartTx << 4) |
                     (ch5 << 5) |
                     (ch6 << 6) |
                     (ch7 << 7);
        }
        return buf;
    };

    // Pre-seed default dynamic claims showcasing BakuProbe hardware zones
    simState.activeClaims.set('HNDL_CAN', { pins: [10, 11], func: 2, observer: 0 }); // Onboard CAN-FD (TCAN334)
    simState.activeClaims.set('HNDL_RS485', { pins: [13, 14, 15], func: 2, observer: 0 }); // Onboard RS-485 (SN65HVD3082)
    simState.activeClaims.set('HNDL_I2C', { pins: [16, 17], func: 3, observer: 0 }); // Bus Master I2C / Qwiic
    simState.activeClaims.set('HNDL_SPI', { pins: [18, 19, 20, 21], func: 1, observer: 0 }); // Bus Master SPI
    simState.activeClaims.set('HNDL_LA', { pins: [24, 25, 26, 27], func: 6, observer: 1 }); // 100MS/s Logic Analyzer

    // Demo Manifest (All Flagship Hardware Options Enabled)
    const DEMO_PINOUT_SVG = `<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 740 680" width="100%" height="100%" class="board-pinout-svg">
  <defs>
    <style>
      .board-bg { fill: #0b1120; stroke: #1e293b; stroke-width: 2; rx: 14px; }
      .pcb-zone { fill: #0f172a; stroke: #334155; stroke-width: 1.5; rx: 8px; }
      .zone-accent-1 { stroke: #38bdf8; }
      .zone-accent-2 { stroke: #f59e0b; }
      .zone-accent-3 { stroke: #10b981; }
      .zone-accent-4 { stroke: #818cf8; }
      .zone-accent-5 { stroke: #ec4899; }
      .pin-pad { fill: #1e293b; stroke: #64748b; stroke-width: 1.5; cursor: pointer; transition: all 0.2s ease; }
      .pin-pad:hover { fill: #38bdf8; stroke: #ffffff; stroke-width: 2.5; }
      .board-pin:hover text { fill: #38bdf8; font-weight: bold; }
      .zone-title { font-family: system-ui, sans-serif; font-size: 11px; font-weight: 700; fill: #f8fafc; letter-spacing: 0.5; }
      .pin-label { font-family: monospace, sans-serif; font-size: 10px; fill: #cbd5e1; user-select: none; }
      .pin-fn { font-family: sans-serif; font-size: 9px; fill: #94a3b8; }
      .mcu-box { fill: #020617; stroke: #3b82f6; stroke-width: 2; rx: 6px; }
    </style>
  </defs>

  <!-- Background Canvas -->
  <rect x="10" y="10" width="720" height="660" class="board-bg" />

  <!-- Central RP2350B MCU -->
  <rect x="290" y="270" width="160" height="130" class="mcu-box" />
  <text x="370" y="325" font-family="system-ui, sans-serif" font-size="14" font-weight="900" fill="#38bdf8" text-anchor="middle">RP2350B</text>
  <text x="370" y="345" font-family="monospace, sans-serif" font-size="10" fill="#94a3b8" text-anchor="middle">QFN-80 • 48 GPIOs</text>
  <text x="370" y="365" font-family="sans-serif" font-size="9" fill="#10b981" text-anchor="middle">16MB Flash • 8MB PSRAM</text>

  <!-- ZONE 1: Target Programmer Header (Top Left) -->
  <g id="zone-1" class="board-zone">
    <rect x="30" y="30" width="310" height="210" class="pcb-zone zone-accent-1" />
    <text x="45" y="55" class="zone-title" fill="#38bdf8">ZONE 1: TARGET PROGRAMMER (1.2V - 5.0V)</text>
    
    <g id="pin-gp4" class="board-pin">
      <rect x="45" y="75" width="55" height="22" rx="3" class="pin-pad" />
      <text x="72" y="90" class="pin-label" text-anchor="middle">GP4</text>
      <text x="110" y="90" class="pin-fn">SWCLK</text>
    </g>
    <g id="pin-gp5" class="board-pin">
      <rect x="45" y="105" width="55" height="22" rx="3" class="pin-pad" />
      <text x="72" y="120" class="pin-label" text-anchor="middle">GP5</text>
      <text x="110" y="120" class="pin-fn">SWDIO</text>
    </g>
    <g id="pin-gp6" class="board-pin">
      <rect x="45" y="135" width="55" height="22" rx="3" class="pin-pad" />
      <text x="72" y="150" class="pin-label" text-anchor="middle">GP6</text>
      <text x="110" y="150" class="pin-fn">nRESET</text>
    </g>
    <g id="pin-gp2" class="board-pin">
      <rect x="45" y="165" width="55" height="22" rx="3" class="pin-pad" />
      <text x="72" y="180" class="pin-label" text-anchor="middle">GP2</text>
      <text x="110" y="180" class="pin-fn">BOOT0</text>
    </g>
    <g id="pin-gp8" class="board-pin">
      <rect x="195" y="75" width="55" height="22" rx="3" class="pin-pad" />
      <text x="222" y="90" class="pin-label" text-anchor="middle">GP8</text>
      <text x="260" y="90" class="pin-fn">UART TX</text>
    </g>
    <g id="pin-gp9" class="board-pin">
      <rect x="195" y="105" width="55" height="22" rx="3" class="pin-pad" />
      <text x="222" y="120" class="pin-label" text-anchor="middle">GP9</text>
      <text x="260" y="120" class="pin-fn">UART RX</text>
    </g>
    <g id="pin-gp3" class="board-pin">
      <rect x="195" y="135" width="55" height="22" rx="3" class="pin-pad" />
      <text x="222" y="150" class="pin-label" text-anchor="middle">GP3</text>
      <text x="260" y="150" class="pin-fn">TGT_PWR</text>
    </g>
  </g>

  <!-- ZONE 2: Industrial Transceivers (Top Right) -->
  <g id="zone-2" class="board-zone">
    <rect x="400" y="30" width="310" height="210" class="pcb-zone zone-accent-2" />
    <text x="415" y="55" class="zone-title" fill="#f59e0b">ZONE 2: CAN-FD &amp; RS-485 TRANSCEIVERS</text>
    
    <g id="pin-gp10" class="board-pin">
      <rect x="415" y="75" width="55" height="22" rx="3" class="pin-pad" />
      <text x="442" y="90" class="pin-label" text-anchor="middle">GP10</text>
      <text x="480" y="90" class="pin-fn">CAN_TX (TCAN334)</text>
    </g>
    <g id="pin-gp11" class="board-pin">
      <rect x="415" y="105" width="55" height="22" rx="3" class="pin-pad" />
      <text x="442" y="120" class="pin-label" text-anchor="middle">GP11</text>
      <text x="480" y="120" class="pin-fn">CAN_RX (TCAN334)</text>
    </g>
    <g id="pin-gp12" class="board-pin">
      <rect x="415" y="135" width="55" height="22" rx="3" class="pin-pad" />
      <text x="442" y="150" class="pin-label" text-anchor="middle">GP12</text>
      <text x="480" y="150" class="pin-fn">120Ω Term Relay</text>
    </g>
    <g id="pin-gp13" class="board-pin">
      <rect x="415" y="165" width="55" height="22" rx="3" class="pin-pad" />
      <text x="442" y="180" class="pin-label" text-anchor="middle">GP13</text>
      <text x="480" y="180" class="pin-fn">RS485_TX (SN65HVD)</text>
    </g>
    <g id="pin-gp14" class="board-pin">
      <rect x="580" y="165" width="55" height="22" rx="3" class="pin-pad" />
      <text x="607" y="180" class="pin-label" text-anchor="middle">GP14</text>
      <text x="645" y="180" class="pin-fn">RS485_RX</text>
    </g>
    <g id="pin-gp15" class="board-pin">
      <rect x="580" y="135" width="55" height="22" rx="3" class="pin-pad" />
      <text x="607" y="150" class="pin-label" text-anchor="middle">GP15</text>
      <text x="645" y="150" class="pin-fn">RS485_DE</text>
    </g>
  </g>

  <!-- ZONE 3: Bus Masters (Mid Left) -->
  <g id="zone-3" class="board-zone">
    <rect x="30" y="260" width="230" height="150" class="pcb-zone zone-accent-3" />
    <text x="45" y="285" class="zone-title" fill="#10b981">ZONE 3: BUS MASTER POD</text>
    <g id="pin-gp16" class="board-pin">
      <rect x="45" y="300" width="50" height="20" rx="3" class="pin-pad" />
      <text x="70" y="314" class="pin-label" text-anchor="middle">GP16</text>
      <text x="105" y="314" class="pin-fn">SDA (Qwiic)</text>
    </g>
    <g id="pin-gp17" class="board-pin">
      <rect x="45" y="325" width="50" height="20" rx="3" class="pin-pad" />
      <text x="70" y="339" class="pin-label" text-anchor="middle">GP17</text>
      <text x="105" y="339" class="pin-fn">SCL (Qwiic)</text>
    </g>
    <g id="pin-gp18" class="board-pin">
      <rect x="45" y="350" width="50" height="20" rx="3" class="pin-pad" />
      <text x="70" y="364" class="pin-label" text-anchor="middle">GP18</text>
      <text x="105" y="364" class="pin-fn">SPI SCK</text>
    </g>
    <g id="pin-gp19" class="board-pin">
      <rect x="45" y="375" width="50" height="20" rx="3" class="pin-pad" />
      <text x="70" y="389" class="pin-label" text-anchor="middle">GP19</text>
      <text x="105" y="389" class="pin-fn">SPI MOSI</text>
    </g>
  </g>

  <!-- ZONE 4: 16-Channel Logic Analyzer (Bottom Left) -->
  <g id="zone-4" class="board-zone">
    <rect x="30" y="430" width="350" height="220" class="pcb-zone zone-accent-4" />
    <text x="45" y="455" class="zone-title" fill="#818cf8">ZONE 4: 16-CH LOGIC ANALYZER (GP24 - GP39)</text>
    <g id="pin-gp24" class="board-pin">
      <rect x="45" y="470" width="45" height="18" rx="2" class="pin-pad" />
      <text x="67" y="483" class="pin-label" text-anchor="middle">D0 (24)</text>
    </g>
    <g id="pin-gp25" class="board-pin">
      <rect x="95" y="470" width="45" height="18" rx="2" class="pin-pad" />
      <text x="117" y="483" class="pin-label" text-anchor="middle">D1 (25)</text>
    </g>
    <g id="pin-gp26" class="board-pin">
      <rect x="145" y="470" width="45" height="18" rx="2" class="pin-pad" />
      <text x="167" y="483" class="pin-label" text-anchor="middle">D2 (26)</text>
    </g>
    <g id="pin-gp27" class="board-pin">
      <rect x="195" y="470" width="45" height="18" rx="2" class="pin-pad" />
      <text x="217" y="483" class="pin-label" text-anchor="middle">D3 (27)</text>
    </g>
    <g id="pin-gp32" class="board-pin">
      <rect x="45" y="500" width="45" height="18" rx="2" class="pin-pad" />
      <text x="67" y="513" class="pin-label" text-anchor="middle">D8 (32)</text>
    </g>
    <g id="pin-gp39" class="board-pin">
      <rect x="195" y="500" width="45" height="18" rx="2" class="pin-pad" />
      <text x="217" y="513" class="pin-label" text-anchor="middle">D15 (39)</text>
    </g>
    <text x="45" y="550" class="pin-fn" fill="#94a3b8">150 MHz Continuous DMA Sampling with Alternating GND</text>
  </g>

  <!-- ZONE 5: 8-Channel Scope & Target Current Sense (Bottom Right) -->
  <g id="zone-5" class="board-zone">
    <rect x="400" y="430" width="310" height="220" class="pcb-zone zone-accent-5" />
    <text x="415" y="455" class="zone-title" fill="#ec4899">ZONE 5: OSCILLOSCOPE &amp; POWER (GP40 - GP47)</text>
    <g id="pin-gp40" class="board-pin">
      <rect x="415" y="470" width="55" height="20" rx="3" class="pin-pad" />
      <text x="442" y="484" class="pin-label" text-anchor="middle">GP40</text>
      <text x="480" y="484" class="pin-fn">Scope CH1 (ADC0)</text>
    </g>
    <g id="pin-gp41" class="board-pin">
      <rect x="415" y="495" width="55" height="20" rx="3" class="pin-pad" />
      <text x="442" y="509" class="pin-label" text-anchor="middle">GP41</text>
      <text x="480" y="509" class="pin-fn">Scope CH2 (ADC1)</text>
    </g>
    <g id="pin-gp46" class="board-pin">
      <rect x="415" y="535" width="55" height="20" rx="3" class="pin-pad" fill="#3b0764" stroke="#c084fc" />
      <text x="442" y="549" class="pin-label" text-anchor="middle" fill="#e9d5ff">GP46</text>
      <text x="480" y="549" class="pin-fn" fill="#d8b4fe">INA180 Current Sense</text>
    </g>
    <g id="pin-gp47" class="board-pin">
      <rect x="415" y="560" width="55" height="20" rx="3" class="pin-pad" fill="#3b0764" stroke="#c084fc" />
      <text x="442" y="574" class="pin-label" text-anchor="middle" fill="#e9d5ff">GP47</text>
      <text x="480" y="574" class="pin-fn" fill="#d8b4fe">VTARGET Voltage Sense</text>
    </g>
    <text x="415" y="615" class="pin-fn" fill="#94a3b8">REF3030 Precision 3.000V Analog Reference</text>
  </g>
</svg>`;

    const BAKUPROBE_DEMO_MANIFEST = {
        board_id: "demo",
        name: "AgentLA Demo",
        vendor: "AgentLA",
        version: "1.0.0",
        assets: {
            logo: "logo.svg",
            favicon: "favicon.svg",
            pinout: "pinout.svg"
        },
        network: {
            ip: "10.10.10.10",
            netmask: "255.255.255.0",
            domain: "agentla.local"
        },
        mcu: {
            chip: "RP2350B",
            package: "QFN-80",
            cores: "Dual ARM Cortex-M33 / Hazard3 RISC-V @ 150MHz",
            clock_max_mhz: 150,
            sram_bytes: 532480,
            flash_bytes: 16777216
        },
        gpio_range: [0, 47],
        power_architecture: {
            target_power: {
                voltage_range: "1.2V - 5.0V",
                firmware_switchable: true,
                current_sense_ic: "TI INA180",
                voltage_sense_divider: "10:1"
            }
        },
        features: {
            has_hardware_can: true,
            has_hardware_rs485: true,
            target_current_sense: true,
            target_power_control: true,
            qwiic_stemma_qt: true
        },
        zones: {
            zone1_programmer: {
                name: "Target Programmer Header",
                gpio_range: [0, 9],
                connector: "2x5 1.27mm / 2.54mm ARM Debug with 1.2V-5.0V Level Translation",
                pins: { trst: 0, tdo: 1, boot0: 2, tdi: 3, swclk: 4, swdio: 5, nrst: 6, sdi: 7, uart_tx: 8, uart_rx: 9 }
            },
            zone2_transceivers: {
                name: "Industrial Differential Transceivers",
                gpio_range: [10, 15],
                can_fd: { chip: "TCAN334", tx_pin: 10, rx_pin: 11, term_120ohm_relay_pin: 12, connector: "3-Pin Screw Terminal" },
                rs485: { chip: "SN65HVD3082", tx_pin: 13, rx_pin: 14, de_re_pin: 15, connector: "3-Pin Screw Terminal" }
            },
            zone3_bus_masters: {
                name: "Dedicated Bus Master Pod & Quick-Connect",
                gpio_range: [16, 23],
                connector: "2x5 2.54mm Shrouded Header with 1.2V-5.0V Level Translation",
                qwiic_stemma_qt: "4-pin JST-SH 1.0mm (SDA=GP16, SCL=GP17)"
            },
            zone4_logic_analyzer: {
                name: "16-Channel Logic Analyzer & Pattern Generator",
                gpio_range: [24, 39],
                channels_count: 16,
                base_pin: 24,
                connector: "2x10 2.54mm Header with Alternating GND"
            },
            zone5_analog_scope: {
                name: "8-Channel Analog Oscilloscope Front-End",
                gpio_range: [40, 47],
                adc_channels: [0, 1, 2, 3, 4, 5, 6, 7]
            }
        }
    };

    // Intercept native fetch
    const originalFetch = window.fetch;
    window.fetch = async function(resource, init) {
        const urlStr = typeof resource === 'string' ? resource : (resource ? resource.url : '');

        // 1. Intercept /mcp endpoint
        if (urlStr.includes('/mcp')) {
            const url = new URL(urlStr, window.location.origin);
            const method = url.searchParams.get('method') || '';
            const name = url.searchParams.get('name') || '';
            const args = url.searchParams.get('args') || '';

            return handleSimulatedMcpCommand(method, name, args);
        }

        // 2. Intercept ok.json ping
        if (urlStr.includes('ok.json')) {
            return new Response(JSON.stringify({ status: "ok", mode: "simulator", target: "RP2350" }), {
                status: 200,
                headers: { 'Content-Type': 'application/json' }
            });
        }

        // 3. Intercept board.json to guarantee all flagship options enabled in Web Demo
        if (urlStr.includes('board.json')) {
            try {
                let reqUrl = resource;
                if (typeof reqUrl === 'string' && reqUrl.startsWith('/')) {
                    reqUrl = reqUrl.substring(1);
                }
                const res = await originalFetch.call(this, reqUrl, init);
                if (res.ok) return res;
            } catch (e) {}
            try {
                const res = await originalFetch.apply(this, arguments);
                if (res.ok) return res;
            } catch (e) {}
            return new Response(JSON.stringify(BAKUPROBE_DEMO_MANIFEST), {
                status: 200,
                headers: { 'Content-Type': 'application/json' }
            });
        }

        // 4. Intercept pinout.svg to guarantee vector graphic preview is always available
        if (urlStr.includes('pinout.svg')) {
            try {
                let reqUrl = resource;
                if (typeof reqUrl === 'string' && reqUrl.startsWith('/')) {
                    reqUrl = reqUrl.substring(1);
                }
                const res = await originalFetch.call(this, reqUrl, init);
                if (res.ok) return res;
            } catch (e) {}
            try {
                const res = await originalFetch.apply(this, arguments);
                if (res.ok) return res;
            } catch (e) {}
            return new Response(DEMO_PINOUT_SVG, {
                status: 200,
                headers: { 'Content-Type': 'image/svg+xml' }
            });
        }

        // 5. Fallback to normal fetch (for static local files)
        return originalFetch.apply(this, arguments);
    };

    function handleSimulatedMcpCommand(method, name, args) {
        const cleanName = (name || '').trim().toUpperCase();
        const cleanArgs = (args || '').trim();

        let responsePayload = { ok: true, result: { stdout: "OK", status: 0 } };

        if (cleanName === 'STATUS') {
            const voltageJitter = (simState.coreVoltage + (Math.random() * 0.02 - 0.01)).toFixed(2);
            responsePayload = {
                ok: true,
                result: {
                    stdout: `SYS_CLK=${simState.sysClockMhz}MHz VREG=${voltageJitter}V STATE=${simState.probeState} MODE=SIMULATOR PROBE=RP2350_REV_A2`,
                    status: 0
                }
            };
        }
        else if (cleanName === 'SAFEDISCOVER') {
            // Build dynamic pin discovery string across full 48-pin range (GP10-47)
            let pinParts = [];
            for (let pin = 10; pin <= 47; pin++) {
                let d = 0;
                let o = 0;
                for (const claim of simState.activeClaims.values()) {
                    if (claim.pins.includes(pin)) {
                        if (claim.observer === 1) o++;
                        else d++;
                    }
                }
                pinParts.push(`${pin}:d=${d}:o=${o}`);
            }
            responsePayload = {
                ok: true,
                result: {
                    stdout: `OK SAFEDISCOVER,${pinParts.join(',')}`,
                    status: 0
                }
            };
        }
        else if (cleanName === 'PINCLAIM') {
            // Parse pins, func, observer
            const pinMatch = cleanArgs.match(/pins=([\d,]+)/i);
            const funcMatch = cleanArgs.match(/func=(\d+)/i);
            const obsMatch = cleanArgs.match(/observer=(\d+)/i);

            const pins = pinMatch ? pinMatch[1].split(',').map(n => parseInt(n.trim(), 10)) : [];
            const func = funcMatch ? parseInt(funcMatch[1], 10) : 0;
            const observer = obsMatch ? parseInt(obsMatch[1], 10) : 0;

            // Enforce GPIO 0-9 fixed programmer reservation
            const touchesFixed = pins.some(p => p < 10);
            if (touchesFixed) {
                responsePayload = {
                    ok: false,
                    error: "ERROR: GPIO 0-9 are strictly reserved for the Fixed Programmer block (non-flexible)",
                    result: {
                        stdout: "ERROR: PIN CLAIM CONFLICT (PINS 0-9 RESERVED FOR PROGRAMMER)",
                        status: 1
                    }
                };
            } else {
                const handleId = `HNDL_${++simState.claimCounter}`;
                simState.activeClaims.set(handleId, { pins, func, observer });
                responsePayload = {
                    ok: true,
                    result: {
                        stdout: `OK PINCLAIM HANDLE=${handleId} PINS=${pins.join(',')} FUNC=${func} OBSERVER=${observer}`,
                        status: 0
                    }
                };
            }
        }
        else if (cleanName === 'PINRELEASE') {
            const handleMatch = cleanArgs.match(/handle=([A-Za-z0-9_]+)/i);
            const handle = handleMatch ? handleMatch[1] : '';
            if (simState.activeClaims.has(handle)) {
                simState.activeClaims.delete(handle);
                responsePayload = { ok: true, result: { stdout: "OK PINRELEASE", status: 0 } };
            } else {
                responsePayload = { ok: true, result: { stdout: `OK PINRELEASE (handle ${handle} released)`, status: 0 } };
            }
        }
        else if (cleanName === 'CAPTURECLAIM') {
            const handleId = `HNDL_CAP_${++simState.claimCounter}`;
            responsePayload = {
                ok: true,
                result: {
                    stdout: `OK CAPTURECLAIM HANDLE=${handleId} ${cleanArgs}`,
                    status: 0
                }
            };
        }
        else if (cleanName === 'OUTCLAIM') {
            const pinMatch = cleanArgs.match(/pin=(\d+)/i);
            const pin = pinMatch ? parseInt(pinMatch[1], 10) : 12;
            const handleId = `HNDL_PWM_${++simState.claimCounter}`;
            simState.activeClaims.set(handleId, { pins: [pin], func: 4, observer: 0 });
            responsePayload = {
                ok: true,
                result: {
                    stdout: `OK OUTCLAIM HANDLE=${handleId} PIN=${pin} MODE=PWM`,
                    status: 0
                }
            };
        }
        else if (cleanName === 'OUTSET') {
            const pinMatch = cleanArgs.match(/pin=(\d+)/i);
            const freqMatch = cleanArgs.match(/freq=(\d+)/i);
            const dutyMatch = cleanArgs.match(/duty=(\d+)/i);
            const pin = pinMatch ? pinMatch[1] : '12';
            const freq = freqMatch ? freqMatch[1] : '1000';
            const duty = dutyMatch ? dutyMatch[1] : '50';
            responsePayload = {
                ok: true,
                result: {
                    stdout: `OK OUTSET PIN=${pin} FREQ=${freq} DUTY=${duty}`,
                    status: 0
                }
            };
        }
        else if (cleanName === 'OUTRELEASE') {
            const pinMatch = cleanArgs.match(/pin=(\d+)/i);
            const pin = pinMatch ? parseInt(pinMatch[1], 10) : 12;
            for (const [h, claim] of simState.activeClaims.entries()) {
                if (claim.pins.includes(pin) && claim.func === 4) {
                    simState.activeClaims.delete(h);
                    break;
                }
            }
            responsePayload = {
                ok: true,
                result: {
                    stdout: `OK OUTRELEASE PIN=${pin}`,
                    status: 0
                }
            };
        }
        else if (cleanName === 'ADCSCOPE' || cleanName === 'ADC') {
            const now = Date.now();
            const v0 = (1.65 + Math.sin(now / 200) * 0.8).toFixed(3);
            const v1 = (0.80 + Math.cos(now / 150) * 0.5).toFixed(3);
            const v2 = (2.20 + Math.sin(now / 300) * 0.4).toFixed(3);
            const v3 = (1.10 + Math.sin(now / 100) * 0.2).toFixed(3);
            responsePayload = {
                ok: true,
                result: {
                    stdout: `OK ADC CH0=${v0} CH1=${v1} CH2=${v2} CH3=${v3} RAW=2048,1000,2700,1360`,
                    status: 0
                }
            };
        }
        else if (cleanName === 'CONFIGURE' || cleanName === 'CAPTURE') {
            responsePayload = {
                ok: true,
                result: {
                    stdout: `OK ${cleanName} SIMULATED_CAPTURE_SUCCESS 100MS/s`,
                    status: 0
                }
            };
        }
        else if (cleanName === 'SWD TEST' || cleanName === 'SWD') {
            responsePayload = {
                ok: true,
                result: {
                    stdout: "OK SWD DPIDR=0x0BE12477 Cortex-M33 TARGET=RP2350_CHIP_PASS STATE=HALTED",
                    status: 0
                }
            };
        }
        else if (cleanName.includes('FLASH')) {
            responsePayload = {
                ok: true,
                result: {
                    stdout: `OK ${cleanName} 0x10000000 4096 BYTES VERIFIED_CRC32_MATCH`,
                    status: 0
                }
            };
        }
        else if (cleanName.includes('I2C')) {
            responsePayload = {
                ok: true,
                result: {
                    stdout: "OK I2C_OP ADDR=0x58 REG=0x00 ACK DATA=0x58 (BMP280 SENSOR DETECTED)",
                    status: 0
                }
            };
        }
        else if (cleanName.includes('SPI')) {
            responsePayload = {
                ok: true,
                result: {
                    stdout: "OK SPI_TRANSFER TX=9F000000 RX=00EF4018 (WINBOND W25Q128 FLASH JEDEC ID)",
                    status: 0
                }
            };
        }
        else if (cleanName.includes('I3C')) {
            responsePayload = {
                ok: true,
                result: {
                    stdout: "OK I3C_OP addr=0x08 pid=023500A1BC06 bcr=0x06 dcr=0xE0 (MIPI I3C BASIC MASTER READY)",
                    status: 0
                }
            };
        }
        else if (cleanName.includes('VTARGET') || cleanName.includes('POWER')) {
            const isOn = !cleanArgs.includes('OFF') && !cleanArgs.includes('0');
            const meas = isOn ? '3.31V' : '0.00V';
            const curr = isOn ? '14mA' : '0mA';
            responsePayload = {
                ok: true,
                result: {
                    stdout: `OK POWER ${isOn ? 'ON' : 'OFF'} (MEASURED: ${meas}, CURRENT: ${curr}, SWITCH: HARDWARE SLIDE 1.8V/3.3V/5.0V)`,
                    status: 0
                }
            };
        }
        else if (cleanName.includes('CAN')) {
            responsePayload = {
                ok: true,
                result: {
                    stdout: "OK CAN TX ID=0x123 DLC=8 DATA=11 22 33 44 55 66 77 88 ACK (TCAN334 ACTIVE)",
                    status: 0
                }
            };
        }
        else if (cleanName.includes('UART')) {
            responsePayload = {
                ok: true,
                result: {
                    stdout: "OK UART TX LEN=4 DATA='PING' RX LEN=4 DATA='PONG'",
                    status: 0
                }
            };
        }
        else if (cleanName.includes('MODBUS')) {
            responsePayload = {
                ok: true,
                result: {
                    stdout: "OK MODBUS SLAVE=1 FC=3 ADDR=0x0001 RESP=0400010002 CRC_OK (0x4B3A)",
                    status: 0
                }
            };
        }
        else if (cleanName.includes('ONEWIRE') || cleanName.includes('1WIRE')) {
            responsePayload = {
                ok: true,
                result: {
                    stdout: "OK 1WIRE RESET_PULSE_ACK ROM=28FF1234567800A1 DATA=4B014B467FFF0C101C (20.68°C)",
                    status: 0
                }
            };
        }
        else if (cleanName.includes('LIN')) {
            responsePayload = {
                ok: true,
                result: {
                    stdout: "OK LIN BREAK_DELIM_SYNC PID=0x3C DATA=01 02 03 04 CHECKSUM_ENHANCED_OK",
                    status: 0
                }
            };
        }
        else if (cleanName.includes('REGISTERS') || cleanName.includes('REGS')) {
            responsePayload = {
                ok: true,
                result: {
                    stdout: "R0=0x20000000 R1=0x00000001 R2=0x00000002 R3=0x00000003 R4=0x00000000 R5=0x00000000 R6=0x00000000 R7=0x00000000 R8=0x00000000 R9=0x00000000 R10=0x00000000 R11=0x00000000 R12=0x00000000 SP=0x20004000 LR=0x0800018D PC=0x08000244 xPSR=0x01000000",
                    status: 0
                }
            };
        }
        else if (cleanName.includes('GPIO') || cleanName.includes('PINSET')) {
            responsePayload = {
                ok: true,
                result: {
                    stdout: `OK GPIO ${cleanArgs}`,
                    status: 0
                }
            };
        }
        else if (cleanName.includes('DEBUG')) {
            responsePayload = {
                ok: true,
                result: {
                    stdout: "0x20000000: 01 23 45 67 89 AB CD EF 10 20 30 40 50 60 70 80\n0x20000010: AA BB CC DD EE FF 00 11 22 33 44 55 66 77 88 99",
                    status: 0
                }
            };
        }

        return new Response(JSON.stringify(responsePayload), {
            status: 200,
            headers: { 'Content-Type': 'application/json' }
        });
    }

    // WebSerial USB Connection Helper
    window.AgentLA_WebSerial = {
        port: null,
        reader: null,
        writer: null,
        isConnected: false,

        async connect() {
            if (!('serial' in navigator)) {
                alert("WebSerial is not supported in this browser. Please use Chrome, Edge, or Opera on desktop or Android.");
                return false;
            }
            try {
                this.port = await navigator.serial.requestPort({
                    filters: [{ usbVendorId: 0x2E8A }] // Raspberry Pi Vendor ID
                });
                await this.port.open({ baudRate: 115200 });
                this.isConnected = true;
                this.writer = this.port.writable.getWriter();
                this.startReadLoop();
                return true;
            } catch (err) {
                console.warn("[WebSerial] Connection cancelled or failed:", err);
                return false;
            }
        },

        async sendCommand(cmd) {
            if (!this.writer) return;
            const encoder = new TextEncoder();
            await this.writer.write(encoder.encode(cmd + "\r\n"));
        },

        async startReadLoop() {
            const decoder = new TextDecoder();
            while (this.port && this.port.readable) {
                this.reader = this.port.readable.getReader();
                try {
                    while (true) {
                        const { value, done } = await this.reader.read();
                        if (done) break;
                        const text = decoder.decode(value);
                        console.log("[WebSerial RX]", text);
                    }
                } catch (err) {
                    console.error("[WebSerial Read Error]", err);
                } finally {
                    this.reader.releaseLock();
                }
            }
        }
    };

})();
