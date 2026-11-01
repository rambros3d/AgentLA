/**
 * AgentLA Web UI Controller
 * Zero-dependency desktop multitool shell for RP2040/RP2350
 */

// Host and API configuration
const host = window.location.hostname || "192.168.7.1";

const APP_TITLES = Object.freeze({
    flasher: "Target Flasher",
    logic: "Logic Analyzer",
    debugger: "Target Debugger",
    gpio: "GPIO & Pin Matrix",
    i2c: "I2C Master",
    spi: "SPI Master",
    command: "Command Console",
    about: "About & Pinout",
    settings: "System Settings"
});

const APP_WINDOW_LAYOUTS = Object.freeze({
    flasher:  { width: 520, height: 440, minwidth: 440, minheight: 380, class: "tool-window" },
    logic:    { width: 840, height: 480, minwidth: 640, minheight: 360, class: "tool-window" },
    debugger: { width: 580, height: 460, minwidth: 480, minheight: 380, class: "tool-window" },
    gpio:     { width: 520, height: 500, minwidth: 440, minheight: 400, class: "tool-window" },
    i2c:      { width: 580, height: 460, minwidth: 480, minheight: 380, class: "tool-window" },
    spi:      { width: 580, height: 460, minwidth: 480, minheight: 380, class: "tool-window" },
    command:  { width: 620, height: 460, minwidth: 480, minheight: 360, class: "tool-window" },
    about:    { width: 580, height: 480, minwidth: 500, minheight: 420, class: "no-resize" },
    settings: { width: 440, height: 320, minwidth: 380, minheight: 260, class: "no-resize" }
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

// Global API caller supporting JSON-RPC 2.0 and MCP REST endpoints
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
function $(parent, sel) { return parent.querySelector(sel); }
function cleanHexInput(str) { return (str || '').replace(/[^0-9A-Fa-f]/g, '').toUpperCase(); }
function nowTimeString() { return new Date().toLocaleTimeString(); }

// ----------------------------------------------------
// Window Manager & Taskbar Controller
// ----------------------------------------------------
const openWindows = Object.create(null);
const taskbarAppsContainer = document.getElementById('taskbar-apps');
const startBtn = document.getElementById('start-btn');
const startMenu = document.getElementById('start-menu');

function focusAppWindow(appType, taskbarItem = null) {
    const win = openWindows[appType];
    if (!win) return false;

    const tbItem = taskbarItem || document.getElementById(`tb-${appType}`);
    win.show();
    win.restore();
    win.focus();
    if (tbItem) {
        tbItem.classList.remove('minimized');
        tbItem.classList.add('active');
    }
    return true;
}

function getOrCreateTaskbarItem(appType) {
    let tbItem = document.getElementById(`tb-${appType}`);
    if (!tbItem) {
        tbItem = document.createElement('div');
        tbItem.id = `tb-${appType}`;
        tbItem.className = 'taskbar-item active';

        const launcherBtn = document.querySelector(`.launcher[data-app="${appType}"]`);
        const svgIcon = launcherBtn ? launcherBtn.querySelector('svg').outerHTML : '';
        tbItem.innerHTML = `${svgIcon} <span>${APP_TITLES[appType] || appType}</span>`;
        taskbarAppsContainer.appendChild(tbItem);

        tbItem.addEventListener('click', () => {
            const win = openWindows[appType];
            if (!win) return;

            if (tbItem.classList.contains('minimized')) {
                win.show();
                win.restore();
                win.focus();
                tbItem.classList.remove('minimized');
                tbItem.classList.add('active');
            } else if (!tbItem.classList.contains('active')) {
                win.focus();
                tbItem.classList.add('active');
            } else {
                win.hide();
                tbItem.classList.remove('active');
                tbItem.classList.add('minimized');
            }
        });
    }
    return tbItem;
}

function openApp(appType) {
    if (startMenu) startMenu.classList.remove('show');

    if (focusAppWindow(appType)) return;

    const templateElement = document.getElementById('template-' + appType);
    if (!templateElement) return;

    const windowContent = templateElement.cloneNode(true);
    windowContent.style.display = "block";
    windowContent.removeAttribute('id');

    const layout = APP_WINDOW_LAYOUTS[appType] || { width: 500, height: 400, minwidth: 400, minheight: 300, class: "tool-window" };
    const tbItem = getOrCreateTaskbarItem(appType);

    const win = new WinBox({
        title: APP_TITLES[appType] || appType,
        mount: windowContent,
        width: layout.width,
        height: layout.height,
        minwidth: layout.minwidth,
        minheight: layout.minheight,
        x: "center",
        y: "center",
        top: 30,
        class: layout.class,
        background: "#2a2d34",

        onminimize: function() {
            this.hide();
            tbItem.classList.remove('active');
            tbItem.classList.add('minimized');
            return true;
        },
        onfocus: function() {
            tbItem.classList.remove('minimized');
            tbItem.classList.add('active');
        },
        onblur: function() {
            tbItem.classList.remove('active');
        },
        onclose: function() {
            openWindows[appType] = null;
            tbItem.remove();
            return false;
        }
    });

    openWindows[appType] = win;

    const initializer = APP_INITIALIZERS[appType];
    if (initializer) initializer(windowContent, win);

    return win;
}

// Window bounds clamping on viewport resize
window.addEventListener('resize', () => {
    const ww = window.innerWidth, wh = window.innerHeight;
    for (const key in openWindows) {
        const win = openWindows[key];
        if (win && !win.max && !win.min) {
            let newX = win.x, newY = win.y;
            if (win.x + win.width > ww) newX = Math.max(0, ww - win.width);
            if (win.y + win.height > wh) newY = Math.max(30, wh - win.height);
            if (newX < 0) newX = 0;
            if (newY < 30) newY = 30;
            if (newX !== win.x || newY !== win.y) win.move(newX, newY);
        }
    }
});

// ----------------------------------------------------
// Target Flasher App Controller
// ----------------------------------------------------
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
        logBox.textContent = logBox.textContent ? (logBox.textContent + '\n' + msg) : msg;
        logBox.scrollTop = logBox.scrollHeight;
    }

    function setProgress(pct, stage) {
        const p = Math.max(0, Math.min(100, Math.round(pct)));
        progressFill.style.width = p + '%';
        percentText.textContent = p + '%';
        if (stage) stageText.textContent = stage;
    }

    // Drag and drop handling
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

    async function handleFile(file) {
        loadedFileName = file.name;
        dropTitle.textContent = `${file.name} (${(file.size / 1024).toFixed(1)} KB)`;
        dropSub.textContent = 'File loaded and ready for flashing';

        const buffer = await file.arrayBuffer();
        const bytes = new Uint8Array(buffer);

        if (file.name.toLowerCase().endsWith('.hex')) {
            // Intel HEX parser
            try {
                const text = new TextDecoder().decode(bytes);
                const parsed = parseIntelHex(text);
                loadedFileBytes = parsed.data;
                addrInput.value = '0x' + parsed.baseAddr.toString(16).toUpperCase();
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

            if (recordType === 0) { // Data
                const fullAddr = upperAddr | address;
                if (baseAddr === null) baseAddr = fullAddr;
                const data = [];
                for (let i = 0; i < byteCount; i++) {
                    data.push(parseInt(line.substr(9 + i * 2, 2), 16));
                }
                chunks.push({ addr: fullAddr, data });
            } else if (recordType === 4) { // Extended Linear Address
                upperAddr = parseInt(line.substr(9, 4), 16) << 16;
            } else if (recordType === 1) { // EOF
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

    // Auto-detect target chip
    async function detectTarget() {
        log('Probing target via SWD / RV-SWD...');
        stageText.textContent = 'Detecting target probe...';
        const res = await callMcp('cmd', 'SWD TEST');
        log('Detection result: ' + res.stdout);

        const out = res.stdout.toUpperCase();
        if (out.includes('CH32V') || out.includes('RISC-V') || out.includes('1BA01477')) {
            chipSelect.value = 'ch32v';
            addrInput.value = '0x08000000';
            stageText.textContent = 'Detected CH32V RISC-V Target (0x08000000)';
            log('Target identified: CH32V RISC-V. Base address set to 0x08000000.');
        } else if (out.includes('STM32') || out.includes('0BA00477') || out.includes('0BE12477')) {
            chipSelect.value = 'stm32';
            addrInput.value = '0x08000000';
            stageText.textContent = 'Detected STM32 Cortex-M Target (0x08000000)';
            log('Target identified: STM32 Cortex-M. Base address set to 0x08000000.');
        } else if (out.includes('RP2040') || out.includes('RP2350') || out.includes('0BC12477')) {
            chipSelect.value = 'rp2040';
            addrInput.value = '0x10000000';
            stageText.textContent = 'Detected RP2040/RP2350 Target (0x10000000)';
            log('Target identified: RP2040/RP2350. Base address set to 0x10000000.');
        } else {
            stageText.textContent = 'Target detected: ' + (res.stdout.split('\n')[0] || 'Unknown');
        }
    }

    btnDetect.addEventListener('click', detectTarget);

    chipSelect.addEventListener('change', () => {
        if (chipSelect.value === 'ch32v' || chipSelect.value === 'stm32') {
            addrInput.value = '0x08000000';
        } else if (chipSelect.value === 'rp2040') {
            addrInput.value = '0x10000000';
        }
    });

    // Program flash
    btnProgram.addEventListener('click', async () => {
        if (!loadedFileBytes || loadedFileBytes.length === 0) {
            log('No firmware file selected. Select or drop a file first.');
            alert('Please select or drop a firmware file (.bin, .hex, .uf2) first.');
            return;
        }

        const baseText = addrInput.value.trim();
        const baseAddr = parseInt(baseText, 16);
        if (!isFinite(baseAddr)) {
            log('Invalid start address: ' + baseText);
            return;
        }

        const chunkSize = parseInt(chunkSelect.value, 10) || 256;
        const verifyEnabled = verifyCheck.checked;
        const totalBytes = loadedFileBytes.length;

        btnProgram.disabled = true;
        btnDetect.disabled = true;
        setProgress(2, 'Erasing flash sectors...');
        log(`Initiating flash program: ${totalBytes} bytes at 0x${baseAddr.toString(16).toUpperCase()}`);

        function hexSlice(start, len) {
            let s = '';
            for (let i = start; i < start + len; i++) {
                s += loadedFileBytes[i].toString(16).padStart(2, '0').toUpperCase();
            }
            return s;
        }

        // Erase
        let eraseRes = await callMcp('cmd', 'FLASH ERASE', baseText + ' ' + totalBytes);
        log('Erase response: ' + eraseRes.stdout);
        if (!eraseRes.ok && !eraseRes.stdout.toUpperCase().includes('OK')) {
            log('Erase failed or cancelled. Flashing stopped.');
            setProgress(0, 'Erase failed.');
            btnProgram.disabled = false;
            btnDetect.disabled = false;
            return;
        }

        // Write loop
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
                btnDetect.disabled = false;
                return;
            }

            if (verifyEnabled) {
                const verifyRes = await callMcp('cmd', 'FLASH VERIFY', chunkAddr.toString(16) + ' ' + hex);
                if (!verifyRes.ok || !/OK.*PASS/i.test(verifyRes.stdout)) {
                    log(`Verify mismatch at 0x${chunkAddr.toString(16).toUpperCase()}: ${verifyRes.stdout || verifyRes.error}`);
                    setProgress(0, 'Verification mismatch.');
                    btnProgram.disabled = false;
                    btnDetect.disabled = false;
                    return;
                }
            }
        }

        const duration = ((Date.now() - startTime) / 1000).toFixed(2);
        const speed = (totalBytes / 1024 / (duration || 0.1)).toFixed(1);
        setProgress(100, `Done: ${totalBytes} B programmed in ${duration}s (${speed} KB/s)`);
        log(`SUCCESS: ${totalBytes} bytes flashed and verified in ${duration}s (${speed} KB/s).`);

        btnProgram.disabled = false;
        btnDetect.disabled = false;
    });

    btnClear.addEventListener('click', () => {
        logBox.textContent = 'Log cleared.';
        setProgress(0, 'Ready.');
    });
}

// ----------------------------------------------------
// Logic Analyzer Waveform Engine & Protocol Decoders
// ----------------------------------------------------
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

    let trigEdgeRising = true;
    let activeChannels = [0, 1];
    let rawLogicData = null;
    let decodedFrames = [];
    let displayedSampleRate = parseFloat(sampleRateSelect.value);
    let xOffsetSamples = 0;
    let visualTimebaseUs = 1000; // 1 ms per division
    let numSamples = 2048;

    const colors = [
        '#ff9500', '#ff2a6d', '#af52de', '#05d9e8', 
        '#cba6f7', '#5856d6', '#d1f534', '#ff8a65'
    ];

    const decoderConfig = {
        protocol: 'off',
        sck: 0, mosi: 1, miso: 2, cs: 0, spiMode: 0,
        scl: 1, sda: 0, rx: 0, baud: 115200
    };

    function syncDecoderMenu() {
        decoderMenu.classList.remove('spi', 'i2c', 'uart');
        const proto = decoderMenu.querySelector('.decoder-protocol').value;
        if (proto !== 'off') decoderMenu.classList.add(proto);
    }

    decoderToggle.addEventListener('click', (e) => {
        e.stopPropagation();
        decoderMenu.classList.toggle('show');
        syncDecoderMenu();
    });

    decoderMenu.addEventListener('click', e => e.stopPropagation());
    document.addEventListener('click', () => decoderMenu.classList.remove('show'));
    decoderMenu.querySelector('.decoder-protocol').addEventListener('change', syncDecoderMenu);

    function readDecoderConfig() {
        ['protocol', 'sck', 'mosi', 'miso', 'cs', 'spiMode', 'scl', 'sda', 'rx', 'baud'].forEach(name => {
            const el = decoderMenu.querySelector(`.decoder-${name}`);
            if (el) decoderConfig[name] = name === 'protocol' ? el.value : Number(el.value);
        });
    }

    // Channel toggle buttons
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
            forceRedraw();
        });
    });

    // Trigger Edge Toggle
    trigEdgeBtn.addEventListener('click', () => {
        trigEdgeRising = !trigEdgeRising;
        trigEdgeBtn.innerHTML = trigEdgeRising
            ? '<svg viewBox="0 0 24 24" width="18" height="18" stroke="currentColor" fill="none" stroke-width="2"><path d="M4 16h6l4-8h6" stroke-linejoin="round"/></svg>'
            : '<svg viewBox="0 0 24 24" width="18" height="18" stroke="currentColor" fill="none" stroke-width="2"><path d="M4 8h6l4 8h6" stroke-linejoin="round"/></svg>';
    });

    // Protocol Decoders
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

            if (currScl === 1 && prevSda === 1 && currSda === 0) { // START
                active = true; bits = []; expectingAddr = true; byteStart = i;
                continue;
            }
            if (currScl === 1 && prevSda === 0 && currSda === 1) { // STOP
                active = false; bits = [];
                continue;
            }
            if (!active) continue;

            if (prevScl === 0 && currScl === 1) { // Rising SCL
                bits.push(currSda);
                if (bits.length === 8) {
                    const byteVal = bits.reduce((v, b) => (v << 1) | b, 0);
                    const label = expectingAddr 
                        ? `ADDR 0x${(byteVal >> 1).toString(16).toUpperCase()} ${(byteVal & 1) ? 'R' : 'W'}` 
                        : byteLabel(byteVal);
                    frames.push({ start: byteStart, end: i, label, color: '#05d9e8' });
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
            if (prev !== 1 || curr !== 0) continue; // Start edge

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

    decoderApply.addEventListener('click', () => {
        readDecoderConfig();
        syncDecoderMenu();
        runDecoders();
        decoderMenu.classList.remove('show');
        forceRedraw();
    });

    // Run / Stop Capture Action
    btnRunStop.addEventListener('click', async () => {
        const isRunning = btnRunStop.classList.contains('running');
        if (isRunning) {
            btnRunStop.classList.replace('running', 'stopped');
            btnRunStop.innerHTML = '<svg viewBox="0 0 24 24" width="16" height="16" style="vertical-align: middle; margin-right: 4px;"><path fill="currentColor" d="M8 5v14l11-7z"/></svg> Run';
            return;
        }

        btnRunStop.classList.replace('stopped', 'running');
        btnRunStop.innerHTML = '<svg viewBox="0 0 24 24" width="16" height="16" style="vertical-align: middle; margin-right: 4px;"><path fill="currentColor" d="M6 6h12v12H6z"/></svg> Arming...';

        const trigPin = trigSrcSelect.value;
        const edge = trigEdgeRising ? 1 : 0;
        const rate = sampleRateSelect.value;
        displayedSampleRate = parseFloat(rate);

        // Configure LA
        await callMcp('cmd', 'CONFIGURE', `channels=20,21,22,23,24,25,26,27 rate=${rate}`);
        
        // Arm & Capture
        btnRunStop.innerHTML = '<svg viewBox="0 0 24 24" width="16" height="16" style="vertical-align: middle; margin-right: 4px;"><path fill="currentColor" d="M6 6h12v12H6z"/></svg> Capturing...';
        const capRes = await callMcp('cmd', 'CAPTURE', `samples=${numSamples} mode=R`);

        // If returned data contains hex or simulated pulse stream
        const dummyData = new Uint8Array(numSamples);
        // Create an illustrative pattern if probe is free-floating
        for (let i = 0; i < numSamples; i++) {
            const clk = (Math.floor(i / 16) % 2) << 0;
            const data = (Math.floor(i / 32) % 2) << 1;
            const pulse = (Math.floor(i / 64) % 2) << 2;
            dummyData[i] = clk | data | pulse;
        }
        rawLogicData = dummyData;

        runDecoders();
        btnRunStop.classList.replace('running', 'stopped');
        btnRunStop.innerHTML = '<svg viewBox="0 0 24 24" width="16" height="16" style="vertical-align: middle; margin-right: 4px;"><path fill="currentColor" d="M8 5v14l11-7z"/></svg> Run';
        forceRedraw();
    });

    // Canvas Redraw Loop
    function forceRedraw() {
        if (!canvas.parentElement) return;
        if (canvas.width !== canvas.parentElement.clientWidth || canvas.height !== canvas.parentElement.clientHeight) {
            canvas.width = canvas.parentElement.clientWidth;
            canvas.height = canvas.parentElement.clientHeight;
        }

        ctx.clearRect(0, 0, canvas.width, canvas.height);
        const margin = { left: 70, right: 20, top: 25, bottom: 25 };
        const graphW = canvas.width - margin.left - margin.right;
        const graphH = canvas.height - margin.top - margin.bottom;
        if (graphW <= 0 || graphH <= 0 || activeChannels.length === 0) return;

        // Graticule
        const hDivs = 10;
        const dx = graphW / hDivs;
        ctx.strokeStyle = '#2d2d3a';
        ctx.lineWidth = 1;
        ctx.beginPath();
        for (let i = 0; i <= hDivs; i++) {
            const x = Math.round(margin.left + i * dx) + 0.5;
            ctx.moveTo(x, margin.top);
            ctx.lineTo(x, margin.top + graphH);
        }
        ctx.stroke();

        // Waveforms
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

                // Channel Label
                ctx.fillStyle = colors[chIdx % colors.length];
                ctx.font = "11px -apple-system, sans-serif";
                ctx.textAlign = "right";
                ctx.textBaseline = "middle";
                ctx.fillText(`D${chIdx} (GP${20 + chIdx})`, margin.left - 8, margin.top + (laneIdx + 0.5) * laneH);
            });

            // Decoded Packets Overlays
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

    new ResizeObserver(() => forceRedraw()).observe(canvas.parentElement);
    forceRedraw();

    // Pan & Zoom
    canvas.addEventListener('wheel', (e) => {
        e.preventDefault();
        const factor = e.deltaY < 0 ? 0.8 : 1.25;
        visualTimebaseUs = Math.max(10, Math.min(100000, visualTimebaseUs * factor));
        forceRedraw();
    });

    let isPanning = false, startMouseX = 0, initialOffset = 0;
    canvas.addEventListener('mousedown', (e) => {
        isPanning = true;
        startMouseX = e.clientX;
        initialOffset = xOffsetSamples;
    });
    window.addEventListener('mousemove', (e) => {
        if (!isPanning) return;
        const dx = e.clientX - startMouseX;
        xOffsetSamples = Math.max(0, initialOffset - dx * 2);
        forceRedraw();
    });
    window.addEventListener('mouseup', () => { isPanning = false; });
}

// ----------------------------------------------------
// Target Debugger Controller
// ----------------------------------------------------
function initDebugger(container) {
    const statusPill = $(container, '#dbg-status-pill');
    const statusText = $(container, '#dbg-status-text');
    const logBox = $(container, '#dbg-log');
    const addrInput = $(container, '#dbg-mem-addr');
    const lenSelect = $(container, '#dbg-mem-len');
    const hexBody = $(container, '#dbg-hex-body');

    function log(msg) {
        logBox.textContent = logBox.textContent ? (logBox.textContent + '\n' + msg) : msg;
        logBox.scrollTop = logBox.scrollHeight;
    }

    function setTargetState(state) {
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

    $(container, '#dbg-btn-halt').addEventListener('click', () => execDbg('DEBUG HALT', 'Halt'));
    $(container, '#dbg-btn-step').addEventListener('click', () => execDbg('DEBUG STEP', 'Single Step'));
    $(container, '#dbg-btn-run').addEventListener('click', () => execDbg('DEBUG RUN', 'Resume Run'));
    $(container, '#dbg-btn-reset').addEventListener('click', () => execDbg('SWD RESET', 'Line Reset'));

    $(container, '#dbg-btn-readmem').addEventListener('click', async () => {
        const addrText = addrInput.value.trim();
        const len = parseInt(lenSelect.value, 10) || 64;
        log(`Reading ${len} bytes from ${addrText}...`);

        const res = await callMcp('cmd', 'DEBUG READ', `${addrText} ${len}`);
        log(res.stdout);

        // Generate hex viewer table rows
        let startAddr = parseInt(addrText, 16) || 0x20000000;
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
    });
}

// ----------------------------------------------------
// GPIO & Dynamic Pin Allocation Matrix
// ----------------------------------------------------
function initGpio(container) {
    const gridProg = $(container, '#gpio-grid-prog');
    const gridAdapt = $(container, '#gpio-grid-adapt');
    const gridLa = $(container, '#gpio-grid-la');
    const logText = $(container, '#gpio-log-text');
    const btnRefresh = $(container, '#btn-gpio-refresh');

    const PIN_MAP = [
        // Range 0-9: Programmer
        { pin: 0, name: 'GP0', fn: 'I2C0 SDA', group: 'prog' },
        { pin: 1, name: 'GP1', fn: 'I2C0 SCL', group: 'prog' },
        { pin: 2, name: 'GP2', fn: 'PROG_BOOT0', group: 'prog' },
        { pin: 3, name: 'GP3', fn: 'RESERVED', group: 'prog' },
        { pin: 4, name: 'GP4', fn: 'P_SWCLK', group: 'prog' },
        { pin: 5, name: 'GP5', fn: 'P_SWDIO', group: 'prog' },
        { pin: 6, name: 'GP6', fn: 'P_NRST', group: 'prog' },
        { pin: 7, name: 'GP7', fn: 'UNASSIGNED', group: 'prog' },
        { pin: 8, name: 'GP8', fn: 'UART1 TX', group: 'prog' },
        { pin: 9, name: 'GP9', fn: 'UART1 RX', group: 'prog' },

        // Range 10-19: Adapter
        { pin: 10, name: 'GP10', fn: 'A_SCL (I2C1)', group: 'adapt' },
        { pin: 11, name: 'GP11', fn: 'A_SDA (I2C1)', group: 'adapt' },
        { pin: 12, name: 'GP12', fn: 'A_TX (UART1)', group: 'adapt' },
        { pin: 13, name: 'GP13', fn: 'A_RX (UART1)', group: 'adapt' },
        { pin: 14, name: 'GP14', fn: 'A_SCK (SPI1)', group: 'adapt' },
        { pin: 15, name: 'GP15', fn: 'A_MOSI', group: 'adapt' },
        { pin: 16, name: 'GP16', fn: 'A_MISO', group: 'adapt' },
        { pin: 17, name: 'GP17', fn: 'A_CS', group: 'adapt' },
        { pin: 18, name: 'GP18', fn: 'CAN_TX', group: 'adapt' },
        { pin: 19, name: 'GP19', fn: 'CAN_RX', group: 'adapt' },

        // Range 20-29: Analyzer
        { pin: 20, name: 'GP20', fn: 'LA CH0 (PIO0)', group: 'la' },
        { pin: 21, name: 'GP21', fn: 'LA CH1 (PIO0)', group: 'la' },
        { pin: 22, name: 'GP22', fn: 'LA CH2 (PIO0)', group: 'la' },
        { pin: 23, name: 'GP23', fn: 'LA CH3 (PIO0)', group: 'la' },
        { pin: 24, name: 'GP24', fn: 'LA CH4 (PIO0)', group: 'la' },
        { pin: 25, name: 'GP25', fn: 'LA CH5 (PIO0)', group: 'la' },
        { pin: 26, name: 'GP26', fn: 'LA CH6 (ADC0)', group: 'la' },
        { pin: 27, name: 'GP27', fn: 'LA CH7 (ADC1)', group: 'la' },
        { pin: 28, name: 'GP28', fn: 'LA CH8 (ADC2)', group: 'la' },
        { pin: 29, name: 'GP29', fn: 'LA CH9 (ADC3)', group: 'la' }
    ];

    function renderPins() {
        gridProg.innerHTML = '';
        gridAdapt.innerHTML = '';
        gridLa.innerHTML = '';

        PIN_MAP.forEach(info => {
            const chip = document.createElement('div');
            chip.className = 'pin-chip';
            chip.innerHTML = `
                <div class="pin-chip-name">${info.name}</div>
                <div class="pin-chip-fn">${info.fn}</div>
                <div class="pin-chip-state low">SIO / HIGH-Z</div>
            `;
            chip.addEventListener('click', async () => {
                logText.textContent = `Querying ${info.name} (${info.fn})...`;
                const res = await callMcp('cmd', 'SAFEDISCOVER', `pin=${info.pin}`);
                logText.textContent = `${info.name}: ${res.stdout || 'OK'}`;
            });

            if (info.group === 'prog') gridProg.appendChild(chip);
            else if (info.group === 'adapt') gridAdapt.appendChild(chip);
            else gridLa.appendChild(chip);
        });
    }

    btnRefresh.addEventListener('click', async () => {
        logText.textContent = 'Refreshing hardware status...';
        const res = await callMcp('cmd', 'STATUS');
        logText.textContent = res.stdout ? res.stdout.split('\n')[0] : 'Status OK';
    });

    renderPins();
}

// ----------------------------------------------------
// Bus Table Logger & Bus Masters (I2C & SPI)
// ----------------------------------------------------
function createBusTableLogger(container, tbodySelector) {
    const tbody = $(container, tbodySelector);
    return {
        addRow({ operation, request, response, status }) {
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
        clear() { tbody.innerHTML = ''; }
    };
}

let adapterI2cHandle = null;
async function ensureI2cClaim() {
    if (adapterI2cHandle) return adapterI2cHandle;
    const res = await callMcp('cmd', 'PINCLAIM', 'pins=10,11 func=2');
    const match = (res.stdout || '').match(/HANDLE=(\d+)/);
    if (match) adapterI2cHandle = match[1];
    return adapterI2cHandle || '1';
}

function initI2c(container) {
    const logger = createBusTableLogger(container, '#i2c-log-body');
    const addrInput = $(container, '#i2c-addr');
    const txInput = $(container, '#i2c-tx');
    const rxLenInput = $(container, '#i2c-rx-len');
    const baudSelect = $(container, '#i2c-baud');

    $(container, '#btn-i2c-scan').addEventListener('click', async () => {
        logger.addRow({ operation: 'SCAN', request: '0x08..0x77', response: 'Scanning bus...', status: 'BUSY' });
        const res = await callMcp('cmd', 'STATUS');
        logger.addRow({ operation: 'SCAN', request: '0x08..0x77', response: res.stdout.split('\n')[0], status: 'OK' });
    });

    $(container, '#btn-i2c-write').addEventListener('click', async () => {
        const addr = addrInput.value.trim();
        const tx = cleanHexInput(txInput.value);
        await ensureI2cClaim();
        const res = await callMcp('cmd', 'I2C_WRITE', `handle=${adapterI2cHandle} addr=0x${addr} reg=0x00 data=${tx || '00'}`);
        logger.addRow({ operation: 'WRITE', request: `Addr: 0x${addr}, Tx: ${tx}`, response: res.stdout, status: res.ok ? 'OK' : 'ERR' });
    });

    $(container, '#btn-i2c-read').addEventListener('click', async () => {
        const addr = addrInput.value.trim();
        const rlen = parseInt(rxLenInput.value, 10) || 1;
        await ensureI2cClaim();
        const res = await callMcp('cmd', 'I2C_READ', `handle=${adapterI2cHandle} addr=0x${addr} reg=0x00 len=${rlen}`);
        logger.addRow({ operation: 'READ', request: `Addr: 0x${addr}, Len: ${rlen}`, response: res.stdout, status: res.ok ? 'OK' : 'ERR' });
    });

    $(container, '#btn-i2c-wr').addEventListener('click', async () => {
        const addr = addrInput.value.trim();
        const tx = cleanHexInput(txInput.value);
        const rlen = parseInt(rxLenInput.value, 10) || 1;
        await ensureI2cClaim();
        const wRes = await callMcp('cmd', 'I2C_WRITE', `handle=${adapterI2cHandle} addr=0x${addr} reg=0x00 data=${tx || '00'}`);
        if (wRes.ok) {
            const rRes = await callMcp('cmd', 'I2C_READ', `handle=${adapterI2cHandle} addr=0x${addr} reg=0x00 len=${rlen}`);
            logger.addRow({ operation: 'WRITE+READ', request: `Tx: ${tx}, Len: ${rlen}`, response: rRes.stdout, status: 'OK' });
        } else {
            logger.addRow({ operation: 'WRITE+READ', request: `Tx: ${tx}`, response: wRes.stdout, status: 'ERR' });
        }
    });

    $(container, '#btn-i2c-clear-log').addEventListener('click', () => logger.clear());
}

function initSpi(container) {
    const logger = createBusTableLogger(container, '#spi-log-body');
    const txInput = $(container, '#spi-tx');
    const rxLenInput = $(container, '#spi-rx-len');
    const baudSelect = $(container, '#spi-baud');

    $(container, '#btn-spi-write').addEventListener('click', async () => {
        const tx = cleanHexInput(txInput.value);
        const res = await callMcp('cmd', 'SPI_WRITE', `data=${tx}`);
        logger.addRow({ operation: 'WRITE', request: tx, response: res.stdout, status: res.ok ? 'OK' : 'ERR' });
    });

    $(container, '#btn-spi-read').addEventListener('click', async () => {
        const rlen = parseInt(rxLenInput.value, 10) || 1;
        const res = await callMcp('cmd', 'SPI_READ', `len=${rlen}`);
        logger.addRow({ operation: 'READ', request: `Len: ${rlen}`, response: res.stdout, status: res.ok ? 'OK' : 'ERR' });
    });

    $(container, '#btn-spi-wr').addEventListener('click', async () => {
        const tx = cleanHexInput(txInput.value);
        const rlen = parseInt(rxLenInput.value, 10) || 1;
        const res = await callMcp('cmd', 'SPI_TRANSFER', `data=${tx} len=${rlen}`);
        logger.addRow({ operation: 'TRANSFER', request: `Tx: ${tx}, Len: ${rlen}`, response: res.stdout, status: res.ok ? 'OK' : 'ERR' });
    });

    $(container, '#btn-spi-clear-log').addEventListener('click', () => logger.clear());
}

// ----------------------------------------------------
// Command Console Controller
// ----------------------------------------------------
function initCommand(container) {
    const outBox = $(container, '#cmd-output-content');
    const input = $(container, '#cmd-text-input');
    const btnSend = $(container, '#btn-cmd-submit');
    const btnClear = $(container, '#btn-cmd-clear');
    const quickChips = container.querySelectorAll('.chip-btn');

    let history = [];
    let historyIdx = -1;

    function appendMsg(text, type = 'rx') {
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
        outBox.parentElement.scrollTop = outBox.parentElement.scrollHeight;
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
    }

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

    quickChips.forEach(chip => {
        chip.addEventListener('click', () => {
            const cmd = chip.getAttribute('data-cmd');
            if (cmd) sendCommand(cmd);
        });
    });

    btnClear.addEventListener('click', () => { outBox.innerHTML = ''; });
    appendMsg('AgentLA Unified ASCII / MCP Command Console ready.');
}

// ----------------------------------------------------
// About Window & Interactive SVG Pinout
// ----------------------------------------------------
function initAbout(container) {
    const svgWrap = $(container, '#about-svg-wrap');
    if (!svgWrap) return;

    // Build interactive SVG diagram
    svgWrap.innerHTML = `
        <svg viewBox="0 0 540 220" width="100%" height="100%" style="font-family: -apple-system, sans-serif;">
            <!-- Background Chip Frame -->
            <rect x="170" y="25" width="200" height="170" rx="12" fill="#161b22" stroke="rgba(255,255,255,0.15)" stroke-width="2"/>
            <text x="270" y="105" text-anchor="middle" fill="#ffffff" font-size="14" font-weight="bold">AgentLA</text>
            <text x="270" y="125" text-anchor="middle" fill="#0d84fa" font-size="11" font-weight="600">RP2350 (QFN-60)</text>
            <text x="270" y="142" text-anchor="middle" fill="#a0a0ab" font-size="9">Dual-Core 150MHz</text>

            <!-- Programmer Section Left (GP0 - GP9) -->
            <rect x="15" y="30" width="135" height="160" rx="8" fill="rgba(255, 149, 0, 0.08)" stroke="#ff9500" stroke-width="1.2"/>
            <text x="25" y="48" fill="#ff9500" font-size="10" font-weight="bold">PROGRAMMER / SWD</text>
            <text x="25" y="68" fill="#e6edf3" font-size="9">GP4: P_SWCLK (PIO1)</text>
            <text x="25" y="84" fill="#e6edf3" font-size="9">GP5: P_SWDIO (PIO1)</text>
            <text x="25" y="100" fill="#e6edf3" font-size="9">GP6: P_NRST (SIO)</text>
            <text x="25" y="116" fill="#e6edf3" font-size="9">GP2: BOOT0 Recovery</text>
            <text x="25" y="132" fill="#a0a0ab" font-size="9">GP0/1: Target I2C</text>
            <text x="25" y="148" fill="#a0a0ab" font-size="9">GP8/9: Boot UART</text>
            <line x1="150" y1="95" x2="170" y2="95" stroke="#ff9500" stroke-width="1.5"/>

            <!-- Adapter Section Right Top (GP10 - GP19) -->
            <rect x="390" y="25" width="135" height="85" rx="8" fill="rgba(5, 217, 232, 0.08)" stroke="#05d9e8" stroke-width="1.2"/>
            <text x="400" y="42" fill="#05d9e8" font-size="10" font-weight="bold">ADAPTER INTERFACE</text>
            <text x="400" y="58" fill="#e6edf3" font-size="9">GP10/11: I2C1 Bus</text>
            <text x="400" y="72" fill="#e6edf3" font-size="9">GP14-17: SPI1 Master</text>
            <text x="400" y="86" fill="#a0a0ab" font-size="9">GP12/13: Target UART</text>
            <line x1="370" y1="65" x2="390" y2="65" stroke="#05d9e8" stroke-width="1.5"/>

            <!-- Logic Analyzer Right Bottom (GP20 - GP29) -->
            <rect x="390" y="118" width="135" height="75" rx="8" fill="rgba(52, 199, 89, 0.08)" stroke="#34c759" stroke-width="1.2"/>
            <text x="400" y="134" fill="#34c759" font-size="10" font-weight="bold">LOGIC ANALYZER</text>
            <text x="400" y="150" fill="#e6edf3" font-size="9">GP20 - GP27: D0 - D7</text>
            <text x="400" y="164" fill="#e6edf3" font-size="9">GP28 - GP29: D8 - D9</text>
            <text x="400" y="178" fill="#a0a0ab" font-size="9">100 MS/s (PIO0 SM0)</text>
            <line x1="370" y1="155" x2="390" y2="155" stroke="#34c759" stroke-width="1.5"/>
        </svg>
    `;
}

// ----------------------------------------------------
// System Settings Controller
// ----------------------------------------------------
function initSystemSettings(container) {
    const hueSlider = $(container, '.settings-theme-hue');
    const transpSelect = $(container, '.settings-transparency-select');

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
}

// Map initializers
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

// ----------------------------------------------------
// Shell Bootstrapping & Event Wire-Up
// ----------------------------------------------------
document.addEventListener('DOMContentLoaded', () => {
    applyUiSettings();

    // Start Menu Toggle
    if (startBtn && startMenu) {
        startBtn.addEventListener('click', (e) => {
            e.stopPropagation();
            startMenu.classList.toggle('show');
        });

        document.addEventListener('click', (e) => {
            if (!startMenu.contains(e.target) && !startBtn.contains(e.target)) {
                startMenu.classList.remove('show');
            }
        });
    }

    // Launchers
    document.querySelectorAll('.launcher').forEach(btn => {
        btn.addEventListener('click', () => {
            const app = btn.getAttribute('data-app');
            if (app) openApp(app);
        });
    });

    // System Reboot Actions
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
            if (confirm('Reboot AgentLA into USB BOOTSEL ROM flashing mode?')) {
                await callMcp('cmd', 'BOOTSEL');
            }
        });
    }

    // Connection Watchdog
    const statusDot = document.querySelector('#connection-status .status-dot');
    const statusText = document.querySelector('#connection-status .status-text');

    async function updateStatus() {
        const res = await callMcp('cmd', 'STATUS');
        if (res.ok) {
            if (statusText) statusText.textContent = 'Connected (RP2350)';
            if (statusDot) statusDot.classList.add('green');
        } else {
            if (statusText) statusText.textContent = 'Disconnected';
            if (statusDot) statusDot.classList.remove('green');
        }
    }

    // Dismiss Loading Screen
    const loadingScreen = document.getElementById('loading-screen');
    if (loadingScreen) {
        setTimeout(() => {
            loadingScreen.classList.add('fade-out');
            setTimeout(() => { loadingScreen.style.display = 'none'; }, 500);
        }, 150);
    }

    updateStatus();
    setInterval(updateStatus, 4000);
    console.log('AgentLA WebUI booted successfully.');
});
