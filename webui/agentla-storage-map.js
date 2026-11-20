/**
 * agentla-storage-map.js - Standardized LittleFS Flash Sector Grid Wear Visualizer
 * Visualizes 440x4 KB User FS blocks and 256x4 KB System FS blocks with
 * wear-leveling counts, allocation status heatmap, flash address mapping,
 * and block inspection tooltips.
 * Strictly zero emojis and zero external dependencies.
 */

(function(global) {
  'use strict';

  const USER_FS_BASE = 0x00248000;
  const USER_FS_BLOCKS = 440;
  const SYSTEM_FS_BASE = 0x00148000;
  const SYSTEM_FS_BLOCKS = 256;
  const BLOCK_SIZE = 4096;

  class AgentlaStorageMap {
    constructor(container, options = {}) {
      this.container = typeof container === 'string' ? document.querySelector(container) : container;
      if (!this.container) {
        throw new Error('[AgentlaStorageMap] Valid container element required');
      }

      this.options = Object.assign({
        partition: 'user', // 'user' (440 blocks) | 'system' (256 blocks)
        blockSize: BLOCK_SIZE,
        interactive: true,
        showLegend: true
      }, options);

      this.selectedBlock = null;
      this.hoverBlock = null;
      this.listeners = {};

      this.initBlocks();
      this.initCanvas();
    }

    initBlocks() {
      const count = (this.options.partition === 'system') ? SYSTEM_FS_BLOCKS : USER_FS_BLOCKS;
      const baseAddr = (this.options.partition === 'system') ? SYSTEM_FS_BASE : USER_FS_BASE;

      this.blocks = new Array(count);
      for (let i = 0; i < count; i++) {
        this.blocks[i] = {
          index: i,
          address: baseAddr + i * BLOCK_SIZE,
          state: 'free', // 'free' | 'data' | 'meta' | 'wear'
          eraseCount: 0,
          filePath: null
        };
      }
    }

    initCanvas() {
      this.container.innerHTML = '';
      this.canvas = document.createElement('canvas');
      this.canvas.className = 'agentla-storage-canvas';
      this.canvas.style.display = 'block';
      this.canvas.style.width = '100%';
      this.canvas.style.height = '100%';
      this.canvas.style.imageRendering = 'pixelated';
      this.ctx = this.canvas.getContext('2d');
      this.container.appendChild(this.canvas);

      if (this.options.interactive) {
        this.canvas.addEventListener('mousemove', (e) => this.onMouseMove(e));
        this.canvas.addEventListener('mouseleave', () => this.onMouseLeave());
        this.canvas.addEventListener('click', (e) => this.onClick(e));
      }

      this.resizeObserver = new ResizeObserver(() => this.resize());
      this.resizeObserver.observe(this.container);
      this.resize();
    }

    resize() {
      const rect = this.container.getBoundingClientRect();
      const dpr = window.devicePixelRatio || 1;
      const w = Math.max(rect.width || 320, 100);
      const h = Math.max(rect.height || 180, 100);

      this.canvas.width = Math.floor(w * dpr);
      this.canvas.height = Math.floor(h * dpr);
      this.ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
      this.width = w;
      this.height = h;
      this.render();
    }

    setPartition(partition) {
      if (this.options.partition !== partition) {
        this.options.partition = partition;
        this.selectedBlock = null;
        this.hoverBlock = null;
        this.initBlocks();
        this.render();
      }
    }

    setAllocation(blockMap) {
      // blockMap: array or object of { index, state, eraseCount, filePath }
      if (!blockMap) return;
      if (Array.isArray(blockMap)) {
        blockMap.forEach(b => {
          if (b.index >= 0 && b.index < this.blocks.length) {
            Object.assign(this.blocks[b.index], b);
          }
        });
      } else {
        Object.keys(blockMap).forEach(key => {
          const idx = parseInt(key, 10);
          if (idx >= 0 && idx < this.blocks.length) {
            Object.assign(this.blocks[idx], blockMap[key]);
          }
        });
      }
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

    getBlockAtCoords(px, py) {
      const count = this.blocks.length;
      const w = this.width;
      const h = this.height;
      const padding = 12;

      const availW = w - padding * 2;
      const availH = h - padding * 2;
      // Calculate optimal grid columns
      const cols = Math.min(32, Math.max(16, Math.floor(Math.sqrt(count * (availW / availH)))));
      const rows = Math.ceil(count / cols);

      const cellW = (availW - (cols - 1) * 2) / cols;
      const cellH = (availH - (rows - 1) * 2) / rows;
      const cellSize = Math.min(cellW, cellH);

      const gridStartX = padding + (availW - cols * (cellSize + 2)) / 2;
      const gridStartY = padding + (availH - rows * (cellSize + 2)) / 2;

      const relX = px - gridStartX;
      const relY = py - gridStartY;

      if (relX < 0 || relY < 0) return null;
      const col = Math.floor(relX / (cellSize + 2));
      const row = Math.floor(relY / (cellSize + 2));

      if (col >= 0 && col < cols && row >= 0 && row < rows) {
        const idx = row * cols + col;
        if (idx < count) {
          return this.blocks[idx];
        }
      }
      return null;
    }

    onMouseMove(e) {
      const rect = this.canvas.getBoundingClientRect();
      const px = e.clientX - rect.left;
      const py = e.clientY - rect.top;
      const block = this.getBlockAtCoords(px, py);
      if (block !== this.hoverBlock) {
        this.hoverBlock = block;
        this.render();
      }
    }

    onMouseLeave() {
      if (this.hoverBlock) {
        this.hoverBlock = null;
        this.render();
      }
    }

    onClick(e) {
      const rect = this.canvas.getBoundingClientRect();
      const px = e.clientX - rect.left;
      const py = e.clientY - rect.top;
      const block = this.getBlockAtCoords(px, py);
      if (block) {
        this.selectedBlock = block;
        this.render();
        this.emit('select', block);
      }
    }

    render() {
      if (!this.ctx || !this.width || !this.height) return;
      const ctx = this.ctx;
      const w = this.width;
      const h = this.height;
      const count = this.blocks.length;

      ctx.clearRect(0, 0, w, h);

      // Background
      ctx.fillStyle = '#090d16';
      ctx.fillRect(0, 0, w, h);

      const padding = 12;
      const availW = w - padding * 2;
      const availH = h - padding * 2 - (this.options.showLegend ? 24 : 0);
      const cols = Math.min(32, Math.max(16, Math.floor(Math.sqrt(count * (availW / availH)))));
      const rows = Math.ceil(count / cols);

      const cellW = (availW - (cols - 1) * 2) / cols;
      const cellH = (availH - (rows - 1) * 2) / rows;
      const cellSize = Math.max(3, Math.min(cellW, cellH));

      const gridStartX = padding + (availW - cols * (cellSize + 2)) / 2;
      const gridStartY = padding + (availH - rows * (cellSize + 2)) / 2;

      // Color palette for block states
      const colorMap = {
        free: '#1e293b',    // Slate empty
        data: '#0284c7',    // Primary blue data
        meta: '#f59e0b',    // Amber metadata
        wear: '#ef4444'     // Error red wear
      };

      for (let i = 0; i < count; i++) {
        const b = this.blocks[i];
        const row = Math.floor(i / cols);
        const col = i % cols;
        const x = gridStartX + col * (cellSize + 2);
        const y = gridStartY + row * (cellSize + 2);

        const isHover = (this.hoverBlock === b);
        const isSelected = (this.selectedBlock === b);

        ctx.fillStyle = colorMap[b.state] || colorMap.free;
        ctx.fillRect(x, y, cellSize, cellSize);

        if (isSelected) {
          ctx.strokeStyle = '#ffffff';
          ctx.lineWidth = 1.5;
          ctx.strokeRect(x - 1, y - 1, cellSize + 2, cellSize + 2);
        } else if (isHover) {
          ctx.strokeStyle = '#38bdf8';
          ctx.lineWidth = 1;
          ctx.strokeRect(x - 0.5, y - 0.5, cellSize + 1, cellSize + 1);
        }
      }

      // Legend & Tooltip Overlay
      if (this.options.showLegend) {
        ctx.save();
        ctx.font = '10px monospace';
        ctx.fillStyle = '#64748b';
        const legendY = h - 8;

        const drawLegendItem = (lx, color, text) => {
          ctx.fillStyle = color;
          ctx.fillRect(lx, legendY - 8, 8, 8);
          ctx.fillStyle = '#94a3b8';
          ctx.fillText(text, lx + 12, legendY);
          return lx + 12 + ctx.measureText(text).width + 12;
        };

        let currX = padding;
        currX = drawLegendItem(currX, colorMap.free, 'FREE');
        currX = drawLegendItem(currX, colorMap.data, 'DATA');
        currX = drawLegendItem(currX, colorMap.meta, 'META');
        currX = drawLegendItem(currX, colorMap.wear, 'WEAR');

        // Target block details if hovered/selected
        const activeBlock = this.hoverBlock || this.selectedBlock;
        if (activeBlock) {
          const infoText = `BLK #${activeBlock.index} (0x${activeBlock.address.toString(16).toUpperCase()}) [${activeBlock.state.toUpperCase()}]`;
          ctx.fillStyle = '#38bdf8';
          ctx.textAlign = 'right';
          ctx.fillText(infoText, w - padding, legendY);
        }
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
  if (typeof customElements !== 'undefined' && !customElements.get('agentla-storage-map')) {
    customElements.define('agentla-storage-map', class extends HTMLElement {
      connectedCallback() {
        if (!this.view) {
          this.view = new AgentlaStorageMap(this, {
            partition: this.getAttribute('partition') || 'user'
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

  global.AgentlaStorageMap = AgentlaStorageMap;
})(typeof window !== 'undefined' ? window : globalThis);
