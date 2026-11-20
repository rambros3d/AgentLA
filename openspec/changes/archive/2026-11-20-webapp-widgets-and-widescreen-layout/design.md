## Context

AgentLA's WebUI runs directly from the RP2350 microcontroller's in-ROM `SYSTEM_FS` (1 MB flash partition) and serves embedded web applications over USB NCM / Ethernet. While DaisyUI 5 provides universal CSS components and uPlot handles time-series strip charts, physical actuator and sensor instrumentation currently lacks standard micro-libraries for rotary dials, multi-axis joysticks, 3D attitude, and domain-specific visualizers. Furthermore, existing web applications (`barefoc.html`, `servo.html`, `neopixel.html`, `sensor.html`, `files.html`) use vertically stacked single-column containers that look cramped and waste horizontal space on wide-screen displays.

## Goals / Non-Goals

**Goals:**
- Provide canonical, ultra-lightweight, zero-dependency micro-libraries in `webui/`:
  - `pureknob.min.js`: Canvas2D rotary knobs, endless encoders, radial gauges, numeric entry (<4 KB gzip).
  - `nipplejs.min.js`: Multi-touch 2D virtual analog joystick & XY touchpad (<3.5 KB gzip).
  - `ogl.min.js`: Minimalist WebGL 3D engine with native Quaternion (`Quat`) and Euler math (<15 KB gzip).
  - `agentla-led-view.js`: Shared Neopixel LED array engine (1D strip, 2D matrix, circular rings) with bloom and current estimation (<3 KB gzip).
  - `agentla-servo-view.js`: Shared servo horn visualizer with travel arcs ($0\text{--}180^\circ$) and keyframe timeline sequencer (<3 KB gzip).
  - `agentla-thermal-view.js`: Bilinear interpolation heatmap engine for AMG8833/MLX90640 thermal cameras (<3 KB gzip).
  - `agentla-pinout-view.js`: Interactive 40-pin RP2350 header visualizer (<3.5 KB gzip).
  - `agentla-storage-map.js`: LittleFS flash sector grid visualizer (<2.5 KB gzip).
- Overhaul web application viewports (`barefoc.html`, `servo.html`, `neopixel.html`, `sensor.html`, `files.html`) into responsive, multi-column wide-screen layouts (`grid grid-cols-1 lg:grid-cols-12`).
- Register static HTTP routes in `firmware-rs/src/web.rs` with gzip compression and immutable caching.
- Expand `SKILLS/agentla-webapp/SKILL.md` with guidelines and copy-paste boilerplates.

**Non-Goals:**
- Heavy 3D or graphing frameworks (Three.js, Babylon.js, Chart.js, D3 are strictly excluded).
- RF, Acoustic & Frequency Domain (DSP) waterfall / constellation engines (explicitly excluded per user request).
- Dynamic server-side UI rendering (all UI remains client-side vanilla ES6 + Canvas/WebGL).

## Decisions

### Decision 1: `pure-knob` for Rotary Controls & Gauges over SVG or heavy audio plugins
- *Choice*: `pureknob.min.js` (~12 KB raw, ~3.8 KB gzipped).
- *Rationale*: Renders directly to HTML5 Canvas2D with zero dependencies. Handles mouse drag, touch drag, scroll wheel, and double-click keyboard numeric entry out of the box. Fully customizable track colors, arc limits, and formatters.
- *Alternatives Considered*: GoogleChromeLabs `input-knob` (SVG-based web component, less flexible for needle gauges), custom Canvas from scratch (reinventing wheel).

### Decision 2: `nipplejs` for 2D Multi-Axis Virtual Joystick
- *Choice*: `nipplejs.min.js` (~10 KB raw, ~3.2 KB gzipped).
- *Rationale*: Proven virtual joystick engine with full multi-touch support. Emits both Polar $(\theta, r)$ and Cartesian $(x, y)$ coordinates in $[-1.0, 1.0]$.
- *Alternatives Considered*: Custom mouse/touch drag listener (fragile multi-touch handling), HTML range dual sliders (clunky for 2D gimbal tracking).

### Decision 3: `OGL` for 3D Orientation & IMU Attitude over Three.js
- *Choice*: `ogl.min.js` (~45 KB raw, ~14.5 KB gzipped).
- *Rationale*: Minimalist WebGL scene-graph with native `Transform`, `Camera`, `Mesh`, `Program`, and crucial 3D math (`Vec3`, `Mat4`, `Quat`, `Euler`). It is 1/15th the size of Three.js (>650 KB), fitting easily inside the 1 MB ROM partition.
- *Alternatives Considered*: Three.js (prohibitive size >650 KB), Zdog (pseudo-3D Canvas2D, lacks true WebGL quaternion rotation and lighting).

### Decision 4: Responsive 12-Column Multi-Column Wide-Screen Grid Architecture
- *Choice*: `grid grid-cols-1 lg:grid-cols-12 gap-4 items-stretch h-full`.
  - Widescreen displays split real estate:
    - 4 to 5 columns for controls, dials, setpoint inputs, and parameter cards.
    - 7 to 8 columns for primary visualizers, scopes, 3D canvases, and waveforms.
  - Viewports remain full bleed (`height: calc(100vh - 48px)`) without dead whitespace shelves.
- *Rationale*: Eliminates vertical scrolling on desktop displays while remaining responsive on narrow screens.

## Risks / Trade-offs

- **[Risk] LittleFS ROM Partition Headroom**: Adding ~8 new JS files could eat flash space.
  - *Mitigation*: Total added compressed size is ~28-35 KB, utilizing <6% of the 618 KB free headroom in the 1 MB partition.
- **[Risk] WebGL Compatibility on Older Browsers**: WebGL might fail on ancient embedded browsers.
  - *Mitigation*: `OGL` initialization is wrapped in standard WebGL context detection with fallback to 2D projections.
- **[Risk] CSS Clashes with DaisyUI**: Third-party widget CSS could alter global resets.
  - *Mitigation*: `pure-knob` and `OGL` use raw Canvas/WebGL without external CSS files. `nipplejs` styles are scoped to its container.
