/**
 * agentla-thermal-view.js - Standardized Thermal Camera Heatmap Engine
 * Supports AMG8833 (8x8) and MLX90640 (32x24) thermal sensors with
 * high-performance 2D bilinear interpolation, Ironbow/Inferno palettes,
 * interactive spot measurement crosshairs, and live temperature telemetry.
 * Strictly zero emojis and zero external dependencies.
 */

(function(global) {
  'use strict';

  // Precomputed Palette Gradient Tables (256 RGB entries each)
  function createPaletteLut(stops) {
    const canvas = document.createElement('canvas');
    canvas.width = 256;
    canvas.height = 1;
    const ctx = canvas.getContext('2d');
    const grad = ctx.createLinearGradient(0, 0, 256, 0);
    for (const [stop, color] of stops) {
      grad.addColorStop(stop, color);
    }
    ctx.fillStyle = grad;
    ctx.fillRect(0, 0, 256, 1);
    const imgData = ctx.getImageData(0, 0, 256, 1).data;
    const lut = new Uint32Array(256);
    for (let i = 0; i < 256; i++) {
      const idx = i * 4;
      // Store as ABGR uint32 for fast direct canvas ImageData manipulation
      lut[i] = (255 << 24) | (imgData[idx + 2] << 16) | (imgData[idx + 1] << 8) | imgData[idx];
    }
    return lut;
  }

  const PALETTES = {
    ironbow: createPaletteLut([
      [0.0, '#000004'],
      [0.15, '#1b0c41'],
      [0.35, '#4f0b69'],
      [0.55, '#8a226a'],
      [0.75, '#c8444a'],
      [0.90, '#f27a24'],
      [1.0, '#fbf8a4']
    ]),
    inferno: createPaletteLut([
      [0.0, '#000000'],
      [0.2, '#3b0f70'],
      [0.4, '#8c2981'],
      [0.6, '#de4968'],
      [0.8, '#fe9f6d'],
      [1.0, '#fcfdbf']
    ]),
    rainbow: createPaletteLut([
      [0.0, '#0000ff'],
      [0.25, '#00ffff'],
      [0.5, '#00ff00'],
      [0.75, '#ffff00'],
      [1.0, '#ff0000']
    ])
  };

  class AgentlaThermalView {
    constructor(container, options = {}) {
      this.container = typeof container === 'string' ? document.querySelector(container) : container;
      if (!this.container) {
        throw new Error('[AgentlaThermalView] Valid container element required');
      }

      this.options = Object.assign({
        sensor: 'amg8833',    // 'amg8833' (8x8) | 'mlx90640' (32x24)
        sensorWidth: 8,
        sensorHeight: 8,
        palette: 'ironbow',   // 'ironbow' | 'inferno' | 'rainbow'
        autoRange: true,
        minTempC: 15.0,
        maxTempC: 60.0,
        showCrosshair: true,
        showStats: true
      }, options);

      if (this.options.sensor === 'mlx90640') {
        this.options.sensorWidth = 32;
        this.options.sensorHeight = 24;
      } else if (this.options.sensor === 'amg8833') {
        this.options.sensorWidth = 8;
        this.options.sensorHeight = 8;
      }

      this.hoverX = null;
      this.hoverY = null;
      this.tempStats = { min: 0, max: 0, avg: 0, spot: null };

      // Initialize buffer with default ambient data
      const totalPixels = this.options.sensorWidth * this.options.sensorHeight;
      this.rawTemps = new Float32Array(totalPixels);
      for (let i = 0; i < totalPixels; i++) {
        this.rawTemps[i] = 22.0;
      }

      this.initCanvas();
    }

    initCanvas() {
      this.container.innerHTML = '';
      this.canvas = document.createElement('canvas');
      this.canvas.className = 'agentla-thermal-canvas';
      this.canvas.style.display = 'block';
      this.canvas.style.width = '100%';
      this.canvas.style.height = '100%';
      this.canvas.style.imageRendering = 'auto';
      this.ctx = this.canvas.getContext('2d');
      this.container.appendChild(this.canvas);

      if (this.options.showCrosshair) {
        this.canvas.addEventListener('mousemove', (e) => this.onMouseMove(e));
        this.canvas.addEventListener('mouseleave', () => this.onMouseLeave());
      }

      this.resizeObserver = new ResizeObserver(() => this.resize());
      this.resizeObserver.observe(this.container);
      this.resize();
    }

    resize() {
      const rect = this.container.getBoundingClientRect();
      const dpr = window.devicePixelRatio || 1;
      const w = Math.max(rect.width || 240, 80);
      const h = Math.max(rect.height || 240, 80);

      this.canvas.width = Math.floor(w * dpr);
      this.canvas.height = Math.floor(h * dpr);
      this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      this.width = w;
      this.height = h;

      // Internal offscreen buffer for bilinear interpolation
      this.offscreenW = Math.min(256, Math.floor(w));
      this.offscreenH = Math.min(256, Math.floor(h));
      this.offscreenCanvas = document.createElement('canvas');
      this.offscreenCanvas.width = this.offscreenW;
      this.offscreenCanvas.height = this.offscreenH;
      this.offscreenCtx = this.offscreenCanvas.getContext('2d');
      this.offscreenImgData = this.offscreenCtx.createImageData(this.offscreenW, this.offscreenH);
      this.offscreenBuf32 = new Uint32Array(this.offscreenImgData.data.buffer);

      this.render();
    }

    onMouseMove(e) {
      const rect = this.canvas.getBoundingClientRect();
      this.hoverX = e.clientX - rect.left;
      this.hoverY = e.clientY - rect.top;
      this.render();
    }

    onMouseLeave() {
      this.hoverX = null;
      this.hoverY = null;
      this.tempStats.spot = null;
      this.render();
    }

    setSensor(type) {
      this.options.sensor = type;
      if (type === 'mlx90640') {
        this.options.sensorWidth = 32;
        this.options.sensorHeight = 24;
      } else {
        this.options.sensorWidth = 8;
        this.options.sensorHeight = 8;
      }
      this.rawTemps = new Float32Array(this.options.sensorWidth * this.options.sensorHeight);
      this.render();
    }

    setPalette(name) {
      if (PALETTES[name]) {
        this.options.palette = name;
        this.render();
      }
    }

    setRange(minC, maxC) {
      this.options.autoRange = false;
      this.options.minTempC = minC;
      this.options.maxTempC = maxC;
      this.render();
    }

    setFrame(temps) {
      if (!temps || temps.length === 0) return;
      const total = this.options.sensorWidth * this.options.sensorHeight;
      const count = Math.min(total, temps.length);

      let min = temps[0];
      let max = temps[0];
      let sum = 0;

      for (let i = 0; i < count; i++) {
        const val = temps[i];
        this.rawTemps[i] = val;
        if (val < min) min = val;
        if (val > max) max = val;
        sum += val;
      }

      this.tempStats.min = Math.round(min * 10) / 10;
      this.tempStats.max = Math.round(max * 10) / 10;
      this.tempStats.avg = Math.round((sum / count) * 10) / 10;

      if (this.options.autoRange) {
        this.options.minTempC = Math.floor(min);
        this.options.maxTempC = Math.ceil(max) + 0.1;
      }

      this.render();
    }

    getStats() {
      return Object.assign({}, this.tempStats);
    }

    sampleBilinear(u, v) {
      const sw = this.options.sensorWidth;
      const sh = this.options.sensorHeight;

      // Coordinate in sensor grid space
      const gx = u * (sw - 1);
      const gy = v * (sh - 1);

      const x0 = Math.floor(gx);
      const y0 = Math.floor(gy);
      const x1 = Math.min(x0 + 1, sw - 1);
      const y1 = Math.min(y0 + 1, sh - 1);

      const tx = gx - x0;
      const ty = gy - y0;

      const p00 = this.rawTemps[y0 * sw + x0];
      const p10 = this.rawTemps[y0 * sw + x1];
      const p01 = this.rawTemps[y1 * sw + x0];
      const p11 = this.rawTemps[y1 * sw + x1];

      // Bilinear interpolation
      const top = p00 * (1 - tx) + p10 * tx;
      const bottom = p01 * (1 - tx) + p11 * tx;
      return top * (1 - ty) + bottom * ty;
    }

    render() {
      if (!this.ctx || !this.width || !this.height || !this.offscreenW) return;
      const ctx = this.ctx;
      const w = this.width;
      const h = this.height;

      const lut = PALETTES[this.options.palette] || PALETTES.ironbow;
      const minT = this.options.minTempC;
      const maxT = Math.max(minT + 0.1, this.options.maxTempC);
      const rangeT = maxT - minT;

      const offW = this.offscreenW;
      const offH = this.offscreenH;
      const buf32 = this.offscreenBuf32;

      // Bilinear upsampling to offscreen buffer
      for (let y = 0; y < offH; y++) {
        const v = y / (offH - 1);
        const rowOffset = y * offW;
        for (let x = 0; x < offW; x++) {
          const u = x / (offW - 1);
          const t = this.sampleBilinear(u, v);
          const norm = Math.max(0, Math.min(1, (t - minT) / rangeT));
          const lutIdx = Math.floor(norm * 255);
          buf32[rowOffset + x] = lut[lutIdx];
        }
      }

      this.offscreenCtx.putImageData(this.offscreenImgData, 0, 0);

      // Draw offscreen buffer scaled up to main canvas
      ctx.clearRect(0, 0, w, h);
      ctx.imageSmoothingEnabled = true;
      ctx.drawImage(this.offscreenCanvas, 0, 0, w, h);

      // Interactive Crosshair & Spot Temp
      if (this.hoverX !== null && this.hoverY !== null) {
        const u = Math.max(0, Math.min(1, this.hoverX / w));
        const v = Math.max(0, Math.min(1, this.hoverY / h));
        const spotTemp = Math.round(this.sampleBilinear(u, v) * 10) / 10;
        this.tempStats.spot = spotTemp;

        ctx.save();
        ctx.strokeStyle = 'rgba(255, 255, 255, 0.85)';
        ctx.lineWidth = 1;
        ctx.setLineDash([4, 4]);

        ctx.beginPath();
        ctx.moveTo(this.hoverX, 0);
        ctx.lineTo(this.hoverX, h);
        ctx.moveTo(0, this.hoverY);
        ctx.lineTo(w, this.hoverY);
        ctx.stroke();

        ctx.setLineDash([]);
        ctx.beginPath();
        ctx.arc(this.hoverX, this.hoverY, 4, 0, Math.PI * 2);
        ctx.fillStyle = '#ffffff';
        ctx.fill();

        // Spot label box
        const label = `${spotTemp} C`;
        ctx.font = 'bold 11px monospace';
        const textWidth = ctx.measureText(label).width;
        const boxX = Math.min(w - textWidth - 12, this.hoverX + 8);
        const boxY = Math.max(16, this.hoverY - 8);

        ctx.fillStyle = 'rgba(15, 23, 42, 0.85)';
        ctx.fillRect(boxX - 3, boxY - 12, textWidth + 6, 16);
        ctx.fillStyle = '#38bdf8';
        ctx.fillText(label, boxX, boxY);
        ctx.restore();
      }

      // Live Stats Overlay (HUD)
      if (this.options.showStats) {
        ctx.save();
        ctx.fillStyle = 'rgba(15, 23, 42, 0.75)';
        ctx.fillRect(6, 6, 130, 48);
        ctx.strokeStyle = '#334155';
        ctx.strokeRect(6, 6, 130, 48);

        ctx.font = '10px monospace';
        ctx.fillStyle = '#94a3b8';
        ctx.fillText(`MIN: ${this.tempStats.min} C`, 12, 20);
        ctx.fillText(`MAX: ${this.tempStats.max} C`, 12, 34);
        ctx.fillText(`AVG: ${this.tempStats.avg} C`, 12, 48);
        ctx.restore();
      }
    }

    destroy() {
      if (this.resizeObserver) {
        this.resizeObserver.disconnect();
      }
      this.container.innerHTML = '';
    }
  }

  // Register Custom Element
  if (typeof customElements !== 'undefined' && !customElements.get('agentla-thermal-view')) {
    customElements.define('agentla-thermal-view', class extends HTMLElement {
      connectedCallback() {
        if (!this.view) {
          this.view = new AgentlaThermalView(this, {
            sensor: this.getAttribute('sensor') || 'amg8833',
            palette: this.getAttribute('palette') || 'ironbow'
          });
        }
      }
      disconnectedCallback() {
        if (this.view) {
          this.view.destroy();
          this.view = null;
        }
      }
    });
  }

  global.AgentlaThermalView = AgentlaThermalView;
})(typeof window !== 'undefined' ? window : globalThis);
