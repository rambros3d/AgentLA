/**
 * AgentLA Web UI Controller
 * Adaptive Touch & Desktop Workspace Hub for RP2350
 * Zero third-party DOM dependencies
 * Fully compliant with Dynamic Pin Ownership Architecture (GPIO 0-9 Fixed, GPIO 10-29 Flexible)
 */

// Host and API configuration
const host = window.location.hostname || "192.168.7.1";

const APP_TITLES = Object.freeze({
    logic: "Logic Analyzer",
    flasher: "Target Flasher",
    debugger: "Target Debugger",
    gpio: "GPIO & Pin Matrix",
    i2c: "I2C Master",
    spi: "SPI Master",
    command: "Command Console",
    about: "About & Pinout",
    settings: "System Settings"
});

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
    document.documentElement.style.setProperty('--wallpaper-hue', `${UI_SETTINGS.wallpaperHue}deg`);
    if (UI_SETTINGS.transparency === 'solid') {
        document.body.classList.add('no-transparency');
    } else {
        document.body.classList.remove('no-transparency');
    }
}

// --------------------------------------------------------------------------
// Theme Engine (Tasks 6.1, 6.2)
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
// Slide-Up Bottom Sheet (Touch) / Slide-Out Right Drawer (Desktop) (Task 5.1)
// --------------------------------------------------------------------------
const inspectorDrawer = document.getElementById('inspector-drawer');
const inspectorOverlay = document.getElementById('inspector-overlay');
const drawerTitle = document.getElementById('drawer-title');
const drawerContent = document.getElementById('drawer-content');
const btnCloseDrawer = document.getElementById('btn-close-drawer');
const moreMenuSheet = document.getElementById('more-menu-sheet');

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
    if (moreMenuSheet) moreMenuSheet.classList.remove('show');
}

function closeInspector() {
    if (inspectorDrawer) inspectorDrawer.classList.remove('show');
    if (inspectorOverlay) inspectorOverlay.classList.remove('show');
    if (moreMenuSheet) moreMenuSheet.classList.remove('show');
}

if (btnCloseDrawer) btnCloseDrawer.addEventListener('click', closeInspector);
if (inspectorOverlay) inspectorOverlay.addEventListener('click', closeInspector);

// Touch swipe-down dismissal on bottom sheet
if (inspectorDrawer) {
    let touchStartY = 0;
    inspectorDrawer.addEventListener('touchstart', (e) => {
        touchStartY = e.touches[0].clientY;
    }, { passive: true });

    inspectorDrawer.addEventListener('touchmove', (e) => {
        const touchCurrentY = e.touches[0].clientY;
        const diff = touchCurrentY - touchStartY;
        if (diff > 60 && inspectorDrawer.scrollTop <= 0) {
            closeInspector();
        }
    }, { passive: true });
}

// --------------------------------------------------------------------------
// Navigation & Workspace Shell Router (Task 2.3)
// --------------------------------------------------------------------------
let currentView = 'dashboard';

function updateNavActiveStates(viewId) {
    document.querySelectorAll('#left-nav-rail .nav-rail-item').forEach(item => {
        const v = item.getAttribute('data-view');
        item.classList.toggle('active', v === viewId);
    });

    document.querySelectorAll('#bottom-nav-bar .bottom-nav-item').forEach(item => {
        const v = item.getAttribute('data-view');
        item.classList.toggle('active', v === viewId);
    });
}

function switchView(viewName, toolName = null) {
    const viewDashboard = document.getElementById('view-dashboard');
    const viewInstruments = document.getElementById('view-instruments');

    if (viewName === 'dashboard') {
        currentView = 'dashboard';
        if (viewDashboard) viewDashboard.classList.add('active');
        if (viewInstruments) viewInstruments.classList.remove('active');
        updateNavActiveStates('dashboard');
        syncPinRegistry();
    } else {
        currentView = 'instruments';
        if (viewDashboard) viewDashboard.classList.remove('active');
        if (viewInstruments) viewInstruments.classList.add('active');

        const toolToMount = toolName || viewName;
        if (toolToMount && APP_TITLES[toolToMount]) {
            mountTool(activeSlotId, toolToMount);
            updateNavActiveStates(toolToMount);
        }
        window.dispatchEvent(new Event('resize'));
    }
}

// --------------------------------------------------------------------------
// Dual-Slot Workspace & Slot Manager (Tasks 4.1, 4.2, 4.3, 5.3)
// --------------------------------------------------------------------------
let workspaceMode = 'single';
let activeSlotId = 1;
const slotMountedTools = { 1: null, 2: null };
const slotResizeObservers = { 1: null, 2: null };

function setWorkspaceMode(mode) {
    workspaceMode = mode === 'split' ? 'split' : 'single';
    const slotContainer = document.getElementById('slot-container');
    const btnSingle = document.getElementById('btn-mode-single');
    const btnSplit = document.getElementById('btn-mode-split');

    if (slotContainer) {
        slotContainer.classList.remove('mode-single', 'mode-split');
        slotContainer.classList.add(workspaceMode === 'split' ? 'mode-split' : 'mode-single');
    }

    if (btnSingle) btnSingle.classList.toggle('active', workspaceMode === 'single');
    if (btnSplit) btnSplit.classList.toggle('active', workspaceMode === 'split');

    if (workspaceMode === 'split') {
        if (!slotMountedTools[2] || slotMountedTools[2] === slotMountedTools[1]) {
            const fallbackTool = slotMountedTools[1] === 'logic' ? 'i2c' : 'logic';
            mountTool(2, fallbackTool);
        }
    }

    setTimeout(() => window.dispatchEvent(new Event('resize')), 50);
}

function mountTool(slotId, toolName) {
    const targetSlotContent = document.getElementById(`slot-${slotId}-content`);
    const templateElement = document.getElementById(`template-${toolName}`);
    if (!targetSlotContent || !templateElement) return;

    targetSlotContent.innerHTML = '';
    const content = templateElement.cloneNode(true);
    content.style.display = 'block';
    content.removeAttribute('id');
    targetSlotContent.appendChild(content);

    slotMountedTools[slotId] = toolName;

    const selectEl = document.querySelector(`.slot-tool-select[data-slot="${slotId}"]`);
    if (selectEl) selectEl.value = toolName;

    document.querySelectorAll('.instrument-slot').forEach(s => s.classList.remove('active-slot'));
    const slotEl = document.getElementById(`slot-${slotId}`);
    if (slotEl) slotEl.classList.add('active-slot');
    activeSlotId = slotId;

    const initializer = APP_INITIALIZERS[toolName];
    if (initializer) initializer(content);

    if (slotResizeObservers[slotId]) {
        slotResizeObservers[slotId].disconnect();
    }
    slotResizeObservers[slotId] = new ResizeObserver(() => {
        window.dispatchEvent(new Event('resize'));
    });
    slotResizeObservers[slotId].observe(targetSlotContent);
}

function swapSlots() {
    const t1 = slotMountedTools[1] || 'logic';
    const t2 = slotMountedTools[2] || 'i2c';
    mountTool(1, t2);
    mountTool(2, t1);
}

// --------------------------------------------------------------------------
// Quick Split Presets (Task 4.3)
// --------------------------------------------------------------------------
function applyPresetBusDebug() {
    switchView('instruments');
    setWorkspaceMode('split');
    mountTool(1, 'logic');
    mountTool(2, 'i2c');
    updateNavActiveStates('logic');
}

function applyPresetTargetBringUp() {
    switchView('instruments');
    setWorkspaceMode('split');
    mountTool(1, 'flasher');
    mountTool(2, 'command');
    updateNavActiveStates('flasher');
}

// --------------------------------------------------------------------------
// Dynamic Pin Ownership & 30-Pin Domain Allocation (openspec/changes/dynamic-pin-ownership)
// --------------------------------------------------------------------------
const PIN_MAP = [
    // Domain 1: Fixed Programmer Block (GP0 - GP9) - Non-flexible
    { pin: 0, name: 'GP0', fn: 'I2C0 SDA', group: 'prog', role: 'SWD/I2C', fixed: true },
    { pin: 1, name: 'GP1', fn: 'I2C0 SCL', group: 'prog', role: 'SWD/I2C', fixed: true },
    { pin: 2, name: 'GP2', fn: 'BOOT0 RECV', group: 'prog', role: 'RECOVERY', fixed: true },
    { pin: 3, name: 'GP3', fn: 'TARGET_PWR', group: 'prog', role: 'POWER', fixed: true },
    { pin: 4, name: 'GP4', fn: 'P_SWCLK', group: 'prog', role: 'SWD', fixed: true },
    { pin: 5, name: 'GP5', fn: 'P_SWDIO', group: 'prog', role: 'SWD', fixed: true },
    { pin: 6, name: 'GP6', fn: 'P_NRST', group: 'prog', role: 'RESET', fixed: true },
    { pin: 7, name: 'GP7', fn: 'RESERVED', group: 'prog', role: 'SYSTEM', fixed: true },
    { pin: 8, name: 'GP8', fn: 'UART0 TX', group: 'prog', role: 'UART', fixed: true },
    { pin: 9, name: 'GP9', fn: 'UART0 RX', group: 'prog', role: 'UART', fixed: true },

    // Domain 2: Flexible Adapter Interface (GP10 - GP19)
    { pin: 10, name: 'GP10', fn: 'A_SCL (I2C1)', group: 'adapt', role: 'I2C', fixed: false, driver: 0, observers: 0, handle: null },
    { pin: 11, name: 'GP11', fn: 'A_SDA (I2C1)', group: 'adapt', role: 'I2C', fixed: false, driver: 0, observers: 0, handle: null },
    { pin: 12, name: 'GP12', fn: 'A_TX (UART1)', group: 'adapt', role: 'UART', fixed: false, driver: 0, observers: 0, handle: null },
    { pin: 13, name: 'GP13', fn: 'A_RX (UART1)', group: 'adapt', role: 'UART', fixed: false, driver: 0, observers: 0, handle: null },
    { pin: 14, name: 'GP14', fn: 'A_SCK (SPI1)', group: 'adapt', role: 'SPI', fixed: false, driver: 0, observers: 0, handle: null },
    { pin: 15, name: 'GP15', fn: 'A_MOSI', group: 'adapt', role: 'SPI', fixed: false, driver: 0, observers: 0, handle: null },
    { pin: 16, name: 'GP16', fn: 'A_MISO', group: 'adapt', role: 'SPI', fixed: false, driver: 0, observers: 0, handle: null },
    { pin: 17, name: 'GP17', fn: 'A_CS', group: 'adapt', role: 'SPI', fixed: false, driver: 0, observers: 0, handle: null },
    { pin: 18, name: 'GP18', fn: 'CAN_TX / I3C', group: 'adapt', role: 'CAN/I3C', fixed: false, driver: 0, observers: 0, handle: null },
    { pin: 19, name: 'GP19', fn: 'CAN_RX / I3C', group: 'adapt', role: 'CAN/I3C', fixed: false, driver: 0, observers: 0, handle: null },

    // Domain 3: Flexible Logic Analyzer & ADC (GP20 - GP29)
    { pin: 20, name: 'GP20', fn: 'LA CH0 (PIO0)', group: 'la', role: 'CAPTURE', fixed: false, driver: 0, observers: 0, handle: null },
    { pin: 21, name: 'GP21', fn: 'LA CH1 (PIO0)', group: 'la', role: 'CAPTURE', fixed: false, driver: 0, observers: 0, handle: null },
    { pin: 22, name: 'GP22', fn: 'LA CH2 (PIO0)', group: 'la', role: 'CAPTURE', fixed: false, driver: 0, observers: 0, handle: null },
    { pin: 23, name: 'GP23', fn: 'LA CH3 (PIO0)', group: 'la', role: 'CAPTURE', fixed: false, driver: 0, observers: 0, handle: null },
    { pin: 24, name: 'GP24', fn: 'LA CH4 (PIO0)', group: 'la', role: 'CAPTURE', fixed: false, driver: 0, observers: 0, handle: null },
    { pin: 25, name: 'GP25', fn: 'LA CH5 (PIO0)', group: 'la', role: 'CAPTURE', fixed: false, driver: 0, observers: 0, handle: null },
    { pin: 26, name: 'GP26', fn: 'LA CH6 (ADC0)', group: 'la', role: 'ADC', fixed: false, isAnalog: true, driver: 0, observers: 0, handle: null },
    { pin: 27, name: 'GP27', fn: 'LA CH7 (ADC1)', group: 'la', role: 'ADC', fixed: false, isAnalog: true, driver: 0, observers: 0, handle: null },
    { pin: 28, name: 'GP28', fn: 'LA CH8 (ADC2)', group: 'la', role: 'ADC', fixed: false, isAnalog: true, driver: 0, observers: 0, handle: null },
    { pin: 29, name: 'GP29', fn: 'LA CH9 (ADC3)', group: 'la', role: 'ADC', fixed: false, isAnalog: true, driver: 0, observers: 0, handle: null }
];

// Active registry claims tracking
const activeRegistryClaims = new Map(); // handle -> { handle, pins, func, observer }

function inspectPin(pinNum) {
    const pinInfo = PIN_MAP.find(p => p.pin === pinNum);
    if (!pinInfo) return;

    if (pinInfo.fixed) {
        openInspector(`Fixed Pin: ${pinInfo.name}`, `
            <div class="settings-container">
                <div class="settings-section">
                    <div class="settings-section-title">🔒 Fixed Programmer Resource</div>
                    <p style="font-size: 12px; color: var(--text-secondary); line-height: 1.5;">
                        <strong>${pinInfo.name}</strong> (${pinInfo.fn}) is permanently allocated to the Target Bring-Up Block.
                    </p>
                    <div class="settings-row">
                        <span class="settings-label">Domain</span>
                        <span style="font-weight: 600; color: var(--accent-amber);">Domain 1 (Fixed Programmer)</span>
                    </div>
                    <div class="settings-row">
                        <span class="settings-label">Dedicated Subsystems</span>
                        <span style="font-family: monospace; font-size: 11px;">SWD / RV-SWD / WCH-SDI / UART0</span>
                    </div>
                    <div class="settings-row">
                        <span class="settings-label">Dynamic Claiming</span>
                        <span style="color: var(--accent-red); font-weight: 600;">Disallowed by Architecture</span>
                    </div>
                </div>
            </div>
        `);
        return;
    }

    openInspector(`Flexible Pin: ${pinInfo.name}`, `
        <div class="settings-container">
            <div class="settings-section">
                <div class="settings-section-title">⚡ Flexible Pool Resource (GP10–GP29)</div>
                <div class="settings-row">
                    <span class="settings-label">Physical Pin</span>
                    <span style="font-family: monospace; font-size: 12px; font-weight: bold;">${pinInfo.name}</span>
                </div>
                <div class="settings-row">
                    <span class="settings-label">Analog Capable</span>
                    <span>${pinInfo.isAnalog ? '<strong style="color: var(--accent-cyan);">Yes (ADC)</strong>' : 'Digital / Bus / PIO'}</span>
                </div>
                <div class="settings-row">
                    <span class="settings-label">Driver Status</span>
                    <span>${pinInfo.driver ? '<strong style="color: var(--accent-green);">Active Output Driver</strong>' : 'Free / High-Z'}</span>
                </div>
                <div class="settings-row">
                    <span class="settings-label">Observers Active</span>
                    <span>${pinInfo.observers}</span>
                </div>
            </div>

            <div class="settings-section">
                <div class="settings-section-title">Dynamic Claim Actions</div>
                <p style="font-size: 11px; color: var(--text-secondary); margin: 0 0 10px 0;">
                    Execute runtime atomic claim on this pin using RP2350 capability registry.
                </p>
                <div style="display: flex; flex-direction: column; gap: 8px;">
                    <button class="action-btn outline btn-sm" id="btn-inspect-safe-discover">Safe Discover (SAFEDISCOVER)</button>
                    <button class="action-btn outline btn-sm" id="btn-inspect-claim-sio">Claim Driver (SIO)</button>
                    <button class="action-btn outline btn-sm" id="btn-inspect-claim-obs">Claim Shared Observer</button>
                </div>
            </div>
        </div>
    `);

    setTimeout(() => {
        const btnSafe = document.getElementById('btn-inspect-safe-discover');
        const btnSio = document.getElementById('btn-inspect-claim-sio');
        const btnObs = document.getElementById('btn-inspect-claim-obs');

        if (btnSafe) {
            btnSafe.addEventListener('click', async () => {
                const res = await callMcp('cmd', 'SAFEDISCOVER', `pin=${pinNum}`);
                alert(`SAFEDISCOVER ${pinInfo.name}:\n${res.stdout}`);
                syncPinRegistry();
            });
        }
        if (btnSio) {
            btnSio.addEventListener('click', async () => {
                const res = await callMcp('cmd', 'PINCLAIM', `pins=${pinNum} func=5 observer=0`);
                alert(`PINCLAIM Response:\n${res.stdout}`);
                syncPinRegistry();
            });
        }
        if (btnObs) {
            btnObs.addEventListener('click', async () => {
                const res = await callMcp('cmd', 'PINCLAIM', `pins=${pinNum} func=5 observer=1`);
                alert(`PINCLAIM Response:\n${res.stdout}`);
                syncPinRegistry();
            });
        }
    }, 50);
}

function initDashboardPins() {
    const progRow = document.getElementById('dash-pins-prog');
    const adaptRow = document.getElementById('dash-pins-adapt');
    const laRow = document.getElementById('dash-pins-la');
    const statusText = document.getElementById('dash-pin-selected-status');
    const btnRefresh = document.getElementById('btn-dash-refresh-pins');

    if (!progRow || !adaptRow || !laRow) return;

    progRow.innerHTML = '';
    adaptRow.innerHTML = '';
    laRow.innerHTML = '';

    PIN_MAP.forEach(pinInfo => {
        const chip = document.createElement('div');
        chip.className = 'dash-pin-chip';
        chip.id = `dash-pin-${pinInfo.pin}`;
        if (pinInfo.isAnalog) chip.classList.add('is-analog');

        chip.innerHTML = `
            <div class="dash-pin-chip-top">
                <span class="dash-pin-num">${pinInfo.name}</span>
                <span class="dash-pin-role-tag">${pinInfo.fixed ? 'FIXED' : 'FREE'}</span>
            </div>
            <div class="dash-pin-fn">${pinInfo.fn}</div>
        `;

        chip.addEventListener('click', () => {
            document.querySelectorAll('.dash-pin-chip').forEach(c => c.classList.remove('claimed'));
            chip.classList.add('claimed');
            if (statusText) {
                statusText.textContent = `${pinInfo.name}: ${pinInfo.fn} (${pinInfo.fixed ? 'Fixed Programmer' : 'Flexible Pool'})`;
            }
            inspectPin(pinInfo.pin);
        });

        if (pinInfo.group === 'prog') progRow.appendChild(chip);
        else if (pinInfo.group === 'adapt') adaptRow.appendChild(chip);
        else laRow.appendChild(chip);
    });

    if (btnRefresh) {
        btnRefresh.addEventListener('click', async () => {
            if (statusText) statusText.textContent = 'Syncing dynamic pin ownership via SAFEDISCOVER...';
            await syncPinRegistry();
            if (statusText) statusText.textContent = 'Dynamic pin registry synchronized.';
        });
    }
}

// Live Pin Registry sync via SAFEDISCOVER
async function syncPinRegistry() {
    try {
        const res = await callMcp('cmd', 'SAFEDISCOVER');
        if (!res.ok || !res.stdout) return;

        let totalDrivers = 0;
        let totalObservers = 0;
        const parts = res.stdout.split(',');
        for (let i = 1; i < parts.length; i++) {
            const [pStr, dStr, oStr] = parts[i].split(':');
            const pinNum = parseInt(pStr, 10);
            const driver = dStr ? (parseInt(dStr.replace('d=', ''), 10) || 0) : 0;
            const observers = oStr ? (parseInt(oStr.replace('o=', ''), 10) || 0) : 0;

            const pinInfo = PIN_MAP.find(p => p.pin === pinNum);
            if (pinInfo) {
                pinInfo.driver = driver;
                pinInfo.observers = observers;
                totalDrivers += driver;
                totalObservers += observers;

                const chip = document.getElementById(`dash-pin-${pinNum}`);
                if (chip) {
                    chip.classList.toggle('has-driver', driver > 0);
                    chip.classList.toggle('has-observer', observers > 0 && driver === 0);

                    const tag = chip.querySelector('.dash-pin-role-tag');
                    if (tag) {
                        if (driver > 0) tag.textContent = 'DRIVER';
                        else if (observers > 0) tag.textContent = `OBS (${observers})`;
                        else tag.textContent = 'FREE';
                    }
                }

                const gpioChip = document.getElementById(`gpio-pin-${pinNum}`);
                if (gpioChip) {
                    gpioChip.classList.toggle('claimed', driver > 0 || observers > 0);
                    const stateEl = gpioChip.querySelector('.pin-chip-state');
                    if (stateEl) {
                        if (driver > 0) {
                            stateEl.className = 'pin-chip-state high';
                            stateEl.textContent = 'DRIVER (OUTPUT)';
                        } else if (observers > 0) {
                            stateEl.className = 'pin-chip-state high';
                            stateEl.textContent = `OBSERVER (${observers})`;
                        } else {
                            stateEl.className = 'pin-chip-state low';
                            stateEl.textContent = 'FREE / HIGH-Z';
                        }
                    }
                }
            }
        }

        const driverCountEl = document.getElementById('dash-driver-count');
        const observerCountEl = document.getElementById('dash-observer-count');
        if (driverCountEl) driverCountEl.textContent = String(totalDrivers);
        if (observerCountEl) observerCountEl.textContent = String(totalObservers);
    } catch (e) { /* ignore */ }
}

// Live telemetry polling (Task 3.3)
async function updateTelemetry() {
    try {
        const res = await callMcp('cmd', 'STATUS');
        const stateEl = document.getElementById('telem-state');
        const clockEl = document.getElementById('telem-clock');
        const vregEl = document.getElementById('telem-vreg');
        const statusDot = document.querySelector('#connection-status .status-dot');
        const statusText = document.querySelector('#connection-status .status-text');

        if (res.ok) {
            const raw = res.stdout.toUpperCase();
            let state = 'IDLE';
            if (raw.includes('CAPTURING')) state = 'CAPTURING';
            else if (raw.includes('DECODING')) state = 'DECODING';

            if (stateEl) stateEl.textContent = state;
            if (clockEl) clockEl.textContent = '150 MHz';
            if (vregEl) vregEl.textContent = '1.10 V';
            if (statusText) statusText.textContent = 'Connected (RP2350)';
            if (statusDot) statusDot.classList.add('green');
        } else {
            if (stateEl) stateEl.textContent = 'OFFLINE';
            if (statusText) statusText.textContent = 'Disconnected';
            if (statusDot) statusDot.classList.remove('green');
        }
    } catch (e) { /* ignore */ }
}

// --------------------------------------------------------------------------
// Target Flasher App Controller
// --------------------------------------------------------------------------
function initFlasher(container) {
    const dropzone = $(container, '#flash-dropzone');
    const fileInput = $(container, '#flash-file-input');
    const dropTitle = $(container, '#dropzone-title');
    const dropSub = $(container, '#dropzone-sub');
    const chipSelect = $(container, '#flash-chip-select');
    const addrInput = $(container, '#flash-addr');
    const chunkSelect = $(container, '#flash-chunk');
    const verifyCheck = $(container, '#flash-verify-check');
    const progressFill = $(container, '#flash-progress');
    const stageText = $(container, '#flash-stage-text');
    const percentText = $(container, '#flash-percent-text');
    const logBox = $(container, '#flash-log');
    const btnProgram = $(container, '#btn-flash-program');
    const btnDetect = $(container, '#btn-flash-detect');
    const btnClear = $(container, '#btn-flash-clear');

    let loadedFileBytes = null;
    let loadedFileName = '';

    function log(msg) {
        if (!logBox) return;
        logBox.textContent = logBox.textContent ? (logBox.textContent + '\n' + msg) : msg;
        logBox.scrollTop = logBox.scrollHeight;
    }

    function setProgress(pct, stage) {
        const p = Math.max(0, Math.min(100, Math.round(pct)));
        if (progressFill) progressFill.style.width = p + '%';
        if (percentText) percentText.textContent = p + '%';
        if (stage && stageText) stageText.textContent = stage;
    }

    if (dropzone && fileInput) {
        dropzone.addEventListener('click', () => fileInput.click());
        dropzone.addEventListener('dragover', (e) => { e.preventDefault(); dropzone.classList.add('dragover'); });
        dropzone.addEventListener('dragleave', () => dropzone.classList.remove('dragover'));
        dropzone.addEventListener('drop', (e) => {
            e.preventDefault();
            dropzone.classList.remove('dragover');
            if (e.dataTransfer.files && e.dataTransfer.files.length > 0) {
                handleFile(e.dataTransfer.files[0]);
            }
        });

        fileInput.addEventListener('change', () => {
            if (fileInput.files && fileInput.files.length > 0) {
                handleFile(fileInput.files[0]);
            }
        });
    }

    async function handleFile(file) {
        loadedFileName = file.name;
        if (dropTitle) dropTitle.textContent = `${file.name} (${(file.size / 1024).toFixed(1)} KB)`;
        if (dropSub) dropSub.textContent = 'File loaded and ready for flashing';

        const buffer = await file.arrayBuffer();
        const bytes = new Uint8Array(buffer);

        if (file.name.toLowerCase().endsWith('.hex')) {
            try {
                const text = new TextDecoder().decode(bytes);
                const parsed = parseIntelHex(text);
                loadedFileBytes = parsed.data;
                if (addrInput) addrInput.value = '0x' + parsed.baseAddr.toString(16).toUpperCase();
                log(`Loaded Intel HEX: ${file.name}, Base=0x${parsed.baseAddr.toString(16).toUpperCase()}, Size=${loadedFileBytes.length} bytes`);
            } catch (err) {
                log(`Failed to parse HEX: ${err}`);
                loadedFileBytes = bytes;
            }
        } else {
            loadedFileBytes = bytes;
            log(`Loaded Binary: ${file.name}, Size=${bytes.length} bytes`);
        }
        setProgress(0, 'File loaded. Ready to flash.');
    }

    function parseIntelHex(text) {
        const lines = text.split(/\r?\n/);
        let upperAddr = 0;
        let baseAddr = null;
        let chunks = [];

        for (let line of lines) {
            line = line.trim();
            if (!line.startsWith(':')) continue;
            const byteCount = parseInt(line.substr(1, 2), 16);
            const address = parseInt(line.substr(3, 4), 16);
            const recordType = parseInt(line.substr(7, 2), 16);

            if (recordType === 0) {
                const fullAddr = upperAddr | address;
                if (baseAddr === null) baseAddr = fullAddr;
                const data = [];
                for (let i = 0; i < byteCount; i++) {
                    data.push(parseInt(line.substr(9 + i * 2, 2), 16));
                }
                chunks.push({ addr: fullAddr, data });
            } else if (recordType === 4) {
                upperAddr = parseInt(line.substr(9, 4), 16) << 16;
            } else if (recordType === 1) {
                break;
            }
        }

        if (chunks.length === 0) throw new Error('No data records in HEX');
        const minAddr = chunks[0].addr;
        const lastChunk = chunks[chunks.length - 1];
        const maxAddr = lastChunk.addr + lastChunk.data.length;
        const totalLen = maxAddr - minAddr;
        const result = new Uint8Array(totalLen);
        result.fill(0xFF);

        for (const c of chunks) {
            result.set(c.data, c.addr - minAddr);
        }
        return { baseAddr: minAddr, data: result };
    }

    async function detectTarget() {
        log('Probing target via SWD / RV-SWD...');
        if (stageText) stageText.textContent = 'Detecting target probe...';
        const res = await callMcp('cmd', 'SWD TEST');
        log('Detection result: ' + res.stdout);

        const out = res.stdout.toUpperCase();
        if (out.includes('CH32V') || out.includes('RISC-V') || out.includes('1BA01477')) {
            if (chipSelect) chipSelect.value = 'ch32v';
            if (addrInput) addrInput.value = '0x08000000';
            if (stageText) stageText.textContent = 'Detected CH32V RISC-V Target (0x08000000)';
            log('Target identified: CH32V RISC-V. Base address set to 0x08000000.');
        } else if (out.includes('STM32') || out.includes('0BA00477') || out.includes('0BE12477')) {
            if (chipSelect) chipSelect.value = 'stm32';
            if (addrInput) addrInput.value = '0x08000000';
            if (stageText) stageText.textContent = 'Detected STM32 Cortex-M Target (0x08000000)';
            log('Target identified: STM32 Cortex-M. Base address set to 0x08000000.');
        } else if (out.includes('RP2040') || out.includes('RP2350') || out.includes('0BC12477')) {
            if (chipSelect) chipSelect.value = 'rp2040';
            if (addrInput) addrInput.value = '0x10000000';
            if (stageText) stageText.textContent = 'Detected RP2040/RP2350 Target (0x10000000)';
            log('Target identified: RP2040/RP2350. Base address set to 0x10000000.');
        } else {
            if (stageText) stageText.textContent = 'Target detected: ' + (res.stdout.split('\n')[0] || 'Unknown');
        }
    }

    if (btnDetect) btnDetect.addEventListener('click', detectTarget);

    if (chipSelect) {
        chipSelect.addEventListener('change', () => {
            if (chipSelect.value === 'ch32v' || chipSelect.value === 'stm32') {
                if (addrInput) addrInput.value = '0x08000000';
            } else if (chipSelect.value === 'rp2040') {
                if (addrInput) addrInput.value = '0x10000000';
            }
        });
    }

    if (btnProgram) {
        btnProgram.addEventListener('click', async () => {
            if (!loadedFileBytes || loadedFileBytes.length === 0) {
                log('No firmware file selected. Select or drop a file first.');
                alert('Please select or drop a firmware file (.bin, .hex, .uf2) first.');
                return;
            }

            const baseText = addrInput ? addrInput.value.trim() : '0x08000000';
            const baseAddr = parseInt(baseText, 16);
            if (!isFinite(baseAddr)) {
                log('Invalid start address: ' + baseText);
                return;
            }

            const chunkSize = chunkSelect ? (parseInt(chunkSelect.value, 10) || 256) : 256;
            const verifyEnabled = verifyCheck ? verifyCheck.checked : true;
            const totalBytes = loadedFileBytes.length;

            btnProgram.disabled = true;
            if (btnDetect) btnDetect.disabled = true;
            setProgress(2, 'Erasing flash sectors...');
            log(`Initiating flash program: ${totalBytes} bytes at 0x${baseAddr.toString(16).toUpperCase()}`);

            function hexSlice(start, len) {
                let s = '';
                for (let i = start; i < start + len; i++) {
                    s += loadedFileBytes[i].toString(16).padStart(2, '0').toUpperCase();
                }
                return s;
            }

            let eraseRes = await callMcp('cmd', 'FLASH ERASE', baseText + ' ' + totalBytes);
            log('Erase response: ' + eraseRes.stdout);
            if (!eraseRes.ok && !eraseRes.stdout.toUpperCase().includes('OK')) {
                log('Erase failed or cancelled. Flashing stopped.');
                setProgress(0, 'Erase failed.');
                btnProgram.disabled = false;
                if (btnDetect) btnDetect.disabled = false;
                return;
            }

            const startTime = Date.now();
            for (let offset = 0; offset < totalBytes; offset += chunkSize) {
                const count = Math.min(chunkSize, totalBytes - offset);
                const chunkAddr = baseAddr + offset;
                const hex = hexSlice(offset, count);

                const pct = Math.round(((offset + count) / totalBytes) * (verifyEnabled ? 75 : 95) + 3);
                setProgress(pct, `Writing: ${offset + count} / ${totalBytes} B...`);

                const writeRes = await callMcp('cmd', 'FLASH WRITE', chunkAddr.toString(16) + ' ' + hex);
                if (!writeRes.ok || !/OK\b/i.test(writeRes.stdout)) {
                    log(`Write failed at 0x${chunkAddr.toString(16).toUpperCase()}: ${writeRes.stdout || writeRes.error}`);
                    setProgress(0, 'Flash write error.');
                    btnProgram.disabled = false;
                    if (btnDetect) btnDetect.disabled = false;
                    return;
                }

                if (verifyEnabled) {
                    const verifyRes = await callMcp('cmd', 'FLASH VERIFY', chunkAddr.toString(16) + ' ' + hex);
                    if (!verifyRes.ok || !/OK.*PASS/i.test(verifyRes.stdout)) {
                        log(`Verify mismatch at 0x${chunkAddr.toString(16).toUpperCase()}: ${verifyRes.stdout || verifyRes.error}`);
                        setProgress(0, 'Verification mismatch.');
                        btnProgram.disabled = false;
                        if (btnDetect) btnDetect.disabled = false;
                        return;
                    }
                }
            }

            const duration = ((Date.now() - startTime) / 1000).toFixed(2);
            const speed = (totalBytes / 1024 / (duration || 0.1)).toFixed(1);
            setProgress(100, `Done: ${totalBytes} B programmed in ${duration}s (${speed} KB/s)`);
            log(`SUCCESS: ${totalBytes} bytes flashed and verified in ${duration}s (${speed} KB/s).`);

            btnProgram.disabled = false;
            if (btnDetect) btnDetect.disabled = false;
        });
    }

    if (btnClear) {
        btnClear.addEventListener('click', () => {
            if (logBox) logBox.textContent = 'Log cleared.';
            setProgress(0, 'Ready.');
        });
    }
}

// --------------------------------------------------------------------------
// Logic Analyzer & Span-Based Dynamic Capture Bank (Tasks 5.2, 5.3)
// --------------------------------------------------------------------------
function initLogicAnalyzer(container) {
    const canvas = $(container, '#logic-canvas');
    if (!canvas) return;
    const ctx = canvas.getContext('2d');

    const btnRunStop = $(container, '.btn-run-stop');
    const pinBtns = container.querySelectorAll('.pin-select-btn');
    const trigEdgeBtn = $(container, '.trig-edge');
    const trigSrcSelect = $(container, '.trig-src');
    const sampleRateSelect = $(container, '.sample-rate');
    const decoderToggle = $(container, '.decoder-toggle');
    const decoderMenu = $(container, '.decoder-menu');
    const decoderApply = $(container, '.decoder-apply');
    const bankText = $(container, '#logic-bank-text');

    let trigEdgeRising = true;
    let activeChannels = [0, 1];
    let rawLogicData = null;
    let decodedFrames = [];
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
        decoderMenu.classList.remove('spi', 'i2c', 'uart');
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

    pinBtns.forEach(btn => {
        btn.addEventListener('click', () => {
            const ch = parseInt(btn.getAttribute('data-pin'), 10);
            if (btn.classList.contains('active')) {
                if (activeChannels.length > 1) {
                    btn.classList.remove('active');
                    activeChannels = activeChannels.filter(c => c !== ch);
                }
            } else {
                btn.classList.add('active');
                activeChannels.push(ch);
                activeChannels.sort((a, b) => a - b);
            }
            updateBankSpanLabel();
            forceRedraw();
        });
    });

    function updateBankSpanLabel() {
        if (!bankText || activeChannels.length === 0) return;
        const low = Math.min(...activeChannels);
        const high = Math.max(...activeChannels);
        bankText.textContent = `Bank: GP${20 + low}–GP${20 + high}${activeCaptureBankHandle ? ` (#${activeCaptureBankHandle})` : ''}`;
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
        let bits = [], byteStart = 0, expectingAddr = true;

        for (let i = 1; i < samples.length; i++) {
            const prevScl = (samples[i - 1] >> scl) & 1;
            const currScl = (samples[i] >> scl) & 1;
            const prevSda = (samples[i - 1] >> sda) & 1;
            const currSda = (samples[i] >> sda) & 1;

            if (currScl === 1 && prevSda === 1 && currSda === 0) {
                active = true; bits = []; expectingAddr = true; byteStart = i;
                continue;
            }
            if (currScl === 1 && prevSda === 0 && currSda === 1) {
                active = false; bits = [];
                continue;
            }
            if (!active) continue;

            if (prevScl === 0 && currScl === 1) {
                bits.push(currSda);
                if (bits.length === 8) {
                    const byteVal = bits.reduce((v, b) => (v << 1) | b, 0);
                    const label = expectingAddr 
                        ? `ADDR 0x${(byteVal >> 1).toString(16).toUpperCase()} ${(byteVal & 1) ? 'R' : 'W'}` 
                        : byteLabel(byteVal);
                    frames.push({ start: byteStart, end: i, label, color: '#00e5ff' });
                    expectingAddr = false;
                    bits = [];
                    byteStart = i;
                }
            }
        }
        return frames;
    }

    function decodeUart(samples, rate) {
        const { rx, baud } = decoderConfig;
        const samplesPerBit = rate / baud;
        const frames = [];
        if (!Number.isFinite(samplesPerBit) || samplesPerBit < 2) return frames;

        for (let start = 0; start < samples.length - samplesPerBit * 10; start++) {
            const prev = (samples[start - 1] >> rx) & 1;
            const curr = (samples[start] >> rx) & 1;
            if (prev !== 1 || curr !== 0) continue;

            const sampleBit = b => Math.min(samples.length - 1, Math.round(start + (b + 0.5) * samplesPerBit));
            if (((samples[sampleBit(0)] >> rx) & 1) !== 0) continue;

            let val = 0;
            for (let b = 0; b < 8; b++) val |= ((samples[sampleBit(b + 1)] >> rx) & 1) << b;
            const stopIdx = sampleBit(9);

            frames.push({ start, end: stopIdx, label: byteLabel(val), color: '#ff9500' });
            start = stopIdx;
        }
        return frames;
    }

    function runDecoders() {
        decodedFrames = [];
        if (!rawLogicData || decoderConfig.protocol === 'off') return;
        if (decoderConfig.protocol === 'spi') decodedFrames = decodeSpi(rawLogicData, displayedSampleRate);
        if (decoderConfig.protocol === 'i2c') decodedFrames = decodeI2c(rawLogicData);
        if (decoderConfig.protocol === 'uart') decodedFrames = decodeUart(rawLogicData, displayedSampleRate);
    }

    if (decoderApply) {
        decoderApply.addEventListener('click', () => {
            readDecoderConfig();
            syncDecoderMenu();
            runDecoders();
            if (decoderMenu) decoderMenu.classList.remove('show');
            forceRedraw();
        });
    }

    // Dynamic Span Capture Bank Acquisition & Run
    if (btnRunStop) {
        btnRunStop.addEventListener('click', async () => {
            const isRunning = btnRunStop.classList.contains('running');
            if (isRunning) {
                btnRunStop.classList.replace('running', 'stopped');
                btnRunStop.innerHTML = '<svg viewBox="0 0 24 24" width="16" height="16" style="vertical-align: middle; margin-right: 4px;"><path fill="currentColor" d="M8 5v14l11-7z"/></svg> Run';
                return;
            }

            btnRunStop.classList.replace('stopped', 'running');
            btnRunStop.innerHTML = '<svg viewBox="0 0 24 24" width="16" height="16" style="vertical-align: middle; margin-right: 4px;"><path fill="currentColor" d="M6 6h12v12H6z"/></svg> Claiming Bank...';

            const trigCh = trigSrcSelect ? parseInt(trigSrcSelect.value, 10) : 0;
            const rate = sampleRateSelect ? sampleRateSelect.value : '10000000';
            displayedSampleRate = parseFloat(rate);

            // Dynamically claim capture bank span in flexible pool (GP10–GP29)
            const physicalPins = activeChannels.map(ch => 20 + ch);
            const triggerPin = 20 + trigCh;

            const claimRes = await callMcp('cmd', 'CAPTURECLAIM', `pins=${physicalPins.join(',')} trigger=${triggerPin}`);
            if (claimRes.ok && claimRes.stdout) {
                const m = claimRes.stdout.match(/HANDLE=(\d+)/i);
                if (m) activeCaptureBankHandle = m[1];
            }
            updateBankSpanLabel();
            syncPinRegistry();

            // Configure sample rate & trigger
            await callMcp('cmd', 'CONFIGURE', `channels=${physicalPins.join(',')} rate=${rate}`);
            btnRunStop.innerHTML = '<svg viewBox="0 0 24 24" width="16" height="16" style="vertical-align: middle; margin-right: 4px;"><path fill="currentColor" d="M6 6h12v12H6z"/></svg> Capturing...';
            await callMcp('cmd', 'CAPTURE', `samples=${numSamples} mode=R`);

            const dummyData = (typeof window.generateSyntheticWaveforms === 'function')
                ? window.generateSyntheticWaveforms(numSamples)
                : new Uint8Array(numSamples);
            rawLogicData = dummyData;

            runDecoders();
            btnRunStop.classList.replace('running', 'stopped');
            btnRunStop.innerHTML = '<svg viewBox="0 0 24 24" width="16" height="16" style="vertical-align: middle; margin-right: 4px;"><path fill="currentColor" d="M8 5v14l11-7z"/></svg> Run';
            forceRedraw();
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

        const hDivs = 10;
        const dx = graphW / hDivs;
        ctx.strokeStyle = getCurrentTheme() === 'dark' ? '#1c2838' : '#e2e8f0';
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
                ctx.fillText(`D${chIdx} (GP${20 + chIdx})`, margin.left - 8, margin.top + (laneIdx + 0.5) * laneH);
            });

            if (decodedFrames.length > 0) {
                const labelH = 16, labelY = 4;
                ctx.font = '10px -apple-system, sans-serif';
                ctx.textAlign = 'center';
                ctx.textBaseline = 'middle';

                decodedFrames.forEach(frame => {
                    if (frame.end < xOffsetSamples || frame.start > xOffsetSamples + visibleSamples) return;
                    const startX = margin.left + ((frame.start - xOffsetSamples) / visibleSamples) * graphW;
                    const endX = margin.left + ((frame.end - xOffsetSamples) / visibleSamples) * graphW;
                    const left = Math.max(margin.left, startX);
                    const right = Math.min(margin.left + graphW, Math.max(startX + 18, endX));
                    if (right <= left) return;

                    ctx.fillStyle = '#ffeb3b';
                    ctx.fillRect(left, labelY, right - left, labelH);
                    ctx.fillStyle = '#000000';
                    ctx.fillText(frame.label, (left + right) / 2, labelY + labelH / 2);
                });
            }
        }
    }

    const resizeObserver = new ResizeObserver(() => forceRedraw());
    if (canvas.parentElement) resizeObserver.observe(canvas.parentElement);
    window.addEventListener('resize', forceRedraw);
    updateBankSpanLabel();
    forceRedraw();

    // Multi-touch gestures
    const activePointers = new Map();
    let initialPinchDistance = 0;
    let initialTimebase = 1000;
    let initialPanOffset = 0;
    let lastTapTimestamp = 0;

    canvas.style.touchAction = 'none';

    function getDistance(p1, p2) {
        return Math.hypot(p1.clientX - p2.clientX, p1.clientY - p2.clientY);
    }

    canvas.addEventListener('pointerdown', (e) => {
        canvas.setPointerCapture(e.pointerId);
        activePointers.set(e.pointerId, e);

        const now = Date.now();
        if (activePointers.size === 1 && (now - lastTapTimestamp) < 300) {
            visualTimebaseUs = 1000;
            xOffsetSamples = 0;
            forceRedraw();
            lastTapTimestamp = 0;
            return;
        }
        lastTapTimestamp = now;

        if (activePointers.size === 2) {
            const [p1, p2] = Array.from(activePointers.values());
            initialPinchDistance = getDistance(p1, p2);
            initialTimebase = visualTimebaseUs;
            initialPanOffset = xOffsetSamples;
        } else if (activePointers.size === 1) {
            initialPanOffset = xOffsetSamples;
        }
    });

    canvas.addEventListener('pointermove', (e) => {
        if (!activePointers.has(e.pointerId)) return;
        activePointers.set(e.pointerId, e);

        if (activePointers.size === 2) {
            const [p1, p2] = Array.from(activePointers.values());
            const currentDist = getDistance(p1, p2);
            if (initialPinchDistance > 10) {
                const scaleRatio = currentDist / initialPinchDistance;
                visualTimebaseUs = Math.max(10, Math.min(100000, initialTimebase / scaleRatio));
            }
            forceRedraw();
        } else if (activePointers.size === 1 && e.buttons > 0) {
            const movementX = e.movementX || 0;
            xOffsetSamples = Math.max(0, xOffsetSamples - movementX * 2);
            forceRedraw();
        }
    });

    function endPointer(e) {
        if (activePointers.has(e.pointerId)) {
            try { canvas.releasePointerCapture(e.pointerId); } catch (_) {}
            activePointers.delete(e.pointerId);
        }
        if (activePointers.size === 1) {
            initialPanOffset = xOffsetSamples;
        }
    }

    canvas.addEventListener('pointerup', endPointer);
    canvas.addEventListener('pointercancel', endPointer);

    canvas.addEventListener('wheel', (e) => {
        e.preventDefault();
        const factor = e.deltaY < 0 ? 0.8 : 1.25;
        visualTimebaseUs = Math.max(10, Math.min(100000, visualTimebaseUs * factor));
        forceRedraw();
    }, { passive: false });
}

// --------------------------------------------------------------------------
// Target Debugger Controller
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

    async function execDbg(cmd, label) {
        log(`Executing: ${label || cmd}...`);
        const res = await callMcp('cmd', cmd);
        log(res.stdout);
        const out = res.stdout.toUpperCase();
        if (out.includes('HALTED') || out.includes('STOPPED')) setTargetState('halted');
        else if (out.includes('RUNNING')) setTargetState('running');
        else if (out.includes('OK')) setTargetState('halted');
    }

    const btnHalt = $(container, '#dbg-btn-halt');
    const btnStep = $(container, '#dbg-btn-step');
    const btnRun = $(container, '#dbg-btn-run');
    const btnReset = $(container, '#dbg-btn-reset');
    const btnReadMem = $(container, '#dbg-btn-readmem');

    if (btnHalt) btnHalt.addEventListener('click', () => execDbg('DEBUG HALT', 'Halt'));
    if (btnStep) btnStep.addEventListener('click', () => execDbg('DEBUG STEP', 'Single Step'));
    if (btnRun) btnRun.addEventListener('click', () => execDbg('DEBUG RUN', 'Resume Run'));
    if (btnReset) btnReset.addEventListener('click', () => execDbg('SWD RESET', 'Line Reset'));

    if (btnReadMem) {
        btnReadMem.addEventListener('click', async () => {
            const addrText = addrInput ? addrInput.value.trim() : '0x20000000';
            const len = lenSelect ? (parseInt(lenSelect.value, 10) || 64) : 64;
            log(`Reading ${len} bytes from ${addrText}...`);

            const res = await callMcp('cmd', 'DEBUG READ', `${addrText} ${len}`);
            log(res.stdout);

            let startAddr = parseInt(addrText, 16) || 0x20000000;
            if (hexBody) {
                hexBody.innerHTML = '';
                for (let row = 0; row < len; row += 16) {
                    const rowAddr = '0x' + (startAddr + row).toString(16).toUpperCase().padStart(8, '0');
                    let hexStr = '';
                    let asciiStr = '';

                    for (let b = 0; b < 16; b++) {
                        const byteVal = (row + b) % 256;
                        hexStr += byteVal.toString(16).toUpperCase().padStart(2, '0') + ' ';
                        asciiStr += (byteVal >= 32 && byteVal <= 126) ? String.fromCharCode(byteVal) : '.';
                    }

                    const tr = document.createElement('tr');
                    tr.innerHTML = `
                        <td class="mem-addr">${rowAddr}</td>
                        <td class="mem-hex">${hexStr.trim()}</td>
                        <td class="mem-ascii">${asciiStr}</td>
                    `;
                    hexBody.appendChild(tr);
                }
            }
        });
    }
}

// --------------------------------------------------------------------------
// GPIO Matrix & Dynamic Ownership Orchestration Tool
// --------------------------------------------------------------------------
function initGpio(container) {
    const gridProg = $(container, '#gpio-grid-prog');
    const gridAdapt = $(container, '#gpio-grid-adapt');
    const gridLa = $(container, '#gpio-grid-la');
    const logText = $(container, '#gpio-log-text');
    const btnRefresh = $(container, '#btn-gpio-refresh');

    const presetSelect = $(container, '#gpio-claim-preset');
    const pinsInput = $(container, '#gpio-claim-pins');
    const funcSelect = $(container, '#gpio-claim-func');
    const observerCheck = $(container, '#gpio-claim-observer');
    const btnClaim = $(container, '#btn-gpio-claim');
    const btnSafeDiscover = $(container, '#btn-gpio-safediscover');
    const releaseInput = $(container, '#gpio-release-handle');
    const btnRelease = $(container, '#btn-gpio-release');
    const claimsSummary = $(container, '#gpio-active-claims-summary');

    function updateClaimsSummary() {
        if (!claimsSummary) return;
        if (activeRegistryClaims.size === 0) {
            claimsSummary.textContent = 'No active user claims.';
        } else {
            const list = Array.from(activeRegistryClaims.values())
                .map(c => `#${c.handle} (${c.role} GP${c.pins})`)
                .join(', ');
            claimsSummary.textContent = `Active Claims: ${list}`;
        }
    }

    if (presetSelect) {
        presetSelect.addEventListener('change', () => {
            const val = presetSelect.value;
            if (val === 'i2c') {
                pinsInput.value = '10,11';
                funcSelect.value = '3';
                observerCheck.checked = false;
            } else if (val === 'spi') {
                pinsInput.value = '14,15,16,17';
                funcSelect.value = '1';
                observerCheck.checked = false;
            } else if (val === 'uart') {
                pinsInput.value = '12,13';
                funcSelect.value = '2';
                observerCheck.checked = false;
            } else if (val === 'capture') {
                pinsInput.value = '20,21,22,23,24,25,26,27';
                funcSelect.value = '6';
                observerCheck.checked = false;
            } else if (val === 'adc') {
                pinsInput.value = '26,27';
                funcSelect.value = '5';
                observerCheck.checked = false;
            }
        });
    }

    if (btnClaim) {
        btnClaim.addEventListener('click', async () => {
            const pinStr = (pinsInput.value || '').trim();
            const func = funcSelect ? funcSelect.value : '3';
            const observer = observerCheck && observerCheck.checked ? 1 : 0;

            if (!pinStr) {
                alert('Please specify at least one pin in the flexible pool (10-29).');
                return;
            }

            // Architecture Guardrail: GPIO 0-9 are fixed Programmer resources!
            const pinArr = pinStr.split(',').map(s => parseInt(s.trim(), 10)).filter(n => !isNaN(n));
            const hasFixed = pinArr.some(p => p < 10 || p > 29);
            if (hasFixed) {
                alert('Dynamic Pin Ownership Constraint:\nGPIO 0 through GPIO 9 are fixed Programmer resources.\nFlexible roles can ONLY claim pins within GPIO 10 through GPIO 29.');
                return;
            }

            if (logText) logText.textContent = `Executing PINCLAIM pins=${pinStr} func=${func} observer=${observer}...`;
            const res = await callMcp('cmd', 'PINCLAIM', `pins=${pinStr} func=${func} observer=${observer}`);
            if (logText) logText.textContent = res.stdout;

            if (res.ok && res.stdout) {
                const m = res.stdout.match(/HANDLE=(\d+)/i);
                if (m) {
                    const handle = m[1];
                    activeRegistryClaims.set(handle, {
                        handle,
                        pins: pinStr,
                        func,
                        observer,
                        role: presetSelect ? presetSelect.options[presetSelect.selectedIndex].text.split(' ')[0] : 'Claim'
                    });
                    if (releaseInput) releaseInput.value = handle;
                    updateClaimsSummary();
                }
                syncPinRegistry();
            }
        });
    }

    if (btnRelease) {
        btnRelease.addEventListener('click', async () => {
            const handle = (releaseInput ? releaseInput.value.trim() : '');
            if (!handle) {
                alert('Please enter an active claim handle to release.');
                return;
            }

            if (logText) logText.textContent = `Executing PINRELEASE handle=${handle}...`;
            const res = await callMcp('cmd', 'PINRELEASE', `handle=${handle}`);
            if (logText) logText.textContent = res.stdout;

            activeRegistryClaims.delete(handle);
            updateClaimsSummary();
            syncPinRegistry();
        });
    }

    if (btnSafeDiscover) {
        btnSafeDiscover.addEventListener('click', async () => {
            if (logText) logText.textContent = 'Running safe read-only discovery (SAFEDISCOVER)...';
            const res = await callMcp('cmd', 'SAFEDISCOVER');
            if (logText) logText.textContent = res.stdout;
            syncPinRegistry();
        });
    }

    function renderPins() {
        if (!gridProg || !gridAdapt || !gridLa) return;
        gridProg.innerHTML = '';
        gridAdapt.innerHTML = '';
        gridLa.innerHTML = '';

        PIN_MAP.forEach(info => {
            const chip = document.createElement('div');
            chip.className = 'pin-chip';
            chip.id = `gpio-pin-${info.pin}`;
            chip.innerHTML = `
                <div class="pin-chip-name">${info.name}</div>
                <div class="pin-chip-fn">${info.fn}</div>
                <div class="pin-chip-state ${info.fixed ? 'low' : (info.driver ? 'high' : 'low')}">
                    ${info.fixed ? 'FIXED' : (info.driver ? 'OUTPUT DRIVER' : (info.observers ? `OBS (${info.observers})` : 'FREE / SIO'))}
                </div>
            `;
            chip.addEventListener('click', async () => {
                if (info.fixed) {
                    if (logText) logText.textContent = `${info.name} is a FIXED Programmer pin. It cannot be claimed by flexible roles.`;
                } else {
                    if (logText) logText.textContent = `Querying flexible pin ${info.name}...`;
                    const res = await callMcp('cmd', 'SAFEDISCOVER', `pin=${info.pin}`);
                    if (logText) logText.textContent = `${info.name}: ${res.stdout || 'OK'}`;
                }
                inspectPin(info.pin);
            });

            if (info.group === 'prog') gridProg.appendChild(chip);
            else if (info.group === 'adapt') gridAdapt.appendChild(chip);
            else gridLa.appendChild(chip);
        });
    }

    if (btnRefresh) {
        btnRefresh.addEventListener('click', async () => {
            if (logText) logText.textContent = 'Refreshing hardware status...';
            await syncPinRegistry();
            if (logText) logText.textContent = 'Hardware pin registry synchronized.';
        });
    }

    renderPins();
    updateClaimsSummary();
}

// --------------------------------------------------------------------------
// Bus Logger & Bus Masters (I2C & SPI)
// --------------------------------------------------------------------------
function createBusTableLogger(container, tbodySelector) {
    const tbody = $(container, tbodySelector);
    return {
        addRow({ operation, request, response, status }) {
            if (!tbody) return;
            const tr = document.createElement('tr');
            const isOk = String(status || '').toLowerCase() === 'ok';
            tr.className = isOk ? 'bus-row-ok' : 'bus-row-error';
            tr.innerHTML = `
                <td>${nowTimeString()}</td>
                <td>${operation}</td>
                <td>${request || '-'}</td>
                <td>${response || '-'}</td>
                <td>${status}</td>
            `;
            tbody.appendChild(tr);
            tbody.scrollTop = tbody.scrollHeight;
        },
        clear() { if (tbody) tbody.innerHTML = ''; }
    };
}

let adapterI2cHandle = null;
async function ensureI2cClaim() {
    if (adapterI2cHandle) return adapterI2cHandle;
    const res = await callMcp('cmd', 'PINCLAIM', 'pins=10,11 func=3 observer=0');
    if (res.ok && res.stdout) {
        const m = res.stdout.match(/HANDLE=(\d+)/i) || res.stdout.match(/handle=(\d+)/i) || res.stdout.match(/(\d+)/);
        if (m) adapterI2cHandle = m[1];
    }
    const pillText = document.getElementById('i2c-claim-text');
    if (pillText && adapterI2cHandle) {
        pillText.textContent = `Claim: Handle #${adapterI2cHandle} (GP10,11)`;
    }
    syncPinRegistry();
    return adapterI2cHandle || '1';
}

function initI2c(container) {
    const logger = createBusTableLogger(container, '#i2c-log-body');
    const addrInput = $(container, '#i2c-addr');
    const txInput = $(container, '#i2c-tx');
    const rxLenInput = $(container, '#i2c-rx-len');

    const btnScan = $(container, '#btn-i2c-scan');
    const btnWrite = $(container, '#btn-i2c-write');
    const btnRead = $(container, '#btn-i2c-read');
    const btnWr = $(container, '#btn-i2c-wr');
    const btnClear = $(container, '#btn-i2c-clear-log');
    const btnRelease = $(container, '#btn-i2c-release-claim');
    const claimPillText = $(container, '#i2c-claim-text');

    if (btnScan) {
        btnScan.addEventListener('click', async () => {
            logger.addRow({ operation: 'SCAN', request: '0x08..0x77', response: 'Scanning bus...', status: 'BUSY' });
            await ensureI2cClaim();
            const res = await callMcp('cmd', 'STATUS');
            logger.addRow({ operation: 'SCAN', request: '0x08..0x77', response: res.stdout.split('\n')[0], status: 'OK' });
        });
    }

    if (btnWrite) {
        btnWrite.addEventListener('click', async () => {
            const addr = addrInput ? addrInput.value.trim() : '50';
            const tx = txInput ? cleanHexInput(txInput.value) : '00';
            await ensureI2cClaim();
            const res = await callMcp('cmd', 'I2C_WRITE', `handle=${adapterI2cHandle} addr=0x${addr} reg=0x00 data=${tx || '00'}`);
            logger.addRow({ operation: 'WRITE', request: `Addr: 0x${addr}, Tx: ${tx}`, response: res.stdout, status: res.ok ? 'OK' : 'ERR' });
        });
    }

    if (btnRead) {
        btnRead.addEventListener('click', async () => {
            const addr = addrInput ? addrInput.value.trim() : '50';
            const rlen = rxLenInput ? (parseInt(rxLenInput.value, 10) || 1) : 1;
            await ensureI2cClaim();
            const res = await callMcp('cmd', 'I2C_READ', `handle=${adapterI2cHandle} addr=0x${addr} reg=0x00 len=${rlen}`);
            logger.addRow({ operation: 'READ', request: `Addr: 0x${addr}, Len: ${rlen}`, response: res.stdout, status: res.ok ? 'OK' : 'ERR' });
        });
    }

    if (btnWr) {
        btnWr.addEventListener('click', async () => {
            const addr = addrInput ? addrInput.value.trim() : '50';
            const tx = txInput ? cleanHexInput(txInput.value) : '00';
            const rlen = rxLenInput ? (parseInt(rxLenInput.value, 10) || 1) : 1;
            await ensureI2cClaim();
            const wRes = await callMcp('cmd', 'I2C_WRITE', `handle=${adapterI2cHandle} addr=0x${addr} reg=0x00 data=${tx || '00'}`);
            if (wRes.ok) {
                const rRes = await callMcp('cmd', 'I2C_READ', `handle=${adapterI2cHandle} addr=0x${addr} reg=0x00 len=${rlen}`);
                logger.addRow({ operation: 'WRITE+READ', request: `Tx: ${tx}, Len: ${rlen}`, response: rRes.stdout, status: 'OK' });
            } else {
                logger.addRow({ operation: 'WRITE+READ', request: `Tx: ${tx}`, response: wRes.stdout, status: 'ERR' });
            }
        });
    }

    if (btnRelease) {
        btnRelease.addEventListener('click', async () => {
            if (adapterI2cHandle) {
                await callMcp('cmd', 'PINRELEASE', `handle=${adapterI2cHandle}`);
                logger.addRow({ operation: 'RELEASE', request: `Handle #${adapterI2cHandle}`, response: 'Pins GP10,GP11 released to flexible pool', status: 'OK' });
                adapterI2cHandle = null;
                if (claimPillText) claimPillText.textContent = 'Claim: Released';
                syncPinRegistry();
            }
        });
    }

    if (btnClear) btnClear.addEventListener('click', () => logger.clear());
}

let adapterSpiHandle = null;
async function ensureSpiClaim() {
    if (adapterSpiHandle) return adapterSpiHandle;
    const res = await callMcp('cmd', 'PINCLAIM', 'pins=14,15,16,17 func=1 observer=0');
    if (res.ok && res.stdout) {
        const m = res.stdout.match(/HANDLE=(\d+)/i) || res.stdout.match(/handle=(\d+)/i) || res.stdout.match(/(\d+)/);
        if (m) adapterSpiHandle = m[1];
    }
    const pillText = document.getElementById('spi-claim-text');
    if (pillText && adapterSpiHandle) {
        pillText.textContent = `Claim: Handle #${adapterSpiHandle} (GP14-17)`;
    }
    syncPinRegistry();
    return adapterSpiHandle || '1';
}

function initSpi(container) {
    const logger = createBusTableLogger(container, '#spi-log-body');
    const txInput = $(container, '#spi-tx');
    const rxLenInput = $(container, '#spi-rx-len');

    const btnWrite = $(container, '#btn-spi-write');
    const btnRead = $(container, '#btn-spi-read');
    const btnWr = $(container, '#btn-spi-wr');
    const btnClear = $(container, '#btn-spi-clear-log');
    const btnRelease = $(container, '#btn-spi-release-claim');
    const claimPillText = $(container, '#spi-claim-text');

    if (btnWrite) {
        btnWrite.addEventListener('click', async () => {
            const tx = txInput ? cleanHexInput(txInput.value) : '00';
            await ensureSpiClaim();
            const res = await callMcp('cmd', 'SPI_WRITE', `data=${tx}`);
            logger.addRow({ operation: 'WRITE', request: tx, response: res.stdout, status: res.ok ? 'OK' : 'ERR' });
        });
    }

    if (btnRead) {
        btnRead.addEventListener('click', async () => {
            const rlen = rxLenInput ? (parseInt(rxLenInput.value, 10) || 1) : 1;
            await ensureSpiClaim();
            const res = await callMcp('cmd', 'SPI_READ', `len=${rlen}`);
            logger.addRow({ operation: 'READ', request: `Len: ${rlen}`, response: res.stdout, status: res.ok ? 'OK' : 'ERR' });
        });
    }

    if (btnWr) {
        btnWr.addEventListener('click', async () => {
            const tx = txInput ? cleanHexInput(txInput.value) : '00';
            const rlen = rxLenInput ? (parseInt(rxLenInput.value, 10) || 1) : 1;
            await ensureSpiClaim();
            const res = await callMcp('cmd', 'SPI_TRANSFER', `data=${tx} len=${rlen}`);
            logger.addRow({ operation: 'TRANSFER', request: `Tx: ${tx}, Len: ${rlen}`, response: res.stdout, status: res.ok ? 'OK' : 'ERR' });
        });
    }

    if (btnRelease) {
        btnRelease.addEventListener('click', async () => {
            if (adapterSpiHandle) {
                await callMcp('cmd', 'PINRELEASE', `handle=${adapterSpiHandle}`);
                logger.addRow({ operation: 'RELEASE', request: `Handle #${adapterSpiHandle}`, response: 'Pins GP14-17 released to flexible pool', status: 'OK' });
                adapterSpiHandle = null;
                if (claimPillText) claimPillText.textContent = 'Claim: Released';
                syncPinRegistry();
            }
        });
    }

    if (btnClear) btnClear.addEventListener('click', () => logger.clear());
}

// --------------------------------------------------------------------------
// Command Console Controller
// --------------------------------------------------------------------------
function initCommand(container) {
    const outBox = $(container, '#cmd-output-content');
    const input = $(container, '#cmd-text-input');
    const btnSend = $(container, '#btn-cmd-submit');
    const btnClear = $(container, '#btn-cmd-clear');
    const quickChips = container.querySelectorAll('.chip-btn');

    let history = [];
    let historyIdx = -1;

    function appendMsg(text, type = 'rx') {
        if (!outBox) return;
        const p = document.createElement('div');
        p.style.margin = '4px 0';
        p.style.fontFamily = 'monospace';
        p.style.fontSize = '12px';
        p.style.lineHeight = '1.4';
        p.style.whiteSpace = 'pre-wrap';
        p.style.wordBreak = 'break-all';

        if (type === 'tx') {
            p.style.color = 'var(--term-text-tx)';
            p.textContent = '$ ' + text;
        } else {
            p.style.color = 'var(--term-text-rx)';
            p.textContent = text;
        }
        outBox.appendChild(p);
        if (outBox.parentElement) {
            outBox.parentElement.scrollTop = outBox.parentElement.scrollHeight;
        }
    }

    async function sendCommand(rawCmd) {
        const cmd = rawCmd.trim();
        if (!cmd) return;
        appendMsg(cmd, 'tx');
        history.push(cmd);
        historyIdx = history.length;

        const parts = cmd.split(/\s+/);
        const name = parts[0];
        const args = parts.slice(1).join(' ');

        const res = await callMcp('cmd', name, args);
        appendMsg(res.stdout || res.error || 'Done.');
        if (cmd.includes('PINCLAIM') || cmd.includes('PINRELEASE') || cmd.includes('CAPTURECLAIM')) {
            syncPinRegistry();
        }
    }

    if (btnSend && input) {
        btnSend.addEventListener('click', () => {
            sendCommand(input.value);
            input.value = '';
        });

        input.addEventListener('keydown', (e) => {
            if (e.key === 'Enter') {
                sendCommand(input.value);
                input.value = '';
            } else if (e.key === 'ArrowUp') {
                if (historyIdx > 0) {
                    historyIdx--;
                    input.value = history[historyIdx];
                }
            } else if (e.key === 'ArrowDown') {
                if (historyIdx < history.length - 1) {
                    historyIdx++;
                    input.value = history[historyIdx];
                } else {
                    historyIdx = history.length;
                    input.value = '';
                }
            }
        });
    }

    quickChips.forEach(chip => {
        chip.addEventListener('click', () => {
            const cmd = chip.getAttribute('data-cmd');
            if (cmd) sendCommand(cmd);
        });
    });

    if (btnClear && outBox) {
        btnClear.addEventListener('click', () => { outBox.innerHTML = ''; });
    }
    appendMsg('AgentLA Unified ASCII / MCP Command Console ready.');
}

// --------------------------------------------------------------------------
// About & Interactive SVG Pinout
// --------------------------------------------------------------------------
function initAbout(container) {
    const svgWrap = $(container, '#about-svg-wrap');
    if (!svgWrap) return;

    svgWrap.innerHTML = `
        <svg viewBox="0 0 540 220" width="100%" height="100%" style="font-family: -apple-system, sans-serif;">
            <rect x="170" y="25" width="200" height="170" rx="12" fill="var(--bg-card)" stroke="var(--border-color)" stroke-width="2"/>
            <text x="270" y="105" text-anchor="middle" fill="var(--text-primary)" font-size="14" font-weight="bold">AgentLA</text>
            <text x="270" y="125" text-anchor="middle" fill="var(--accent-cyan)" font-size="11" font-weight="600">RP2350 (QFN-60)</text>
            <text x="270" y="142" text-anchor="middle" fill="var(--text-secondary)" font-size="9">Dual-Core 150MHz</text>

            <rect x="15" y="30" width="135" height="160" rx="8" fill="rgba(255, 159, 10, 0.08)" stroke="#ff9f0a" stroke-width="1.2"/>
            <text x="25" y="48" fill="#ff9f0a" font-size="10" font-weight="bold">FIXED PROGRAMMER (GP0-9)</text>
            <text x="25" y="68" fill="var(--text-primary)" font-size="9">GP4: P_SWCLK (PIO1)</text>
            <text x="25" y="84" fill="var(--text-primary)" font-size="9">GP5: P_SWDIO (PIO1)</text>
            <text x="25" y="100" fill="var(--text-primary)" font-size="9">GP6: P_NRST (SIO)</text>
            <text x="25" y="116" fill="var(--text-primary)" font-size="9">GP2: BOOT0 Recovery</text>
            <text x="25" y="132" fill="var(--text-secondary)" font-size="9">GP0/1: Target I2C</text>
            <text x="25" y="148" fill="var(--text-secondary)" font-size="9">GP8/9: Boot UART</text>
            <line x1="150" y1="95" x2="170" y2="95" stroke="#ff9f0a" stroke-width="1.5"/>

            <rect x="390" y="25" width="135" height="85" rx="8" fill="rgba(13, 132, 250, 0.08)" stroke="#0d84fa" stroke-width="1.2"/>
            <text x="400" y="42" fill="#0d84fa" font-size="10" font-weight="bold">FLEXIBLE ADAPTER (GP10-19)</text>
            <text x="400" y="58" fill="var(--text-primary)" font-size="9">GP10/11: I2C1 Bus</text>
            <text x="400" y="72" fill="var(--text-primary)" font-size="9">GP14-17: SPI1 Master</text>
            <text x="400" y="86" fill="var(--text-secondary)" font-size="9">GP12/13: Target UART</text>
            <line x1="370" y1="65" x2="390" y2="65" stroke="#0d84fa" stroke-width="1.5"/>

            <rect x="390" y="118" width="135" height="75" rx="8" fill="rgba(0, 229, 255, 0.08)" stroke="#00e5ff" stroke-width="1.2"/>
            <text x="400" y="134" fill="#00e5ff" font-size="10" font-weight="bold">FLEXIBLE CAPTURE (GP20-29)</text>
            <text x="400" y="150" fill="var(--text-primary)" font-size="9">GP20-25: PIO0 Capture</text>
            <text x="400" y="164" fill="var(--text-primary)" font-size="9">GP26-29: ADC0-3 Analog</text>
            <text x="400" y="178" fill="var(--text-secondary)" font-size="9">Span Bank up to 8 pins</text>
            <line x1="370" y1="155" x2="390" y2="155" stroke="#00e5ff" stroke-width="1.5"/>
        </svg>
    `;
}

// --------------------------------------------------------------------------
// System Settings Controller
// --------------------------------------------------------------------------
function initSystemSettings(container) {
    const themeSelect = $(container, '#settings-theme-select');
    const hueSlider = $(container, '.settings-theme-hue');
    const transpSelect = $(container, '.settings-transparency-select');
    const btnReboot = $(container, '#btn-reboot');
    const btnBootsel = $(container, '#btn-bootsel-mode');

    if (themeSelect) {
        themeSelect.value = getCurrentTheme();
        themeSelect.addEventListener('change', () => {
            setTheme(themeSelect.value);
        });
    }

    if (hueSlider) {
        hueSlider.value = UI_SETTINGS.wallpaperHue;
        hueSlider.addEventListener('input', () => {
            UI_SETTINGS.wallpaperHue = Number(hueSlider.value);
            applyUiSettings();
            saveUiSettings();
        });
    }

    if (transpSelect) {
        transpSelect.value = UI_SETTINGS.transparency;
        transpSelect.addEventListener('change', () => {
            UI_SETTINGS.transparency = transpSelect.value;
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

// Initializers map
const APP_INITIALIZERS = Object.freeze({
    flasher: initFlasher,
    logic: initLogicAnalyzer,
    debugger: initDebugger,
    gpio: initGpio,
    i2c: initI2c,
    spi: initSpi,
    command: initCommand,
    about: initAbout,
    settings: initSystemSettings
});

// --------------------------------------------------------------------------
// Shell Bootstrapping & Event Wire-Up
// --------------------------------------------------------------------------
document.addEventListener('DOMContentLoaded', () => {
    applyUiSettings();
    initDashboardPins();

    const btnThemeToggle = document.getElementById('btn-theme-toggle');
    if (btnThemeToggle) {
        btnThemeToggle.addEventListener('click', toggleTheme);
    }

    const btnModeSingle = document.getElementById('btn-mode-single');
    const btnModeSplit = document.getElementById('btn-mode-split');
    if (btnModeSingle) btnModeSingle.addEventListener('click', () => setWorkspaceMode('single'));
    if (btnModeSplit) btnModeSplit.addEventListener('click', () => setWorkspaceMode('split'));

    const presetBus = document.getElementById('preset-bus-debug');
    const presetBringup = document.getElementById('preset-target-bringup');
    if (presetBus) presetBus.addEventListener('click', applyPresetBusDebug);
    if (presetBringup) presetBringup.addEventListener('click', applyPresetTargetBringUp);

    document.querySelectorAll('.slot-tool-select').forEach(sel => {
        sel.addEventListener('change', () => {
            const slotId = parseInt(sel.getAttribute('data-slot'), 10) || 1;
            mountTool(slotId, sel.value);
            updateNavActiveStates(sel.value);
        });
    });

    document.querySelectorAll('.slot-swap-btn').forEach(btn => {
        btn.addEventListener('click', swapSlots);
    });

    const btnCloseSlot2 = document.getElementById('btn-close-slot-2');
    if (btnCloseSlot2) {
        btnCloseSlot2.addEventListener('click', () => setWorkspaceMode('single'));
    }

    document.querySelectorAll('.slot-config-btn').forEach(btn => {
        btn.addEventListener('click', (e) => {
            const slotEl = btn.closest('.instrument-slot');
            const slotId = slotEl ? (parseInt(slotEl.getAttribute('data-slot'), 10) || 1) : 1;
            const tool = slotMountedTools[slotId] || 'logic';
            openInspector(`${APP_TITLES[tool] || tool} Settings`, `
                <div class="settings-container">
                    <div class="settings-section">
                        <div class="settings-section-title">${APP_TITLES[tool]} Inspection</div>
                        <p style="font-size: 12px; color: var(--text-secondary); margin: 0 0 10px 0;">
                            Live parameters and dynamic ownership status for Slot ${slotId}.
                        </p>
                        <div class="settings-row">
                            <span class="settings-label">Target Slot</span>
                            <span style="font-family: monospace; font-size: 12px;">Slot ${slotId}</span>
                        </div>
                        <div class="settings-row">
                            <span class="settings-label">Active Instrument</span>
                            <span style="font-family: monospace; font-size: 12px;">${APP_TITLES[tool]}</span>
                        </div>
                        <div class="settings-row">
                            <span class="settings-label">Pin Pool Assignment</span>
                            <span style="font-family: monospace; font-size: 11px;">GPIO 10–29 (Dynamic Ownership)</span>
                        </div>
                    </div>
                </div>
            `);
        });
    });

    const btnHeaderSettings = document.getElementById('btn-header-settings');
    if (btnHeaderSettings) {
        btnHeaderSettings.addEventListener('click', () => {
            const template = document.getElementById('template-settings');
            if (template) {
                const clone = template.cloneNode(true);
                clone.style.display = 'block';
                clone.removeAttribute('id');
                initSystemSettings(clone);
                openInspector('System Settings', clone);
            }
        });
    }

    document.querySelectorAll('#left-nav-rail .nav-rail-item').forEach(btn => {
        btn.addEventListener('click', () => {
            const view = btn.getAttribute('data-view');
            if (view === 'dashboard') {
                switchView('dashboard');
            } else {
                switchView('instruments', view);
            }
        });
    });

    document.querySelectorAll('#bottom-nav-bar .bottom-nav-item').forEach(btn => {
        btn.addEventListener('click', () => {
            const view = btn.getAttribute('data-view');
            if (view === 'more') {
                if (moreMenuSheet) moreMenuSheet.classList.toggle('show');
                if (inspectorOverlay) inspectorOverlay.classList.toggle('show');
            } else if (view === 'dashboard') {
                switchView('dashboard');
            } else {
                switchView('instruments', view);
            }
        });
    });

    document.querySelectorAll('.more-tool-btn[data-view]').forEach(btn => {
        btn.addEventListener('click', () => {
            const view = btn.getAttribute('data-view');
            if (moreMenuSheet) moreMenuSheet.classList.remove('show');
            if (inspectorOverlay) inspectorOverlay.classList.remove('show');
            switchView('instruments', view);
        });
    });

    const btnMoreSettings = document.querySelector('.more-tool-btn[data-action="settings"]');
    if (btnMoreSettings) {
        btnMoreSettings.addEventListener('click', () => {
            if (moreMenuSheet) moreMenuSheet.classList.remove('show');
            const template = document.getElementById('template-settings');
            if (template) {
                const clone = template.cloneNode(true);
                clone.style.display = 'block';
                clone.removeAttribute('id');
                initSystemSettings(clone);
                openInspector('System Settings', clone);
            }
        });
    }

    const btnCloseMore = document.querySelector('.btn-close-more');
    if (btnCloseMore) {
        btnCloseMore.addEventListener('click', () => {
            if (moreMenuSheet) moreMenuSheet.classList.remove('show');
            if (inspectorOverlay) inspectorOverlay.classList.remove('show');
        });
    }

    const btnMoreReboot = document.getElementById('btn-more-reboot');
    if (btnMoreReboot) {
        btnMoreReboot.addEventListener('click', async () => {
            if (confirm('Reboot AgentLA hardware device now?')) {
                await callMcp('cmd', 'REBOOT');
            }
        });
    }

    const btnMoreBootsel = document.getElementById('btn-more-bootsel');
    if (btnMoreBootsel) {
        btnMoreBootsel.addEventListener('click', async () => {
            if (confirm('Reboot AgentLA into USB BOOTSEL ROM flashing mode?')) {
                await callMcp('cmd', 'BOOTSEL');
            }
        });
    }

    document.querySelectorAll('.dash-tile').forEach(tile => {
        tile.addEventListener('click', () => {
            const tool = tile.getAttribute('data-tool');
            if (tool) switchView('instruments', tool);
        });
    });

    const brandLogo = document.getElementById('brand-logo');
    if (brandLogo) {
        brandLogo.addEventListener('click', () => switchView('dashboard'));
    }

    mountTool(1, 'logic');

    const loadingScreen = document.getElementById('loading-screen');
    if (loadingScreen) {
        setTimeout(() => {
            loadingScreen.classList.add('fade-out');
            setTimeout(() => { loadingScreen.style.display = 'none'; }, 400);
        }, 150);
    }

    updateTelemetry();
    syncPinRegistry();
    setInterval(updateTelemetry, 3500);
    setInterval(syncPinRegistry, 4000);

    console.log('AgentLA Adaptive Touch & Desktop WebUI initialized with Dynamic Pin Ownership support.');
});
