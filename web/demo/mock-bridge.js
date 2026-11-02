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

    // Pre-seed some default dynamic claims in flexible pool
    simState.activeClaims.set('HNDL_101', { pins: [14, 15], func: 3, observer: 0 }); // I2C1
    simState.activeClaims.set('HNDL_102', { pins: [16, 17, 18, 19], func: 1, observer: 0 }); // SPI1

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

        // 3. Fallback to normal fetch (for static local files)
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
            // Build dynamic pin discovery string
            // Format: OK SAFEDISCOVER,10:d=X:o=Y,11:d=X:o=Y...
            let pinParts = [];
            for (let pin = 10; pin <= 29; pin++) {
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

    // Auto-wire demo connect button if present
    document.addEventListener('DOMContentLoaded', () => {
        const btnWebSerial = document.getElementById('btn-webserial-connect');
        if (btnWebSerial) {
            btnWebSerial.addEventListener('click', async () => {
                if (window.AgentLA_WebSerial) {
                    const connected = await window.AgentLA_WebSerial.connect();
                    if (connected) {
                        btnWebSerial.textContent = 'USB Connected';
                        btnWebSerial.style.borderColor = '#34c759';
                        btnWebSerial.style.color = '#34c759';
                    }
                }
            });
        }
    });

})();
