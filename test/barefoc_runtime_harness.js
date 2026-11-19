// Minimal DOM/bridge stub harness to execute barefoc.html inline JS and catch runtime errors.
const fs = require('fs');
const html = fs.readFileSync(require('path').join(__dirname, '..', 'webui', 'barefoc.html'), 'utf8');
const script = [...html.matchAll(/<script>([\s\S]*?)<\/script>/g)].map(m => m[1]).pop();

// Build element registry from HTML ids/classes.
const ids = [...html.matchAll(/id="([^"]+)"/g)].map(m => m[1]);
const classes = new Set();
for (const m of html.matchAll(/class="([^"]+)"/g)) m[1].split(/\s+/).forEach(c => classes.add(c));

let rafCount = 0;
function mkEl(id) {
  let _id = id || '';
  const el = {
    _cls: new Set(), style: {}, dataset: {}, children: [], value: '', textContent: '',
    get id(){ return _id; },
    set id(v){ _id = v; registry[v] = el; },
    checked: false, disabled: false, isConnected: true,
    classList: {
      add(...c){ c.forEach(x=>el._cls.add(x)); }, remove(...c){ c.forEach(x=>el._cls.delete(x)); },
      toggle(c,f){ if(f===undefined) f=!el._cls.has(c); f?el._cls.add(c):el._cls.delete(c); },
      contains(c){ return el._cls.has(c); }, replace(a,b){ if(el._cls.delete(a)) el._cls.add(b); }
    },
    set className(v){ el._cls = new Set(String(v).split(/\s+/).filter(Boolean)); },
    get className(){ return [...el._cls].join(' '); },
    set innerHTML(v){ el._html = v; el.children = []; },
    get innerHTML(){ return el._html || ''; },
    appendChild(c){ el.children.push(c); c.parentElement = el; return c; },
    insertBefore(c){ el.children.unshift(c); c.parentElement = el; return c; },
    removeChild(c){ el.children = el.children.filter(x=>x!==c); },
    remove(){ if(el.parentElement) el.parentElement.removeChild(el); },
    addEventListener(){}, removeEventListener(){}, setAttribute(){}, getAttribute(){ return null; },
    cloneNode(){ return mkEl(id); },
    setPointerCapture(){}, releasePointerCapture(){}, getBoundingClientRect(){ return {left:0,top:0,width:1024,height:320}; },
    getContext(){ return ctx2d; },
    querySelector(sel){ return mkEl(sel); }, querySelectorAll(){ return []; },
    focus(){}, click(){}, scrollTop:0, scrollHeight:0, width:1024, height:320,
    parentElement: null
  };
  return el;
}
const ctx2d = new Proxy({}, { get:(t,p)=>{ if(p==='canvas') return {}; return ()=>{}; }, set:()=>true });
const registry = {};
for (const id of ids) registry[id] = mkEl(id);

// A few elements need specific initial values.
if (registry['sel-protocol']) registry['sel-protocol'].value = 'commander';
if (registry['sel-transport']) registry['sel-transport'].value = 'simulation';
if (registry['sel-baud']) registry['sel-baud'].value = '115200';
if (registry['input-can-node']) registry['input-can-node'].value = '1';
if (registry['sel-trace-a']) registry['sel-trace-a'].value = 'target';
if (registry['sel-trace-b']) registry['sel-trace-b'].value = 'angle';
if (registry['sel-step-type']) registry['sel-step-type'].value = 'angle';
if (registry['sel-drv-model']) registry['sel-drv-model'].value = 'DRV8316';
if (registry['term-input']) registry['term-input'].value = '';

function querySelectorAll(sel) {
  // Handle document.querySelectorAll('.nav-tab-btn') etc by ids that have the class.
  if (sel.startsWith('.')) {
    const cls = sel.slice(1);
    return ids.filter(id => {
      const m = html.match(new RegExp('id="'+id+'"[^>]*class="[^"]*\\b'+cls+'\\b|class="[^"]*\\b'+cls+'\\b[^"]*"[^>]*id="'+id+'"'));
      return !!m;
    }).map(id => registry[id]);
  }
  if (sel === '.view-panel') return ids.filter(i=>i.startsWith('view-')).map(i=>registry[i]);
  if (sel === '.nav-tab-btn') return []; // legacy hidden path
  return [];
}

global.document = {
  getElementById: (id) => registry[id] || null,
  querySelector: (sel) => {
    if (sel === 'main') return mkEl('main');
    if (sel === '#view-dashboard .grid') return mkEl('hud-grid');
    if (sel === '.nav-tab-btn') return null;
    return mkEl('q');
  },
  querySelectorAll,
  createElement: (tag) => mkEl(tag),
  addEventListener(){}, removeEventListener(){},
  documentElement: { setAttribute(){}, classList: { add(){}, remove(){}, toggle(){}, contains(){return false;} } },
  body: mkEl('body')
};
global.window = {
  location: { protocol:'http:', host:'localhost' },
  addEventListener(){}, matchMedia(){ return {matches:false}; },
  FeatureRegistry: { hasTargetPowerControl: true }
};
global.localStorage = { getItem(){ return null; }, setItem(){} };
global.performance = { now: () => Date.now() };
global.requestAnimationFrame = (fn) => { if (rafCount++ < 3) fn(); return 1; };
global.setInterval = () => 1;
global.clearInterval = () => {};
global.setTimeout = (fn) => { try{ fn(); }catch(e){}; return 1; };
global.alert = () => {};
global.fetch = () => Promise.resolve({ json: () => Promise.resolve({}) });
global.WebSocket = function(){ this.readyState = 0; this.close=()=>{}; this.send=()=>{}; };
global.AgentLA = {
  i2c: { scan: async()=>[], write: async()=>{}, transfer: async()=>new Uint8Array(4) },
  spi: { transfer: async()=>new Uint8Array(2) },
  telemetry: { emergencySafeState: async()=>{} }
};
global.navigator = {};
global.Blob = function(){}; global.URL = { createObjectURL:()=> 'blob:x', revokeObjectURL(){} };
global.FileReader = function(){};

try {
  const probe = `\n;(function(){ console.log('RUNTIME OK: script executed without error');
  console.log('STUDIO_V2:', STUDIO_V2, 'shellRoot:', typeof shellRoot !== 'undefined' && !!shellRoot, 'viewMode:', typeof studioViewMode !== 'undefined' && studioViewMode);
  console.log('targets:', JSON.stringify(studio.domains.targets.map(t=>t.id+':'+t.axes.length)));
  console.log('dashboards:', studio.domains.dashboards.map(d=>d.name).join(','));
  console.log('params:', Object.keys(studio.domains.params).length ? Object.keys(studio.domains.params[Object.keys(studio.domains.params)[0]]).length : 0);
  console.log('footer chips:', (document.getElementById('studio-footer') ? document.getElementById('studio-footer').children.length : 'n/a'));
  console.log('studio-view children:', document.getElementById('studio-view') ? document.getElementById('studio-view').children.length : 'n/a');
  console.log('dial:', document.getElementById('dial-center-text').textContent);
  console.log('--- interactive probes ---');
  switchStudioView('wizard');
  console.log('wizard pages:', wizardState.pages.length, 'first:', wizardState.pages[0].title);
  // advance through pages by marking choices and clicking Next equivalent
  wizardState.choiceMade[wizardState.pages[0].key] = true;
  console.log('wizard can-advance:', !!wizardState.choiceMade[wizardState.pages[0].key]);
  switchStudioView('params');
  console.log('params view children:', document.getElementById('studio-view').children.length);
  const node = explorer.allNodes(studio.domains.activeTargetId)[0];
  addWidgetToDash('slider', node);
  console.log('dash widgets after add:', activeDash().controls.length);
  registerSampled('custom.param');
  console.log('sampled count:', studio.domains.sampledProperties.length);
  console.log('decodeErrors:', decodeErrors('motor', 3).join(','));
  console.log('config export target:', studio.domains.activeTargetId);
  applyWizardConfig();
  console.log('applyWizardConfig OK');
  })();`;
  eval(script + probe);
} catch (e) {
  console.error('RUNTIME ERROR:', e.stack || e.message);
  process.exit(1);
}
