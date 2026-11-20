/**
 * agentla-servo-view.js - Standardized Parametric Servo Horn Visualizer & Keyframe Sequencer
 * Parametric horn models: Single Arm, Double Arm, Cross, and Circular Drum/Wheel.
 * Features travel arc limits, mounting hole splines, target ghost needle,
 * and built-in timeline keyframe sequencer with cubic easing.
 * Strictly zero emojis and zero external dependencies.
 */

(function(global) {
  'use strict';

  class AgentlaServoView {
    constructor(container, options = {}) {
      this.container = typeof container === 'string' ? document.querySelector(container) : container;
      if (!this.container) {
        throw new Error('[AgentlaServoView] Valid container element required');
      }

      this.options = Object.assign({
        hornType: 'single', // 'single' | 'double' | 'cross' | 'wheel'
        angle: 90,          // Current angle in degrees
        targetAngle: null,  // Target setpoint angle (optional needle)
        minAngle: 0,        // Minimum physical angle
        maxAngle: 180,      // Maximum physical angle
        colorHorn: '#cbd5e1',// Horn body color
        colorTrack: '#334155',// Arc track color
        colorAccent: '#38bdf8',// Active angle indicator
        interactive: true   // Allow dragging to set angle
      }, options);

      this.currentAngle = this.options.angle;
      this.targetAngle = this.options.targetAngle;
      this.isDragging = false;
      this.listeners = {};

      // Timeline Keyframe Sequencer
      this.timeline = {
        playing: false,
        durationMs: 3000,
        loop: true,
        currentTimeMs: 0,
        keyframes: [
          { timeMs: 0, angle: 0 },
          { timeMs: 1000, angle: 90 },
          { timeMs: 2000, angle: 180 },
          { timeMs: 3000, angle: 0 }
        ],
        rafId: null,
        lastTimestamp: 0
      };

      this.initCanvas();
    }

    initCanvas() {
      this.container.innerHTML = '';
      this.canvas = document.createElement('canvas');
      this.canvas.className = 'agentla-servo-canvas';
      this.canvas.style.display = 'block';
      this.canvas.style.width = '100%';
      this.canvas.style.height = '100%';
      this.canvas.style.touchAction = 'none';
      this.canvas.style.cursor = this.options.interactive ? 'grab' : 'default';
      this.ctx = this.canvas.getContext('2d', { alpha: true });
      this.container.appendChild(this.canvas);

      if (this.options.interactive) {
        this.canvas.addEventListener('pointerdown', (e) => this.onPointerDown(e));
        window.addEventListener('pointermove', (e) => this.onPointerMove(e));
        window.addEventListener('pointerup', () => this.onPointerUp());
      }

      this.resizeObserver = new ResizeObserver(() => this.resize());
      this.resizeObserver.observe(this.container);
      this.resize();
    }

    resize() {
      const rect = this.container.getBoundingClientRect();
      const dpr = window.devicePixelRatio || 1;
      const w = Math.max(rect.width || 220, 80);
      const h = Math.max(rect.height || 220, 80);

      this.canvas.width = Math.floor(w * dpr);
      this.canvas.height = Math.floor(h * dpr);
      this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      this.width = w;
      this.height = h;
      this.render();
    }

    on(event, callback) {
      if (!this.listeners[event]) this.listeners[event] = [];
      this.listeners[event].push(callback);
    }

    emit(event, data) {
      const list = this.listeners[event];
      if (list) {
        for (let i = 0; i < list.length; i++) {
          list[i](data);
        }
      }
    }

    onPointerDown(e) {
      this.isDragging = true;
      this.canvas.style.cursor = 'grabbing';
      this.updateAngleFromPointer(e);
    }

    onPointerMove(e) {
      if (!this.isDragging) return;
      this.updateAngleFromPointer(e);
    }

    onPointerUp() {
      if (this.isDragging) {
        this.isDragging = false;
        this.canvas.style.cursor = 'grab';
      }
    }

    updateAngleFromPointer(e) {
      const rect = this.canvas.getBoundingClientRect();
      const cx = rect.left + rect.width / 2;
      const cy = rect.top + rect.height / 2;
      const dx = e.clientX - cx;
      const dy = e.clientY - cy;

      // Map cartesian angle to servo degree (0 deg = left or bottom, standard 0-180 arc)
      let rad = Math.atan2(dy, dx); // [-PI, PI]
      let deg = (rad * 180 / Math.PI) + 180; // [0, 360]
      // Constrain within min/max
      deg = Math.max(this.options.minAngle, Math.min(this.options.maxAngle, Math.round(deg)));
      this.setAngle(deg, true);
    }

    setAngle(deg, emitEvent = false) {
      this.currentAngle = Math.max(this.options.minAngle, Math.min(this.options.maxAngle, deg));
      this.render();
      if (emitEvent) {
        this.emit('change', this.currentAngle);
      }
    }

    setTargetAngle(deg) {
      this.targetAngle = (deg !== null && deg !== undefined)
        ? Math.max(this.options.minAngle, Math.min(this.options.maxAngle, deg))
        : null;
      this.render();
    }

    setHornType(type) {
      this.options.hornType = type || 'single';
      this.render();
    }

    setRange(minAngle, maxAngle) {
      this.options.minAngle = minAngle;
      this.options.maxAngle = maxAngle;
      this.currentAngle = Math.max(minAngle, Math.min(maxAngle, this.currentAngle));
      this.render();
    }

    getAngle() {
      return this.currentAngle;
    }

    // --- Keyframe Timeline Sequencer ---
    addKeyframe(timeMs, angle) {
      this.timeline.keyframes.push({ timeMs, angle });
      this.timeline.keyframes.sort((a, b) => a.timeMs - b.timeMs);
    }

    play() {
      if (this.timeline.playing) return;
      this.timeline.playing = true;
      this.timeline.lastTimestamp = performance.now();
      const loop = (ts) => {
        if (!this.timeline.playing) return;
        const delta = ts - this.timeline.lastTimestamp;
        this.timeline.lastTimestamp = ts;
        this.timeline.currentTimeMs += delta;

        if (this.timeline.currentTimeMs > this.timeline.durationMs) {
          if (this.timeline.loop) {
            this.timeline.currentTimeMs = this.timeline.currentTimeMs % this.timeline.durationMs;
          } else {
            this.timeline.currentTimeMs = this.timeline.durationMs;
            this.pause();
            return;
          }
        }

        const interpolatedAngle = this.interpolateTimeline(this.timeline.currentTimeMs);
        this.setAngle(interpolatedAngle, true);
        this.timeline.rafId = requestAnimationFrame(loop);
      };
      this.timeline.rafId = requestAnimationFrame(loop);
      this.emit('play', null);
    }

    pause() {
      this.timeline.playing = false;
      if (this.timeline.rafId) {
        cancelAnimationFrame(this.timeline.rafId);
        this.timeline.rafId = null;
      }
      this.emit('pause', null);
    }

    interpolateTimeline(timeMs) {
      const kfs = this.timeline.keyframes;
      if (kfs.length === 0) return this.currentAngle;
      if (timeMs <= kfs[0].timeMs) return kfs[0].angle;
      if (timeMs >= kfs[kfs.length - 1].timeMs) return kfs[kfs.length - 1].angle;

      let idx = 0;
      while (idx < kfs.length - 1 && kfs[idx + 1].timeMs < timeMs) {
        idx++;
      }
      const k0 = kfs[idx];
      const k1 = kfs[idx + 1];
      const t = (timeMs - k0.timeMs) / (k1.timeMs - k0.timeMs);
      // Smooth cubic ease-in-out
      const ease = t < 0.5 ? 4 * t * t * t : 1 - Math.pow(-2 * t + 2, 3) / 2;
      return k0.angle + (k1.angle - k0.angle) * ease;
    }

    render() {
      if (!this.ctx || !this.width || !this.height) return;
      const ctx = this.ctx;
      const w = this.width;
      const h = this.height;
      const cx = w / 2;
      const cy = h / 2;
      const radius = Math.min(w, h) * 0.42;

      ctx.clearRect(0, 0, w, h);

      // Servo Case outline (rectangular casing)
      ctx.save();
      ctx.fillStyle = '#0f172a';
      ctx.strokeStyle = '#1e293b';
      ctx.lineWidth = 2;
      const caseSize = radius * 1.5;
      ctx.beginPath();
      ctx.roundRect(cx - caseSize / 2, cy - caseSize / 2, caseSize, caseSize, 12);
      ctx.fill();
      ctx.stroke();

      // Travel Arc Guide Track
      const minRad = (this.options.minAngle - 180) * Math.PI / 180;
      const maxRad = (this.options.maxAngle - 180) * Math.PI / 180;

      ctx.beginPath();
      ctx.arc(cx, cy, radius, minRad, maxRad, false);
      ctx.strokeStyle = this.options.colorTrack;
      ctx.lineWidth = 6;
      ctx.lineCap = 'round';
      ctx.stroke();

      // Degree tick marks (every 30 degrees)
      ctx.fillStyle = '#64748b';
      ctx.font = '9px monospace';
      ctx.textAlign = 'center';
      ctx.textBaseline = 'middle';
      for (let a = this.options.minAngle; a <= this.options.maxAngle; a += 30) {
        const rad = (a - 180) * Math.PI / 180;
        const tx = cx + Math.cos(rad) * (radius - 12);
        const ty = cy + Math.sin(rad) * (radius - 12);
        ctx.beginPath();
        ctx.arc(tx, ty, 1.5, 0, Math.PI * 2);
        ctx.fill();
      }

      // Target Angle Ghost Needle
      if (this.targetAngle !== null) {
        const targetRad = (this.targetAngle - 180) * Math.PI / 180;
        ctx.save();
        ctx.beginPath();
        ctx.setLineDash([3, 3]);
        ctx.moveTo(cx, cy);
        ctx.lineTo(cx + Math.cos(targetRad) * (radius + 8), cy + Math.sin(targetRad) * (radius + 8));
        ctx.strokeStyle = '#f59e0b';
        ctx.lineWidth = 2;
        ctx.stroke();
        ctx.restore();
      }

      // Horn Rendering with rotation
      const hornAngleRad = (this.currentAngle - 180) * Math.PI / 180;
      ctx.translate(cx, cy);
      ctx.rotate(hornAngleRad);

      this.renderHornShape(ctx, radius * 0.9);

      // Spline center screw
      ctx.beginPath();
      ctx.arc(0, 0, radius * 0.22, 0, Math.PI * 2);
      ctx.fillStyle = '#1e293b';
      ctx.fill();
      ctx.strokeStyle = '#475569';
      ctx.lineWidth = 2;
      ctx.stroke();

      // Metal screw center
      ctx.beginPath();
      ctx.arc(0, 0, radius * 0.12, 0, Math.PI * 2);
      ctx.fillStyle = '#94a3b8';
      ctx.fill();
      ctx.strokeStyle = '#334155';
      ctx.lineWidth = 1.5;
      ctx.beginPath();
      ctx.moveTo(-radius * 0.08, 0);
      ctx.lineTo(radius * 0.08, 0);
      ctx.moveTo(0, -radius * 0.08);
      ctx.lineTo(0, radius * 0.08);
      ctx.stroke();

      ctx.restore();

      // Digital Angle Badge at bottom
      ctx.save();
      ctx.fillStyle = this.options.colorAccent;
      ctx.font = 'bold 12px monospace';
      ctx.textAlign = 'center';
      ctx.fillText(`${Math.round(this.currentAngle)} deg`, cx, h - 8);
      ctx.restore();
    }

    renderHornShape(ctx, armLength) {
      const type = this.options.hornType;
      const hubR = armLength * 0.32;
      ctx.fillStyle = this.options.colorHorn;
      ctx.strokeStyle = '#64748b';
      ctx.lineWidth = 1.5;

      const drawArm = (angleRad, length) => {
        ctx.save();
        ctx.rotate(angleRad);
        ctx.beginPath();
        ctx.moveTo(0, -hubR * 0.6);
        ctx.lineTo(length, -hubR * 0.3);
        ctx.arc(length, 0, hubR * 0.3, -Math.PI / 2, Math.PI / 2);
        ctx.lineTo(0, hubR * 0.6);
        ctx.closePath();
        ctx.fill();
        ctx.stroke();

        // Spline holes along arm
        ctx.fillStyle = '#1e293b';
        const holeCount = 3;
        for (let i = 1; i <= holeCount; i++) {
          const hx = hubR + (length - hubR) * (i / (holeCount + 0.5));
          ctx.beginPath();
          ctx.arc(hx, 0, 2.5, 0, Math.PI * 2);
          ctx.fill();
        }
        ctx.restore();
      };

      if (type === 'wheel') {
        // Circular Winch / Wheel Horn
        ctx.beginPath();
        ctx.arc(0, 0, armLength * 0.85, 0, Math.PI * 2);
        ctx.fill();
        ctx.stroke();

        // Perimeter holes
        ctx.fillStyle = '#1e293b';
        const numHoles = 8;
        for (let i = 0; i < numHoles; i++) {
          const a = (i / numHoles) * Math.PI * 2;
          const hx = Math.cos(a) * (armLength * 0.68);
          const hy = Math.sin(a) * (armLength * 0.68);
          ctx.beginPath();
          ctx.arc(hx, hy, 2.5, 0, Math.PI * 2);
          ctx.fill();
        }
      } else if (type === 'cross') {
        drawArm(0, armLength);
        drawArm(Math.PI / 2, armLength * 0.85);
        drawArm(Math.PI, armLength);
        drawArm(-Math.PI / 2, armLength * 0.85);
      } else if (type === 'double') {
        drawArm(0, armLength);
        drawArm(Math.PI, armLength);
      } else {
        // Single Arm
        drawArm(0, armLength);
      }
    }

    destroy() {
      this.pause();
      if (this.resizeObserver) {
        this.resizeObserver.disconnect();
      }
      this.container.innerHTML = '';
    }
  }

  // Register Custom Element
  if (typeof customElements !== 'undefined' && !customElements.get('agentla-servo-view')) {
    customElements.define('agentla-servo-view', class extends HTMLElement {
      connectedCallback() {
        if (!this.view) {
          this.view = new AgentlaServoView(this, {
            hornType: this.getAttribute('horn-type') || 'single',
            angle: parseFloat(this.getAttribute('angle') || '90'),
            minAngle: parseFloat(this.getAttribute('min-angle') || '0'),
            maxAngle: parseFloat(this.getAttribute('max-angle') || '180')
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

  global.AgentlaServoView = AgentlaServoView;
})(typeof window !== 'undefined' ? window : globalThis);
