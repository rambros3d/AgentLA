---
name: agentla-webapp
description: Guidelines and architectural standards for developing AgentLA User Web Applications (mandatory DaisyUI 5 usage, uPlot strip-chart integration, dual-mode embedding, theme reactivity, and zero-emoji compliance).
license: MIT
metadata:
  version: "1.0.0"
  author: "AgentLA Team"
  tools_required: ["agentla-mcp"]
---

# AgentLA Web Application Development Guide (`agentla-webapp`)

This skill defines the architectural standards, UI/UX patterns, styling conventions, real-time plotting rules, and packaging specifications for developing web applications for AgentLA.

Whether authoring an in-ROM system app (`webui/*.html`) or an installable user package (`apps/<name>/` stored in LittleFS `USER_FS`), all applications must strictly adhere to these standards.

---

## 1. Core Architectural Invariants

### 1.1 Dual-Mode Operation: Embedded Iframe vs Standalone Tab

AgentLA applications must execute seamlessly in two contexts:
1. **Embedded Mode**: Hosted inside an iframe within the AgentLA Shell (`/home#/<app-name>` or `/lab.html`).
2. **Standalone Mode**: Opened directly as a top-level tab (e.g., `http://10.10.10.10/apps/<name>/index.html` or `/barefoc.html`).

#### Embedding Detection Protocol
Every application `<head>` must include a synchronous execution snippet before CSS renders to prevent layout flashes:

```html
<script>
  (function() {
    const isEmbedded = (window.self !== window.top) || 
                       (new URLSearchParams(window.location.search).get('embed') === '1');
    if (isEmbedded) {
      document.documentElement.classList.add('is-embedded');
    }
  })();
</script>
```

#### Header & Navigation Suppression
When `is-embedded` is present on `<html>`, the application's top navigation bar, brand logo, and standalone docks must be automatically hidden, allowing the host shell to govern global navigation:

```css
/* Suppress inner navigation and header when embedded inside AgentLA shell */
.is-embedded .app-header,
.is-embedded #top-nav-bar,
.is-embedded #brand-header,
.is-embedded #agentla-dock {
  display: none !important;
}

.is-embedded body {
  padding-top: 0 !important;
  height: 100vh !important;
  overflow: hidden;
}
```

### 1.2 System ROM Asset Linking

To conserve LittleFS storage and memory on the RP2350 target, applications must link shared ROM-baked assets via root-relative paths rather than bundling local duplicates:

```html
<!-- Core Stylesheets -->
<link rel="stylesheet" href="/daisyui.css">
<link rel="stylesheet" href="/style.css">
<link rel="stylesheet" href="/themes.css">

<!-- Real-Time Plotting Engine (When Charting is Needed) -->
<link rel="stylesheet" href="/uplot.min.css">
<script src="/uplot.min.js"></script>

<!-- AgentLA Bridge (Floating Dock & System Integration) -->
<script src="/agentla-bridge.js"></script>
```

### 1.3 Viewport Geometry & Dock Placement
- **Full-Bleed Canvas**: Application viewports must span the full display height (`height: 100vh` or `height: calc(100vh - 48px)`). Never leave dead margin shelves.
- **Bottom Dock Anchor**: System and app docks must anchor flush to the bottom edge (`bottom: 0`, not floating with dead whitespace underneath).
- **Horizontal Centering**: Launcher views and modal dialogs must preserve horizontal centering across all zoom levels with `margin-inline: auto`.

### 1.4 Strict Quality & Design Invariants
- **Zero Emojis**: Emojis are strictly forbidden anywhere in AgentLA. Do not use emojis in UI labels, tooltips, console logs, comments, or documentation. Use standard technical abbreviations, text labels, or SVG icons (Solar icon set).
- **Zero CSS Dot Indicators**: Never use floating dot indicators or pips below active dock items, tabs, or buttons (e.g., `.dock-icon-btn.active::after`). Active states must be rendered through surface styling (subtle background tints or border highlights).

---

## 2. UI Styling Standards: Mandatory DaisyUI 5

All visual components must use **DaisyUI 5** semantic classes. Never write bespoke raw CSS for buttons, toggles, cards, badges, or inputs.

### 2.1 Essential DaisyUI Components

| Component | DaisyUI Classes | Example Usage |
| :--- | :--- | :--- |
| **Buttons** | `btn btn-primary btn-xs` | Primary action button |
| **Danger Buttons** | `btn btn-outline btn-error btn-xs` | Quick-stop or reset action |
| **Toggles / Switches** | `toggle toggle-xs toggle-success` | Hardware enable or power toggle |
| **Cards & Panels** | `card bg-base-200 border border-base-300` | Telemetry containers and parameter groups |
| **Badges / Status** | `badge badge-outline badge-xs font-mono` | Mode indicator or connection state |
| **Button Groups** | `join` with `join-item btn btn-xs` | Segmented controls and mode selectors |
| **Form Inputs** | `input input-bordered input-xs font-mono` | Numeric setpoints and register addresses |
| **Sliders / Range** | `range range-xs range-primary` | Real-time throttle or position slider |
| **Tables** | `table table-xs table-zebra font-mono` | MultiBus register maps and telemetry tables |
| **Alerts** | `alert alert-warning text-xs` | Bus fault or safety warnings |

### 2.2 Semantic Color Tokens & Theme Palette

DaisyUI 5 uses OKLCH color tokens that adapt automatically across all AgentLA themes (e.g., `nothing`, `industrial`, `caramellatte`, `emerald`, `forest`):

```html
<!-- Background Surfaces -->
<div class="bg-base-100">Canvas Surface</div>
<div class="bg-base-200 border border-base-300">Card Surface</div>
<div class="bg-base-300">Inset Container</div>

<!-- Typography Colors -->
<span class="text-base-content font-bold">Primary Text</span>
<span class="text-base-content/60 text-xs">Secondary Technical Label</span>

<!-- Semantic Status Accents -->
<span class="text-primary font-mono">Channel 1</span>
<span class="text-accent font-mono">Velocity</span>
<span class="text-success font-mono">Active / OK</span>
<span class="text-warning font-mono">Over-Current Alert</span>
<span class="text-error font-mono">Fault State</span>
```

### 2.3 Dynamic Theme Reactivity

Applications must remain reactive when the user changes the system theme:

```javascript
// Observe theme changes on document root to update canvases or charts
const themeObserver = new MutationObserver((mutations) => {
  for (const mutation of mutations) {
    if (mutation.attributeName === 'data-theme') {
      const currentTheme = document.documentElement.getAttribute('data-theme');
      onThemeUpdated(currentTheme);
    }
  }
});
themeObserver.observe(document.documentElement, { attributes: true, attributeFilter: ['data-theme'] });

function onThemeUpdated(theme) {
  // Re-sample computed CSS variables for uPlot / Canvas2D rendering
  const style = getComputedStyle(document.documentElement);
  const primaryColor = style.getPropertyValue('--color-primary').trim();
  const baseContent = style.getPropertyValue('--color-base-content').trim();
  // Update chart axes and series colors
}
```

---

## 3. Real-Time Data Visualization & Widget Standards

To provide high-density instrumentation without exceeding embedded flash memory or causing garbage-collection pauses, AgentLA defines a standardized suite of micro-libraries.

### 3.1 Overview of Standardized Widget & Visualization Engines

| Widget / Engine | Canonical Library | Footprint (Gzip) | Primary Capabilities |
| :--- | :--- | :--- | :--- |
| **Time-Series / Waveforms** | `uPlot` (`/uplot.min.js`) | ~22.7 KB | Multi-channel 60 FPS oscilloscope, logic signals, ADC traces |
| **Rotary Dials & Gauges** | `pure-knob` (`/pureknob.min.js`) | ~3.8 KB | Endless encoders, bounded arcs, needle gauges, numeric entry |
| **2D Joystick / XY-Pad** | `nipplejs` (`/nipplejs.min.js`) | ~3.2 KB | 2-DOF virtual analog joystick, pan-tilt and gimbal steering |
| **3D & IMU Orientation** | `OGL` (`/ogl.min.js`) | ~14.5 KB | Minimal WebGL 3D scene, quaternion / Euler rotation, 3D meshes |
| **Addressable LED Engine** | `agentla-led-view.js` | ~2.5 KB | 1D strip, 2D matrix, circular ring Neopixel simulation with bloom |
| **Servo Kinematics Engine** | `agentla-servo-view.js` | ~2.8 KB | Multi-horn geometry, angular travel arcs, keyframe timeline |
| **Thermal Camera Heatmap** | `agentla-thermal-view.js` | ~2.4 KB | Bilinear interpolation of AMG8833/MLX90640 with Ironbow palette |
| **RP2350 Pinout Visualizer** | `agentla-pinout-view.js` | ~2.2 KB | Interactive 40-pin Pico 2 header with live Pin Arbiter badges |
| **LittleFS Sector Grid** | `agentla-storage-map.js` | ~2.5 KB | 440x4 KB User and 256x4 KB System flash sector wear heatmap |

---

### 3.2 Time-Series & Oscilloscopes: uPlot

- **When to Use**: High-frequency analog ADC channels, PWM captures, motor telemetry ($I_q, I_d, V_{\text{bus}}$, velocity), sensor logging.
- **Why**: Under 50 KB raw, zero dependencies, Canvas2D direct blit supporting 100,000+ points at 60 FPS without DOM overhead.
- **Pattern**:
  ```javascript
  const plot = new uPlot({
    width: 600,
    height: 240,
    scales: { x: { time: false }, y: { auto: true } },
    axes: [
      { stroke: 'var(--color-base-content)', grid: { stroke: 'rgba(255,255,255,0.06)' } },
      { stroke: 'var(--color-base-content)', grid: { stroke: 'rgba(255,255,255,0.06)' } }
    ],
    series: [
      { label: "Time (s)" },
      { label: "Velocity", stroke: 'var(--color-primary)', width: 1.5 }
    ]
  }, data, containerEl);
  ```

---

### 3.3 Rotary Knobs, Dials & Radial Gauges: pure-knob

- **When to Use**: Motor setpoint dials, servo horn target angles, potentiometer simulators, voltage/current limiters, volume/gain controls.
- **Why**: ~3.8 KB gzipped Canvas2D element. Supports touch, mouse drag, mouse wheel, keyboard arrows, and double-click numeric entry.
- **Setup Pattern**:
  ```javascript
  // Create pure-knob element
  const knob = pureknob.createKnob(120, 120);
  knob.setProperty('angleStart', -0.75 * Math.PI);
  knob.setProperty('angleEnd', 0.75 * Math.PI);
  knob.setProperty('colorFG', getComputedStyle(document.documentElement).getPropertyValue('--color-primary').trim());
  knob.setProperty('colorBG', 'rgba(255, 255, 255, 0.08)');
  knob.setProperty('valMin', 0);
  knob.setProperty('valMax', 100);
  knob.setProperty('val', 50);
  knob.setProperty('fnValueToString', (val) => `${val.toFixed(1)} %`);

  // Listen to interactive adjustments
  knob.addListener((knobInstance, val) => {
    onTargetChanged(val);
  });

  const domNode = knob.node();
  containerEl.appendChild(domNode);
  ```

---

### 3.4 2D Virtual Joystick & XY-Pad: nipplejs

- **When to Use**: 2-DOF differential gimbals, pan-tilt camera servos, 2-axis motor stages, robotic end-effector positioning, rover drive control.
- **Why**: ~3.2 KB gzipped. Seamless multi-touch pointer tracking with polar and cartesian coordinate output.
- **Setup Pattern**:
  ```javascript
  const joystick = nipplejs.create({
    zone: document.getElementById('joystick-zone'),
    mode: 'static',
    position: { left: '50%', top: '50%' },
    color: getComputedStyle(document.documentElement).getPropertyValue('--color-primary').trim(),
    size: 120
  });

  joystick.on('move', (evt, data) => {
    // data.vector = { x, y } in range [-1.0, 1.0]
    // data.angle = { radian, degree }
    // data.distance = distance from center
    sendGimbalControl(data.vector.x, data.vector.y);
  });

  joystick.on('end', () => {
    // Return to center / neutral position
    sendGimbalControl(0, 0);
  });
  ```

---

### 3.5 3D Orientation & IMU Attitude: OGL

- **When to Use**: 6-DOF / 9-DOF IMU orientation viewers (MPU6050, BNO055, ICM-42688), drone attitude indicators, 3D robotics arm pose, STL part preview.
- **Why**: Only ~14.5 KB gzipped (versus >650 KB for Three.js). Complete WebGL scene graph with native Quaternion math (`Quat`).
- **Setup Pattern**:
  ```javascript
  import { Renderer, Camera, Transform, Mesh, Box, Program } from '/ogl.min.js';

  const renderer = new Renderer({ canvas: document.getElementById('gl-canvas'), antialias: true, alpha: true });
  const gl = renderer.gl;
  const camera = new Camera(gl, { fov: 35 });
  camera.position.set(0, 0, 5);

  const scene = new Transform();
  const geometry = new Box(gl, { width: 1.5, height: 0.2, depth: 2.2 }); // PCB board shape
  const program = new Program(gl, {
    vertex: `
      attribute vec3 position;
      attribute vec3 normal;
      uniform mat4 modelViewMatrix;
      uniform mat4 projectionMatrix;
      varying vec3 vNormal;
      void main() {
        vNormal = normal;
        gl_Position = projectionMatrix * modelViewMatrix * vec4(position, 1.0);
      }
    `,
    fragment: `
      precision highp float;
      varying vec3 vNormal;
      void main() {
        vec3 light = normalize(vec3(0.5, 1.0, 0.3));
        float d = max(dot(vNormal, light), 0.2);
        gl_FragColor = vec4(vec3(0.2, 0.7, 0.9) * d, 1.0);
      }
    `
  });

  const pcbMesh = new Mesh(gl, { geometry, program });
  pcbMesh.setParent(scene);

  // Update attitude from incoming IMU quaternion [w, x, y, z]
  function updateImuOrientation(w, x, y, z) {
    pcbMesh.quaternion.set(x, y, z, w);
    renderer.render({ scene, camera });
  }
  ```

---

### 3.6 Specialized Embedded Hardware Engines

#### A. Neopixel LED Visualizer (`agentla-led-view.js`)
- Simulates 1D strips, 2D matrices (serpentine or zigzag), and circular rings (12, 16, 24, 60 LEDs).
- Real-time Canvas2D rendering with radial phosphor bloom and live mA current calculation.
```javascript
const ledView = new AgentlaLedView('#led-container', {
  topology: 'matrix',
  matrixWidth: 8,
  matrixHeight: 8,
  serpentine: true,
  bloom: true
});
ledView.setPixels([[255, 0, 0], [0, 255, 0], ...]);
const power = ledView.getPowerStats(); // { currentMa, powerMw, ledCount }
```

#### B. Servo Horn & Motion Visualizer (`agentla-servo-view.js`)
- Renders parametric servo horns (single arm, double arm, 4-point cross, circular winch drum).
- Displays mechanical angle limits (0-180 deg) and pulse widths (500-2500 us) with keyframe timeline sequencing.
```javascript
const servoView = new AgentlaServoView('#servo-container', {
  hornType: 'single',
  angle: 90,
  minAngle: 0,
  maxAngle: 180,
  interactive: true
});
servoView.on('change', (deg) => sendPwmPulse(deg));
```

#### C. Thermal Camera Heatmap (`agentla-thermal-view.js`)
- Renders 8x8 (AMG8833) or 32x24 (MLX90640) thermal sensor streams.
- Bilinear interpolation upscales pixel blocks into smooth thermal heatmaps with Ironbow, Rainbow, or Inferno palettes.
```javascript
const thermalView = new AgentlaThermalView('#thermal-container', {
  sensor: 'amg8833',
  palette: 'ironbow',
  autoRange: true
});
thermalView.setFrame(tempArrayFloat32);
```

#### D. RP2350 40-Pin Header Visualizer (`agentla-pinout-view.js`)
- Interactive dual-inline 40-pin header for Raspberry Pi Pico 2 / RP2350.
- Shows live Pin Arbiter ownership badges and alternate pin functions.
```javascript
const pinoutView = new AgentlaPinoutView('#pinout-container');
pinoutView.setOwnership({
  'GP16': { owner: 'i2c0' },
  'GP17': { owner: 'i2c0' },
  'GP22': { owner: 'ws2812' }
});
pinoutView.on('select', (pin) => console.log('Selected pin', pin));
```

#### E. LittleFS Flash Sector Grid (`agentla-storage-map.js`)
- Renders 440 blocks (USER_FS, 1760 KB) or 256 blocks (SYSTEM_FS, 1024 KB).
- Color-coded allocation states (FREE, DATA, META, WEAR) with block address inspector.
```javascript
const storageMap = new AgentlaStorageMap('#storage-map-container', {
  partition: 'user',
  interactive: true,
  showLegend: true
});
storageMap.setPartition('system');
```

---

---

## 4. Hardware Safety & Pin Arbiter Integration

AgentLA is an active physical hardware bridge. Applications that manipulate GPIOs, buses, or power rails must follow safety protocols:

### 4.1 Pin Arbiter Ownership
Never drive or configure RP2350 pins without claiming them through the Pin Arbiter:

```javascript
// Claim Pin 0 and Pin 1 for I2C bus master
async function claimHardwarePins() {
  const resp = await fetch('/api/v1/system/pins/claim', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({
      mask: 0x0003, // GP0 (SDA) + GP1 (SCL)
      owner: "barefoc_studio",
      description: "MultiBus I2C Telemetry"
    })
  });
  if (!resp.ok) {
    console.error("Failed to acquire pin ownership from arbiter");
  }
}

// Release pins when application exits or closes
async function releaseHardwarePins() {
  await fetch('/api/v1/system/pins/release', {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ mask: 0x0003 })
  });
}
window.addEventListener('beforeunload', releaseHardwarePins);
```

### 4.2 Emergency Safe State (`Shift + Escape`)
Every actuator or motor control application must bind the global emergency hotkey to immediately disarm target hardware:

```javascript
window.addEventListener('keydown', (e) => {
  if (e.shiftKey && e.key === 'Escape') {
    e.preventDefault();
    triggerEmergencySafe();
  }
});

async function triggerEmergencySafe() {
  // 1. Send urgent safe command to target
  try {
    await fetch('/api/v1/system/emergency_safe', { method: 'POST' });
  } catch (err) {
    console.error("Emergency safe dispatch failed", err);
  }
  // 2. Toggle UI switches to disarmed state
  const enableSwitch = document.getElementById('chk-motor-enable');
  if (enableSwitch) enableSwitch.checked = false;
}
```

---

## 5. Application Packaging & Manifest Specification

User applications are bundled into standard `ustar` tarballs and stored in LittleFS `USER_FS` (`0x00248000..0x00400000`, 1760 KB partition).

### 5.1 App Directory Structure
```
my-app/
├── app.json        (Required: Manifest metadata)
├── index.html      (Required: Application entry point)
└── icon.svg        (Required: Vector icon, 24x24 or 32x32)
```

### 5.2 Manifest (`app.json`) Format

```json
{
  "name": "my-app",
  "title": "Sensor Studio",
  "version": "1.0.0",
  "author": "Engineering Team",
  "description": "Real-time I2C/SPI sensor acquisition and uPlot signal analysis.",
  "category": "Instrumentation",
  "tags": ["Sensors", "I2C", "SPI", "Telemetry"],
  "icon": "icon.svg",
  "requires": ["i2c", "gpio_flexible"],
  "min_firmware_version": "1.0.0",
  "homepage": "https://github.com/agentla/apps/sensor-studio",
  "files": [
    "index.html",
    "icon.svg"
  ]
}
```

#### Manifest Invariants:
- `name`: Max 32 characters, lowercase ASCII, hyphen-separated (`[a-z0-9-]`).
- `files`: Must list all bundled files. Subdirectories are supported up to 8 levels deep.
- Tarball size: Max 768 KB (`MAX_TAR_BYTES`).

### 5.3 Building & Installing Packages

#### Packaging via CLI:
```bash
# Create installable tarball without path prefixes
cd apps/my-app
tar -cf ../packages/my-app.tar app.json index.html icon.svg
```

#### Installing via REST API:
```bash
curl -X POST --data-binary @packages/my-app.tar \
  http://10.10.10.10/api/v1/apps/install
```

---

## 6. Complete User Application Starter Template

Here is a turnkey, zero-emoji, DaisyUI 5, uPlot-integrated starter template:

```html
<!DOCTYPE html>
<html lang="en" data-theme="nothing">
<head>
  <meta charset="UTF-8">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <title>Signal Monitor</title>

  <!-- Dual-Mode Embedding Detection -->
  <script>
    (function() {
      const isEmbedded = (window.self !== window.top) || 
                         (new URLSearchParams(window.location.search).get('embed') === '1');
      if (isEmbedded) {
        document.documentElement.classList.add('is-embedded');
      }
    })();
  </script>

  <!-- System ROM Assets -->
  <link rel="stylesheet" href="/daisyui.css">
  <link rel="stylesheet" href="/style.css">
  <link rel="stylesheet" href="/themes.css">
  <link rel="stylesheet" href="/uplot.min.css">
  <script src="/uplot.min.js"></script>
  <script src="/agentla-bridge.js"></script>

  <style>
    /* Prevent inner header display when inside AgentLA shell iframe */
    .is-embedded .top-bar-header {
      display: none !important;
    }
    body {
      margin: 0;
      padding: 0;
      background-color: var(--color-base-100);
      color: var(--color-base-content);
      font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", Roboto, sans-serif;
    }
  </style>
</head>
<body class="p-4 flex flex-col gap-4 min-h-screen">

  <!-- Top Bar (Hidden when embedded) -->
  <header class="top-bar-header flex items-center justify-between border-b border-base-300 pb-3">
    <div class="flex items-center gap-2">
      <span class="text-sm font-bold uppercase tracking-wider text-base-content/80">Signal Monitor</span>
      <div class="badge badge-outline badge-xs font-mono" id="badge-status">READY</div>
    </div>
    <div class="flex items-center gap-2">
      <label class="cursor-pointer label p-0 gap-2">
        <span class="label-text text-xs font-mono">Stream</span>
        <input type="checkbox" id="chk-stream-enable" class="toggle toggle-xs toggle-primary" checked />
      </label>
    </div>
  </header>

  <!-- Main Grid Layout -->
  <main class="grid grid-cols-1 lg:grid-cols-12 gap-4 flex-1">
    
    <!-- Left Column: Telemetry & Controls (4 cols) -->
    <section class="lg:col-span-4 flex flex-col gap-3">
      
      <!-- Metrics Card -->
      <div class="card bg-base-200 border border-base-300 p-3 rounded-xl flex flex-col gap-2">
        <span class="text-xs font-bold uppercase tracking-wider text-base-content/80">Telemetry Readings</span>
        <div class="grid grid-cols-2 gap-2">
          <div class="bg-base-300/40 p-2 rounded-lg flex flex-col">
            <span class="text-[10px] font-mono uppercase text-base-content/60">Signal Value</span>
            <span class="text-sm font-bold font-mono text-primary" id="val-signal">0.00 V</span>
          </div>
          <div class="bg-base-300/40 p-2 rounded-lg flex flex-col">
            <span class="text-[10px] font-mono uppercase text-base-content/60">Sample Rate</span>
            <span class="text-sm font-bold font-mono text-accent" id="val-rate">100 Hz</span>
          </div>
        </div>
      </div>

      <!-- Action Card -->
      <div class="card bg-base-200 border border-base-300 p-3 rounded-xl flex flex-col gap-2">
        <span class="text-xs font-bold uppercase tracking-wider text-base-content/80">Configuration</span>
        <div class="join w-full">
          <input type="number" id="input-freq" class="input input-bordered input-xs join-item w-full font-mono" value="100" />
          <button id="btn-apply-freq" class="btn btn-xs btn-primary join-item">Set</button>
        </div>
      </div>

    </section>

    <!-- Right Column: uPlot Real-Time Scope (8 cols) -->
    <section class="lg:col-span-8 flex flex-col gap-2">
      <div class="card bg-base-200 border border-base-300 p-3 rounded-xl flex flex-col gap-2 flex-1">
        <div class="flex items-center justify-between">
          <span class="text-xs font-bold uppercase tracking-wider text-base-content/80">Real-Time Waveform</span>
          <div class="badge badge-outline badge-xs font-mono" id="badge-fps">60 FPS</div>
        </div>
        <div id="uplot-chart-container" class="w-full flex-1 min-h-[240px]"></div>
      </div>
    </section>

  </main>

  <script>
    // Initialize uPlot and live simulation
    const container = document.getElementById('uplot-chart-container');
    const bufSize = 500;
    const timestamps = new Float64Array(bufSize);
    const seriesData = new Float32Array(bufSize);

    for (let i = 0; i < bufSize; i++) {
      timestamps[i] = i * 0.01;
      seriesData[i] = Math.sin(i * 0.1);
    }

    const plotData = [timestamps, seriesData];
    const style = getComputedStyle(document.documentElement);

    const plot = new uPlot({
      width: container.clientWidth || 500,
      height: 240,
      scales: { x: { time: false }, y: { auto: true } },
      axes: [
        {
          stroke: style.getPropertyValue('--color-base-content').trim() || '#a6adbb',
          grid: { stroke: 'rgba(255,255,255,0.06)' }
        },
        {
          stroke: style.getPropertyValue('--color-base-content').trim() || '#a6adbb',
          grid: { stroke: 'rgba(255,255,255,0.06)' }
        }
      ],
      series: [
        { label: "Time (s)" },
        {
          label: "Signal",
          stroke: style.getPropertyValue('--color-primary').trim() || '#3b82f6',
          width: 1.5
        }
      ]
    }, plotData, container);

    new ResizeObserver(() => {
      if (container.clientWidth > 0) {
        plot.setSize({ width: container.clientWidth, height: 240 });
      }
    }).observe(container);
  </script>
</body>
</html>
```

---

## 7. Verification Checklist

Before releasing or deploying an AgentLA application:
- [ ] **DaisyUI 5 Compliance**: No bespoke button, card, or toggle CSS classes.
- [ ] **Zero Emojis**: Cleaned of all emoji codepoints across HTML, JS, and documentation.
- [ ] **Dual-Mode Embedding**: Tested both standalone and inside an iframe (`?embed=1`).
- [ ] **uPlot Usage**: Uses `/uplot.min.js` and `/uplot.min.css` for time-series / waveforms.
- [ ] **Zero CSS Dot Indicators**: No `.dock-icon-btn.active::after` or floating dot pips.
- [ ] **Pin Arbiter**: Claims pins before usage and releases them on exit.
- [ ] **Emergency Hotkey**: Listens for `Shift + Escape` to disarm any active actuation.
- [ ] **Storage Bounds**: Total package size remains well within the 768 KB limit.
