/**
 * agentla-led-view.js - Standardized Neopixel LED Visualizer Engine
 * Supports 1D linear strips, 2D matrices (serpentine/zigzag), and circular rings.
 * Features Canvas2D radial bloom rendering and real-time power dissipation telemetry.
 * Strictly zero emojis and zero external dependencies.
 */

(function(global) {
  'use strict';

  class AgentlaLedView {
    constructor(container, options = {}) {
      this.container = typeof container === 'string' ? document.querySelector(container) : container;
      if (!this.container) {
        throw new Error('[AgentlaLedView] Valid container element required');
      }

      this.options = Object.assign({
        topology: 'strip',       // 'strip' | 'matrix' | 'ring'
        count: 16,               // Total LEDs for strip or ring
        matrixWidth: 8,          // Width for matrix
        matrixHeight: 8,         // Height for matrix
        serpentine: true,        // Serpentine/zigzag layout for matrix
        ledSize: 14,             // LED radius/size in px
        spacing: 6,              // Spacing between LEDs in px
        bloom: true,             // Enable radial bloom shader
        voltage: 5.0,            // Supply voltage (V) for power calculation
        quiescentCurrentMa: 1.0, // Quiescent idle current per LED (mA)
        maxChannelCurrentMa: 20.0// Max current per RGB channel at 255 (mA)
      }, options);

      this.pixels = [];
      this.initCanvas();
      this.setTopology(this.options.topology, this.options);
    }

    initCanvas() {
      this.container.innerHTML = '';
      this.canvas = document.createElement('canvas');
      this.canvas.className = 'agentla-led-canvas';
      this.canvas.style.display = 'block';
      this.canvas.style.width = '100%';
      this.canvas.style.height = '100%';
      this.canvas.style.imageRendering = 'pixelated';
      this.ctx = this.canvas.getContext('2d', { alpha: true });
      this.container.appendChild(this.canvas);

      this.resizeObserver = new ResizeObserver(() => this.resize());
      this.resizeObserver.observe(this.container);
      this.resize();
    }

    resize() {
      const rect = this.container.getBoundingClientRect();
      const dpr = window.devicePixelRatio || 1;
      const w = Math.max(rect.width || 300, 100);
      const h = Math.max(rect.height || 180, 100);

      this.canvas.width = Math.floor(w * dpr);
      this.canvas.height = Math.floor(h * dpr);
      this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      this.width = w;
      this.height = h;
      this.render();
    }

    setTopology(type, params = {}) {
      this.options.topology = type || 'strip';
      if (params.count !== undefined) this.options.count = Math.max(1, parseInt(params.count, 10));
      if (params.matrixWidth !== undefined) this.options.matrixWidth = Math.max(1, parseInt(params.matrixWidth, 10));
      if (params.matrixHeight !== undefined) this.options.matrixHeight = Math.max(1, parseInt(params.matrixHeight, 10));
      if (params.serpentine !== undefined) this.options.serpentine = Boolean(params.serpentine);
      if (params.bloom !== undefined) this.options.bloom = Boolean(params.bloom);

      const totalLeds = this.options.topology === 'matrix'
        ? (this.options.matrixWidth * this.options.matrixHeight)
        : this.options.count;

      if (this.pixels.length !== totalLeds) {
        const nextPixels = new Array(totalLeds);
        for (let i = 0; i < totalLeds; i++) {
          nextPixels[i] = (i < this.pixels.length) ? this.pixels[i] : [0, 0, 0];
        }
        this.pixels = nextPixels;
      }
      this.render();
    }

    setPixels(buffer) {
      if (!buffer) return;
      const count = this.pixels.length;

      if (buffer instanceof Uint8Array || Array.isArray(buffer)) {
        if (buffer.length > 0 && typeof buffer[0] === 'number' && buffer.length >= count * 3) {
          // Flat byte stream [r0, g0, b0, r1, g1, b1, ...]
          for (let i = 0; i < count; i++) {
            const idx = i * 3;
            this.pixels[i] = [buffer[idx], buffer[idx + 1], buffer[idx + 2]];
          }
        } else {
          // Array of [r, g, b] or hex numbers
          for (let i = 0; i < count; i++) {
            if (i >= buffer.length) break;
            const item = buffer[i];
            if (Array.isArray(item)) {
              this.pixels[i] = [item[0] || 0, item[1] || 0, item[2] || 0];
            } else if (typeof item === 'number') {
              this.pixels[i] = [(item >> 16) & 0xff, (item >> 8) & 0xff, item & 0xff];
            } else if (item && typeof item === 'object') {
              this.pixels[i] = [item.r || 0, item.g || 0, item.b || 0];
            }
          }
        }
      }
      this.render();
    }

    setPixel(index, r, g, b) {
      if (index >= 0 && index < this.pixels.length) {
        this.pixels[index] = [r & 0xff, g & 0xff, b & 0xff];
        this.render();
      }
    }

    clear() {
      for (let i = 0; i < this.pixels.length; i++) {
        this.pixels[i] = [0, 0, 0];
      }
      this.render();
    }

    getPowerStats() {
      const count = this.pixels.length;
      const v = this.options.voltage;
      const iIdle = this.options.quiescentCurrentMa;
      const iMaxCh = this.options.maxChannelCurrentMa;

      let totalCurrentMa = 0;
      for (let i = 0; i < count; i++) {
        const [r, g, b] = this.pixels[i] || [0, 0, 0];
        const ledCurrent = iIdle + (r / 255) * iMaxCh + (g / 255) * iMaxCh + (b / 255) * iMaxCh;
        totalCurrentMa += ledCurrent;
      }

      const maxCurrentMa = count * (iIdle + iMaxCh * 3);
      const powerMw = totalCurrentMa * v;
      const powerW = powerMw / 1000.0;

      return {
        ledCount: count,
        voltage: v,
        currentMa: Math.round(totalCurrentMa * 10) / 10,
        maxCurrentMa: Math.round(maxCurrentMa * 10) / 10,
        powerMw: Math.round(powerMw),
        powerW: Math.round(powerW * 100) / 100
      };
    }

    computeCoordinates() {
      const coords = [];
      const topo = this.options.topology;
      const count = this.pixels.length;
      const w = this.width;
      const h = this.height;

      if (topo === 'strip') {
        const padding = 20;
        const availableW = Math.max(w - padding * 2, 40);
        const step = count > 1 ? availableW / (count - 1) : 0;
        const cy = h / 2;
        const startX = count > 1 ? padding : w / 2;

        for (let i = 0; i < count; i++) {
          coords.push({
            x: startX + i * step,
            y: cy,
            r: Math.min(this.options.ledSize, Math.max(4, step * 0.42))
          });
        }
      } else if (topo === 'matrix') {
        const cols = this.options.matrixWidth;
        const rows = this.options.matrixHeight;
        const padding = 20;
        const availW = Math.max(w - padding * 2, 40);
        const availH = Math.max(h - padding * 2, 40);
        const stepX = cols > 1 ? availW / (cols - 1) : 0;
        const stepY = rows > 1 ? availH / (rows - 1) : 0;
        const step = Math.min(stepX, stepY);
        const offsetX = (w - (cols - 1) * step) / 2;
        const offsetY = (h - (rows - 1) * step) / 2;
        const ledR = Math.max(3, Math.min(this.options.ledSize, step * 0.38));

        for (let y = 0; y < rows; y++) {
          for (let x = 0; x < cols; x++) {
            let physicalX = x;
            if (this.options.serpentine && (y % 2 === 1)) {
              physicalX = cols - 1 - x;
            }
            coords.push({
              x: offsetX + physicalX * step,
              y: offsetY + y * step,
              r: ledR
            });
          }
        }
      } else if (topo === 'ring') {
        const cx = w / 2;
        const cy = h / 2;
        const maxRadius = Math.max(10, Math.min(w, h) / 2 - 24);
        const radius = maxRadius;
        const stepAngle = (2 * Math.PI) / count;
        const arcDist = (2 * Math.PI * radius) / count;
        const ledR = Math.max(3, Math.min(this.options.ledSize, arcDist * 0.42));

        for (let i = 0; i < count; i++) {
          const angle = -Math.PI / 2 + i * stepAngle;
          coords.push({
            x: cx + Math.cos(angle) * radius,
            y: cy + Math.sin(angle) * radius,
            r: ledR
          });
        }
      }

      return coords;
    }

    render() {
      if (!this.ctx || !this.width || !this.height) return;
      const ctx = this.ctx;
      const w = this.width;
      const h = this.height;

      ctx.clearRect(0, 0, w, h);

      // Background subtle circuit board grid
      ctx.fillStyle = '#090d16';
      ctx.fillRect(0, 0, w, h);

      const coords = this.computeCoordinates();
      const bloom = this.options.bloom;

      // Pass 1: Bloom pass (radial gradient)
      if (bloom) {
        ctx.save();
        ctx.globalCompositeOperation = 'screen';
        for (let i = 0; i < coords.length; i++) {
          const c = coords[i];
          const [r, g, b] = this.pixels[i] || [0, 0, 0];
          if (r === 0 && g === 0 && b === 0) continue;

          const bloomRadius = c.r * 2.8;
          const grad = ctx.createRadialGradient(c.x, c.y, c.r * 0.2, c.x, c.y, bloomRadius);
          grad.addColorStop(0, `rgba(${r}, ${g}, ${b}, 0.8)`);
          grad.addColorStop(0.4, `rgba(${r}, ${g}, ${b}, 0.35)`);
          grad.addColorStop(1, `rgba(${r}, ${g}, ${b}, 0.0)`);

          ctx.fillStyle = grad;
          ctx.beginPath();
          ctx.arc(c.x, c.y, bloomRadius, 0, Math.PI * 2);
          ctx.fill();
        }
        ctx.restore();
      }

      // Pass 2: LED die and solder bezel
      for (let i = 0; i < coords.length; i++) {
        const c = coords[i];
        const [r, g, b] = this.pixels[i] || [0, 0, 0];
        const isLit = (r > 0 || g > 0 || b > 0);

        // Bezel / Pad ring
        ctx.beginPath();
        ctx.arc(c.x, c.y, c.r + 1.5, 0, Math.PI * 2);
        ctx.fillStyle = '#1e293b';
        ctx.fill();
        ctx.lineWidth = 1;
        ctx.strokeStyle = '#334155';
        ctx.stroke();

        // LED Lens / Emitter
        ctx.beginPath();
        ctx.arc(c.x, c.y, c.r, 0, Math.PI * 2);
        if (isLit) {
          const coreGrad = ctx.createRadialGradient(c.x, c.y, c.r * 0.1, c.x, c.y, c.r);
          coreGrad.addColorStop(0, '#ffffff');
          coreGrad.addColorStop(0.3, `rgb(${r}, ${g}, ${b})`);
          coreGrad.addColorStop(1, `rgb(${Math.floor(r * 0.65)}, ${Math.floor(g * 0.65)}, ${Math.floor(b * 0.65)})`);
          ctx.fillStyle = coreGrad;
        } else {
          ctx.fillStyle = '#0f172a';
        }
        ctx.fill();

        // Lens specular reflection
        ctx.beginPath();
        ctx.arc(c.x - c.r * 0.28, c.y - c.r * 0.28, c.r * 0.25, 0, Math.PI * 2);
        ctx.fillStyle = isLit ? 'rgba(255, 255, 255, 0.45)' : 'rgba(255, 255, 255, 0.08)';
        ctx.fill();
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
  if (typeof customElements !== 'undefined' && !customElements.get('agentla-led-view')) {
    customElements.define('agentla-led-view', class extends HTMLElement {
      connectedCallback() {
        if (!this.view) {
          this.view = new AgentlaLedView(this, {
            topology: this.getAttribute('topology') || 'strip',
            count: parseInt(this.getAttribute('count') || '16', 10),
            matrixWidth: parseInt(this.getAttribute('matrix-width') || '8', 10),
            matrixHeight: parseInt(this.getAttribute('matrix-height') || '8', 10)
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

  global.AgentlaLedView = AgentlaLedView;
})(typeof window !== 'undefined' ? window : globalThis);
