// AgentLA Landing Page & Subnet Discovery Script
document.addEventListener('DOMContentLoaded', () => {
    initPinoutVisualizer();
    initDeviceDiscovery();
});

// RP2350 30-Pin Definitions
const PIN_MAP = [
    { num: 0, role: "SWCLK", domain: "fixed", desc: "Fixed Programmer: Primary SWD/JTAG Target Clock. Driven by dedicated PIO state machine." },
    { num: 1, role: "SWDIO", domain: "fixed", desc: "Fixed Programmer: Primary SWD/JTAG Target Bidirectional Data with hardware turnaround." },
    { num: 2, role: "nRST", domain: "fixed", desc: "Fixed Programmer: Open-drain Hardware Target Reset line with external pull-up." },
    { num: 3, role: "TDI", domain: "fixed", desc: "Fixed Programmer: JTAG TDI input line / Secondary RV-SWD channel." },
    { num: 4, role: "TDO", domain: "fixed", desc: "Fixed Programmer: JTAG TDO output capture line with glitch filter." },
    { num: 5, role: "TRST", domain: "fixed", desc: "Fixed Programmer: Optional JTAG Test Reset line." },
    { num: 6, role: "SDI", domain: "fixed", desc: "Fixed Programmer: WCH QingKe 1-wire SDI bidirectional single-line debug interface." },
    { num: 7, role: "CLK_EXT", domain: "fixed", desc: "Fixed Programmer: Auxiliary reference target clock output (up to 50 MHz)." },
    { num: 8, role: "PROG_RX", domain: "fixed", desc: "Fixed Programmer: Dedicated target UART RX bridge." },
    { num: 9, role: "PROG_TX", domain: "fixed", desc: "Fixed Programmer: Dedicated target UART TX bridge." },
    { num: 10, role: "I2C_SDA", domain: "flexible", desc: "Flexible Dynamic Pool: Native i2c-tiny-usb default SDA line (GP10)." },
    { num: 11, role: "I2C_SCL", domain: "flexible", desc: "Flexible Dynamic Pool: Native i2c-tiny-usb default SCL line (GP11)." },
    { num: 12, role: "RS485_TX", domain: "flexible", desc: "RS-485 Transceiver: UART0 TX / Transceiver Driver Input (DI). Standby Hi-Z." },
    { num: 13, role: "RS485_RX", domain: "flexible", desc: "RS-485 Transceiver: UART0 RX / Transceiver Receiver Output (RO). Standby Hi-Z." },
    { num: 14, role: "RS485_DE", domain: "flexible", desc: "RS-485 Transceiver: Driver Enable / Receiver Enable (DE/~RE) control line." },
    { num: 15, role: "FLEX", domain: "flexible", desc: "Flexible Dynamic Pool: Claimable for Logic Analyzer D5, I2C1 SCL, or PWM." },
    { num: 16, role: "FLEX", domain: "flexible", desc: "Flexible Dynamic Pool: Claimable for Logic Analyzer D6, SPI1 SCK, or PWM." },
    { num: 17, role: "FLEX", domain: "flexible", desc: "Flexible Dynamic Pool: Claimable for Logic Analyzer D7, SPI1 MOSI, or PWM." },
    { num: 18, role: "CAN_TX", domain: "flexible", desc: "CAN Transceiver: Dedicated SocketCAN gs_usb TXD line. Standby Hi-Z until can0 brought up." },
    { num: 19, role: "CAN_RX", domain: "flexible", desc: "CAN Transceiver: Dedicated SocketCAN gs_usb RXD line. Standby Hi-Z until can0 brought up." },
    { num: 20, role: "FLEX", domain: "flexible", desc: "Flexible Dynamic Pool: User GPIO, High-speed PWM slice, or PIO auxiliary observer." },
    { num: 21, role: "FLEX", domain: "flexible", desc: "Flexible Dynamic Pool: User GPIO, High-speed PWM slice, or PIO auxiliary observer." },
    { num: 22, role: "CAN_TERM", domain: "flexible", desc: "CAN Transceiver: Switchable 120Ω bus termination toggle / Logic Analyzer CH2." },
    { num: 23, role: "FLEX", domain: "flexible", desc: "Flexible Dynamic Pool: User GPIO or onboard SMPS power-save pin." },
    { num: 24, role: "FLEX", domain: "flexible", desc: "Flexible Dynamic Pool: User GPIO or VBUS presence sense line." },
    { num: 25, role: "FLEX", domain: "flexible", desc: "Flexible Dynamic Pool: User GPIO or onboard activity LED driver." },
    { num: 26, role: "ADC0", domain: "adc", desc: "Analog & Flexible Pool: High-impedance 500 kS/s ADC Channel 0 / GPIO 26." },
    { num: 27, role: "ADC1", domain: "adc", desc: "Analog & Flexible Pool: High-impedance 500 kS/s ADC Channel 1 / GPIO 27." },
    { num: 28, role: "ADC2", domain: "adc", desc: "Analog & Flexible Pool: High-impedance 500 kS/s ADC Channel 2 / GPIO 28." },
    { num: 29, role: "ADC3", domain: "adc", desc: "Analog & Flexible Pool: High-impedance 500 kS/s ADC Channel 3 / VSYS sense / GPIO 29." }
];

function initPinoutVisualizer() {
    const container = document.getElementById('pinout-chips-container');
    const nameEl = document.getElementById('detail-pin-name');
    const badgeEl = document.getElementById('detail-badge');
    const descEl = document.getElementById('detail-desc');
    if (!container) return;

    PIN_MAP.forEach((pin, idx) => {
        const chip = document.createElement('div');
        chip.className = `pin-chip ${pin.domain}-pin`;
        chip.setAttribute('data-pin', pin.num);
        chip.innerHTML = `
            <span class="pin-chip-num">GP${pin.num}</span>
            <span class="pin-chip-role">${pin.role}</span>
        `;

        chip.addEventListener('click', () => {
            document.querySelectorAll('.pin-chip').forEach(c => c.classList.remove('active'));
            chip.classList.add('active');

            nameEl.textContent = `GPIO ${pin.num} (${pin.role})`;
            badgeEl.textContent = pin.domain.toUpperCase() + (pin.domain === 'fixed' ? ' (LOCKED)' : ' (CLAIMABLE)');
            badgeEl.style.backgroundColor = pin.domain === 'fixed' ? 'rgba(255, 159, 10, 0.2)' : 'rgba(0, 229, 255, 0.2)';
            badgeEl.style.color = pin.domain === 'fixed' ? '#ff9f0a' : '#00e5ff';
            descEl.textContent = pin.desc;
        });

        container.appendChild(chip);

        // Select first pin by default
        if (idx === 0) chip.click();
    });
}

function initDeviceDiscovery() {
    const checkBtn = document.getElementById('btn-check-device');
    const dot = document.getElementById('discovery-dot');
    const title = document.getElementById('discovery-title');
    const statusText = document.getElementById('discovery-status-text');
    const openDirect = document.getElementById('btn-open-direct');
    if (!checkBtn) return;

    checkBtn.addEventListener('click', () => {
        dot.className = 'status-indicator-dot checking';
        title.textContent = 'Probing Local Network...';
        statusText.textContent = 'Testing connection to http://192.168.7.1...';
        checkBtn.disabled = true;

        // Non-blocking ping test
        probeAgentLA('http://192.168.7.1', 2000)
            .then(online => {
                checkBtn.disabled = false;
                if (online) {
                    dot.className = 'status-indicator-dot active';
                    title.textContent = 'AgentLA Probe Detected!';
                    statusText.textContent = 'A physical AgentLA device is responding at 192.168.7.1. Click below to launch.';
                    openDirect.style.display = 'inline-flex';
                } else {
                    dot.className = 'status-indicator-dot';
                    title.textContent = 'Probe Not Detected on 192.168.7.1';
                    statusText.textContent = 'Device did not respond. Ensure USB is connected and your host IP is 192.168.7.x, or explore the live web demo below.';
                    openDirect.style.display = 'none';
                }
            });
    });
}

function probeAgentLA(url, timeoutMs) {
    return new Promise(resolve => {
        let finished = false;
        const timer = setTimeout(() => {
            if (!finished) {
                finished = true;
                resolve(false);
            }
        }, timeoutMs);

        // Test using image ping (bypasses CORS restrictions)
        const img = new Image();
        img.onload = () => {
            if (!finished) {
                finished = true;
                clearTimeout(timer);
                resolve(true);
            }
        };
        img.onerror = () => {
            // Even an error can indicate network presence if not timed out, but check fetch as well
            fetch(url + '/ok.json', { mode: 'no-cors', cache: 'no-store' })
                .then(() => {
                    if (!finished) {
                        finished = true;
                        clearTimeout(timer);
                        resolve(true);
                    }
                })
                .catch(() => {
                    if (!finished) {
                        finished = true;
                        clearTimeout(timer);
                        resolve(false);
                    }
                });
        };
        img.src = `${url}/wallpaper.svg?t=${Date.now()}`;
    });
}
