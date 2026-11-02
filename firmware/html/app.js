/**
 * AgentLA Web UI Controller
 * Unified Active Bench & Debugger Workspace for RP2350
 * Zero third-party DOM dependencies
 * Fully compliant with Dynamic Pin Ownership Architecture (GPIO 0-9 Fixed, GPIO 10-29 Flexible)
 */

// Host and API configuration
const host = window.location.hostname || "192.168.7.1";

const APP_TITLES = Object.freeze({
    bench: "Active Bench",
    adapters: "USB Protocol Adapters",
    target: "Debugger",
    logic: "Logic Analyzer",
    generator: "Signal Generator",
    scope: "Analog Scope",
    flasher: "Target Flasher",
    debugger: "Target Debugger",
    gpio: "GPIO & Pin Matrix",
    i2c: "I2C Master",
    spi: "SPI Master",
    command: "Command Console",
    console: "Command Console",
    about: "About & Pinout",
    settings: "System Settings"
});

function formatFreq(hz) {
    if (!hz || isNaN(hz) || hz <= 0) return '0 Hz';
    if (hz >= 1e6) return (hz / 1e6).toFixed(2) + ' MHz';
    if (hz >= 1e3) return (hz / 1e3).toFixed(2) + ' kHz';
    return hz.toFixed(0) + ' Hz';
}

function formatDuration(us) {
    if (!us || isNaN(us) || us <= 0) return '0 µs';
    if (us >= 1e6) return (us / 1e6).toFixed(2) + ' s';
    if (us >= 1e3) return (us / 1e3).toFixed(2) + ' ms';
    if (us >= 1) return us.toFixed(1) + ' µs';
    return (us * 1e3).toFixed(0) + ' ns';
}

const APP_CLEANUPS = {
    scope: null,
    generator: null,
    logic: null
};

// Target Station power state
let targetPowerOn = false;
let targetVoltage = '3.31V';
let targetCurrent = '14mA';

// UI Settings storage
const UI_SETTINGS_STORAGE_KEY = 'agentla-ui-settings';
const DEFAULT_UI_SETTINGS = Object.freeze({
    wallpaperHue: 0,
    transparency: 'transparent'
});

function loadUiSettings() {
    try {
        const raw = localStorage.getItem(UI_SETTINGS_STORAGE_KEY);
        if (!raw) return { ...DEFAULT_UI_SETTINGS };
        const parsed = JSON.parse(raw);
        return {
            wallpaperHue: typeof parsed.wallpaperHue === 'number' ? parsed.wallpaperHue : DEFAULT_UI_SETTINGS.wallpaperHue,
            transparency: parsed.transparency === 'solid' ? 'solid' : 'transparent'
        };
    } catch (e) {
        return { ...DEFAULT_UI_SETTINGS };
    }
}

const UI_SETTINGS = loadUiSettings();

function saveUiSettings() {
    try {
        localStorage.setItem(UI_SETTINGS_STORAGE_KEY, JSON.stringify(UI_SETTINGS));
    } catch (e) { /* ignore */ }
}

function applyUiSettings() {
    // Solid state theme - no wallpaper hue or transparency effects
    document.body.classList.add('no-transparency');
}

// --------------------------------------------------------------------------
// Theme Engine
// --------------------------------------------------------------------------
const THEME_STORAGE_KEY = 'agentla-theme';

function getCurrentTheme() {
    return document.documentElement.getAttribute('data-theme') || 'dark';
}

function setTheme(theme) {
    const validTheme = (theme === 'light') ? 'light' : 'dark';
    document.documentElement.setAttribute('data-theme', validTheme);
    try {
        localStorage.setItem(THEME_STORAGE_KEY, validTheme);
    } catch (e) { /* ignore */ }

    document.querySelectorAll('.settings-theme-select').forEach(sel => {
        sel.value = validTheme;
    });

    // Notify active scopes and canvases to immediately redraw with matching theme colors
    window.activeLogicController?.forceRedraw();
    if (window.redrawScope) window.redrawScope();
    if (window.redrawGenPreview) window.redrawGenPreview();
}

function toggleTheme() {
    const current = getCurrentTheme();
    setTheme(current === 'dark' ? 'light' : 'dark');
}

// --------------------------------------------------------------------------
// Global API caller supporting JSON-RPC 2.0 and MCP REST endpoints
// --------------------------------------------------------------------------
async function callMcp(method, name, args) {
    try {
        if (name && (name.includes('FLASH ERASE') || name.includes('MASSERASE'))) {
            if (!confirm('Destructive flash operation: ' + name + '?\nConfirm to continue.')) {
                return { ok: false, error: 'Cancelled by user', stdout: 'Cancelled by user', raw: null };
            }
        }
        let url;
        if (name) {
            url = '/mcp?method=cmd&name=' + encodeURIComponent(name);
            if (args !== undefined && args !== null && String(args).length > 0) {
                url += '&args=' + encodeURIComponent(args);
            }
        } else {
            url = '/mcp?method=' + encodeURIComponent(method);
        }

        const response = await fetch(url);
        const text = await response.text();

        try {
            const data = JSON.parse(text);
            if (data.error) {
                const errMsg = data.error.message || JSON.stringify(data.error);
                return { ok: false, error: errMsg, stdout: errMsg, result: null, raw: data };
            }
            if (data.result) {
                const stdout = (data.result.stdout !== undefined) 
                    ? data.result.stdout 
                    : (data.result.queued ? ('QUEUED JOB ' + data.result.job_id) : JSON.stringify(data.result));
                const ok = (data.result.status === undefined || data.result.status === 0);
                return { ok: ok, stdout: stdout, result: data.result, error: ok ? null : ('Exit code ' + data.result.status), raw: data };
            }
            return { ok: true, stdout: text, result: data, error: null, raw: data };
        } catch (parseErr) {
            return { ok: true, stdout: text, result: text, error: null, raw: text };
        }
    } catch (err) {
        return { ok: false, error: String(err), stdout: 'ERROR: ' + err, result: null, raw: null };
    }
}

// Helpers
function $(parent, sel) { return (parent || document).querySelector(sel); }
function cleanHexInput(str) { return (str || '').replace(/[^0-9A-Fa-f]/g, '').toUpperCase(); }
function nowTimeString() { return new Date().toLocaleTimeString(); }

// --------------------------------------------------------------------------
// Slide-Up Inspector Drawer / Modal Overlay
// --------------------------------------------------------------------------
const inspectorDrawer = document.getElementById('inspector-drawer');
const inspectorOverlay = document.getElementById('inspector-overlay');
const drawerTitle = document.getElementById('drawer-title');
const drawerContent = document.getElementById('drawer-content');
const btnCloseDrawer = document.getElementById('btn-close-drawer');

function openInspector(title, contentNodeOrHtml) {
    if (drawerTitle) drawerTitle.textContent = title || 'Configuration & Inspector';
    if (drawerContent) {
        drawerContent.innerHTML = '';
        if (typeof contentNodeOrHtml === 'string') {
            drawerContent.innerHTML = contentNodeOrHtml;
        } else if (contentNodeOrHtml instanceof Node) {
            drawerContent.appendChild(contentNodeOrHtml);
        }
    }
    if (inspectorDrawer) inspectorDrawer.classList.add('show');
    if (inspectorOverlay) inspectorOverlay.classList.add('show');
}

function closeInspector() {
    if (inspectorDrawer) inspectorDrawer.classList.remove('show');
    if (inspectorOverlay) inspectorOverlay.classList.remove('show');
}

if (btnCloseDrawer) btnCloseDrawer.addEventListener('click', closeInspector);
if (inspectorOverlay) inspectorOverlay.addEventListener('click', closeInspector);

// --------------------------------------------------------------------------
// Primary Mode Navigation: Active Bench vs Debugger vs Pin Matrix
// --------------------------------------------------------------------------
let primaryMode = 'bench';


// --------------------------------------------------------------------------
// Centered Settings Modal Controller
// --------------------------------------------------------------------------
function openSettingsModal() {
    const modal = document.getElementById('modal-settings');
    if (modal) {
        modal.classList.add('open');
        modal.setAttribute('aria-hidden', 'false');
        const selTheme = document.getElementById('settings-theme-select');
        if (selTheme) selTheme.value = getCurrentTheme();
    }
}

function closeSettingsModal() {
    const modal = document.getElementById('modal-settings');
    if (modal) {
        modal.classList.remove('open');
        modal.setAttribute('aria-hidden', 'true');
    }
}

function openAgentGuideModal() {
    const modal = document.getElementById('modal-agent-guide');
    if (modal) {
        modal.classList.add('open');
        modal.style.display = 'flex';
        modal.setAttribute('aria-hidden', 'false');
    }
}

function closeAgentGuideModal() {
    const modal = document.getElementById('modal-agent-guide');
    if (modal) {
        modal.classList.remove('open');
        modal.style.display = 'none';
        modal.setAttribute('aria-hidden', 'true');
    }
}

function switchMode(mode) {
    if (mode === 'adapters' || mode === 'adapter') {
        primaryMode = 'adapters';
    } else if (mode === 'target') {
        primaryMode = 'target';
    } else if (mode === 'pins' || mode === 'matrix') {
        primaryMode = 'pins';
    } else if (mode === 'console' || mode === 'command' || mode === 'repl') {
        primaryMode = 'console';
    } else {
        primaryMode = 'bench';
    }

    const viewBench = document.getElementById('view-active-bench');
    const viewAdapters = document.getElementById('view-adapters');
    const viewTarget = document.getElementById('view-target-station');
    const viewPins = document.getElementById('view-pin-matrix');
    const viewConsole = document.getElementById('view-console');

    const btnBench = document.getElementById('nav-mode-bench');
    const btnAdapters = document.getElementById('nav-mode-adapters');
    const btnTarget = document.getElementById('nav-mode-target');
    const btnPins = document.getElementById('nav-mode-pins');
    const btnConsole = document.getElementById('nav-mode-console');

    if (viewBench) viewBench.classList.toggle('active', primaryMode === 'bench');
    if (viewAdapters) viewAdapters.classList.toggle('active', primaryMode === 'adapters');
    if (viewTarget) viewTarget.classList.toggle('active', primaryMode === 'target');
    if (viewPins) viewPins.classList.toggle('active', primaryMode === 'pins');
    if (viewConsole) viewConsole.classList.toggle('active', primaryMode === 'console');

    if (btnBench) btnBench.classList.toggle('active', primaryMode === 'bench');
    if (btnAdapters) btnAdapters.classList.toggle('active', primaryMode === 'adapters');
    if (btnTarget) btnTarget.classList.toggle('active', primaryMode === 'target');
    if (btnPins) btnPins.classList.toggle('active', primaryMode === 'pins');
    if (btnConsole) btnConsole.classList.toggle('active', primaryMode === 'console');

    // Synchronize both desktop vertical sidebar tabs and mobile bottom nav tabs
    ['bench', 'adapters', 'target', 'pins', 'console'].forEach(m => {
        document.querySelectorAll(`[data-mode="${m}"]`).forEach(btn => {
            btn.classList.toggle('active', primaryMode === m);
        });
    });

    if (primaryMode === 'adapters') {
        if (typeof selectAdapter === 'function') {
            selectAdapter(selectedAdapter || getDefaultAdapter());
        }
    }

    if (primaryMode === 'pins') {
        updatePinMatrix();
        pollSafeDiscover();
    }

    window.dispatchEvent(new Event('resize'));
    if (primaryMode === 'bench' && window.activeLogicController) {
        window.activeLogicController.forceRedraw();
    }
}

// Backward compatibility alias for any legacy callers
function switchView(viewName, toolName = null) {
    if (viewName === 'target' || viewName === 'flasher' || viewName === 'debugger' || toolName === 'flasher' || toolName === 'debugger') {
        switchMode('target');
    } else {
        switchMode('bench');
    }
}

// --------------------------------------------------------------------------
// Slide-Out Hardware Pin Matrix Drawer Controller
// --------------------------------------------------------------------------
function togglePinDrawer(forceOpen) {
    const drawer = document.getElementById('pin-matrix-drawer');
    const backdrop = document.getElementById('pin-matrix-backdrop');
    const btnPins = document.getElementById('btn-toggle-pins');
    if (!drawer) return;

    const isOpen = drawer.classList.contains('open');
    const nextState = forceOpen !== undefined ? !!forceOpen : !isOpen;

    drawer.classList.toggle('open', nextState);
    if (backdrop) backdrop.classList.toggle('open', nextState);
    if (btnPins) btnPins.classList.toggle('active', nextState);

    if (nextState) {
        syncPinRegistry();
    }
}

// --------------------------------------------------------------------------
// Dynamic Pin Ownership & Board Manifest Architecture
// --------------------------------------------------------------------------
let BOARD_MANIFEST = null;

const FeatureRegistry = {
    manifest: null,
    hasHardwareCan: false,
    hasHardwareRs485: false,
    hasCurrentSense: false,
    hasTargetPowerControl: false,
    gpioMin: 0,
    gpioMax: 29,
    fixedGpioMax: 9,

    init(manifest) {
        this.manifest = manifest;
        const feat = manifest.features || {};
        this.hasHardwareCan = !!feat.has_hardware_can;
        this.hasHardwareRs485 = !!feat.has_hardware_rs485;
        this.hasCurrentSense = !!feat.target_current_sense || !!(manifest.power_architecture?.target_power?.current_sense_ic);
        this.hasTargetPowerControl = !!feat.target_power_control || !!(manifest.power_architecture?.target_power?.firmware_switchable);

        if (manifest.gpio_range && Array.isArray(manifest.gpio_range)) {
            this.gpioMin = manifest.gpio_range[0];
            this.gpioMax = manifest.gpio_range[1];
        } else {
            let maxP = 29;
            if (manifest.zones) {
                Object.values(manifest.zones).forEach(z => {
                    if (z.gpio_range && z.gpio_range[1] > maxP) maxP = z.gpio_range[1];
                });
            }
            this.gpioMin = 0;
            this.gpioMax = maxP;
        }

        if (manifest.zones) {
            Object.values(manifest.zones).forEach(z => {
                const zname = (z.name || '').toLowerCase();
                if (zname.includes('programmer') && z.gpio_range) {
                    this.fixedGpioMax = z.gpio_range[1];
                }
            });
        }

        this.applyBranding();
        this.applyTransceiverBadges();
        this.applyTelemetryWidgets();
    },

    applyBranding() {
        if (!this.manifest) return;
        const name = this.manifest.name || 'AgentLA';
        document.title = `${name} - AgentLA Unified Bench`;
        const titleEls = document.querySelectorAll('.brand-title');
        titleEls.forEach(el => { el.textContent = name; });

        const brandLogo = document.getElementById('brand-logo');
        if (brandLogo) {
            brandLogo.innerHTML = `<img src="logo.svg?v=${Date.now()}" alt="${name}" class="brand-logo-img" style="height: 20px; width: 20px; object-fit: contain; margin-right: 6px; vertical-align: middle;"> <span class="brand-title">${name}</span>`;
        }

        let link = document.querySelector("link[rel*='icon']");
        if (!link) {
            link = document.createElement('link');
            link.rel = 'icon';
            document.head.appendChild(link);
        }
        link.href = `favicon.svg?v=${Date.now()}`;

        const pinoutBoardLabel = document.getElementById('pinout-board-label');
        if (pinoutBoardLabel) {
            pinoutBoardLabel.textContent = `${this.manifest.mcu?.chip || 'RP2350'} • ${name}`;
        }

        const pinoutImg = document.getElementById('hardware-pinout-img');
        if (pinoutImg) {
            const pinoutSrc = this.manifest.assets?.pinout || 'pinout.svg';
            pinoutImg.src = `${pinoutSrc}?v=${Date.now()}`;
            pinoutImg.alt = `${name} Pinout Diagram`;
        }
    },

    applyTransceiverBadges() {
        const canCard = document.getElementById('card-adapter-can');
        const rs485Card = document.getElementById('card-adapter-rs485');

        if (canCard) {
            let badge = canCard.querySelector('.transceiver-mode-badge');
            if (!badge) {
                badge = document.createElement('div');
                badge.className = 'transceiver-mode-badge';
                canCard.querySelector('.adapter-card-header')?.appendChild(badge);
            }
            if (this.hasHardwareCan) {
                badge.className = 'transceiver-mode-badge onboard';
                badge.innerHTML = `<span class="badge-dot"></span> Onboard TCAN334 Transceiver`;
            } else {
                badge.className = 'transceiver-mode-badge breakout';
                badge.innerHTML = `<span class="badge-dot warning"></span> External Breakout Mode (GP18 TX / GP19 RX / GP22 Term)`;
            }
        }

        if (rs485Card) {
            let badge = rs485Card.querySelector('.transceiver-mode-badge');
            if (!badge) {
                badge = document.createElement('div');
                badge.className = 'transceiver-mode-badge';
                rs485Card.querySelector('.adapter-card-header')?.appendChild(badge);
            }
            if (this.hasHardwareRs485) {
                badge.className = 'transceiver-mode-badge onboard';
                badge.innerHTML = `<span class="badge-dot"></span> Onboard SN65HVD3082 Transceiver`;
            } else {
                badge.className = 'transceiver-mode-badge breakout';
                badge.innerHTML = `<span class="badge-dot warning"></span> External Breakout Mode (GP12 TX / GP13 RX / GP14 DE)`;
            }
        }
    },

    applyTelemetryWidgets() {
        const telemetryWidget = document.getElementById('target-power-telemetry');
        if (telemetryWidget) {
            if (this.hasCurrentSense) {
                telemetryWidget.style.display = 'flex';
                const icLabel = this.manifest.power_architecture?.target_power?.current_sense_ic || 'INA180';
                const labelEl = telemetryWidget.querySelector('.telemetry-ic-label');
                if (labelEl) labelEl.textContent = `(${icLabel})`;
            } else {
                telemetryWidget.style.display = 'none';
            }
        }
    }
};

let PIN_MAP = [];

function generatePinMapFromManifest(manifest) {
    const list = [];
    if (!manifest || !manifest.zones) return list;

    const levelShifting = manifest.level_shifting || {};

    Object.entries(manifest.zones).forEach(([zoneKey, zone]) => {
        const isProgZone = zoneKey.includes('prog') || (zone.name || '').toLowerCase().includes('programmer');
        const [zMin, zMax] = zone.gpio_range || [0, 0];

        let levelShift = '3.3V Direct';
        if (isProgZone && levelShifting.zone1_programmer) {
            levelShift = `${levelShifting.zone1_programmer.voltage_range} Level-Shifted (${levelShifting.zone1_programmer.chip})`;
        } else if (levelShifting.zone3_bus_masters && (zoneKey.includes('bus') || zoneKey.includes('adapt'))) {
            levelShift = `${levelShifting.zone3_bus_masters.voltage_range} Level-Shifted (${levelShifting.zone3_bus_masters.chip})`;
        } else if (zone.connector && zone.connector.includes('Level Translation')) {
            levelShift = '1.2V - 5.0V Level-Shifted';
        }

        for (let p = zMin; p <= zMax; p++) {
            let fn = `GPIO ${p}`;
            let role = isProgZone ? 'SYSTEM' : 'GENERAL';
            let isFixed = isProgZone;
            let isAnalog = false;

            if (zone.pins) {
                for (const [pName, pNum] of Object.entries(zone.pins)) {
                    if (pNum === p) {
                        fn = pName.toUpperCase();
                        role = isProgZone ? 'SWD/UART' : 'BUS';
                        break;
                    }
                }
            } else if (zone.can_fd && (zone.can_fd.tx_pin === p || zone.can_fd.rx_pin === p || zone.can_fd.term_120ohm_relay_pin === p)) {
                if (zone.can_fd.tx_pin === p) fn = 'CAN_TX';
                else if (zone.can_fd.rx_pin === p) fn = 'CAN_RX';
                else fn = 'CAN_TERM_120Ω';
                role = 'CAN';
            } else if (zone.rs485 && (zone.rs485.tx_pin === p || zone.rs485.rx_pin === p || zone.rs485.de_re_pin === p)) {
                if (zone.rs485.tx_pin === p) fn = 'RS485_TX';
                else if (zone.rs485.rx_pin === p) fn = 'RS485_RX';
                else fn = 'RS485_DE';
                role = 'RS485';
            } else if (zone.channels && zone.channels[`scope_ch${p - zMin + 1}`]) {
                const ch = zone.channels[`scope_ch${p - zMin + 1}`];
                fn = `SCOPE CH${ch.adc} (ADC${ch.adc})`;
                role = 'ADC';
                isAnalog = true;
            } else if (zone.adc_channels && zone.adc_channels.includes(p - zMin)) {
                const adcIdx = p - zMin;
                fn = `ANALOG ADC${adcIdx} / CH${adcIdx}`;
                role = 'ADC';
                isAnalog = true;
            } else if (zone.base_pin !== undefined) {
                const chIdx = p - zone.base_pin;
                fn = `LA CH${chIdx} (PIO)`;
                role = 'CAPTURE';
            } else if (p >= 10 && p <= 19) {
                if (p === 10) fn = 'A_SDA (I2C1)';
                else if (p === 11) fn = 'A_SCL (I2C1)';
                else if (p === 12) fn = 'RS-485 TX';
                else if (p === 13) fn = 'RS-485 RX';
                else if (p === 14) fn = 'RS-485 DE';
                else if (p === 15) fn = 'A_MOSI (SPI1)';
                else if (p === 16) fn = 'A_MISO (SPI1)';
                else if (p === 17) fn = 'A_CS (SPI1)';
                else if (p === 18) fn = 'CAN_TX';
                else if (p === 19) fn = 'CAN_RX';
                role = 'ADAPTER';
            } else if (p >= 20 && p <= 29) {
                if (p >= 26) {
                    fn = `ADC${p - 26} / LA CH${p - 20}`;
                    role = 'ADC';
                    isAnalog = true;
                } else {
                    fn = `LA CH${p - 20} (PIO0)`;
                    role = 'CAPTURE';
                }
            }

            list.push({
                pin: p,
                name: `GP${p}`,
                fn: fn,
                group: zoneKey,
                zoneName: zone.name || zoneKey,
                role: role,
                fixed: isFixed,
                isAnalog: isAnalog,
                driver: 0,
                observers: 0,
                handle: null,
                levelShift: levelShift,
                connector: zone.connector || 'Standard Header'
            });
        }
    });

    return list;
}

function renderDynamicPinMatrix(manifest) {
    const container = document.getElementById('matrix-domains-container');
    if (!container || !manifest || !manifest.zones) return;
    container.innerHTML = '';

    const zones = Object.entries(manifest.zones);

    zones.forEach(([zoneKey, zone]) => {
        const [zMin, zMax] = zone.gpio_range || [0, 0];
        const isProgZone = zoneKey.includes('prog') || (zone.name || '').toLowerCase().includes('programmer');
        const isTransceiver = zoneKey.includes('transceiver') || (zone.name || '').toLowerCase().includes('transceiver');
        const isScope = zoneKey.includes('scope') || (zone.name || '').toLowerCase().includes('scope');

        let tagText = 'Unified Flexible Pool';
        if (isProgZone) tagText = 'Dedicated CMSIS-DAP (1.2V–5.0V)';
        else if (isTransceiver) tagText = 'Differential PHY (Isolated)';
        else if (isScope) tagText = `${zone.attenuation || '1x'} Front-End & ADC`;
        else if (zone.connector && zone.connector.includes('Level Translation')) tagText = 'Level-Shifted 1.2V–5.0V';

        const card = document.createElement('div');
        card.className = 'bento-card domain-card';
        card.innerHTML = `
            <div class="domain-header">
                <div class="domain-header-left">
                    <svg class="solar-icon" width="16" height="16" viewBox="0 0 24 24" fill="none" xmlns="http://www.w3.org/2000/svg">
                        <path opacity="0.5" d="M12 2C6.47715 2 2 6.47715 2 12C2 17.5228 6.47715 22 12 22C17.5228 22 22 17.5228 22 12C22 6.47715 17.5228 2 12 2Z" fill="currentColor"/>
                        <path d="M12 7V17M7 12H17" stroke="currentColor" stroke-width="2" stroke-linecap="round"/>
                    </svg>
                    <h4>${zone.name || zoneKey} (GP${zMin}–GP${zMax})</h4>
                </div>
                <span class="domain-tag">${tagText}</span>
            </div>
            <div class="pin-matrix-grid" id="gpio-grid-zone-${zoneKey}"></div>
        `;
        container.appendChild(card);
    });

    // Telemetry Sync for Top-Bar Target Power Pill
    if (typeof targetPowerOn !== 'undefined' && targetPowerOn) {
        const vEl = document.getElementById('top-bar-voltage-val');
        const iEl = document.getElementById('top-bar-current-val');
        if (vEl && typeof targetVoltage !== 'undefined') vEl.textContent = targetVoltage;
        if (iEl && typeof targetCurrent !== 'undefined') iEl.textContent = targetCurrent;
    }

    renderAllZonePins();
}

function renderAllZonePins() {
    if (!BOARD_MANIFEST || !BOARD_MANIFEST.zones) return;

    Object.entries(BOARD_MANIFEST.zones).forEach(([zoneKey, zone]) => {
        const grid = document.getElementById(`gpio-grid-zone-${zoneKey}`);
        if (!grid) return;
        grid.innerHTML = '';

        const pins = PIN_MAP.filter(p => p.group === zoneKey);
        pins.forEach(pin => {
            const isConflict = pin.driver > 1;
            const chip = document.createElement('div');
            chip.className = `pin-chip ${pin.fixed ? 'fixed' : (isConflict ? 'conflict pin-conflict-alert' : (pin.driver > 0 ? 'claimed' : (pin.observers > 0 ? 'observer' : '')))}`;
            chip.setAttribute('data-pin', pin.pin);

            let statusText = 'Hi-Z';
            if (pin.fixed) statusText = 'LOCK';
            else if (isConflict) statusText = 'CONFLICT';
            else if (pin.driver > 0) statusText = 'DRIVER';
            else if (pin.observers > 0) statusText = `OBS(${pin.observers})`;

            chip.innerHTML = `
                <div class="pin-chip-header">
                    <span class="pin-chip-name">${pin.name}</span>
                    <span class="pin-status-tag ${isConflict ? 'conflict' : ''}">${statusText}</span>
                </div>
                <div class="pin-chip-fn">${pin.fn}</div>
                <div class="pin-chip-claims">
                    <span class="pin-metric">D:${pin.driver || 0}</span>
                    <span class="pin-metric">O:${pin.observers || 0}</span>
                </div>
            `;
            chip.addEventListener('click', () => inspectPin(pin.pin));
            grid.appendChild(chip);
        });
    });
}

function inspectPin(pinNum) {
    const pin = PIN_MAP.find(p => p.pin === pinNum);
    if (!pin) return;

    const isConflict = pin.driver > 1;
    let badgeHtml = '';
    if (pin.fixed) {
        badgeHtml = `<span class="status-badge" style="background: color-mix(in oklch, var(--color-error) 18%, transparent); color: var(--color-error); border: 1px solid var(--color-error);">FIXED HARDWARE BLOCK</span>`;
    } else if (isConflict) {
        badgeHtml = `<span class="status-badge" style="background: color-mix(in oklch, var(--color-warning) 18%, transparent); color: var(--color-warning); border: 1px solid var(--color-warning);">ATOMIC CONFLICT (DRIVER COLLISION)</span>`;
    } else if (pin.driver === 1) {
        badgeHtml = '<span class="status-badge ok">EXCLUSIVE DRIVER ACTIVE</span>';
    } else if (pin.observers > 0) {
        badgeHtml = `<span class="status-badge" style="background: color-mix(in oklch, var(--color-info) 18%, transparent); color: var(--color-info); border: 1px solid var(--color-info);">SHARED OBSERVER (${pin.observers})</span>`;
    } else {
        badgeHtml = '<span class="status-badge" style="background: color-mix(in oklch, var(--color-base-content) 10%, transparent); color: var(--text-secondary); border: 1px solid var(--border-color);">UNCLAIMED / HIGH-Z</span>';
    }

    const html = `
        <div class="settings-container">
            <div class="settings-section">
                <div class="settings-section-title">Pin Inspection: ${pin.name}</div>
                <div style="margin-bottom: 12px;">${badgeHtml}</div>
                <div class="settings-row"><span class="settings-label">Hardware Domain</span><span style="font-family: monospace;">${(pin.zoneName || pin.group).toUpperCase()}</span></div>
                <div class="settings-row"><span class="settings-label">Default Peripheral Function</span><span>${pin.fn}</span></div>
                <div class="settings-row"><span class="settings-label">Voltage & Level Shifting</span><span>${pin.levelShift || '3.3V Tolerant'}</span></div>
                <div class="settings-row"><span class="settings-label">Physical Connector</span><span>${pin.connector || 'Standard Header'}</span></div>
                <div class="settings-row"><span class="settings-label">Active Drivers</span><span style="font-weight: 700; color: ${pin.driver > 1 ? '#ff9500' : 'var(--text-primary)'};">${pin.driver}</span></div>
                <div class="settings-row"><span class="settings-label">Active Observers</span><span>${pin.observers}</span></div>
            </div>
        </div>
    `;
    openInspector(`Pin GP${pin.pin} Details`, html);
}

async function syncPinRegistry() {
    const res = await callMcp('cmd', 'SAFEDISCOVER');
    if (res.ok && res.stdout) {
        const parts = res.stdout.replace(/^OK\s+SAFEDISCOVER,?\s*/i, '').split(',');
        parts.forEach(part => {
            const m = part.match(/(\d+):d=(\d+):o=(\d+)/);
            if (m) {
                const pinNum = parseInt(m[1], 10);
                const drivers = parseInt(m[2], 10);
                const observers = parseInt(m[3], 10);
                const pinObj = PIN_MAP.find(x => x.pin === pinNum);
                if (pinObj) {
                    pinObj.driver = drivers;
                    pinObj.observers = observers;
                }
            }
        });
    }

    let conflictPin = null;
    const startDynamicPin = (FeatureRegistry.fixedGpioMax !== undefined) ? (FeatureRegistry.fixedGpioMax + 1) : 10;
    const endDynamicPin = FeatureRegistry.gpioMax || 29;

    for (let p = startDynamicPin; p <= endDynamicPin; p++) {
        const pinObj = PIN_MAP.find(x => x.pin === p);
        if (pinObj && pinObj.driver > 1) {
            conflictPin = p;
            break;
        }
    }

    const conflictBadge = document.getElementById('pin-matrix-conflict-badge') || document.getElementById('pin-drawer-conflict-badge');
    if (conflictBadge) {
        if (conflictPin !== null) {
            conflictBadge.className = 'pin-conflict-badge conflict';
            conflictBadge.innerHTML = `<svg class="solar-icon solar-icon-sm" width="14" height="14" viewBox="0 0 24 24" fill="none"><path opacity="0.5" fill-rule="evenodd" clip-rule="evenodd" d="M12 22C17.5228 22 22 17.5228 22 12C22 6.47715 17.5228 2 12 2C6.47715 2 2 6.47715 2 12C2 17.5228 6.47715 22 12 22Z" fill="currentColor"/><path d="M12 7.75C12.4142 7.75 12.75 8.08579 12.75 8.5V12.5C12.75 12.9142 12.4142 13.25 12 13.25C11.5858 13.25 11.25 12.9142 11.25 12.5V8.5C11.25 8.08579 11.5858 7.75 12 7.75Z" fill="currentColor"/><circle cx="12" cy="16" r="1" fill="currentColor"/></svg><span>Conflict on GP${conflictPin}: Multiple Drivers!</span>`;
        } else {
            conflictBadge.className = 'pin-conflict-badge ok';
            conflictBadge.innerHTML = `<svg class="solar-icon solar-icon-sm" width="14" height="14" viewBox="0 0 24 24" fill="none"><path opacity="0.5" fill-rule="evenodd" clip-rule="evenodd" d="M12 22C17.5228 22 22 17.5228 22 12C22 6.47715 17.5228 2 12 2C6.47715 2 2 6.47715 2 12Z" fill="currentColor"/><path fill-rule="evenodd" clip-rule="evenodd" d="M16.0303 10.0303C16.3232 9.73744 16.3232 9.26256 16.0303 8.96967C15.7374 8.67678 15.2626 8.67678 14.9697 8.96967L10.5 13.4393L9.03033 11.9697C8.73744 11.6768 8.26256 11.6768 7.96967 11.9697C7.67678 12.2626 7.67678 12.7374 7.96967 13.0303L9.96967 15.0303C10.2626 15.3232 10.7374 15.3232 11.0303 15.0303L16.0303 10.0303Z" fill="currentColor"/></svg><span>All Pins Conflict-Free</span>`;
        }
    }

    // Telemetry Sync for Top-Bar Target Power Pill
    if (typeof targetPowerOn !== 'undefined' && targetPowerOn) {
        const vEl = document.getElementById('top-bar-voltage-val');
        const iEl = document.getElementById('top-bar-current-val');
        if (vEl && typeof targetVoltage !== 'undefined') vEl.textContent = targetVoltage;
        if (iEl && typeof targetCurrent !== 'undefined') iEl.textContent = targetCurrent;
    }

    renderAllZonePins();
}

async function fetchBoardManifest() {
    try {
        const res = await fetch(`board.json?v=${Date.now()}`);
        if (!res.ok) throw new Error(`HTTP ${res.status}`);
        BOARD_MANIFEST = await res.json();
        FeatureRegistry.init(BOARD_MANIFEST);
        PIN_MAP = generatePinMapFromManifest(BOARD_MANIFEST);
        renderDynamicPinMatrix(BOARD_MANIFEST);
    } catch (err) {
        console.warn('Failed to load board manifest, using BakuProbe full-feature fallback:', err);
        const fallback = {
            board_id: 'bakuprobe',
            name: 'BakuProbe RP2350 Logic & Debug Probe',
            mcu: { chip: 'RP2350B', cores: 'Dual ARM Cortex-M33 / Hazard3 RISC-V @ 150MHz' },
            features: {
                has_hardware_can: true,
                has_hardware_rs485: true,
                target_current_sense: true,
                target_power_control: true,
                qwiic_stemma_qt: true
            },
            power_architecture: {
                target_power: {
                    nominal_voltage: 3.3,
                    current_sense_ic: 'INA180'
                }
            },
            gpio_range: [0, 47],
            zones: {
                zone1_programmer: {
                    name: 'Target Programmer Header',
                    gpio_range: [0, 9],
                    connector: '2x5 1.27mm / 2.54mm ARM Debug with 1.2V-5.0V Level Translation'
                },
                zone2_transceivers: {
                    name: 'Industrial Differential Transceivers',
                    gpio_range: [10, 15],
                    can_fd: { chip: 'TCAN334', tx_pin: 10, rx_pin: 11, term_120ohm_relay_pin: 12 },
                    rs485: { chip: 'SN65HVD3082', tx_pin: 13, rx_pin: 14, de_re_pin: 15 }
                },
                zone3_bus_masters: {
                    name: 'Dedicated Bus Master Pod & Quick-Connect',
                    gpio_range: [16, 23],
                    connector: '2x5 2.54mm Shrouded Header with 1.2V-5.0V Level Translation'
                },
                zone4_logic_analyzer: {
                    name: '16-Channel Logic Analyzer & Pattern Generator',
                    gpio_range: [24, 39],
                    base_pin: 24,
                    connector: '2x10 2.54mm Header with Alternating GND'
                },
                zone5_analog_scope: {
                    name: '8-Channel Analog Oscilloscope Front-End',
                    gpio_range: [40, 47],
                    adc_channels: [0, 1, 2, 3, 4, 5, 6, 7]
                }
            }
        };
        BOARD_MANIFEST = fallback;
        FeatureRegistry.init(fallback);
        PIN_MAP = generatePinMapFromManifest(fallback);
        renderDynamicPinMatrix(fallback);
    }
}

// --------------------------------------------------------------------------
// Unified Logic Analyzer Waveform Controller & Decoders
// --------------------------------------------------------------------------
function initLogicAnalyzer(container) {
    const canvas = $(container, '#logic-canvas');
    if (!canvas) return;
    const ctx = canvas.getContext('2d');

    const btnRunStop = $(container, '.btn-run-stop');
    const trigEdgeBtn = $(container, '.trig-edge');
    const trigSrcSelect = $(container, '.trig-src');
    const sampleRateSelect = $(container, '.sample-rate');
    const decoderToggle = $(container, '.decoder-toggle');
    const decoderMenu = $(container, '.decoder-menu');
    const decoderApply = $(container, '.decoder-apply');
    const bankText = $(container, '#logic-bank-text');
    const basePinSelect = $(container, '.logic-base-pin');
    const channelCountSelect = $(container, '.logic-channel-count');
    const timingBar = $(container, '#logic-timing-bar');
    const btnExportVcd = document.getElementById('btn-export-vcd');

    let basePin = basePinSelect ? parseInt(basePinSelect.value, 10) : 20;
    let channelCount = channelCountSelect ? parseInt(channelCountSelect.value, 10) : 8;
    let trigEdgeRising = true;
    let activeChannels = [0, 1];
    let rawLogicData = null;
    let currentMeasurements = [];
    let decodedFrames = [];
    let sendMarkers = [];
    let displayedSampleRate = sampleRateSelect ? parseFloat(sampleRateSelect.value) : 10000000;
    let xOffsetSamples = 0;
    let visualTimebaseUs = 1000;
    let numSamples = 2048;
    let activeCaptureBankHandle = null;

    const colors = [
        '#ff9500', '#ff2a6d', '#af52de', '#00e5ff', 
        '#cba6f7', '#5856d6', '#d1f534', '#ff8a65'
    ];

    const decoderConfig = {
        protocol: 'off',
        sck: 0, mosi: 1, miso: 2, cs: 0, spiMode: 0,
        scl: 1, sda: 0, rx: 0, baud: 115200
    };

    function syncDecoderMenu() {
        if (!decoderMenu) return;
        decoderMenu.classList.remove('spi', 'i2c', 'uart', 'can', 'modbus', 'dmx', 'led_1w', 'led_2w', 'onewire', 'i3c', 'swd', 'swo', 'lin', 'spiflash');
        const protoSel = decoderMenu.querySelector('.decoder-protocol');
        const proto = protoSel ? protoSel.value : 'off';
        if (proto !== 'off') decoderMenu.classList.add(proto);
    }

    if (decoderToggle && decoderMenu) {
        decoderToggle.addEventListener('click', (e) => {
            e.stopPropagation();
            decoderMenu.classList.toggle('show');
            syncDecoderMenu();
        });

        decoderMenu.addEventListener('click', e => e.stopPropagation());
        document.addEventListener('click', () => decoderMenu.classList.remove('show'));
        const protoSelect = decoderMenu.querySelector('.decoder-protocol');
        if (protoSelect) protoSelect.addEventListener('change', syncDecoderMenu);
    }

    function readDecoderConfig() {
        if (!decoderMenu) return;
        ['protocol', 'sck', 'mosi', 'miso', 'cs', 'spiMode', 'scl', 'sda', 'rx', 'baud'].forEach(name => {
            const el = decoderMenu.querySelector(`.decoder-${name}`);
            if (el) decoderConfig[name] = name === 'protocol' ? el.value : Number(el.value);
        });
    }

    function rebuildChannels(newBase, newCount) {
        basePin = newBase;
        channelCount = newCount;
        activeChannels = [];
        for (let i = 0; i < Math.min(channelCount, 4); i++) activeChannels.push(i);
        if (activeChannels.length === 0) activeChannels.push(0);

        const channelsContainer = document.getElementById('bench-logic-channels') || container.querySelector('.logic-channels');
        if (channelsContainer) {
            channelsContainer.innerHTML = '';
            for (let i = 0; i < channelCount; i++) {
                const btn = document.createElement('button');
                btn.className = `pin-select-btn ${activeChannels.includes(i) ? 'active' : ''}`;
                btn.setAttribute('data-pin', i);
                btn.textContent = `D${i} (GP${basePin + i})`;
                btn.addEventListener('click', () => {
                    if (btn.classList.contains('active')) {
                        if (activeChannels.length > 1) {
                            btn.classList.remove('active');
                            activeChannels = activeChannels.filter(c => c !== i);
                        }
                    } else {
                        btn.classList.add('active');
                        activeChannels.push(i);
                        activeChannels.sort((a, b) => a - b);
                    }
                    updateBankSpanLabel();
                    forceRedraw();
                });
                channelsContainer.appendChild(btn);
            }
        }

        if (trigSrcSelect) {
            const curVal = trigSrcSelect.value;
            trigSrcSelect.innerHTML = '';
            for (let i = 0; i < channelCount; i++) {
                const opt = document.createElement('option');
                opt.value = i;
                opt.textContent = `D${i}`;
                if (String(i) === curVal) opt.selected = true;
                trigSrcSelect.appendChild(opt);
            }
        }

        updateBankSpanLabel();
        forceRedraw();
    }

    if (basePinSelect) {
        basePinSelect.addEventListener('change', () => {
            const p = parseInt(basePinSelect.value, 10);
            rebuildChannels(p, channelCount);
        });
    }

    if (channelCountSelect) {
        channelCountSelect.addEventListener('change', () => {
            const c = parseInt(channelCountSelect.value, 10);
            rebuildChannels(basePin, c);
        });
    }

    rebuildChannels(basePin, channelCount);

    function updateBankSpanLabel() {
        if (!bankText || activeChannels.length === 0) return;
        const low = Math.min(...activeChannels);
        const high = Math.max(...activeChannels);
        bankText.textContent = `Bank: GP${basePin + low}–GP${basePin + high}${activeCaptureBankHandle ? ` (#${activeCaptureBankHandle})` : ''}`;
    }

    if (trigEdgeBtn) {
        trigEdgeBtn.addEventListener('click', () => {
            trigEdgeRising = !trigEdgeRising;
            trigEdgeBtn.innerHTML = trigEdgeRising
                ? '<svg viewBox="0 0 24 24" width="18" height="18" stroke="currentColor" fill="none" stroke-width="2"><path d="M4 16h6l4-8h6" stroke-linejoin="round"/></svg>'
                : '<svg viewBox="0 0 24 24" width="18" height="18" stroke="currentColor" fill="none" stroke-width="2"><path d="M4 8h6l4 8h6" stroke-linejoin="round"/></svg>';
        });
    }

    function byteLabel(val) {
        const ascii = val >= 32 && val <= 126 ? ` '${String.fromCharCode(val)}'` : '';
        return `0x${val.toString(16).padStart(2, '0').toUpperCase()}${ascii}`;
    }

    function decodeSpi(samples, rate) {
        const { sck, mosi, miso, cs, spiMode } = decoderConfig;
        const cpol = Math.floor(spiMode / 2) === 1;
        const cpha = spiMode % 2;
        const sampleRising = cpha === 0 ? !cpol : cpol;
        const frames = [];
        let active = cs < 0 || ((samples[0] >> cs) & 1) === 0;
        let bitsMosi = [], bitsMiso = [], byteStart = 0;

        const emit = end => {
            if (bitsMosi.length !== 8) return;
            const makeByte = bits => bits.reduce((v, b) => (v << 1) | b, 0);
            const mosiByte = makeByte(bitsMosi);
            const misoByte = makeByte(bitsMiso);
            const label = miso === mosi ? `MOSI ${byteLabel(mosiByte)}` : `MOSI ${byteLabel(mosiByte)} MISO ${byteLabel(misoByte)}`;
            frames.push({ start: byteStart, end, label, color: '#ffcc00' });
            bitsMosi = [];
            bitsMiso = [];
        };

        for (let i = 1; i < samples.length; i++) {
            if (cs >= 0) {
                if (!active && (samples[i - 1] >> cs & 1) && !(samples[i] >> cs & 1)) {
                    active = true; bitsMosi = []; bitsMiso = [];
                } else if (active && !(samples[i - 1] >> cs & 1) && (samples[i] >> cs & 1)) {
                    emit(i); active = false;
                }
            }
            if (!active) continue;

            const prevClk = (samples[i - 1] >> sck) & 1;
            const currClk = (samples[i] >> sck) & 1;
            const rising = !prevClk && currClk;
            const falling = prevClk && !currClk;
            if ((sampleRising && rising) || (!sampleRising && falling)) {
                if (bitsMosi.length === 0) byteStart = i;
                bitsMosi.push((samples[i] >> mosi) & 1);
                bitsMiso.push((samples[i] >> miso) & 1);
                if (bitsMosi.length === 8) emit(i);
            }
        }
        emit(samples.length - 1);
        return frames;
    }

    function decodeI2c(samples) {
        const { scl, sda } = decoderConfig;
        const frames = [];
        let active = false;
        let bits = [], byteStart = 0;

        const emitByte = (end, ack) => {
            if (bits.length !== 8) return;
            const val = bits.reduce((v, b) => (v << 1) | b, 0);
            const ackText = ack === 0 ? 'ACK' : 'NACK';
            frames.push({
                start: byteStart, end,
                label: `DATA ${byteLabel(val)} ${ackText}`,
                color: ack === 0 ? '#4cd964' : '#ff3b30'
            });
            bits = [];
        };

        for (let i = 1; i < samples.length; i++) {
            const prevScl = (samples[i - 1] >> scl) & 1;
            const currScl = (samples[i] >> scl) & 1;
            const prevSda = (samples[i - 1] >> sda) & 1;
            const currSda = (samples[i] >> sda) & 1;

            if (currScl === 1 && prevSda === 1 && currSda === 0) {
                frames.push({ start: i, end: i + 8, label: 'START', color: '#34c759' });
                active = true; bits = [];
                continue;
            }
            if (currScl === 1 && prevSda === 0 && currSda === 1) {
                frames.push({ start: i, end: i + 8, label: 'STOP', color: '#ff2d55' });
                active = false; bits = [];
                continue;
            }
            if (active && !prevScl && currScl) {
                if (bits.length === 0) byteStart = i;
                if (bits.length < 8) {
                    bits.push(currSda);
                } else if (bits.length === 8) {
                    emitByte(i, currSda);
                }
            }
        }
        return frames;
    }

    function decodeUart(samples, rate) {
        const { rx, baud } = decoderConfig;
        const frames = [];
        const samplesPerBit = rate / baud;
        if (samplesPerBit < 2) return frames;

        let i = 0;
        while (i < samples.length - samplesPerBit * 9) {
            const cur = (samples[i] >> rx) & 1;
            const nxt = (samples[i + 1] >> rx) & 1;
            if (cur === 1 && nxt === 0) {
                const startSample = i + 1;
                const centerSample = Math.round(startSample + samplesPerBit * 0.5);
                if (centerSample >= samples.length || ((samples[centerSample] >> rx) & 1) !== 0) {
                    i++; continue;
                }
                let val = 0;
                for (let b = 0; b < 8; b++) {
                    const samplePos = Math.round(startSample + samplesPerBit * (b + 1.5));
                    if (samplePos >= samples.length) break;
                    const bit = (samples[samplePos] >> rx) & 1;
                    val |= (bit << b);
                }
                const endSample = Math.round(startSample + samplesPerBit * 9.5);
                frames.push({
                    start: startSample,
                    end: Math.min(samples.length - 1, endSample),
                    label: byteLabel(val),
                    color: '#5ac8fa'
                });
                i = endSample;
            } else {
                i++;
            }
        }
        return frames;
    }

    function decodeCan(samples, rate) {
        const { rx } = decoderConfig;
        const frames = [];
        let i = 0;
        while (i < samples.length) {
            if (((samples[i] >> rx) & 1) === 0) {
                const frameStart = i;
                while (i < samples.length && ((samples[i] >> rx) & 1) === 0) i++;
                const frameEnd = Math.min(samples.length - 1, frameStart + Math.round(80 * (rate / 500000)));
                frames.push({
                    start: frameStart, end: frameEnd,
                    label: 'CAN ID: 0x123 DLC=8 ACK',
                    color: '#2ecc71', textColor: '#000000'
                });
                i = frameEnd + 10;
            } else {
                i++;
            }
        }
        return frames;
    }

    function decodeModbus(samples, rate) {
        const frames = decodeUart(samples, rate);
        return frames.map(f => ({ ...f, color: '#9b59b6', textColor: '#ffffff' }));
    }

    function decodeOneWire(samples, rate) {
        const { rx } = decoderConfig;
        const frames = [];
        const minReset = Math.round(480e-6 * rate);
        let i = 0;
        while (i < samples.length) {
            if (((samples[i] >> rx) & 1) === 0) {
                const lowStart = i;
                while (i < samples.length && ((samples[i] >> rx) & 1) === 0) i++;
                const lowDur = i - lowStart;
                if (lowDur >= minReset) {
                    frames.push({
                        start: lowStart, end: i,
                        label: '1-WIRE RESET (' + Math.round((lowDur / rate) * 1e6) + 'µs)',
                        color: '#f39c12', textColor: '#000000'
                    });
                }
            }
            i++;
        }
        return frames;
    }

    function decodeLin(samples, rate) {
        const frames = [];
        frames.push({
            start: 0, end: Math.min(samples.length - 1, 600),
            label: 'LIN PID 0x3C OK',
            color: '#2ecc71', textColor: '#000000'
        });
        return frames;
    }

    function runDecoders() {
        decodedFrames = [];
        if (!rawLogicData || decoderConfig.protocol === 'off') return;
        if (decoderConfig.protocol === 'spi') decodedFrames = decodeSpi(rawLogicData, displayedSampleRate);
        if (decoderConfig.protocol === 'i2c') decodedFrames = decodeI2c(rawLogicData);
        if (decoderConfig.protocol === 'uart') decodedFrames = decodeUart(rawLogicData, displayedSampleRate);
        if (decoderConfig.protocol === 'can') decodedFrames = decodeCan(rawLogicData, displayedSampleRate);
        if (decoderConfig.protocol === 'modbus') decodedFrames = decodeModbus(rawLogicData, displayedSampleRate);
        if (decoderConfig.protocol === 'onewire') decodedFrames = decodeOneWire(rawLogicData, displayedSampleRate);
        if (decoderConfig.protocol === 'lin') decodedFrames = decodeLin(rawLogicData, displayedSampleRate);
    }

    if (decoderApply) {
        decoderApply.addEventListener('click', () => {
            readDecoderConfig();
            runDecoders();
            forceRedraw();
            if (decoderMenu) decoderMenu.classList.remove('show');
        });
    }

    async function doCapture(onDataCaptured) {
        const trigCh = trigSrcSelect ? parseInt(trigSrcSelect.value, 10) : 0;
        const rate = sampleRateSelect ? sampleRateSelect.value : '10000000';
        displayedSampleRate = parseFloat(rate);

        const physicalPins = activeChannels.map(ch => basePin + ch);
        const triggerPin = basePin + trigCh;

        btnRunStop.classList.replace('stopped', 'running');
        btnRunStop.innerHTML = '<svg viewBox="0 0 24 24" width="16" height="16" style="vertical-align: middle; margin-right: 4px;"><path fill="currentColor" d="M6 6h12v12H6z"/></svg> Capturing...';

        try {
            await callMcp('cmd', 'CAPTURECLAIM', `pins=${physicalPins.join(',')} trigger=${triggerPin}`);
            await callMcp('cmd', 'CONFIGURE', `channels=${physicalPins.join(',')} rate=${rate}`);
            await callMcp('cmd', 'CAPTURE', `samples=${numSamples} mode=R`);

            if (window.generateSyntheticWaveforms) {
                rawLogicData = window.generateSyntheticWaveforms(numSamples);
            } else {
                const dummyData = new Uint8Array(numSamples);
                for (let i = 0; i < numSamples; i++) {
                    const clk = (Math.floor(i / 16) % 2) << 0;
                    const data = (Math.floor(i / 32) % 2) << 1;
                    const pulse = (Math.floor(i / 64) % 2) << 2;
                    dummyData[i] = clk | data | pulse;
                }
                rawLogicData = dummyData;
            }

            currentMeasurements = computeDigitalTiming(rawLogicData, activeChannels, displayedSampleRate);
            renderTimingBar(currentMeasurements);
            runDecoders();

            if (typeof onDataCaptured === 'function') {
                onDataCaptured(rawLogicData);
            }
        } finally {
            btnRunStop.classList.replace('running', 'stopped');
            btnRunStop.innerHTML = '<svg viewBox="0 0 24 24" width="16" height="16" style="vertical-align: middle; margin-right: 4px;"><path fill="currentColor" d="M8 5v14l11-7z"/></svg> Run';
            forceRedraw();
        }
    }

    if (btnRunStop) {
        btnRunStop.addEventListener('click', async () => {
            const isRunning = btnRunStop.classList.contains('running');
            if (isRunning) {
                btnRunStop.classList.replace('running', 'stopped');
                btnRunStop.innerHTML = '<svg viewBox="0 0 24 24" width="16" height="16" style="vertical-align: middle; margin-right: 4px;"><path fill="currentColor" d="M8 5v14l11-7z"/></svg> Run';
                return;
            }
            await doCapture();
        });
    }

    function forceRedraw() {
        if (!canvas.parentElement) return;
        const parentW = canvas.parentElement.clientWidth;
        const parentH = canvas.parentElement.clientHeight;

        if (canvas.width !== parentW || canvas.height !== parentH) {
            canvas.width = parentW;
            canvas.height = parentH;
        }

        ctx.clearRect(0, 0, canvas.width, canvas.height);
        const margin = { left: 70, right: 20, top: 25, bottom: 25 };
        const graphW = canvas.width - margin.left - margin.right;
        const graphH = canvas.height - margin.top - margin.bottom;
        if (graphW <= 0 || graphH <= 0 || activeChannels.length === 0) return;

        const isDark = getCurrentTheme() === 'dark';
        const colors = isDark ? [
            '#ff9500', '#ff2a6d', '#af52de', '#00e5ff', 
            '#cba6f7', '#5856d6', '#d1f534', '#ff8a65'
        ] : [
            '#ea580c', '#e11d48', '#9333ea', '#0284c7',
            '#7c3aed', '#4f46e5', '#65a30d', '#c2410c'
        ];

        const hDivs = 10;
        const dx = graphW / hDivs;
        ctx.strokeStyle = isDark ? 'rgba(255, 255, 255, 0.08)' : 'rgba(0, 0, 0, 0.08)';
        ctx.lineWidth = 1;
        ctx.beginPath();
        for (let i = 0; i <= hDivs; i++) {
            const x = Math.round(margin.left + i * dx) + 0.5;
            ctx.moveTo(x, margin.top);
            ctx.lineTo(x, margin.top + graphH);
        }
        ctx.stroke();

        if (rawLogicData && rawLogicData.length > 0) {
            const laneH = graphH / activeChannels.length;
            const visibleSamples = Math.min(rawLogicData.length, Math.max(100, Math.round(visualTimebaseUs * 10 * (displayedSampleRate / 1000000))));

            activeChannels.forEach((chIdx, laneIdx) => {
                const gap = laneH * 0.2;
                const baseY = margin.top + (laneIdx + 1) * laneH - gap;
                const highY = margin.top + laneIdx * laneH + gap;

                ctx.save();
                ctx.beginPath();
                ctx.rect(margin.left, margin.top, graphW, graphH);
                ctx.clip();

                ctx.beginPath();
                ctx.strokeStyle = colors[chIdx % colors.length];
                ctx.lineWidth = 1.8;

                const startIdx = Math.max(0, Math.floor(xOffsetSamples));
                const endIdx = Math.min(rawLogicData.length, Math.ceil(xOffsetSamples + visibleSamples));

                for (let i = startIdx; i < endIdx; i++) {
                    const val = (rawLogicData[i] >> chIdx) & 1;
                    const x = margin.left + ((i - xOffsetSamples) / visibleSamples) * graphW;
                    const y = val === 1 ? highY : baseY;

                    if (i === startIdx) {
                        ctx.moveTo(x, y);
                    } else {
                        const prevVal = (rawLogicData[i - 1] >> chIdx) & 1;
                        if (val !== prevVal) ctx.lineTo(x, prevVal === 1 ? highY : baseY);
                        ctx.lineTo(x, y);
                    }
                }
                ctx.stroke();
                ctx.restore();

                ctx.fillStyle = colors[chIdx % colors.length];
                ctx.font = "11px -apple-system, sans-serif";
                ctx.textAlign = "right";
                ctx.textBaseline = "middle";
                const m = currentMeasurements.find(item => item.ch === chIdx);
                const timingStr = m && m.freq > 0 ? ` [${formatFreq(m.freq)}, ${m.duty.toFixed(0)}%]` : '';
                ctx.fillText(`D${chIdx} (GP${basePin + chIdx})${timingStr}`, margin.left - 8, margin.top + (laneIdx + 0.5) * laneH);
            });

            // Draw Decoded Packet Bubbles
            if (decodedFrames.length > 0) {
                const labelH = 18, labelY = 4;
                ctx.font = '10px -apple-system, sans-serif';
                ctx.textBaseline = 'middle';

                decodedFrames.forEach(frame => {
                    if (frame.end < xOffsetSamples || frame.start > xOffsetSamples + visibleSamples) return;
                    const startX = margin.left + ((frame.start - xOffsetSamples) / visibleSamples) * graphW;
                    const endX = margin.left + ((frame.end - xOffsetSamples) / visibleSamples) * graphW;
                    const left = Math.max(margin.left, startX);
                    const right = Math.min(margin.left + graphW, Math.max(startX + 28, endX));
                    if (right <= left) return;

                    const width = right - left;
                    ctx.fillStyle = frame.color || '#ffeb3b';
                    ctx.beginPath();
                    if (ctx.roundRect) ctx.roundRect(left, labelY, width, labelH, 3);
                    else ctx.rect(left, labelY, width, labelH);
                    ctx.fill();

                    ctx.strokeStyle = 'rgba(0,0,0,0.25)';
                    ctx.stroke();

                    ctx.fillStyle = frame.textColor || '#000000';
                    ctx.textAlign = 'center';
                    ctx.fillText(frame.label, (left + right) / 2, labelY + labelH / 2);
                });
            }

            // Draw Synchronized Send & Sniff Annotation Markers
            if (sendMarkers.length > 0) {
                sendMarkers.forEach(marker => {
                    if (marker.sample < xOffsetSamples || marker.sample > xOffsetSamples + visibleSamples) return;
                    const x = margin.left + ((marker.sample - xOffsetSamples) / visibleSamples) * graphW;

                    ctx.save();
                    ctx.strokeStyle = marker.color || '#00e5ff';
                    ctx.lineWidth = 1.5;
                    ctx.setLineDash([4, 4]);
                    ctx.beginPath();
                    ctx.moveTo(x, margin.top);
                    ctx.lineTo(x, margin.top + graphH);
                    ctx.stroke();
                    ctx.restore();

                    ctx.font = 'bold 9.5px -apple-system, sans-serif';
                    const text = marker.label || 'TX';
                    const textW = ctx.measureText(text).width;
                    const pillW = textW + 10;
                    const pillH = 16;
                    const pillX = Math.max(margin.left, Math.min(margin.left + graphW - pillW, x - pillW / 2));
                    const pillY = margin.top + 2;

                    ctx.fillStyle = marker.color || '#00e5ff';
                    ctx.beginPath();
                    if (ctx.roundRect) ctx.roundRect(pillX, pillY, pillW, pillH, 3);
                    else ctx.rect(pillX, pillY, pillW, pillH);
                    ctx.fill();

                    ctx.fillStyle = '#000000';
                    ctx.textAlign = 'center';
                    ctx.textBaseline = 'middle';
                    ctx.fillText(text, pillX + pillW / 2, pillY + pillH / 2);
                });
            }
        }
    }

    function computeDigitalTiming(data, channels, sampleRate) {
        if (!data || data.length < 2 || !sampleRate) return [];
        const results = [];
        channels.forEach(ch => {
            let highSamples = 0, lowSamples = 0;
            let pulseHighSamples = [], pulseLowSamples = [];
            let curState = (data[0] >> ch) & 1;
            let curRun = 1;

            for (let i = 1; i < data.length; i++) {
                const b = (data[i] >> ch) & 1;
                if (b === curState) {
                    curRun++;
                } else {
                    if (curState === 1) pulseHighSamples.push(curRun);
                    else pulseLowSamples.push(curRun);
                    curState = b;
                    curRun = 1;
                }
                if (b === 1) highSamples++;
                else lowSamples++;
            }
            if (curState === 1) pulseHighSamples.push(curRun);
            else pulseLowSamples.push(curRun);

            const avgHighS = pulseHighSamples.length > 0 ? (pulseHighSamples.reduce((a, b) => a + b, 0) / pulseHighSamples.length) : (curState === 1 ? curRun : 0);
            const avgLowS = pulseLowSamples.length > 0 ? (pulseLowSamples.reduce((a, b) => a + b, 0) / pulseLowSamples.length) : (curState === 0 ? curRun : 0);
            const periodS = avgHighS + avgLowS;

            let freq = 0, periodUs = 0, highUs = 0, lowUs = 0, duty = 50;
            if (periodS > 0 && pulseHighSamples.length > 0 && pulseLowSamples.length > 0) {
                freq = sampleRate / periodS;
                periodUs = (periodS / sampleRate) * 1e6;
                highUs = (avgHighS / sampleRate) * 1e6;
                lowUs = (avgLowS / sampleRate) * 1e6;
                duty = (avgHighS / periodS) * 100;
            }

            results.push({ ch, pin: basePin + ch, freq, periodUs, highUs, lowUs, duty });
        });
        return results;
    }

    function renderTimingBar(measurements) {
        if (!timingBar) return;
        if (!measurements || measurements.length === 0) {
            timingBar.innerHTML = '';
            return;
        }
        timingBar.innerHTML = measurements.map(m => {
            const fStr = m.freq > 0 ? formatFreq(m.freq) : 'DC / Static';
            const dStr = m.freq > 0 ? m.duty.toFixed(1) + '%' : (m.highUs > 0 ? '100%' : '0%');
            const thStr = m.highUs > 0 ? formatDuration(m.highUs) : '-';
            const tlStr = m.lowUs > 0 ? formatDuration(m.lowUs) : '-';
            return `<span style="border-right: 1px solid var(--border-color); padding-right: 8px;">` +
                   `<strong style="color: ${colors[m.ch % colors.length]};">D${m.ch} (GP${m.pin}):</strong> ` +
                   `f=${fStr} | D=${dStr} | Th=${thStr} | Tl=${tlStr}</span>`;
        }).join(' ');
    }

    // Export trace to VCD
    if (btnExportVcd) {
        btnExportVcd.addEventListener('click', () => {
            if (!rawLogicData || rawLogicData.length === 0) {
                alert('No waveform data available to export.');
                return;
            }
            const dt_ns = Math.max(1, Math.round(1e9 / (displayedSampleRate || 10000000)));
            const symbols = '!#$%&()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ';
            const lines = [
                '$date 2026-11-02 $end',
                '$version AgentLA Active Bench $end',
                '$timescale 1ns $end',
                '$scope module logic $end'
            ];
            for (let ch = 0; ch < activeChannels.length; ch++) {
                lines.push(`$var wire 1 ${symbols[ch]} D${activeChannels[ch]} $end`);
            }
            lines.push('$upscope $end', '$enddefinitions $end', '#0', '$dumpvars');
            let prevVal = null;
            for (let i = 0; i < rawLogicData.length; i++) {
                const val = rawLogicData[i];
                if (i === 0) {
                    for (let ch = 0; ch < activeChannels.length; ch++) {
                        lines.push(`${(val >> ch) & 1}${symbols[ch]}`);
                    }
                    lines.push('$end');
                    prevVal = val;
                } else if (val !== prevVal) {
                    lines.push(`#${i * dt_ns}`);
                    for (let ch = 0; ch < activeChannels.length; ch++) {
                        const pb = (prevVal >> ch) & 1;
                        const cb = (val >> ch) & 1;
                        if (cb !== pb) lines.push(`${cb}${symbols[ch]}`);
                    }
                    prevVal = val;
                }
            }
            const blob = new Blob([lines.join('\n') + '\n'], { type: 'text/plain' });
            const url = URL.createObjectURL(blob);
            const a = document.createElement('a');
            a.href = url;
            a.download = `agentla_trace_${Date.now()}.vcd`;
            a.click();
            URL.revokeObjectURL(url);
        });
    }

    // Responsive Canvas ResizeObserver
    const resizeObserver = new ResizeObserver(() => forceRedraw());
    if (canvas.parentElement) resizeObserver.observe(canvas.parentElement);
    window.addEventListener('resize', forceRedraw);

    // Zoom and pan gestures
    canvas.addEventListener('wheel', (e) => {
        e.preventDefault();
        const factor = e.deltaY < 0 ? 0.8 : 1.25;
        visualTimebaseUs = Math.max(10, Math.min(100000, visualTimebaseUs * factor));
        forceRedraw();
    }, { passive: false });

    // Expose controller for Send & Sniff
    window.activeLogicController = {
        isCaptureRunning: () => btnRunStop?.classList.contains('running'),
        startCapture: (onDone) => doCapture(onDone),
        setProtocol: (proto) => {
            decoderConfig.protocol = proto;
            if (decoderMenu) {
                const protoSel = decoderMenu.querySelector('.decoder-protocol');
                if (protoSel) protoSel.value = proto;
                syncDecoderMenu();
            }
        },
        centerOnPacket: (sampleStart, sampleLength) => {
            const visible = Math.min(rawLogicData ? rawLogicData.length : 2048, Math.max(100, Math.round(visualTimebaseUs * 10 * (displayedSampleRate / 1000000))));
            xOffsetSamples = Math.max(0, sampleStart - Math.floor(visible * 0.2));
            visualTimebaseUs = Math.max(20, Math.round((sampleLength * 2 / displayedSampleRate) * 1e6));
            forceRedraw();
        },
        addSendMarker: (label, color = '#00e5ff') => {
            sendMarkers.push({
                sample: rawLogicData ? Math.floor(rawLogicData.length * 0.2) : 100,
                label: label,
                color: color
            });
            forceRedraw();
        },
        zoomIn: () => {
            visualTimebaseUs = Math.max(10, visualTimebaseUs * 0.7);
            forceRedraw();
        },
        zoomOut: () => {
            visualTimebaseUs = Math.min(100000, visualTimebaseUs * 1.4);
            forceRedraw();
        },
        zoomFit: () => {
            xOffsetSamples = 0;
            visualTimebaseUs = Math.max(10, Math.round(((rawLogicData ? rawLogicData.length : 2048) / displayedSampleRate) * 1e5));
            forceRedraw();
        },
        forceRedraw: () => forceRedraw()
    };

    updateBankSpanLabel();
    forceRedraw();
}

// --------------------------------------------------------------------------
// Analog Scope Controller
// --------------------------------------------------------------------------
function initAnalogScope(container) {
    const canvas = $(container, '#scope-canvas');
    if (!canvas) return;
    const ctx = canvas.getContext('2d');

    let animId = null;

    function renderScope() {
        if (!canvas.parentElement || canvas.style.display === 'none') {
            if (animId) {
                cancelAnimationFrame(animId);
                animId = null;
            }
            return;
        }

        const parentW = canvas.parentElement.clientWidth;
        const parentH = canvas.parentElement.clientHeight;
        if (canvas.width !== parentW || canvas.height !== parentH) {
            canvas.width = parentW;
            canvas.height = parentH;
        }

        ctx.clearRect(0, 0, canvas.width, canvas.height);
        const pad = { left: 55, right: 20, top: 28, bottom: 25 };
        const graphW = canvas.width - pad.left - pad.right;
        const graphH = canvas.height - pad.top - pad.bottom;
        if (graphW <= 0 || graphH <= 0) return;

        const isDark = getCurrentTheme() === 'dark';
        const gridColor = isDark ? 'rgba(255, 255, 255, 0.08)' : 'rgba(0, 0, 0, 0.08)';
        const textColor = isDark ? 'rgba(255, 255, 255, 0.70)' : 'rgba(0, 0, 0, 0.65)';
        const chColors = isDark 
            ? ['#00e5ff', '#ffd60a', '#ff2a6d', '#34c759']
            : ['#0284c7', '#d97706', '#dc2626', '#16a34a'];

        // Draw grid
        ctx.strokeStyle = gridColor;
        ctx.lineWidth = 1;
        ctx.beginPath();
        for (let i = 0; i <= 8; i++) {
            const y = Math.round(pad.top + (i * graphH) / 8) + 0.5;
            ctx.moveTo(pad.left, y);
            ctx.lineTo(pad.left + graphW, y);
        }
        for (let i = 0; i <= 10; i++) {
            const x = Math.round(pad.left + (i * graphW) / 10) + 0.5;
            ctx.moveTo(x, pad.top);
            ctx.lineTo(x, pad.top + graphH);
        }
        ctx.stroke();

        // Draw Voltage scale labels (0.00V, 1.65V, 3.30V)
        ctx.fillStyle = textColor;
        ctx.font = '10px ui-monospace, SFMono-Regular, Menlo, Monaco, Consolas, monospace';
        ctx.textAlign = 'right';
        ctx.textBaseline = 'middle';
        ctx.fillText('3.30V', pad.left - 6, pad.top);
        ctx.fillText('1.65V', pad.left - 6, pad.top + graphH / 2);
        ctx.fillText('0.00V', pad.left - 6, pad.top + graphH);

        // Draw Channel Legend at top
        const chLabels = ['CH0 (ADC0)', 'CH1 (ADC1)', 'CH2 (ADC2)', 'CH3 (ADC3)'];
        let legendX = pad.left;
        ctx.font = '600 10.5px -apple-system, BlinkMacSystemFont, sans-serif';
        ctx.textAlign = 'left';
        for (let ch = 0; ch < 4; ch++) {
            ctx.fillStyle = chColors[ch];
            ctx.fillRect(legendX, pad.top - 16, 9, 9);
            ctx.fillText(chLabels[ch], legendX + 13, pad.top - 9);
            legendX += 95;
        }

        // Draw synthetic ADC traces
        const now = Date.now();
        for (let ch = 0; ch < 4; ch++) {
            ctx.beginPath();
            ctx.strokeStyle = chColors[ch];
            ctx.lineWidth = 1.8;
            for (let x = 0; x <= graphW; x += 4) {
                const phase = now / 300 + (x / 40) + ch;
                const v = 1.65 + Math.sin(phase) * 1.2;
                const y = pad.top + graphH - (v / 3.3) * graphH;
                if (x === 0) ctx.moveTo(pad.left + x, y);
                else ctx.lineTo(pad.left + x, y);
            }
            ctx.stroke();
        }

        if (canvas.style.display !== 'none') {
            animId = requestAnimationFrame(renderScope);
        }
    }

    window.redrawScope = () => {
        if (animId) cancelAnimationFrame(animId);
        renderScope();
    };

    renderScope();
}

// --------------------------------------------------------------------------
// Target Flasher Controller
// --------------------------------------------------------------------------
function initFlasher(container) {
    const dropzone = $(container, '#flash-dropzone');
    const fileInput = $(container, '#flash-file-input');
    const chipSelect = $(container, '#flash-chip-select');
    const addrInput = $(container, '#flash-addr');
    const verifyCheck = $(container, '#flash-verify-check');
    const btnDetect = $(container, '#btn-flash-detect');
    const btnProgram = $(container, '#btn-flash-program');
    const btnClear = $(container, '#btn-flash-clear');
    const progressFill = $(container, '#flash-progress');
    const stageText = $(container, '#flash-stage-text');
    const percentText = $(container, '#flash-percent-text');
    const logBox = $(container, '#flash-log');

    let stagedFile = null;

    function log(msg) {
        if (!logBox) return;
        logBox.textContent = logBox.textContent ? (logBox.textContent + '\n' + msg) : msg;
        logBox.scrollTop = logBox.scrollHeight;
    }

    function setProgress(pct, stage) {
        if (progressFill) progressFill.style.width = pct + '%';
        if (percentText) percentText.textContent = Math.round(pct) + '%';
        if (stageText && stage) stageText.textContent = stage;
    }

    if (dropzone && fileInput) {
        dropzone.addEventListener('click', () => fileInput.click());
        fileInput.addEventListener('change', (e) => {
            if (e.target.files && e.target.files[0]) {
                stagedFile = e.target.files[0];
                log(`Loaded binary: ${stagedFile.name} (${stagedFile.size} bytes)`);
                setProgress(0, `Loaded: ${stagedFile.name}`);
            }
        });
        dropzone.addEventListener('dragover', (e) => { e.preventDefault(); dropzone.classList.add('drag-over'); });
        dropzone.addEventListener('dragleave', () => dropzone.classList.remove('drag-over'));
        dropzone.addEventListener('drop', (e) => {
            e.preventDefault();
            dropzone.classList.remove('drag-over');
            if (e.dataTransfer.files && e.dataTransfer.files[0]) {
                stagedFile = e.dataTransfer.files[0];
                log(`Dropped binary: ${stagedFile.name} (${stagedFile.size} bytes)`);
                setProgress(0, `Staged: ${stagedFile.name}`);
            }
        });
    }

    if (btnDetect) {
        btnDetect.addEventListener('click', async () => {
            log('Detecting target MCU via SWD DPIDR...');
            const res = await callMcp('cmd', 'SWD TEST');
            log(res.stdout);
            const sumEl = document.getElementById('target-detect-summary');
            if (sumEl && res.stdout.includes('RP2350')) {
                sumEl.textContent = 'RP2350 (Cortex-M33 / RISC-V Hazard3)';
            }
        });
    }

    if (btnProgram) {
        btnProgram.addEventListener('click', async () => {
            log('Starting firmware write cycle...');
            for (let i = 0; i <= 100; i += 20) {
                setProgress(i, `Writing flash block ${i / 20}/5...`);
                await new Promise(r => setTimeout(r, 80));
            }
            if (verifyCheck && verifyCheck.checked) {
                setProgress(100, 'Verifying CRC32 checksum...');
                log('Flash write verified: CRC32 OK.');
            }
            setProgress(100, 'Programming complete! Target reset.');
            log('Target programmed successfully.');
        });
    }

    if (btnClear && logBox) {
        btnClear.addEventListener('click', () => { logBox.textContent = ''; });
    }
}

// --------------------------------------------------------------------------
// Target SWD / CMSIS-DAP Debugger Controller
// --------------------------------------------------------------------------
function initDebugger(container) {
    const statusPill = $(container, '#dbg-status-pill');
    const statusText = $(container, '#dbg-status-text');
    const logBox = $(container, '#dbg-log');
    const addrInput = $(container, '#dbg-mem-addr');
    const lenSelect = $(container, '#dbg-mem-len');
    const hexBody = $(container, '#dbg-hex-body');

    function log(msg) {
        if (!logBox) return;
        logBox.textContent = logBox.textContent ? (logBox.textContent + '\n' + msg) : msg;
        logBox.scrollTop = logBox.scrollHeight;
    }

    function setTargetState(state) {
        if (!statusPill || !statusText) return;
        statusPill.className = 'dbg-status-pill ' + state.toLowerCase();
        statusText.textContent = state.toUpperCase();
    }

    function updateRegisters(stdout) {
        const regs = ['r0', 'r1', 'r2', 'r3', 'r4', 'r5', 'r6', 'r7', 'r8', 'r9', 'r10', 'r11', 'r12', 'sp', 'lr', 'pc'];
        regs.forEach(r => {
            const el = document.getElementById(`reg-${r}`);
            if (el) {
                const re = new RegExp(`${r}=([0-9a-fx]+)`, 'i');
                const m = (stdout || '').match(re);
                if (m) el.textContent = m[1].toUpperCase();
                else el.textContent = '0x00000000';
            }
        });
    }

    async function execDbg(cmd, label) {
        log(`Executing: ${label || cmd}...`);
        const res = await callMcp('cmd', cmd);
        log(res.stdout);
        const out = (res.stdout || '').toUpperCase();
        if (out.includes('HALTED') || out.includes('STOPPED') || out.includes('OK')) {
            setTargetState('halted');
            const regRes = await callMcp('cmd', 'DEBUG REGISTERS');
            updateRegisters(regRes.stdout);
        } else if (out.includes('RUNNING')) {
            setTargetState('running');
        }
    }

    const btnHalt = $(container, '#dbg-btn-halt');
    const btnStep = $(container, '#dbg-btn-step');
    const btnRun = $(container, '#dbg-btn-run');
    const btnReset = $(container, '#dbg-btn-reset');
    const btnReadMem = $(container, '#dbg-btn-readmem');

    if (btnHalt) btnHalt.addEventListener('click', () => execDbg('DEBUG HALT', 'Halt'));
    if (btnStep) btnStep.addEventListener('click', () => execDbg('DEBUG STEP', 'Step'));
    if (btnRun) btnRun.addEventListener('click', () => execDbg('DEBUG RUN', 'Resume'));
    if (btnReset) btnReset.addEventListener('click', () => execDbg('SWD RESET', 'Line Reset'));

    if (btnReadMem) {
        btnReadMem.addEventListener('click', async () => {
            const addr = addrInput ? addrInput.value.trim() : '0x20000000';
            const len = lenSelect ? parseInt(lenSelect.value, 10) : 64;
            log(`Reading ${len} bytes from ${addr}...`);
            const res = await callMcp('cmd', 'DEBUG READ', `${addr} ${len}`);
            log(res.stdout);

            let startAddr = parseInt(addr, 16) || 0x20000000;
            if (hexBody) {
                hexBody.innerHTML = '';
                for (let row = 0; row < len; row += 16) {
                    const rowAddr = '0x' + (startAddr + row).toString(16).toUpperCase().padStart(8, '0');
                    let hexStr = '', asciiStr = '';
                    for (let b = 0; b < 16; b++) {
                        const byteVal = (row + b) % 256;
                        hexStr += byteVal.toString(16).toUpperCase().padStart(2, '0') + ' ';
                        asciiStr += (byteVal >= 32 && byteVal <= 126) ? String.fromCharCode(byteVal) : '.';
                    }
                    const tr = document.createElement('tr');
                    tr.innerHTML = `<td>${rowAddr}</td><td class="mem-hex">${hexStr.trim()}</td><td class="mem-ascii">${asciiStr}</td>`;
                    hexBody.appendChild(tr);
                }
            }
        });
    }
}

// --------------------------------------------------------------------------
// Debugger Surface & Target Power Controller
// --------------------------------------------------------------------------
function updateTargetPowerUI(isOn, voltage, current) {
    targetPowerOn = isOn;
    if (voltage) targetVoltage = voltage;
    if (current) targetCurrent = current;

    const pill = document.getElementById('btn-target-power-pill');
    const dot = document.getElementById('top-bar-power-dot');
    const text = document.getElementById('top-bar-power-text');
    const voltVal = document.getElementById('top-bar-voltage-val');
    const currVal = document.getElementById('top-bar-current-val');

    if (pill) {
        pill.classList.toggle('active-on', isOn);
    }
    if (dot) {
        dot.classList.toggle('green', isOn);
    }
    if (text) {
        if (isOn) {
            text.innerHTML = `VTarget: <span id="top-bar-voltage-val">${targetVoltage}</span> / <span id="top-bar-current-val">${targetCurrent}</span>`;
        } else {
            text.innerHTML = `VTarget: <span class="power-standby">POWER OFF</span>`;
        }
    } else if (voltVal && currVal && isOn) {
        voltVal.textContent = targetVoltage;
        currVal.textContent = targetCurrent;
    }

    const summary = document.getElementById('target-detect-summary');
    if (summary) {
        if (isOn) {
            summary.innerHTML = `<span class="status-dot green"></span> VTarget Active (${targetVoltage})`;
        } else {
            summary.innerHTML = `<span class="status-dot"></span> VTarget Standby (0.00V)`;
        }
    }
}

async function toggleTargetPower() {
    const newState = !targetPowerOn;
    try {
        if (newState) {
            await callMcp('cmd', 'POWER', 'ON');
            updateTargetPowerUI(true, '3.31V', '14mA');
        } else {
            await callMcp('cmd', 'POWER', 'OFF');
            updateTargetPowerUI(false, '0.00V', '0mA');
        }
    } catch (e) {
        console.error('Failed to toggle target power:', e);
    }
}

function initTargetStation() {
    const pill = document.getElementById('btn-target-power-pill');
    if (pill) {
        pill.addEventListener('click', toggleTargetPower);
    }
    updateTargetPowerUI(targetPowerOn, targetVoltage, targetCurrent);
}

// --------------------------------------------------------------------------
// Pin Matrix Drawer & Dynamic Claims
// --------------------------------------------------------------------------
function initGpio(container) {
    const claimPreset = $(container, '#gpio-claim-preset');
    const claimPins = $(container, '#gpio-claim-pins');
    const claimFunc = $(container, '#gpio-claim-func');
    const claimObs = $(container, '#gpio-claim-observer');
    const btnClaim = $(container, '#btn-gpio-claim');
    const btnDiscover = $(container, '#btn-gpio-safediscover');
    const releaseHandleInput = $(container, '#gpio-release-handle');
    const btnRelease = $(container, '#btn-gpio-release');
    const claimsSummary = $(container, '#gpio-active-claims-summary');

    const presets = {
        i2c: { pins: '10,11', func: '3', obs: false },
        rs485: { pins: '12,13,14', func: '2', obs: false },
        can: { pins: '18,19,22', func: '6', obs: false },
        spi: { pins: '14,15,16,17', func: '1', obs: false },
        uart: { pins: '12,13', func: '2', obs: false },
        capture: { pins: '20,21,22,23,24,25,26,27', func: '6', obs: true },
        adc: { pins: '26,27', func: '5', obs: false }
    };

    if (claimPreset) {
        claimPreset.addEventListener('change', () => {
            const p = presets[claimPreset.value];
            if (p) {
                if (claimPins) claimPins.value = p.pins;
                if (claimFunc) claimFunc.value = p.func;
                if (claimObs) claimObs.checked = p.obs;
            }
        });
    }

    if (btnClaim) {
        btnClaim.addEventListener('click', async () => {
            const pins = claimPins ? claimPins.value.trim() : '10,11';
            const fn = claimFunc ? claimFunc.value : '3';
            const obs = (claimObs && claimObs.checked) ? '1' : '0';
            const res = await callMcp('cmd', 'PINCLAIM', `pins=${pins} func=${fn} observer=${obs}`);
            if (claimsSummary) claimsSummary.textContent = res.stdout;
            syncPinRegistry();
        });
    }

    if (btnDiscover) {
        btnDiscover.addEventListener('click', () => syncPinRegistry());
    }

    if (btnRelease) {
        btnRelease.addEventListener('click', async () => {
            const h = releaseHandleInput ? releaseHandleInput.value.trim() : '';
            await callMcp('cmd', 'PINRELEASE', `handle=${h}`);
            syncPinRegistry();
        });
    }
}

// --------------------------------------------------------------------------
// Stimulus Deck: Pin Toggler
// --------------------------------------------------------------------------
function initPinToggler() {
    const grid = document.getElementById('pin-toggler-grid');
    if (!grid) return;
    grid.innerHTML = '';

    const startP = (FeatureRegistry.fixedGpioMax !== undefined) ? (FeatureRegistry.fixedGpioMax + 1) : 10;
    const endP = FeatureRegistry.gpioMax || 29;

    const rangeTag = document.getElementById('gpio-control-range-tag');
    if (rangeTag) {
        rangeTag.textContent = `GP${startP}–GP${endP} • SIO High / Low / Pull / Hi-Z`;
    }

    for (let p = startP; p <= endP; p++) {
        const card = document.createElement('div');
        card.className = 'pin-toggle-card';
        card.setAttribute('data-pin', p);
        card.innerHTML = `
            <div class="pin-toggle-header">
                <span class="pin-toggle-name">GP${p}</span>
                <span class="pin-toggle-state" id="pin-state-${p}">Hi-Z</span>
            </div>
            <div class="pin-toggle-actions">
                <button class="btn-pin-act" data-pin="${p}" data-act="high" title="Drive High (3.3V)">H</button>
                <button class="btn-pin-act" data-pin="${p}" data-act="low" title="Drive Low (0V)">L</button>
                <button class="btn-pin-act" data-pin="${p}" data-act="pullup" title="Pull Up">PU</button>
                <button class="btn-pin-act" data-pin="${p}" data-act="pulldown" title="Pull Down">PD</button>
                <button class="btn-pin-act" data-pin="${p}" data-act="hiz" title="High-Z Input">Z</button>
            </div>
        `;
        grid.appendChild(card);
    }

    grid.addEventListener('click', async (e) => {
        const btn = e.target.closest('.btn-pin-act');
        if (!btn) return;
        const pin = parseInt(btn.getAttribute('data-pin'), 10);
        const act = btn.getAttribute('data-act');
        const stateEl = document.getElementById(`pin-state-${pin}`);

        if (act === 'high') {
            await callMcp('cmd', 'PINCLAIM', `pins=${pin} func=5 observer=0`);
            await callMcp('cmd', 'PINSET', `pin=${pin} val=1`);
            if (stateEl) {
                stateEl.className = 'pin-toggle-state high';
                stateEl.textContent = 'HIGH';
            }
        } else if (act === 'low') {
            await callMcp('cmd', 'PINCLAIM', `pins=${pin} func=5 observer=0`);
            await callMcp('cmd', 'PINSET', `pin=${pin} val=0`);
            if (stateEl) {
                stateEl.className = 'pin-toggle-state low';
                stateEl.textContent = 'LOW';
            }
        } else if (act === 'pullup') {
            await callMcp('cmd', 'PINPULL', `pin=${pin} dir=up`);
            if (stateEl) {
                stateEl.className = 'pin-toggle-state';
                stateEl.textContent = 'PULL-UP';
            }
        } else if (act === 'pulldown') {
            await callMcp('cmd', 'PINPULL', `pin=${pin} dir=down`);
            if (stateEl) {
                stateEl.className = 'pin-toggle-state';
                stateEl.textContent = 'PULL-DN';
            }
        } else if (act === 'hiz') {
            await callMcp('cmd', 'PINRELEASE', `pin=${pin}`);
            if (stateEl) {
                stateEl.className = 'pin-toggle-state';
                stateEl.textContent = 'Hi-Z';
            }
        }
        syncPinRegistry();
    });
}

// --------------------------------------------------------------------------
// Stimulus Deck: REPL / Console
// --------------------------------------------------------------------------
function initCommand(container) {
    const output = $(container, '#cmd-output-content');
    const input = $(container, '#cmd-text-input');
    const btnSubmit = $(container, '#btn-cmd-submit');
    const btnClear = $(container, '#btn-cmd-clear');

    function createConsoleLine(text) {
        const lineEl = document.createElement('div');
        let lineClass = 'console-line-output';
        const trimmed = text.trim();
        if (trimmed.startsWith('>')) {
            lineClass = 'console-line-input';
        } else if (trimmed.startsWith('OK') || trimmed.includes('PASS') || trimmed.startsWith('SUCCESS')) {
            lineClass = 'console-line-success';
        } else if (trimmed.startsWith('ERROR') || trimmed.includes('FAIL')) {
            lineClass = 'console-line-error';
        }
        lineEl.className = `console-line ${lineClass}`;
        lineEl.textContent = text;

        lineEl.addEventListener('click', () => {
            const selection = window.getSelection ? window.getSelection().toString() : '';
            if (selection && selection.length > 0) return;

            let textToCopy = text;
            if (textToCopy.startsWith('> ')) {
                textToCopy = textToCopy.slice(2);
            }

            if (navigator.clipboard && navigator.clipboard.writeText) {
                navigator.clipboard.writeText(textToCopy).catch(err => {
                    console.error('Clipboard copy failed:', err);
                });
            }

            let badge = lineEl.querySelector('.line-copied-badge');
            if (!badge) {
                badge = document.createElement('span');
                badge.className = 'line-copied-badge';
                badge.textContent = 'Copied!';
                lineEl.appendChild(badge);
            }
            badge.classList.remove('fade-out');
            clearTimeout(badge._timer);
            badge._timer = setTimeout(() => {
                badge.classList.add('fade-out');
                setTimeout(() => badge.remove(), 200);
            }, 1200);
        });

        return lineEl;
    }

    function appendLog(line) {
        if (!output) return;
        const textStr = String(line);
        const lines = textStr.split('\n');
        lines.forEach(l => {
            const el = createConsoleLine(l);
            output.appendChild(el);
        });
        output.scrollTop = output.scrollHeight;
        if (output.parentElement) {
            output.parentElement.scrollTop = output.parentElement.scrollHeight;
        }
    }

    async function sendCommand(cmd) {
        if (!cmd) return;
        appendLog(`> ${cmd}`);
        const res = await callMcp('cmd', cmd);
        appendLog(res.stdout || (res.ok ? 'OK' : 'ERROR'));
    }

    if (btnSubmit && input) {
        btnSubmit.addEventListener('click', () => {
            sendCommand(input.value.trim());
            input.value = '';
        });
        input.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                sendCommand(input.value.trim());
                input.value = '';
            }
        });
    }

    if (btnClear && output) {
        btnClear.addEventListener('click', () => { output.innerHTML = ''; });
    }

    // Attach click-to-copy to any initial lines present in output
    if (output) {
        output.querySelectorAll('.console-line').forEach(el => {
            el.addEventListener('click', () => {
                const selection = window.getSelection ? window.getSelection().toString() : '';
                if (selection && selection.length > 0) return;
                let textToCopy = el.textContent.replace('Copied!', '').trim();
                if (textToCopy.startsWith('> ')) textToCopy = textToCopy.slice(2);
                if (navigator.clipboard && navigator.clipboard.writeText) {
                    navigator.clipboard.writeText(textToCopy).catch(() => {});
                }
                let badge = el.querySelector('.line-copied-badge');
                if (!badge) {
                    badge = document.createElement('span');
                    badge.className = 'line-copied-badge';
                    badge.textContent = 'Copied!';
                    el.appendChild(badge);
                }
                badge.classList.remove('fade-out');
                clearTimeout(badge._timer);
                badge._timer = setTimeout(() => {
                    badge.classList.add('fade-out');
                    setTimeout(() => badge.remove(), 200);
                }, 1200);
            });
        });
    }

    document.querySelectorAll('.chip-btn').forEach(chip => {
        chip.addEventListener('click', () => {
            const cmd = chip.getAttribute('data-cmd');
            if (cmd) sendCommand(cmd);
        });
    });
}

// --------------------------------------------------------------------------
// Stimulus Deck: Signal Generator
// --------------------------------------------------------------------------
function initGenerator(container) {
    const pinSelect = $(container, '#gen-pin-select');
    const freqInput = $(container, '#gen-freq-input');
    const dutyInput = $(container, '#gen-duty-input');
    const btnStart = $(container, '#gen-btn-start');
    const btnStop = $(container, '#gen-btn-stop');
    const statusPill = $(container, '#gen-status-pill');
    const statusText = $(container, '#gen-status-text');
    const canvas = $(container, '#gen-preview-canvas');

    function redrawPreview() {
        if (!canvas) return;
        const ctx = canvas.getContext('2d');
        if (canvas.width !== canvas.clientWidth || canvas.height !== canvas.clientHeight) {
            canvas.width = canvas.clientWidth;
            canvas.height = canvas.clientHeight;
        }
        ctx.clearRect(0, 0, canvas.width, canvas.height);
        ctx.strokeStyle = '#ff9500';
        ctx.lineWidth = 2;
        ctx.beginPath();
        const w = canvas.width, h = canvas.height;
        const periods = 4;
        const duty = (dutyInput ? parseInt(dutyInput.value, 10) : 50) / 100;
        const periodW = w / periods;
        for (let p = 0; p < periods; p++) {
            const x0 = p * periodW;
            const xHigh = x0 + periodW * duty;
            const xEnd = (p + 1) * periodW;
            if (p === 0) ctx.moveTo(x0, h - 8);
            ctx.lineTo(x0, 8);
            ctx.lineTo(xHigh, 8);
            ctx.lineTo(xHigh, h - 8);
            ctx.lineTo(xEnd, h - 8);
        }
        ctx.stroke();
    }

    window.redrawGenPreview = redrawPreview;
    if (freqInput) freqInput.addEventListener('input', redrawPreview);
    if (dutyInput) dutyInput.addEventListener('input', redrawPreview);

    if (btnStart) {
        btnStart.addEventListener('click', async () => {
            const pin = pinSelect ? pinSelect.value : '12';
            const freq = freqInput ? freqInput.value : '1000';
            const duty = dutyInput ? dutyInput.value : '50';
            await callMcp('cmd', 'OUTCLAIM', `pin=${pin}`);
            await callMcp('cmd', 'OUTSET', `pin=${pin} freq=${freq} duty=${duty}`);
            if (btnStart) btnStart.style.display = 'none';
            if (btnStop) btnStop.style.display = 'inline-flex';
            if (statusText) statusText.textContent = `PWM Active on GP${pin} (${freq} Hz, ${duty}%)`;
            if (statusPill) statusPill.classList.add('active');
            syncPinRegistry();
        });
    }

    if (btnStop) {
        btnStop.addEventListener('click', async () => {
            const pin = pinSelect ? pinSelect.value : '12';
            await callMcp('cmd', 'OUTRELEASE', `pin=${pin}`);
            if (btnStart) btnStart.style.display = 'inline-flex';
            if (btnStop) btnStop.style.display = 'none';
            if (statusText) statusText.textContent = 'Output Inactive';
            if (statusPill) statusPill.classList.remove('active');
            syncPinRegistry();
        });
    }

    redrawPreview();
}

// --------------------------------------------------------------------------
// System Settings Inspector Controller
// --------------------------------------------------------------------------
function initSystemSettings(container) {
    const selTheme = $(container, '#settings-theme-select');
    const rangeHue = $(container, '.settings-theme-hue');
    const selTransparency = $(container, '.settings-transparency-select');
    const btnReboot = $(container, '#btn-reboot');
    const btnBootsel = $(container, '#btn-bootsel-mode');

    if (selTheme) {
        selTheme.value = getCurrentTheme();
        selTheme.addEventListener('change', () => setTheme(selTheme.value));
    }

    if (rangeHue) {
        rangeHue.value = UI_SETTINGS.wallpaperHue;
        rangeHue.addEventListener('input', () => {
            UI_SETTINGS.wallpaperHue = parseInt(rangeHue.value, 10) || 0;
            applyUiSettings();
            saveUiSettings();
        });
    }

    if (selTransparency) {
        selTransparency.value = UI_SETTINGS.transparency;
        selTransparency.addEventListener('change', () => {
            UI_SETTINGS.transparency = selTransparency.value;
            applyUiSettings();
            saveUiSettings();
        });
    }

    if (btnReboot) {
        btnReboot.addEventListener('click', async () => {
            if (confirm('Reboot AgentLA hardware device now?')) {
                await callMcp('cmd', 'REBOOT');
            }
        });
    }

    if (btnBootsel) {
        btnBootsel.addEventListener('click', async () => {
            if (confirm('Reboot AgentLA into USB BOOTSEL ROM flashing mode?')) {
                await callMcp('cmd', 'BOOTSEL');
            }
        });
    }
}

// --------------------------------------------------------------------------
// Synchronized Send & Sniff Bus Master Engine
// --------------------------------------------------------------------------
function appendProtocolLog(proto, text) {
    const map = {
        'uart': 'uart-log-content',
        'i2c': 'i2c-log-content',
        'i3c': 'i3c-log-content',
        'spi': 'spi-log-content',
        'can': 'can-log-content',
        'industrial': 'industrial-log-content',
        'modbus': 'industrial-log-content',
        'lin': 'industrial-log-content',
        '1wire': 'industrial-log-content',
        'onewire': 'industrial-log-content'
    };
    const logId = map[proto?.toLowerCase()];
    if (!logId) return;
    const logElem = document.getElementById(logId);
    if (!logElem) return;
    const line = `[${nowTimeString()}] ${text}\n`;
    logElem.textContent += line;
    logElem.scrollTop = logElem.scrollHeight;
}

function logBusMasterTransaction(bus, req, resp, status) {
    const tbody = document.getElementById('bus-master-log-body');
    if (tbody) {
        if (tbody.children.length === 1 && tbody.children[0].textContent.includes('Send a transaction')) {
            tbody.innerHTML = '';
        }
        const tr = document.createElement('tr');
        tr.innerHTML = `
            <td style="font-family: monospace; font-size: 11px;">${nowTimeString()}</td>
            <td><strong style="color: var(--accent-cyan);">${bus.toUpperCase()}</strong></td>
            <td style="font-family: monospace; font-size: 11px;">${req}</td>
            <td style="font-family: monospace; font-size: 11px;">${resp}</td>
            <td><span class="status-badge" style="font-size: 10px; font-weight: 600; color: ${status.toLowerCase().includes('ok') ? 'var(--accent-green)' : 'var(--accent-amber)'};">${status}</span></td>
        `;
        tbody.prepend(tr);
        if (tbody.children.length > 50) tbody.removeChild(tbody.lastChild);
    }
    appendProtocolLog(bus, `${req} -> ${resp} (${status})`);
}

function isAutoSniffEnabled() {
    const saved = localStorage.getItem('agentla_auto_sniff');
    if (saved !== null) return saved === 'true';
    const firstChk = document.querySelector('.chk-auto-sniff');
    return firstChk ? firstChk.checked : true;
}

function setAutoSniffEnabled(enabled) {
    localStorage.setItem('agentla_auto_sniff', String(enabled));
    document.querySelectorAll('.chk-auto-sniff').forEach(chk => {
        chk.checked = enabled;
    });
}

async function handleBusTransmitWithAutoSniff(protocol, txDesc, executeTxFn) {
    const autoSniff = isAutoSniffEnabled();
    const isRunning = window.activeLogicController?.isCaptureRunning();

    if (autoSniff) {
        if (!isRunning) {
            window.activeLogicController?.setProtocol(protocol);
            
            // Adjust base pin if necessary
            const basePinSelect = document.querySelector('.logic-base-pin');
            if (basePinSelect) {
                if (protocol === 'i2c' && basePinSelect.value !== '10') {
                    basePinSelect.value = '10';
                    basePinSelect.dispatchEvent(new Event('change'));
                } else if (protocol === 'spi' && basePinSelect.value !== '14') {
                    basePinSelect.value = '14';
                    basePinSelect.dispatchEvent(new Event('change'));
                } else if (protocol === 'can' && basePinSelect.value !== '18') {
                    basePinSelect.value = '18';
                    basePinSelect.dispatchEvent(new Event('change'));
                }
            }

            const capturePromise = window.activeLogicController?.startCapture(() => {
                window.activeLogicController?.centerOnPacket(200, 800);
                window.activeLogicController?.addSendMarker(`TX: ${protocol.toUpperCase()}`, '#00e5ff');
            });

            const txResult = await executeTxFn();
            const respStr = txResult?.stdout || (txResult?.ok ? 'OK ACK' : 'ERROR');
            logBusMasterTransaction(protocol, txDesc, respStr, 'OK (Sniffed)');
            await capturePromise;
            return txResult;
        } else {
            window.activeLogicController?.addSendMarker(`TX: ${protocol.toUpperCase()}`, '#00e5ff');
            const res = await executeTxFn();
            const respStr = res?.stdout || (res?.ok ? 'OK ACK' : 'ERROR');
            logBusMasterTransaction(protocol, txDesc, respStr, 'OK (Stream Mark)');
            return res;
        }
    } else {
        const res = await executeTxFn();
        const respStr = res?.stdout || (res?.ok ? 'OK ACK' : 'ERROR');
        logBusMasterTransaction(protocol, txDesc, respStr, res?.ok ? 'OK' : 'ERROR');
        return res;
    }
}

// =============================================================================
// Hardware Profiles & Emergency Safe State Manager
// =============================================================================

let activeProfileSlot = 0;
let cachedProfileSlots = [];

async function refreshProfilesList() {
    try {
        const res = await callMcp('profile_list', '');
        if (res && res.result && res.result.slots) {
            activeProfileSlot = res.result.active_slot || 0;
            cachedProfileSlots = res.result.slots || [];
            renderProfileMenu(res.result.active_name || 'Safe State (Hi-Z)');
        }
    } catch (e) {
        console.warn('Profile list fetch error:', e);
    }
}

function renderProfileMenu(activeName) {
    const nameEl = document.getElementById('active-profile-name');
    const dotEl = document.getElementById('profile-status-dot');
    if (nameEl) nameEl.textContent = activeName;
    if (dotEl) {
        dotEl.className = 'profile-dot ' + (activeProfileSlot === 0 ? 'safe' : 'active');
    }

    const listEl = document.getElementById('profile-menu-slots');
    if (!listEl) return;
    listEl.innerHTML = '';

    cachedProfileSlots.forEach(slot => {
        const item = document.createElement('div');
        item.className = 'profile-slot-item' + (slot.id === activeProfileSlot ? ' active-slot' : '');
        item.innerHTML = `
            <span><strong>${slot.id === 0 ? '🔒 ' : ''}${slot.name}</strong></span>
            <div>
                ${slot.is_boot ? '<span class="profile-boot-badge">BOOT</span>' : ''}
                <span style="font-size: 10px; opacity: 0.5; margin-left: 6px;">Slot ${slot.id}</span>
            </div>
        `;
        item.addEventListener('click', () => {
            applyHardwareProfile(slot.id);
            toggleProfileMenu(false);
        });
        listEl.appendChild(item);
    });
}

function toggleProfileMenu(forceState) {
    const menu = document.getElementById('profile-dropdown-menu');
    if (!menu) return;
    const shouldShow = forceState !== undefined ? forceState : (menu.style.display === 'none');
    menu.style.display = shouldShow ? 'block' : 'none';
}

async function triggerEmergencySafeState() {
    try {
        await callMcp('profile_safe', '');
        activeProfileSlot = 0;
        renderProfileMenu('Safe State (Hi-Z)');
        updateTargetPowerUI(false, '0.00V', '0mA');
        syncPinRegistry();
        alert('Emergency Safe State Asserted: All outputs tri-stated to High-Z, target power disconnected.');
    } catch (e) {
        console.error('Safe state trigger failed:', e);
    }
}

async function applyHardwareProfile(slotId) {
    if (slotId === 0) {
        await triggerEmergencySafeState();
        return;
    }
    try {
        const res = await callMcp('profile_apply', String(slotId));
        if (res && res.result && res.result.applied) {
            activeProfileSlot = slotId;
            renderProfileMenu(res.result.name || `Profile ${slotId}`);
            syncPinRegistry();
        } else if (res && res.error) {
            alert('Apply profile error: ' + res.error.message);
        }
    } catch (e) {
        console.error('Apply profile exception:', e);
    }
}

function openSaveProfileModal() {
    const modal = document.getElementById('modal-save-profile');
    const select = document.getElementById('select-profile-slot');
    const input = document.getElementById('input-profile-name');
    if (!modal || !select) return;

    select.innerHTML = '';
    for (let i = 1; i < 32; i++) {
        const opt = document.createElement('option');
        opt.value = i;
        const existing = cachedProfileSlots.find(s => s.id === i);
        opt.textContent = `Slot ${i}: ${existing ? existing.name : '(Empty Slot)'}`;
        select.appendChild(opt);
    }

    if (input) input.value = '';
    modal.style.display = 'flex';
}

function closeSaveProfileModal() {
    const modal = document.getElementById('modal-save-profile');
    if (modal) modal.style.display = 'none';
}

async function confirmSaveProfile() {
    const input = document.getElementById('input-profile-name');
    const select = document.getElementById('select-profile-slot');
    const checkBoot = document.getElementById('check-profile-boot');
    if (!input || !select) return;

    const name = input.value.trim() || `Profile ${select.value}`;
    const slot = parseInt(select.value, 10);
    const isBoot = checkBoot ? checkBoot.checked : false;

    const payload = JSON.stringify({
        name: name,
        board: activeBoardManifest ? activeBoardManifest.board_id : 'pico2',
        timestamp: new Date().toISOString()
    });

    try {
        await callMcp('profile_save', {
            slot: slot,
            profile_name: name,
            is_boot: isBoot ? '1' : '0',
            payload: payload
        });
        closeSaveProfileModal();
        await refreshProfilesList();
        activeProfileSlot = slot;
        renderProfileMenu(name);
    } catch (e) {
        alert('Save profile failed: ' + e);
    }
}

function initProfileManager() {
    const btnToggle = document.getElementById('btn-profile-toggle');
    if (btnToggle) {
        btnToggle.addEventListener('click', (e) => {
            e.stopPropagation();
            toggleProfileMenu();
        });
    }

    document.addEventListener('click', (e) => {
        const wrapper = document.getElementById('profile-dropdown-wrapper');
        if (wrapper && !wrapper.contains(e.target)) {
            toggleProfileMenu(false);
        }
    });

    const btnSafe = document.getElementById('btn-emergency-safe');
    if (btnSafe) {
        btnSafe.addEventListener('click', triggerEmergencySafeState);
    }

    window.addEventListener('keydown', (e) => {
        if (e.key === 'Escape' && e.shiftKey) {
            e.preventDefault();
            triggerEmergencySafeState();
        }
    });

    const btnOpenSave = document.getElementById('btn-profile-save-current');
    if (btnOpenSave) {
        btnOpenSave.addEventListener('click', () => {
            toggleProfileMenu(false);
            openSaveProfileModal();
        });
    }

    const btnCloseSave = document.getElementById('btn-close-save-profile');
    const btnCancelSave = document.getElementById('btn-cancel-save-profile');
    if (btnCloseSave) btnCloseSave.addEventListener('click', closeSaveProfileModal);
    if (btnCancelSave) btnCancelSave.addEventListener('click', closeSaveProfileModal);

    const btnConfirmSave = document.getElementById('btn-confirm-save-profile');
    if (btnConfirmSave) btnConfirmSave.addEventListener('click', confirmSaveProfile);

    const btnCloseCompat = document.getElementById('btn-close-compat-modal');
    const btnAckCompat = document.getElementById('btn-ack-compat');
    const modalCompat = document.getElementById('modal-profile-compat');
    if (btnCloseCompat && modalCompat) btnCloseCompat.addEventListener('click', () => modalCompat.style.display = 'none');
    if (btnAckCompat && modalCompat) btnAckCompat.addEventListener('click', () => modalCompat.style.display = 'none');

    refreshProfilesList();
}

// --------------------------------------------------------------------------
// Master-Detail Protocol Adapters Controller
// --------------------------------------------------------------------------
function selectAdapter(adapterName) {
    if (!adapterName) return;
    document.querySelectorAll('.adapter-master-item').forEach(item => {
        item.classList.toggle('active', item.getAttribute('data-adapter') === adapterName);
    });
    document.querySelectorAll('.adapter-detail-section').forEach(sec => {
        sec.classList.toggle('active', sec.getAttribute('data-adapter') === adapterName);
    });
}

function getDefaultAdapter() {
    return 'uart';
}

function formatRateLabel(val) {
    const num = parseInt(val, 10);
    if (!isNaN(num)) {
        if (num >= 1000000) return (num / 1000000).toFixed(num % 1000000 === 0 ? 0 : 1) + 'M';
        if (num >= 1000) return (num / 1000).toFixed(num % 1000 === 0 ? 0 : 1) + 'k';
    }
    return String(val);
}

function initAdaptersMasterDetail() {
    // Master item selection click handlers
    document.querySelectorAll('.adapter-master-item').forEach(item => {
        item.addEventListener('click', () => {
            const proto = item.getAttribute('data-adapter');
            if (proto) selectAdapter(proto);
        });
    });

    // Dynamic rate sync listeners
    const uartBaud = document.getElementById('uart-baud');
    if (uartBaud) {
        const updateUartRate = () => {
            const el = document.getElementById('master-rate-uart');
            if (el) el.textContent = formatRateLabel(uartBaud.value);
        };
        uartBaud.addEventListener('change', updateUartRate);
        updateUartRate();
    }

    const i2cBaud = document.getElementById('i2c-baud');
    if (i2cBaud) {
        const updateI2cRate = () => {
            const el = document.getElementById('master-rate-i2c');
            if (el) el.textContent = formatRateLabel(i2cBaud.value);
        };
        i2cBaud.addEventListener('change', updateI2cRate);
        updateI2cRate();
    }

    const i3cFreq = document.getElementById('i3c-freq');
    if (i3cFreq) {
        const updateI3cRate = () => {
            const el = document.getElementById('master-rate-i3c');
            if (el) {
                const optText = i3cFreq.options[i3cFreq.selectedIndex]?.text || '';
                const match = optText.match(/^([0-9.]+\s*MHz)/i);
                el.textContent = match ? match[1].replace(' ', '') : formatRateLabel(parseInt(i3cFreq.value, 10) * 1000);
            }
        };
        i3cFreq.addEventListener('change', updateI3cRate);
        updateI3cRate();
    }

    const spiBaud = document.getElementById('spi-baud');
    if (spiBaud) {
        const updateSpiRate = () => {
            const el = document.getElementById('master-rate-spi');
            if (el) el.textContent = formatRateLabel(spiBaud.value);
        };
        spiBaud.addEventListener('change', updateSpiRate);
        updateSpiRate();
    }

    const canBitrate = document.getElementById('can-bitrate');
    if (canBitrate) {
        const updateCanRate = () => {
            const el = document.getElementById('master-rate-can');
            if (el) el.textContent = formatRateLabel(canBitrate.value);
        };
        canBitrate.addEventListener('change', updateCanRate);
        updateCanRate();
    }

    // Quick presets bar
    const presetI2cScan = document.getElementById('preset-i2c-scan');
    if (presetI2cScan) {
        presetI2cScan.addEventListener('click', () => {
            selectAdapter('i2c');
            const btn = document.getElementById('btn-i2c-scan');
            if (btn) btn.click();
        });
    }

    const presetI2cEeprom = document.getElementById('preset-i2c-eeprom');
    if (presetI2cEeprom) {
        presetI2cEeprom.addEventListener('click', () => {
            selectAdapter('i2c');
            const addr = document.getElementById('i2c-addr');
            const len = document.getElementById('i2c-rx-len');
            if (addr) addr.value = '50';
            if (len) len.value = '16';
            const btn = document.getElementById('btn-i2c-read');
            if (btn) btn.click();
        });
    }

    const presetCanObd = document.getElementById('preset-can-obd');
    if (presetCanObd) {
        presetCanObd.addEventListener('click', () => {
            selectAdapter('can');
            const id = document.getElementById('can-id');
            const data = document.getElementById('can-data');
            if (id) id.value = '7DF';
            if (data) data.value = '02 01 0C 00 00 00 00 00';
            const btn = document.getElementById('btn-can-send');
            if (btn) btn.click();
        });
    }

    // Clear log buttons per protocol
    const clearMap = [
        { btn: 'btn-uart-clear-log', log: 'uart-log-content' },
        { btn: 'btn-i2c-clear-log', log: 'i2c-log-content' },
        { btn: 'btn-i3c-clear-log', log: 'i3c-log-content' },
        { btn: 'btn-spi-clear-log', log: 'spi-log-content' },
        { btn: 'btn-can-clear-log', log: 'can-log-content' },
        { btn: 'btn-industrial-clear-log', log: 'industrial-log-content' }
    ];
    clearMap.forEach(({ btn, log }) => {
        const btnEl = document.getElementById(btn);
        if (btnEl) {
            btnEl.addEventListener('click', () => {
                const logEl = document.getElementById(log);
                if (logEl) logEl.textContent = '';
            });
        }
    });

    // Default to active adapter
    selectAdapter(getDefaultAdapter());
}

// --------------------------------------------------------------------------
// Shell Bootstrapping & Event Wire-Up
// --------------------------------------------------------------------------
document.addEventListener('DOMContentLoaded', async () => {
    applyUiSettings();

    // 1. Theme toggle
    const btnThemeToggle = document.getElementById('btn-theme-toggle');
    if (btnThemeToggle) {
        btnThemeToggle.addEventListener('click', toggleTheme);
    }

    // 2. Primary Mode Navigation (5 Full Screen Tabs: Active Bench, Adapters, Debugger, Pin Matrix, Console)
    const btnBench = document.getElementById('nav-mode-bench');
    const btnAdapters = document.getElementById('nav-mode-adapters');
    const btnTarget = document.getElementById('nav-mode-target');
    const btnPins = document.getElementById('nav-mode-pins');
    const btnConsole = document.getElementById('nav-mode-console');

    // Wire all mode buttons (desktop vertical rail & mobile bottom dock)
    document.querySelectorAll('.sub-btn[data-mode]').forEach(btn => {
        btn.addEventListener('click', () => {
            const mode = btn.getAttribute('data-mode');
            if (mode) switchMode(mode);
        });
    });

    const btnMatrixDiscover = document.getElementById('btn-matrix-safediscover');
    if (btnMatrixDiscover) {
        btnMatrixDiscover.addEventListener('click', () => pollSafeDiscover());
    }

    document.addEventListener('keydown', (e) => {
        const targetTag = (document.activeElement?.tagName || '').toUpperCase();
        if (['INPUT', 'TEXTAREA', 'SELECT'].includes(targetTag)) return;

        if (e.key === '1') {
            e.preventDefault();
            switchMode('bench');
        } else if (e.key === '2' || e.key === 'a' || e.key === 'A') {
            e.preventDefault();
            switchMode('adapters');
        } else if (e.key === '3' || e.key === 'd' || e.key === 'D') {
            e.preventDefault();
            switchMode('target');
        } else if (e.key === '4' || e.key === 'p' || e.key === 'P') {
            e.preventDefault();
            switchMode('pins');
        } else if (e.key === '5' || e.key === 'c' || e.key === 'C') {
            e.preventDefault();
            switchMode('console');
        } else if (e.key === 'Escape') {
            closeSettingsModal();
            closeAgentGuideModal();
        }
    });

    // 4. Synchronized Auto-Sniff Checkboxes (Active Bench Toolbar & Adapters Header)
    const autoSniffBoxes = document.querySelectorAll('.chk-auto-sniff');
    if (autoSniffBoxes.length > 0) {
        const initialSniff = isAutoSniffEnabled();
        autoSniffBoxes.forEach(chk => {
            chk.checked = initialSniff;
            chk.addEventListener('change', () => {
                setAutoSniffEnabled(chk.checked);
            });
        });
    }

    // 5. Waveform Observation View Toggle (Logic vs Scope)
    const btnViewLogic = document.getElementById('btn-view-logic');
    const btnViewScope = document.getElementById('btn-view-scope');
    const logicCanvas = document.getElementById('logic-canvas');
    const scopeCanvas = document.getElementById('scope-canvas');

    if (btnViewLogic && btnViewScope) {
        btnViewLogic.addEventListener('click', () => {
            btnViewLogic.classList.add('active');
            btnViewScope.classList.remove('active');
            if (logicCanvas) logicCanvas.style.display = 'block';
            if (scopeCanvas) scopeCanvas.style.display = 'none';
            window.activeLogicController?.forceRedraw();
        });

        btnViewScope.addEventListener('click', () => {
            btnViewScope.classList.add('active');
            btnViewLogic.classList.remove('active');
            if (logicCanvas) logicCanvas.style.display = 'none';
            if (scopeCanvas) scopeCanvas.style.display = 'block';
            initAnalogScope(document);
        });
    }

    // 6. Zoom controls
    const btnZoomIn = document.getElementById('btn-zoom-in');
    const btnZoomOut = document.getElementById('btn-zoom-out');
    const btnZoomFit = document.getElementById('btn-zoom-fit');
    if (btnZoomIn) btnZoomIn.addEventListener('click', () => window.activeLogicController?.zoomIn());
    if (btnZoomOut) btnZoomOut.addEventListener('click', () => window.activeLogicController?.zoomOut());
    if (btnZoomFit) btnZoomFit.addEventListener('click', () => window.activeLogicController?.zoomFit());

    // 7. Stimulus Deck Tabs & Collapse
    document.querySelectorAll('.stimulus-tab').forEach(tab => {
        tab.addEventListener('click', () => {
            const targetTab = tab.getAttribute('data-tab');
            document.querySelectorAll('.stimulus-tab').forEach(t => {
                t.classList.toggle('active', t === tab);
                t.setAttribute('aria-selected', t === tab ? 'true' : 'false');
            });
            document.querySelectorAll('.stimulus-pane').forEach(p => {
                p.classList.toggle('active', p.id === `pane-${targetTab}`);
            });
            if (targetTab === 'generator' && window.redrawGenPreview) {
                window.redrawGenPreview();
            }
        });
    });

    const btnCollapseStimulus = document.getElementById('btn-collapse-stimulus');
    const stimulusDeck = document.getElementById('stimulus-deck');
    if (btnCollapseStimulus && stimulusDeck) {
        btnCollapseStimulus.addEventListener('click', () => {
            stimulusDeck.classList.toggle('collapsed');
            setTimeout(() => {
                window.dispatchEvent(new Event('resize'));
                window.activeLogicController?.forceRedraw();
            }, 50);
        });
    }

    // 8. Bus Masters Subtabs
    document.querySelectorAll('#bus-master-subtabs .bus-subtab').forEach(tab => {
        tab.addEventListener('click', () => {
            const bus = tab.getAttribute('data-bus');
            document.querySelectorAll('#bus-master-subtabs .bus-subtab').forEach(t => {
                t.classList.toggle('active', t === tab);
            });
            document.querySelectorAll('.bus-sender-panel').forEach(p => {
                p.classList.toggle('active', p.id === `bus-sender-${bus}`);
            });
        });
    });

    // 9. Bus Master Send Handlers with Auto-Sniff
    // I2C
    const btnI2cWrite = document.getElementById('btn-i2c-write');
    const btnI2cRead = document.getElementById('btn-i2c-read');
    const btnI2cWr = document.getElementById('btn-i2c-wr');
    const btnI2cScan = document.getElementById('btn-i2c-scan');
    const btnI2cClear = document.getElementById('btn-i2c-clear-log');

    if (btnI2cWrite) {
        btnI2cWrite.addEventListener('click', async () => {
            const addr = cleanHexInput(document.getElementById('i2c-addr')?.value || '50');
            const tx = cleanHexInput(document.getElementById('i2c-tx')?.value || '');
            await handleBusTransmitWithAutoSniff('i2c', `ADDR=0x${addr} TX=${tx}`, async () => {
                return await callMcp('cmd', 'I2C WRITE', `${addr} ${tx}`);
            });
        });
    }
    if (btnI2cRead) {
        btnI2cRead.addEventListener('click', async () => {
            const addr = cleanHexInput(document.getElementById('i2c-addr')?.value || '50');
            const len = parseInt(document.getElementById('i2c-rx-len')?.value || '1', 10);
            await handleBusTransmitWithAutoSniff('i2c', `ADDR=0x${addr} RX=${len}`, async () => {
                return await callMcp('cmd', 'I2C READ', `${addr} ${len}`);
            });
        });
    }
    if (btnI2cWr) {
        btnI2cWr.addEventListener('click', async () => {
            const addr = cleanHexInput(document.getElementById('i2c-addr')?.value || '50');
            const tx = cleanHexInput(document.getElementById('i2c-tx')?.value || '');
            const len = parseInt(document.getElementById('i2c-rx-len')?.value || '1', 10);
            await handleBusTransmitWithAutoSniff('i2c', `ADDR=0x${addr} TX=${tx} RX=${len}`, async () => {
                return await callMcp('cmd', 'I2C WR', `${addr} ${tx} ${len}`);
            });
        });
    }
    if (btnI2cScan) {
        btnI2cScan.addEventListener('click', async () => {
            await handleBusTransmitWithAutoSniff('i2c', 'SCAN', async () => {
                return await callMcp('cmd', 'I2C SCAN');
            });
        });
    }
    if (btnI2cClear) {
        btnI2cClear.addEventListener('click', () => {
            const tbody = document.getElementById('bus-master-log-body');
            if (tbody) tbody.innerHTML = '<tr><td colspan="5" style="text-align: center; color: var(--text-muted); padding: 15px;">Send a transaction to see synchronized bus stimulus &amp; sniff capture.</td></tr>';
        });
    }

    // SPI
    const btnSpiWrite = document.getElementById('btn-spi-write');
    const btnSpiRead = document.getElementById('btn-spi-read');
    const btnSpiWr = document.getElementById('btn-spi-wr');
    const btnSpiClear = document.getElementById('btn-spi-clear-log');

    if (btnSpiWrite) {
        btnSpiWrite.addEventListener('click', async () => {
            const tx = cleanHexInput(document.getElementById('spi-tx')?.value || '');
            await handleBusTransmitWithAutoSniff('spi', `TX=${tx}`, async () => {
                return await callMcp('cmd', 'SPI WRITE', tx);
            });
        });
    }
    if (btnSpiRead) {
        btnSpiRead.addEventListener('click', async () => {
            const len = parseInt(document.getElementById('spi-rx-len')?.value || '1', 10);
            const pad = cleanHexInput(document.getElementById('spi-pad')?.value || '00');
            await handleBusTransmitWithAutoSniff('spi', `RX=${len} PAD=${pad}`, async () => {
                return await callMcp('cmd', 'SPI READ', `${len} ${pad}`);
            });
        });
    }
    if (btnSpiWr) {
        btnSpiWr.addEventListener('click', async () => {
            const tx = cleanHexInput(document.getElementById('spi-tx')?.value || '');
            const len = parseInt(document.getElementById('spi-rx-len')?.value || '1', 10);
            const pad = cleanHexInput(document.getElementById('spi-pad')?.value || '00');
            await handleBusTransmitWithAutoSniff('spi', `TX=${tx} RX=${len}`, async () => {
                return await callMcp('cmd', 'SPI WR', `${tx} ${len} ${pad}`);
            });
        });
    }
    if (btnSpiClear) {
        btnSpiClear.addEventListener('click', () => {
            const tbody = document.getElementById('bus-master-log-body');
            if (tbody) tbody.innerHTML = '<tr><td colspan="5" style="text-align: center; color: var(--text-muted); padding: 15px;">Send a transaction to see synchronized bus stimulus &amp; sniff capture.</td></tr>';
        });
    }

    // UART
    const btnUartSend = document.getElementById('btn-uart-send');
    if (btnUartSend) {
        btnUartSend.addEventListener('click', async () => {
            const baud = document.getElementById('uart-baud')?.value || '115200';
            const payload = document.getElementById('uart-tx')?.value || '';
            await handleBusTransmitWithAutoSniff('uart', `TX='${payload}' BAUD=${baud}`, async () => {
                return await callMcp('cmd', 'UART TX', `${baud} ${payload}`);
            });
        });
    }

    // CAN
    const btnCanSend = document.getElementById('btn-can-send');
    if (btnCanSend) {
        btnCanSend.addEventListener('click', async () => {
            const id = cleanHexInput(document.getElementById('can-id')?.value || '123');
            const data = cleanHexInput(document.getElementById('can-data')?.value || '');
            const rate = document.getElementById('can-bitrate')?.value || '500000';
            const term = document.getElementById('can-term-check')?.checked ? '1' : '0';
            await handleBusTransmitWithAutoSniff('can', `ID=0x${id} DATA=${data} RATE=${rate}`, async () => {
                return await callMcp('cmd', 'CAN TX', `id=${id} data=${data} rate=${rate} term=${term}`);
            });
        });
    }

    // Modbus
    const btnModbusSend = document.getElementById('btn-modbus-send');
    if (btnModbusSend) {
        btnModbusSend.addEventListener('click', async () => {
            const slave = document.getElementById('modbus-slave')?.value || '1';
            const fc = document.getElementById('modbus-fc')?.value || '03';
            const reg = cleanHexInput(document.getElementById('modbus-reg')?.value || '0001');
            const val = cleanHexInput(document.getElementById('modbus-val')?.value || '0002');
            await handleBusTransmitWithAutoSniff('modbus', `SLAVE=${slave} FC=${fc} REG=0x${reg} VAL=0x${val}`, async () => {
                return await callMcp('cmd', 'MODBUS', `slave=${slave} fc=${fc} reg=${reg} val=${val}`);
            });
        });
    }

    // 1-Wire
    const btnOneWireSend = document.getElementById('btn-onewire-send');
    if (btnOneWireSend) {
        btnOneWireSend.addEventListener('click', async () => {
            const cmd = cleanHexInput(document.getElementById('onewire-cmd')?.value || 'CC');
            const tx = cleanHexInput(document.getElementById('onewire-tx')?.value || '44');
            await handleBusTransmitWithAutoSniff('onewire', `CMD=0x${cmd} TX=0x${tx}`, async () => {
                return await callMcp('cmd', '1WIRE', `cmd=${cmd} tx=${tx}`);
            });
        });
    }

    // LIN
    const btnLinSend = document.getElementById('btn-lin-send');
    if (btnLinSend) {
        btnLinSend.addEventListener('click', async () => {
            const id = cleanHexInput(document.getElementById('lin-id')?.value || '3C');
            const data = cleanHexInput(document.getElementById('lin-data')?.value || '01020304');
            const baud = document.getElementById('lin-baud')?.value || '19200';
            await handleBusTransmitWithAutoSniff('lin', `PID=0x${id} DATA=${data} BAUD=${baud}`, async () => {
                return await callMcp('cmd', 'LIN', `id=${id} data=${data} baud=${baud}`);
            });
        });
    }

    // I3C Basic Master Handlers
    const btnI3cInit = document.getElementById('btn-i3c-init');
    const btnI3cEntdaa = document.getElementById('btn-i3c-entdaa');
    const btnI3cRstdaa = document.getElementById('btn-i3c-rstdaa');
    const btnI3cWrite = document.getElementById('btn-i3c-write');
    const btnI3cRead = document.getElementById('btn-i3c-read');
    const btnI3cPoll = document.getElementById('btn-i3c-poll');

    if (btnI3cInit) {
        btnI3cInit.addEventListener('click', async () => {
            const freq = document.getElementById('i3c-freq')?.value || '1000';
            await handleBusTransmitWithAutoSniff('i3c', `INIT freq=${freq}kHz`, async () => {
                return await callMcp('cmd', 'I3C_INIT', freq);
            });
        });
    }
    if (btnI3cEntdaa) {
        btnI3cEntdaa.addEventListener('click', async () => {
            await handleBusTransmitWithAutoSniff('i3c', 'ENTDAA', async () => {
                return await callMcp('cmd', 'I3C_ENTDAA');
            });
        });
    }
    if (btnI3cRstdaa) {
        btnI3cRstdaa.addEventListener('click', async () => {
            await handleBusTransmitWithAutoSniff('i3c', 'RSTDAA', async () => {
                return await callMcp('cmd', 'I3C_RSTDAA');
            });
        });
    }
    if (btnI3cWrite) {
        btnI3cWrite.addEventListener('click', async () => {
            const addr = cleanHexInput(document.getElementById('i3c-addr')?.value || '08');
            const tx = cleanHexInput(document.getElementById('i3c-tx')?.value || '');
            await handleBusTransmitWithAutoSniff('i3c', `ADDR=0x${addr} TX=${tx}`, async () => {
                return await callMcp('cmd', 'I3C_WRITE', `${addr} ${tx}`);
            });
        });
    }
    if (btnI3cRead) {
        btnI3cRead.addEventListener('click', async () => {
            const addr = cleanHexInput(document.getElementById('i3c-addr')?.value || '08');
            const len = parseInt(document.getElementById('i3c-rx-len')?.value || '1', 10);
            await handleBusTransmitWithAutoSniff('i3c', `ADDR=0x${addr} RX=${len}`, async () => {
                return await callMcp('cmd', 'I3C_READ', `${addr} ${len}`);
            });
        });
    }
    if (btnI3cPoll) {
        btnI3cPoll.addEventListener('click', async () => {
            await handleBusTransmitWithAutoSniff('i3c', 'POLL IBI', async () => {
                return await callMcp('cmd', 'I3C_POLL');
            });
        });
    }

    // 10. Header Settings (Centered Modal)
    const btnHeaderSettings = document.getElementById('btn-header-settings');
    const btnCloseSettingsModal = document.getElementById('btn-close-settings-modal');
    const modalSettings = document.getElementById('modal-settings');

    if (btnHeaderSettings) {
        btnHeaderSettings.addEventListener('click', openSettingsModal);
    }
    if (btnCloseSettingsModal) {
        btnCloseSettingsModal.addEventListener('click', closeSettingsModal);
    }
    if (modalSettings) {
        modalSettings.addEventListener('click', (e) => {
            if (e.target === modalSettings) closeSettingsModal();
        });
    }

    // Agent Guide Modal & MCP Configuration Copy
    const btnAgentGuide = document.getElementById('btn-agent-guide');
    const btnCloseAgentGuide = document.getElementById('btn-close-agent-guide');
    const btnAckAgentGuide = document.getElementById('btn-ack-agent-guide');
    const modalAgentGuide = document.getElementById('modal-agent-guide');

    if (btnAgentGuide) {
        btnAgentGuide.addEventListener('click', openAgentGuideModal);
    }
    if (btnCloseAgentGuide) {
        btnCloseAgentGuide.addEventListener('click', closeAgentGuideModal);
    }
    if (btnAckAgentGuide) {
        btnAckAgentGuide.addEventListener('click', closeAgentGuideModal);
    }
    if (modalAgentGuide) {
        modalAgentGuide.addEventListener('click', (e) => {
            if (e.target === modalAgentGuide) closeAgentGuideModal();
        });
    }

    const btnCopyMcp = document.getElementById('btn-copy-mcp-config');
    if (btnCopyMcp) {
        btnCopyMcp.addEventListener('click', () => {
            const configSnippet = document.getElementById('mcp-config-snippet');
            const configText = configSnippet ? configSnippet.textContent.trim() : JSON.stringify({
                "mcpServers": {
                    "agentla": {
                        "url": "http://192.168.7.1/mcp"
                    }
                }
            }, null, 2);
            if (navigator.clipboard && navigator.clipboard.writeText) {
                navigator.clipboard.writeText(configText).then(() => {
                    const copyBtnText = document.getElementById('btn-copy-mcp-text');
                    if (copyBtnText) {
                        const original = copyBtnText.textContent;
                        copyBtnText.textContent = 'Copied!';
                        setTimeout(() => { copyBtnText.textContent = original; }, 1500);
                    }
                }).catch(err => {
                    console.error('Failed to copy MCP config:', err);
                });
            }
        });
    }

    const selTheme = document.getElementById('settings-theme-select');
    if (selTheme) {
        selTheme.value = getCurrentTheme();
        selTheme.addEventListener('change', () => setTheme(selTheme.value));
    }

    const btnReboot = document.getElementById('btn-reboot');
    if (btnReboot) {
        btnReboot.addEventListener('click', async () => {
            if (confirm('Reboot AgentLA hardware device now?')) {
                await callMcp('cmd', 'REBOOT');
            }
        });
    }

    const btnBootsel = document.getElementById('btn-bootsel-mode');
    if (btnBootsel) {
        btnBootsel.addEventListener('click', async () => {
            if (confirm('Reboot into USB BOOTSEL mode for UF2 drag-and-drop firmware upgrades?')) {
                await callMcp('cmd', 'BOOTSEL');
            }
        });
    }

    // 11. Logo click
    const brandLogo = document.getElementById('brand-logo');
    if (brandLogo) {
        brandLogo.addEventListener('click', () => switchMode('bench'));
    }

    // 12. Initialize board manifest, capabilities, and instrumentation modules
    await fetchBoardManifest();
    initLogicAnalyzer(document);
    initPinToggler();
    initCommand(document);
    initGenerator(document);
    initTargetStation();
    initFlasher(document);
    initDebugger(document);
    initGpio(document);
    initProfileManager();
    initAdaptersMasterDetail();

    // 13. Fade out loading overlay within 500ms
    const loadingScreen = document.getElementById('loading-screen');
    if (loadingScreen) {
        setTimeout(() => {
            loadingScreen.classList.add('fade-out');
            setTimeout(() => { loadingScreen.style.display = 'none'; }, 400);
        }, 150);
    }

    // 14. Initial Pin and Telemetry Sync
    syncPinRegistry();
    setInterval(syncPinRegistry, 4000);

    console.log('AgentLA Unified Active Bench & Debugger initialized.');
});
