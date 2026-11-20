/**
 * agentla-pinout-view.js - Standardized 40-Pin RP2350 Pin Arbiter Visualizer
 * Renders the standard Raspberry Pi Pico 2 / RP2350 40-pin dual-inline header
 * with real-time Pin Arbiter ownership badges, multi-function labels,
 * click-to-inspect pin telemetry, and DaisyUI badge styling.
 * Strictly zero emojis and zero external dependencies.
 */

(function(global) {
  'use strict';

  // Raspberry Pi Pico 2 (RP2350) Pinout Definitions
  const PICO2_PINS_LEFT = [
    { pin: 1,  name: 'GP0',  type: 'gpio', alts: ['UART0 TX', 'I2C0 SDA', 'PWM0 A'] },
    { pin: 2,  name: 'GP1',  type: 'gpio', alts: ['UART0 RX', 'I2C0 SCL', 'PWM0 B'] },
    { pin: 3,  name: 'GND',  type: 'gnd',  alts: ['Ground'] },
    { pin: 4,  name: 'GP2',  type: 'gpio', alts: ['SPI0 SCK', 'I2C1 SDA', 'PWM1 A'] },
    { pin: 5,  name: 'GP3',  type: 'gpio', alts: ['SPI0 TX',  'I2C1 SCL', 'PWM1 B'] },
    { pin: 6,  name: 'GP4',  type: 'gpio', alts: ['SPI0 RX',  'I2C0 SDA', 'PWM2 A'] },
    { pin: 7,  name: 'GP5',  type: 'gpio', alts: ['SPI0 CSn', 'I2C0 SCL', 'PWM2 B'] },
    { pin: 8,  name: 'GND',  type: 'gnd',  alts: ['Ground'] },
    { pin: 9,  name: 'GP6',  type: 'gpio', alts: ['I2C1 SDA', 'PWM3 A'] },
    { pin: 10, name: 'GP7',  type: 'gpio', alts: ['I2C1 SCL', 'PWM3 B'] },
    { pin: 11, name: 'GP8',  type: 'gpio', alts: ['UART1 TX', 'PWM4 A'] },
    { pin: 12, name: 'GP9',  type: 'gpio', alts: ['UART1 RX', 'PWM4 B'] },
    { pin: 13, name: 'GND',  type: 'gnd',  alts: ['Ground'] },
    { pin: 14, name: 'GP10', type: 'gpio', alts: ['SPI1 SCK', 'PWM5 A'] },
    { pin: 15, name: 'GP11', type: 'gpio', alts: ['SPI1 TX',  'PWM5 B'] },
    { pin: 16, name: 'GP12', type: 'gpio', alts: ['SPI1 RX',  'PWM6 A'] },
    { pin: 17, name: 'GP13', type: 'gpio', alts: ['SPI1 CSn', 'PWM6 B'] },
    { pin: 18, name: 'GND',  type: 'gnd',  alts: ['Ground'] },
    { pin: 19, name: 'GP14', type: 'gpio', alts: ['I2C1 SDA', 'PWM7 A'] },
    { pin: 20, name: 'GP15', type: 'gpio', alts: ['I2C1 SCL', 'PWM7 B'] }
  ];

  const PICO2_PINS_RIGHT = [
    { pin: 40, name: 'VBUS',     type: 'pwr', alts: ['5V USB Power'] },
    { pin: 39, name: 'VSYS',     type: 'pwr', alts: ['1.8V to 5.5V Main System Input'] },
    { pin: 38, name: 'GND',      type: 'gnd', alts: ['Ground'] },
    { pin: 37, name: '3V3_EN',   type: 'ctl', alts: ['SMPS Enable Control'] },
    { pin: 36, name: '3V3_OUT',  type: 'pwr', alts: ['3.3V Output Rail'] },
    { pin: 35, name: 'ADC_VREF', type: 'adc', alts: ['ADC Reference Voltage'] },
    { pin: 34, name: 'GP28',     type: 'adc', alts: ['ADC2', 'PWM12 A'] },
    { pin: 33, name: 'GND',      type: 'gnd', alts: ['Ground'] },
    { pin: 32, name: 'GP27',     type: 'adc', alts: ['ADC1', 'PWM11 B', 'I2C1 SCL'] },
    { pin: 31, name: 'GP26',     type: 'adc', alts: ['ADC0', 'PWM11 A', 'I2C1 SDA'] },
    { pin: 30, name: 'RUN',      type: 'ctl', alts: ['Processor Reset Catch'] },
    { pin: 29, name: 'GP22',     type: 'gpio',alts: ['PWM11 A'] },
    { pin: 28, name: 'GND',      type: 'gnd', alts: ['Ground'] },
    { pin: 27, name: 'GP21',     type: 'gpio',alts: ['I2C0 SCL', 'PWM10 B'] },
    { pin: 26, name: 'GP20',     type: 'gpio',alts: ['I2C0 SDA', 'PWM10 A'] },
    { pin: 25, name: 'GP19',     type: 'gpio',alts: ['SPI0 TX',  'PWM9 B'] },
    { pin: 24, name: 'GP18',     type: 'gpio',alts: ['SPI0 SCK', 'PWM9 A'] },
    { pin: 23, name: 'GND',      type: 'gnd', alts: ['Ground'] },
    { pin: 22, name: 'GP17',     type: 'gpio',alts: ['UART0 RX', 'PWM8 B'] },
    { pin: 21, name: 'GP16',     type: 'gpio',alts: ['UART0 TX', 'PWM8 A'] }
  ];

  class AgentlaPinoutView {
    constructor(container, options = {}) {
      this.container = typeof container === 'string' ? document.querySelector(container) : container;
      if (!this.container) {
        throw new Error('[AgentlaPinoutView] Valid container element required');
      }

      this.options = Object.assign({
        interactive: true,
        selectedPin: 1
      }, options);

      this.ownership = {}; // Map of pin/gpio -> { owner, direction, state }
      this.listeners = {};
      this.selectedPin = this.options.selectedPin;

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

    setOwnership(ownershipMap) {
      this.ownership = Object.assign({}, ownershipMap);
      this.render();
    }

    selectPin(pinNumber) {
      this.selectedPin = pinNumber;
      this.render();
      const info = this.getPinInfo(pinNumber);
      this.emit('select', info);
    }

    getPinInfo(pinNumber) {
      const all = PICO2_PINS_LEFT.concat(PICO2_PINS_RIGHT);
      const pinObj = all.find(p => p.pin === pinNumber);
      if (!pinObj) return null;
      const ownerData = this.ownership[pinObj.pin] || this.ownership[pinObj.name] || { owner: 'free' };
      return Object.assign({}, pinObj, ownerData);
    }

    render() {
      this.container.innerHTML = '';
      const root = document.createElement('div');
      root.className = 'agentla-pinout-container w-full h-full flex flex-col gap-2 p-2 font-mono text-xs select-none';

      const header = document.createElement('div');
      header.className = 'flex items-center justify-between border-b border-base-300 pb-2 text-[10px] text-base-content/70';
      header.innerHTML = '<span>RP2350 40-PIN ARBITER HEADER</span><span>PICO 2 PINOUT</span>';
      root.appendChild(header);

      const grid = document.createElement('div');
      grid.className = 'grid grid-cols-2 gap-3 flex-1 overflow-y-auto';

      // Left Column (Pins 1..20)
      const colLeft = document.createElement('div');
      colLeft.className = 'flex flex-col gap-1';
      PICO2_PINS_LEFT.forEach(pin => {
        colLeft.appendChild(this.createPinRow(pin, 'left'));
      });
      grid.appendChild(colLeft);

      // Right Column (Pins 40..21)
      const colRight = document.createElement('div');
      colRight.className = 'flex flex-col gap-1';
      PICO2_PINS_RIGHT.forEach(pin => {
        colRight.appendChild(this.createPinRow(pin, 'right'));
      });
      grid.appendChild(colRight);

      root.appendChild(grid);
      this.container.appendChild(root);
    }

    createPinRow(pin, side) {
      const row = document.createElement('div');
      const isSelected = (this.selectedPin === pin.pin);
      const owner = (this.ownership[pin.pin] && this.ownership[pin.pin].owner) ||
                    (this.ownership[pin.name] && this.ownership[pin.name].owner) || 'free';

      row.className = `flex items-center justify-between p-1 rounded cursor-pointer transition-colors ${
        isSelected ? 'bg-primary/20 border border-primary/50' : 'bg-base-200/60 hover:bg-base-300/60 border border-transparent'
      }`;

      row.addEventListener('click', () => {
        this.selectPin(pin.pin);
      });

      // Pin badge class depending on type / ownership
      let badgeClass = 'badge-ghost';
      if (pin.type === 'pwr') badgeClass = 'badge-error';
      else if (pin.type === 'gnd') badgeClass = 'badge-neutral';
      else if (pin.type === 'adc') badgeClass = 'badge-warning';
      else if (owner !== 'free') badgeClass = 'badge-primary';

      const pinNumSpan = `<span class="text-[10px] text-base-content/50 w-5">${pin.pin}</span>`;
      const nameSpan = `<span class="font-bold text-xs">${pin.name}</span>`;
      const ownerBadge = `<span class="badge ${badgeClass} badge-xs font-mono">${owner !== 'free' ? owner : pin.type}</span>`;

      if (side === 'left') {
        row.innerHTML = `<div class="flex items-center gap-2">${pinNumSpan}${nameSpan}</div><div>${ownerBadge}</div>`;
      } else {
        row.innerHTML = `<div>${ownerBadge}</div><div class="flex items-center gap-2">${nameSpan}${pinNumSpan}</div>`;
      }

      return row;
    }

    destroy() {
      this.container.innerHTML = '';
    }
  }

  // Register Custom Element
  if (typeof customElements !== 'undefined' && !customElements.get('agentla-pinout-view')) {
    customElements.define('agentla-pinout-view', class extends HTMLElement {
      connectedCallback() {
        if (!this.view) {
          this.view = new AgentlaPinoutView(this, {});
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

  global.AgentlaPinoutView = AgentlaPinoutView;
})(typeof window !== 'undefined' ? window : globalThis);
