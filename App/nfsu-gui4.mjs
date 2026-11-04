// GUI v4: observations and cues top-right, proposed decisions bottom-left.
//
// Layout:
//   top-left      compact HUD (mode, tick, keys, speed)
//   top-right     OBSERVATIONS (what memory reports) + CUES (what pixels report)
//   bottom-left   DECISIONS (the candidate actions the policy weighed, with the
//                 chosen one highlighted; the mode buttons live here too)
//
// Still inline styles only (this page's CSP blocks <style>), still built once,
// still cached writes on a 250 ms timer, still click-through except buttons.
//
// Usage: node nfsu-gui4.mjs <targetId> [port] [--off]
import fs from "node:fs";

const [TARGET, PORT = "9222", FLAG = ""] = process.argv.slice(2);
const list = await (await fetch(`http://127.0.0.1:${PORT}/json/list`)).json();
const page = list.find((t) => t.id === TARGET);
if (!page) throw new Error("no target " + TARGET);

const ws = new WebSocket(page.webSocketDebuggerUrl);
await new Promise((r, j) => {
	ws.onopen = r;
	ws.onerror = j;
});
let id = 0;
const q = new Map();
ws.onmessage = (e) => {
	const m = JSON.parse(e.data);
	const s = q.get(m.id);
	if (s) {
		q.delete(m.id);
		s(m.result);
	}
};
const send = (method, params = {}) =>
	new Promise((r) => {
		const n = ++id;
		q.set(n, r);
		ws.send(JSON.stringify({ id: n, method, params }));
	});
const ev = async (x) => {
	const res = await send("Runtime.evaluate", {
		expression: x,
		returnByValue: true,
		awaitPromise: true,
	});
	if (res.exceptionDetails) throw new Error(res.exceptionDetails.text);
	return res.result.value;
};

if (FLAG === "--off") {
	const removed = await ev(
		`(()=>{let n=0;for(const id of ['hermes-hud','hermes-obs','hermes-dec']){const el=document.getElementById(id);if(el){el.remove();n++}}window.__hermesDebug=null;return n})()`,
	);
	console.log(JSON.stringify({ removed }));
	ws.close();
	process.exit(0);
}

const INSTALL = `(()=>{
for (const id of ['hermes-hud','hermes-obs','hermes-dec','hermes-panel','hermes-conf','hermes-style']) { const e = document.getElementById(id); if (e) e.remove() }
const D = window.__hermesDebug = window.__hermesDebug || {};
D.log = D.log || []; D.cache = {};
window.__mode = window.__mode || D.mode || 'memory';
D.mode = window.__mode;
if (!D.hooked) {
  D.hooked = true;
  addEventListener('keydown', e => D.log.push({dir:'in',key:e.code,trusted:e.isTrusted?'y':'n'}), {capture:true});
  addEventListener('keyup',   e => D.log.push({dir:'in',key:e.code,trusted:e.isTrusted?'y':'n'}), {capture:true});
}
D.push = (dir, key, extra) => { D.log.push({dir,key,extra}); if (D.log.length > 200) D.log.splice(0, D.log.length - 200) };

// The panel reads the game itself, on its own timer. Before this it only showed
// what a driver had published, so with no driver running every field was "?" -
// an observation panel that needs a driver to observe is not an observation
// panel.
window.__readCar = () => {
  const n = window.__nfsu2;
  const mem = n && n.session && n.session.proc && n.session.proc.memory;
  if (!mem) return null;
  const dv = new DataView(mem.buffer);
  const u32 = a => dv.getUint32(a, true), f32 = a => dv.getFloat32(a, true), u16 = a => dv.getUint16(a, true);
  const player = u32(0x00d00000); const car = player ? u32(player + 4) : 0;
  if (!car) return null;
  const ctl = u32(car + 0x30);
  return {
    car,
    kmh: Math.round(Math.abs(f32(car + 0x430)) * 3.6),
    speed: +f32(car + 0x430).toFixed(2),
    gear: dv.getInt8(car + 0x434),
    rpm: Math.round(f32(car + 0x440)),
    racetype: u32(0x0089e7b0),
    steer: ctl ? +(dv.getInt16(ctl + 0x1e0, true) / 32767).toFixed(3) : 0,
    yawRate: +f32(car + 0x110).toFixed(3),
    heading: +(u16(car + 0x40c) / 65536 * 360).toFixed(1),
    pos: [+f32(car + 0x60).toFixed(1), +f32(car + 0x64).toFixed(1), +f32(car + 0x68).toFixed(1)],
    vel: [+f32(car + 0x70).toFixed(2), +f32(car + 0x74).toFixed(2), +f32(car + 0x78).toFixed(2)],
    surface: [0,1,2,3].map(k => dv.getUint32(car + 0x394 + 4 * k, true)),
    slip: [0,1,2,3,4,5,6,7].map(k => +f32(car + 0x3b4 + 4 * k).toFixed(2)),
    grounded: [0,1,2,3].map(k => !!dv.getUint8(car + 0x390 + k)),
  };
};

const BOX = 'position:fixed;z-index:2147483647;font:10px/1.35 ui-monospace,SFMono-Regular,Menlo,monospace;color:#9fe8ff;background:rgba(4,8,14,.84);border:1px solid rgba(120,220,255,.32);border-radius:6px;padding:6px 8px;text-shadow:0 0 6px rgba(80,200,255,.3)';
const S = (el, css) => { el.style.cssText = css; return el };
const mk = (id, css) => { const e = document.createElement('div'); e.id = id; S(e, css); document.documentElement.appendChild(e); return e };
const sec = (parent, label) => { const e = S(document.createElement('div'), 'color:#5f7d95;letter-spacing:.08em;margin:6px 0 2px;font-size:8px'); e.textContent = label; parent.appendChild(e) };

const hud = mk('hermes-hud', BOX + ';left:8px;top:8px;white-space:pre;pointer-events:none;min-width:210px');

const obs = mk('hermes-obs', BOX + ';right:8px;top:8px;width:260px;pointer-events:none');
const obsHdr = S(document.createElement('div'), 'display:flex;justify-content:space-between;color:#cfeeff;letter-spacing:.06em');
obsHdr.innerHTML = '<b>OBSERVATIONS</b><span id="ho-mode">-</span>';
obs.appendChild(obsHdr);
const obsBody = S(document.createElement('pre'), 'margin:4px 0 0;color:#9fe8ff;white-space:pre-wrap;font:inherit'); obsBody.id = 'ho-body';
obs.appendChild(obsBody);
sec(obs, 'CUES');
const cueBody = S(document.createElement('pre'), 'margin:0;color:#9fe8ff;white-space:pre-wrap;font:inherit'); cueBody.id = 'hc-body';
obs.appendChild(cueBody);

const dec = mk('hermes-dec', BOX + ';left:8px;bottom:118px;width:268px;max-height:calc(100vh - 260px);overflow:auto;pointer-events:none');
const decHdr = S(document.createElement('div'), 'color:#cfeeff;letter-spacing:.06em'); decHdr.textContent = 'DECISIONS';
dec.appendChild(decHdr);
const modesEl = S(document.createElement('div'), 'display:grid;grid-template-columns:1fr 1fr;gap:3px;margin:5px 0');
dec.appendChild(modesEl);
// Who is driving, and whether the model pipe is open at all. Both are page
// state (window.__power, window.__pipe) so any driver - JS or Rust - reads
// the same switch the person clicked.
const powerEl = S(document.createElement('div'), 'display:grid;grid-template-columns:repeat(3,1fr);gap:3px;margin:3px 0');
dec.appendChild(powerEl);
const pipeEl = S(document.createElement('div'), 'display:grid;grid-template-columns:1fr;gap:3px;margin:3px 0 5px');
dec.appendChild(pipeEl);
const decBody = S(document.createElement('div'), 'margin-top:2px'); decBody.id = 'hd-body';
dec.appendChild(decBody);
// What it is driving right now, in one line, so the panel answers "what is it
// doing" before it lists what it could do.
const driving = S(document.createElement('div'), 'margin:4px 0 2px;color:#eaf7ff'); driving.id = 'hd-driving';
dec.appendChild(driving);

const MODES = [
  {id:'gps',    label:'GPS',      hint:'follow the route arrow'},
  {id:'road',   label:'Road',     hint:'visual only, arrow ignored'},
  {id:'memory', label:'Free roam',hint:'memory loop only, no pixels'},
  {id:'ui',     label:'UI',       hint:'ingest the page UI (not built)'},
  {id:'race',   label:'Race',     hint:'memory flags + arrow'},
];
const BTN = 'font:inherit;color:#b9dcf2;background:rgba(255,255,255,.03);border:1px solid rgba(120,220,255,.22);border-radius:4px;padding:2px 4px;cursor:pointer;pointer-events:auto';
const BTN_ON = BTN + ';border-color:rgba(120,220,255,.85);background:rgba(120,220,255,.2);color:#eaf7ff';
const btns = {};
for (const m of MODES) {
  const b = document.createElement('button');
  b.textContent = m.label; b.title = m.hint; b.dataset.mode = m.id;
  S(b, BTN);
  b.addEventListener('click', () => { window.__mode = m.id; D.mode = m.id; paint(true) });
  modesEl.appendChild(b); btns[m.id] = b;
}

// Three powers, each a different amount of machinery behind the wheel:
//   user    rules only, no model at all - basic driving
//   agentic agent + Agentic + model: the rules drive, the model argues
//   model   fully autonomous: the model's answer is the decision
const POWERS = [
  {id:'user',    label:'User',    hint:'rules only - basic driving, no model'},
  {id:'agentic', label:'Agentic', hint:'agent + Agentic + model: the model improves the rules'},
  {id:'model',   label:'Model',   hint:'fully autonomous: the model decides'},
];
window.__power = window.__power || 'agentic';
window.__pipe = window.__pipe || 'on';
const powerBtns = {};
for (const p of POWERS) {
  const b = document.createElement('button');
  b.textContent = p.label; b.title = p.hint; b.dataset.power = p.id;
  S(b, BTN);
  b.addEventListener('click', () => { window.__power = p.id; paint(true) });
  powerEl.appendChild(b); powerBtns[p.id] = b;
}
const pipeBtn = document.createElement('button');
pipeBtn.dataset.pipe = 'toggle'; S(pipeBtn, BTN);
pipeBtn.addEventListener('click', () => { window.__pipe = window.__pipe === 'on' ? 'off' : 'on'; paint(true) });
pipeEl.appendChild(pipeBtn);

// Three pipes, each independently switchable, because a remote decision model is
// billed per call and a local one costs only cycles:
//   LOCAL   the rules engine (always available, free)
//   REMOTE  Cloudflare Clef (pricey, per token)
//   PIPE    the wire itself - off means no request is made at all
window.__local = window.__local || 'on';
window.__remote = window.__remote || 'on';
const SWITCHES = [
  {id:'local',  label:'LOCAL',  get:()=>window.__local,  set:(v)=>{window.__local=v}},
  {id:'remote', label:'REMOTE', get:()=>window.__remote, set:(v)=>{window.__remote=v}},
  {id:'pipe',   label:'PIPE',   get:()=>window.__pipe,   set:(v)=>{window.__pipe=v}},
];
const switchEl = S(document.createElement('div'), 'display:grid;grid-template-columns:repeat(3,1fr);gap:3px;margin:3px 0');
dec.insertBefore(switchEl, pipeEl);
const switchBtns = {};
for (const s of SWITCHES) {
  const b = document.createElement('button');
  b.dataset.switch = s.id; S(b, BTN);
  b.addEventListener('click', () => { s.set(s.get() === 'on' ? 'off' : 'on'); paint(true) });
  switchEl.appendChild(b); switchBtns[s.id] = b;
}

// Knobs for the settings a person actually tunes while watching it drive.
const KNOBS = [
  {id:'target', label:'target km/h', value:45, min:5, max:120, step:5},
  {id:'cruise', label:'cruise m/s',  value:12, min:2, max:40,  step:1},
  {id:'turn',   label:'turn m/s',    value:7,  min:1, max:20,  step:1},
  {id:'every',  label:'model every', value:10, min:1, max:60,  step:1},
];
window.__knobs = window.__knobs || {};
const knobEl = S(document.createElement('div'), 'display:grid;grid-template-columns:1fr 1fr;gap:3px;margin:3px 0 5px');
dec.insertBefore(knobEl, pipeEl);
for (const k of KNOBS) {
  const cell = S(document.createElement('label'), 'display:flex;justify-content:space-between;gap:4px;color:#8fb8cf;font-size:9px');
  const name = S(document.createElement('span'), ''); name.textContent = k.label;
  const input = S(document.createElement('input'), 'width:46px;font:inherit;color:#eaf7ff;background:rgba(255,255,255,.06);border:1px solid rgba(120,220,255,.25);border-radius:3px;pointer-events:auto;text-align:right');
  input.type = 'number'; input.min = String(k.min); input.max = String(k.max); input.step = String(k.step);
  input.dataset.knob = k.id;
  input.value = String(window.__knobs[k.id] !== undefined ? window.__knobs[k.id] : k.value);
  window.__knobs[k.id] = Number(input.value);
  input.addEventListener('input', () => { window.__knobs[k.id] = Number(input.value); });
  cell.appendChild(name); cell.appendChild(input);
  knobEl.appendChild(cell);
}

// One row per candidate action; the chosen one is highlighted, so the list reads
// as "what it considered" rather than only "what it did".
const ACTIONS = ['steer left','steer right','centred','cruise','roam turn','reverse','unsupported'];
const rows = {};
for (const a of ACTIONS) {
  const row = S(document.createElement('div'), 'display:flex;justify-content:space-between;gap:6px;padding:1px 4px;border-radius:3px;color:#8fb8cf');
  const name = S(document.createElement('span'), ''); name.textContent = a;
  const val = S(document.createElement('span'), 'color:#5f7d95'); val.textContent = '-';
  row.appendChild(name); row.appendChild(val);
  decBody.appendChild(row); rows[a] = {row, val};
}

const set = (node, text) => { if (D.cache[node.id] !== text) { node.textContent = text; D.cache[node.id] = text } };
const modeEl = obsHdr.querySelector('#ho-mode');

function paint() {
  const n = window.__nfsu2, s = n && n.session, i = s && s.input, st = i && i.status ? i.status() : null;
  let stats = {}; try { stats = s && s.stats ? s.stats() : {} } catch (e) {}
  const a = D.arrowData || {}, d = D.decision || {}, au = D.auto || {};
  // Live read first: the panel observes the game whether or not a driver is
  // running, and falls back to the last published state only if memory is gone.
  let car = {};
  try { car = window.__readCar() || D.car || {}; } catch (e) { car = D.car || {}; }

  if (D.cache.__mode !== D.mode) {
    D.cache.__mode = D.mode;
    for (const m of MODES) S(btns[m.id], D.mode === m.id ? BTN_ON : BTN);
  }
  set(modeEl, D.mode + (au.on ? ' · live' : ' · idle'));

  // Power and pipe are read from the page every paint, so a click in another
  // tab or a driver flipping them is reflected without a message passing system.
  const Power = window.__power || 'agentic';
  const Local = window.__local || 'on';
  const Remote = window.__remote || 'on';
  const Pipe = window.__pipe || 'on';
  const Key = Power + Local + Remote + Pipe;
  if (D.cache.__switchKey !== Key) {
    D.cache.__switchKey = Key;
    for (const p of POWERS) S(powerBtns[p.id], Power === p.id ? BTN_ON : BTN);
    const States = { local: Local, remote: Remote, pipe: Pipe };
    for (const s of SWITCHES) {
      const On = States[s.id] === 'on';
      const b = switchBtns[s.id];
      b.textContent = s.label + (On ? ' ON' : ' OFF');
      S(b, On ? BTN_ON : BTN + ';border-color:rgba(255,120,120,.75);color:#ffb3b3');
    }
    pipeBtn.textContent = Pipe === 'on' ? 'PIPE ON · click to break' : 'PIPE OFF · click to restore';
    S(pipeBtn, Pipe === 'on' ? BTN_ON : BTN + ';border-color:rgba(255,120,120,.75);color:#ffb3b3');
  }

  set(hud,
    'HUD ' + ((n && n.phase) || '?') + '  tick ' + (au.ticks || 0) + '\\n' +
    'mode  ' + D.mode + (au.on ? ' ON' : ' OFF') + '\\n' +
    'power ' + Power + ' · local ' + Local + ' · remote ' + Remote + ' · pipe ' + Pipe + '\\n' +
    'keys  ' + (st ? st.keysHeld : '?') + '  drop ' + (st ? st.dropped : '?') + '\\n' +
    'gpu   ' + (stats['gpu encode ms'] ? stats['gpu encode ms'].toFixed(2) : '?') + 'ms'
  );

  set(obsBody,
    'speed  ' + (car.kmh === undefined ? '?' : car.kmh) + ' km/h  gear ' + (car.gear === undefined ? '?' : car.gear) + '  rpm ' + (car.rpm === undefined ? '?' : car.rpm) + '\\n' +
    'race   ' + (car.racetype === undefined ? '?' : car.racetype) + '  steer ' + (car.steer === undefined ? '?' : car.steer) + '  yaw ' + (car.yawRate === undefined ? '?' : car.yawRate) + '\\n' +
    'heading ' + (car.heading === undefined ? '?' : car.heading) + '\\n' +
    'pos    ' + (car.pos ? car.pos.join(', ') : '?') + '\\n' +
    'vel    ' + (car.vel ? car.vel.join(', ') : '?') + '\\n' +
    'surface ' + (car.surface ? car.surface.join(',') : '?') + '  ground ' + (car.grounded ? car.grounded.map(g => g ? '1' : '0').join('') : '?') + '\\n' +
    'slip   ' + (car.slip ? car.slip.join(',') : '?')
  );
  const cue = (a.cx === null || a.cx === undefined) ? 'none' : (a.cx + ' , ' + (a.cy === null ? '-' : a.cy));
  const px = a.pixels || 0;
  const cueConf = (a.cx === null || a.cx === undefined) ? 0 : Math.min(1, px / 600);
  const bar = v => { const k = Math.max(0, Math.min(8, Math.round(v * 8))); return '█'.repeat(k) + '░'.repeat(8 - k) };
  set(cueBody, 'arrow  ' + cue + '\\n' + 'pixels ' + px + '  ' + bar(cueConf) + ' ' + Math.round(cueConf * 100) + '%');

  for (const act of ACTIONS) {
    const chosen = (d.action || '') === act;
    const r = rows[act];
    S(r.row, 'display:flex;justify-content:space-between;gap:6px;padding:1px 4px;border-radius:3px;' +
      (chosen ? 'background:rgba(120,220,255,.18);color:#eaf7ff' : 'color:#8fb8cf'));
    set(r.val, chosen ? (d.ms ? d.ms + 'ms' : 'now') : '-');
  }
  set(driving, 'driving: ' + D.mode + ' · ' + (car.kmh === undefined ? '?' : car.kmh) + ' km/h · ' +
    ((a.cx === null || a.cx === undefined) ? 'no cue' : 'cue ' + a.cx.toFixed(2)) + ' · ' + (d.action || '-'));
}

D.paint = paint;
paint();
// One timer per page, not one per install: a second install must replace the
// first rather than run beside it, or two writers race and the panels disagree.
if (D._paintTimer) clearInterval(D._paintTimer);
D._paintTimer = setInterval(paint, 250);
return { installed: true, obs: !!document.getElementById('hermes-obs'), dec: !!document.getElementById('hermes-dec'), mode: D.mode };
})()`;

const out = await ev(INSTALL);
console.log(
	JSON.stringify({ page: page.url, target: TARGET, ...out }, null, 2),
);
ws.close();
