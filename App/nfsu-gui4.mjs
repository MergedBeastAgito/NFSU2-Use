// GUI v4: observations and cues top-right, proposed decisions bottom-left.
//
// Layout:
//   top-left      compact HUD (mode, tick, keys, speed)
//   top-right     OBSERVATIONS (what memory reports) + CUES (what pixels report)
//   bottom-left   DECISIONS (the candidate actions the policy weighed, with the
//                 chosen one highlighted; the mode buttons live here too)
//   bottom-right  WIRE (the last request and reply bodies, redacted)
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
	if (res.exceptionDetails)
		throw new Error(
			(res.exceptionDetails.exception &&
				(res.exceptionDetails.exception.description ||
					res.exceptionDetails.exception.value)) ||
				res.exceptionDetails.text,
		);
	return res.result.value;
};

if (FLAG === "--off") {
	const removed = await ev(
		`(()=>{let n=0;for(const id of ['hermes-hud','hermes-obs','hermes-dec','hermes-wire']){const el=document.getElementById(id);if(el){el.remove();n++}}window.__hermesDebug=null;return n})()`,
	);
	console.log(JSON.stringify({ removed }));
	ws.close();
	process.exit(0);
}

const INSTALL = `(()=>{
for (const id of ['hermes-hud','hermes-obs','hermes-dec','hermes-wire','hermes-panel','hermes-conf','hermes-style']) { const e = document.getElementById(id); if (e) e.remove() }
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

const hud = mk('hermes-hud', BOX + ';left:8px;top:8px;white-space:pre;pointer-events:auto;user-select:text;min-width:210px');

const obs = mk('hermes-obs', BOX + ';right:8px;top:8px;width:260px;pointer-events:auto;user-select:text');
const obsHdr = S(document.createElement('div'), 'display:flex;justify-content:space-between;color:#cfeeff;letter-spacing:.06em');
obsHdr.innerHTML = '<b>OBSERVATIONS</b><span id="ho-mode">-</span>';
obs.appendChild(obsHdr);
const obsBody = S(document.createElement('pre'), 'margin:4px 0 0;color:#9fe8ff;white-space:pre-wrap;font:inherit'); obsBody.id = 'ho-body';
obs.appendChild(obsBody);
sec(obs, 'CUES');
const cueBody = S(document.createElement('pre'), 'margin:0;color:#9fe8ff;white-space:pre-wrap;font:inherit'); cueBody.id = 'hc-body';
obs.appendChild(cueBody);

const dec = mk('hermes-dec', BOX + ';left:8px;bottom:118px;width:664px;max-height:calc(100vh - 240px);overflow:auto;pointer-events:auto;user-select:text');
const decHdr = S(document.createElement('div'), 'color:#cfeeff;letter-spacing:.06em'); decHdr.textContent = 'DECISIONS';
dec.appendChild(decHdr);
// The panel is columnar, and the columns are the panel: three of them, each
// answering one kind of question and each read top-down on its own. The table
// of candidates sits beside the switches that shape it rather than above them,
// because a reader comparing what the car chose against who chose it is reading
// across, not down. That is what the pipe row was trying to say and could not:
// a column is a layout, not a character.
const Columns = S(document.createElement('div'), 'display:grid;grid-template-columns:1.2fr 1fr 1fr;gap:0 10px;align-items:start;margin-top:3px');
dec.appendChild(Columns);
const ColTable = S(document.createElement('div'), 'display:flex;flex-direction:column;min-width:0'); Columns.appendChild(ColTable);
const ColDrive = S(document.createElement('div'), 'display:flex;flex-direction:column;min-width:0'); Columns.appendChild(ColDrive);
const ColSense = S(document.createElement('div'), 'display:flex;flex-direction:column;min-width:0'); Columns.appendChild(ColSense);
const decBody = S(document.createElement('div'), 'margin-top:2px'); decBody.id = 'hd-body';
ColTable.appendChild(decBody);
// What it is driving right now, in one line, so the panel answers "what is it
// doing" before it lists what it could do.
const driving = S(document.createElement('div'), 'margin:4px 0 2px;color:#eaf7ff'); driving.id = 'hd-driving';
ColTable.appendChild(driving);

const BTN = 'font:inherit;color:#b9dcf2;background:rgba(255,255,255,.03);border:1px solid rgba(120,220,255,.22);border-radius:4px;padding:2px 4px;cursor:pointer;pointer-events:auto';
const BTN_ON = BTN + ';border-color:rgba(120,220,255,.85);background:rgba(120,220,255,.2);color:#eaf7ff';
const BTN_OFF = BTN + ';border-color:rgba(255,120,120,.75);color:#ffb3b3';
const LBL = 'color:#5f7d95;letter-spacing:.08em;font-size:8px;margin:6px 0 2px';
// One labelled group per question, so a switch is never read as a peer of a
// switch that answers something else.
function Group(Name, Cols, Parent) {
  const Into = Parent || dec;
  const Head = S(document.createElement('div'), LBL); Head.textContent = Name;
  const Row = S(document.createElement('div'), 'display:grid;grid-template-columns:repeat(' + Cols + ',1fr);gap:3px');
  Into.appendChild(Head); Into.appendChild(Row);
  return Row;
}

// MODE - the four conversations a tick can have, and the game decides which
// one: a car in a race is racing and a car against a wall is recovering,
// whatever the last person clicked. All four are shown at once so the lit cell
// says where the car sits in the set rather than only what it is called.
const MODES = ['cruise', 'race', 'recover', 'mission'];
const modeRow = Group('MODE · read from the game', 4, ColDrive);
const modeCells = {};
for (const M of MODES) {
  const c = S(document.createElement('div'), 'text-align:center;padding:2px 0;border:1px solid rgba(120,220,255,.14);border-radius:4px;color:#4f6b80');
  c.textContent = M.charAt(0).toUpperCase() + M.slice(1);
  modeRow.appendChild(c); modeCells[M] = c;
}

// DRIVER - who makes the decision. One axis with three points, and the two
// halves it turns on are named beside it rather than offered as their own
// switches: Rules alone costs nothing, Model alone never falls back, Agentic is
// the pair with the gate deciding which one wins.
const POWERS = [
  {id:'user',    label:'Rules',   hint:'the rule engine alone - no model, no cost'},
  {id:'agentic', label:'Agentic', hint:'the rules drive and the model argues; an answer replaces them only above the gate'},
  {id:'model',   label:'Model',   hint:'the model decides; the rules fill in only what it cannot perform'},
];
window.__power = window.__power || 'agentic';
window.__pipe = window.__pipe || 'on';
const powerEl = Group('DRIVER · who decides', 3, ColDrive);
const powerBtns = {};
for (const p of POWERS) {
  const b = document.createElement('button');
  b.textContent = p.label; b.title = p.hint; b.dataset.power = p.id;
  S(b, BTN);
  b.addEventListener('click', () => { window.__power = p.id; paint(true) });
  powerEl.appendChild(b); powerBtns[p.id] = b;
}
const halves = S(document.createElement('div'), 'color:#5f7d95;font-size:9px;margin:2px 0 0'); halves.id = 'hd-halves';
ColDrive.appendChild(halves);

// SENSE - what the driver can see. Memory reads the car out of the game's own
// memory, Pixels reads the road and the route arrow from a captured frame, and
// Both is the pair. This is the question the old GPS/Road/Free roam row was
// answering, badly: those were senses, not modes.
const SENSES = [
  {id:'memory', label:'Memory', hint:'the car from the game memory - no pixels'},
  {id:'gps',    label:'Both',   hint:'memory plus the route arrow from a frame'},
  {id:'road',   label:'Pixels', hint:'the road and the arrow from frames only'},
];
const senseEl = Group('SENSE · what it can see', 3, ColSense);
const senseBtns = {};
for (const q of SENSES) {
  const b = document.createElement('button');
  b.textContent = q.label; b.title = q.hint; b.dataset.sense = q.id;
  S(b, BTN);
  b.addEventListener('click', () => { window.__mode = q.id; D.mode = q.id; paint(true) });
  senseEl.appendChild(b); senseBtns[q.id] = b;
}

// WIRE - whether a request leaves at all. Its own group because it is a
// transport fact and not an authority: with the pipe broken the driver still
// drives, and the WIRE panel says nothing was sent rather than implying the
// model was asked and had no opinion.
const pipeEl = Group('WIRE · does a request leave', 1, ColSense);
const pipeBtn = document.createElement('button');
pipeBtn.dataset.pipe = 'toggle'; S(pipeBtn, BTN);
pipeBtn.addEventListener('click', () => { window.__pipe = window.__pipe === 'on' ? 'off' : 'on'; paint(true) });
pipeEl.appendChild(pipeBtn);

// TUNING - the numbers a person changes while watching the JS harness drive.
// The Rust driver takes its own values as flags (--target, --every,
// --threshold), so these are labelled as the harness's rather than offered as
// if they steered the process that is actually driving: a knob that looks like
// it sets the pace and does not is worse than no knob.
const KNOBS = [
  {id:'target', label:'target km/h', value:45, min:5, max:120, step:5},
  {id:'cruise', label:'cruise m/s',  value:12, min:2, max:40,  step:1},
  {id:'turn',   label:'turn m/s',    value:7,  min:1, max:20,  step:1},
  {id:'every',  label:'model every', value:10, min:1, max:60,  step:1},
];
window.__knobs = window.__knobs || {};
const knobEl = Group('TUNING · page harness only', 2, ColSense);
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
// DECISIONS reads as a table, not a list: four columns - the candidate, the
// duty it would hold, the model's probability for it, the model's confidence -
// with a pipe between every pair. A columnar readout lets the eye compare a
// candidate's duty against its probability on one line; a list of rows makes
// the reader hold one row at a time and compare from memory.
//
// The pipe rule sits directly under the DECISIONS title, which is what makes
// the panel read as a header rule rather than a divider: five pipes for four
// columns and the two edges. The same pipes appear inside every row, so the
// columns line up down the panel instead of only at the top.
const ACTIONS = ['steer left','steer right','centred','cruise','roam turn','reverse','unsupported'];
const GRID = 'display:grid;grid-template-columns:1fr 7px 30px 7px 26px 7px 24px;gap:0 1px;align-items:baseline;padding:1px 3px;border-radius:3px';
const PIPESTYLE = 'color:#3d566b';
const rows = {};
for (const a of ACTIONS) {
  const row = S(document.createElement('div'), GRID + ';color:#8fb8cf');
  const slug = a.split(' ').join('-');
  const name = S(document.createElement('span'), 'white-space:nowrap;overflow:hidden;text-overflow:ellipsis');
  name.textContent = a;
  const mk = (col, width) => { const e = S(document.createElement('span'), 'color:#5f7d95;text-align:right'); e.id = 'hd-' + slug + '-' + col; e.textContent = '-'; return e };
  const pipe = () => S(document.createElement('span'), PIPESTYLE + ';text-align:center');
  const [p1, p2, p3] = [pipe(), pipe(), pipe()];
  for (const p of [p1, p2, p3]) p.textContent = '|';
  const ms = mk('ms'), prob = mk('p'), conf = mk('c');
  row.append(name, p1, ms, p2, prob, p3, conf);
  decBody.appendChild(row); rows[a] = {row, ms, prob, conf};
}

const set = (node, text) => { if (D.cache[node.id] !== text) { node.textContent = text; D.cache[node.id] = text } };
const modeEl = obsHdr.querySelector('#ho-mode');

// What crossed the wire, last exchange only. The decision rows say what the
// policy chose; this says what it was asked and what came back, which is the
// only way to tell a refused request from a confident answer. Bottom-right,
// because the three corners already belong to the HUD, the observations and
// the decisions, and this is the fourth question a reader asks.
const WIRE_PRE = 'margin:0;color:#9fe8ff;white-space:pre-wrap;word-break:break-word;font:inherit;font-size:9px';
const WIRE_HDR = 'color:#cfeeff;letter-spacing:.06em';
const NOTHING = 'nothing sent yet';
// The panel scrolls inside its own box: a request is taller than the corner it
// lives in, and a panel that grew to fit would cover the game it describes.
const wireEl = mk('hermes-wire', BOX + ';right:8px;bottom:8px;width:390px;max-height:46vh;overflow:auto;pointer-events:auto;user-select:text');
const wireHdr = S(document.createElement('div'), WIRE_HDR); wireHdr.id = 'hw-hdr';
wireEl.appendChild(wireHdr);
// The panel is a column of typed boxes, built once. A request does not rebuild
// them, it writes into them: that is what keeps the layout still while the
// content changes every sixth tick, so a reader's eye stays on the box it was
// reading and a new lane appearing is the only thing that moves anything.
//
// Colour is the type and nothing else - a state is green, a choice cyan, a
// score violet, the verdict red, the history strip grey - so two boxes of the
// same colour are the same kind of thing. That is the whole legend, and it is
// why the colour is a token rather than a mood.
const TYPE = {
  state:   '#7fffc4',
  choice:  '#7fe3ff',
  score:   '#b79bff',
  noul:    '#ffd27f',
  verdict: '#ffb3b3',
  history: '#8fb8cf',
};
const wireBody = S(document.createElement('div'), 'display:flex;flex-direction:column;margin:2px 0 0'); wireBody.id = 'hw-body';
wireEl.appendChild(wireBody);
// One box, created once. Its title is a slot too: a lane that changes its
// question rewrites this text rather than replacing the node.
function Box(Title, Colour, Parent, Id) {
  const Frame = S(document.createElement('div'), 'border:1px solid ' + Colour + '44;border-left:2px solid ' + Colour + ';border-radius:4px;padding:2px 5px;margin:0 0 4px;min-width:0');
  const Head = S(document.createElement('div'), 'color:' + Colour + ';font-size:8px;letter-spacing:.06em;margin-bottom:2px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis');
  Head.textContent = Title;
  const Body = S(document.createElement('div'), 'font-size:9px;min-width:0');
  Frame.appendChild(Head); Frame.appendChild(Body);
  (Parent || wireBody).appendChild(Frame);
  if (Id) Frame.id = Id;
  return { Frame, Head, Body };
}
const stateBox = Box('STATE · what was sent', TYPE.state, null, 'hw-state');
const laneBox = Box('QUESTIONS · one lane each', TYPE.choice, null, 'hw-lanes');
const verdictBox = Box('VERDICT · what came back', TYPE.verdict, null, 'hw-verdict');
const historyBox = Box('HISTORY · one column per request', TYPE.history, null, 'hw-history');
sec(wireEl, 'RECEIVE');
const wireRecv = S(document.createElement('pre'), WIRE_PRE); wireRecv.id = 'hw-recv';
wireEl.appendChild(wireRecv);

// Scrub by name and by shape. A named secret is easy; the shape is what
// matters, because a token in a body has no name - it is just a long run of
// alphanumerics that a screenshot would preserve. The bodies carry no
// credential today, and the panel is built so that stays true if one appears.
const REDACT_NAMES = ['CLOUDFLARE_API_TOKEN', 'Authorization', 'api_key', 'apiKey'];
const redact = (x) => {
  if (x === undefined || x === null) return '';
  let t = String(x);
  t = t.replace(/Bearer\s+[^\s"',}]+/gi, '***');
  t = t.replace(/[A-Za-z0-9_-]{20,}/g, '***');
  for (const k of REDACT_NAMES) t = t.split(k).join('***');
  return t;
};
// Pretty when it parses, raw when it does not: a malformed reply is the one a
// reader most needs to see, so a parse failure shows the bytes, never a blank.
const pretty = (x) => {
  const t = (x === undefined || x === null) ? '' : String(x);
  if (!t) return '';
  try { return JSON.stringify(JSON.parse(t), null, 1) } catch (e) { return t }
};
// One writer for both bodies: text through the same cache as every other
// field, and the dim colour swapped only when the empty state flips.
const wireWrite = (node, text) => {
  set(node, text);
  const empty = text === NOTHING;
  const k = node.id + ':dim';
  if (D.cache[k] !== empty) { D.cache[k] = empty; S(node, WIRE_PRE + (empty ? ';color:#5f7d95' : '')) }
};

// Every node is created the first time its key appears and reused forever: the
// state rows by field name, the lane boxes by question id, the chips by option.
// A request that asks the same question writes new numbers into the same nodes,
// which is what makes the display still while the content changes under it.
const CHIP = 'padding:1px 5px;border:1px solid rgba(120,220,255,.18);border-radius:10px;color:#7fa8c0;font-size:9px;white-space:nowrap';
const CHIP_ON = 'padding:1px 5px;border:1px solid rgba(120,220,255,.9);border-radius:10px;background:rgba(120,220,255,.22);color:#eaf7ff;font-size:9px;white-space:nowrap';
const DIM = 'color:#5f7d95;font-size:9px';
D.pool = D.pool || { rows: {}, lanes: {}, verdicts: {}, history: [], blank: null };
function drawWire(Send, Recv, Wire) {
  let Body = null, Answers = null;
  try { Body = JSON.parse(Send) } catch (e) { Body = null }
  try {
    const R = JSON.parse(Recv);
    Answers = (R && R.result && R.result.answers) || (R && R.answers) || null;
  } catch (e) { Answers = null }
  const Pool = D.pool;

  // STATE - a row per field, keyed by name and never rebuilt.
  const State = (Body && Body.state) || {};
  const Fields = Object.keys(State);
  if (D.cache.__stateEmpty !== !Fields.length) {
    D.cache.__stateEmpty = !Fields.length;
    stateBox.Body.textContent = '';
    if (!Fields.length) { const N = S(document.createElement('div'), DIM); N.textContent = NOTHING; stateBox.Body.appendChild(N) }
  }
  for (const F of Fields) {
    let R = Pool.rows[F];
    if (!R) {
      const Row = S(document.createElement('div'), 'display:flex;justify-content:space-between;gap:6px');
      const K = S(document.createElement('span'), 'color:#5f7d95'); K.textContent = F;
      const V = S(document.createElement('span'), 'color:#9fe8ff;text-align:right;overflow:hidden;text-overflow:ellipsis;white-space:nowrap;max-width:210px');
      V.id = 'hw-v-' + F;
      Row.appendChild(K); Row.appendChild(V);
      stateBox.Body.appendChild(Row);
      R = Pool.rows[F] = { Row, V };
    }
    set(R.V, typeof State[F] === 'string' ? State[F] : JSON.stringify(State[F]));
  }

  // QUESTIONS - one box per question id, coloured by the question's own type.
  const Questions = (Body && Body.questions) || {};
  const Ids = Object.keys(Questions);
  if (!Ids.length) {
    if (!Pool.blank) { Pool.blank = S(document.createElement('div'), DIM); Pool.blank.textContent = NOTHING; laneBox.Body.appendChild(Pool.blank) }
  } else if (Pool.blank) { Pool.blank.remove(); Pool.blank = null }
  for (const Id of Ids) {
    const Q = Questions[Id] || {};
    const A = Answers && Answers[Id];
    const Colour = TYPE[Q.type] || TYPE.noul;
    let L = Pool.lanes[Id];
    if (!L) {
      const Frame = S(document.createElement('div'), 'border-left:2px solid ' + Colour + ';padding-left:4px;margin:0 0 3px');
      const Head = S(document.createElement('div'), 'color:' + Colour + ';font-size:9px;margin-bottom:2px;white-space:nowrap;overflow:hidden;text-overflow:ellipsis');
      const Track = S(document.createElement('div'), 'display:flex;flex-wrap:wrap;gap:3px;align-items:center');
      Frame.appendChild(Head); Frame.appendChild(Track);
      laneBox.Body.appendChild(Frame);
      L = Pool.lanes[Id] = { Frame, Head, Track, Sig: '', Chips: {} };
    }
    const Options = Array.isArray(Q.criteria) ? Q.criteria : Object.keys(Q.criteria || {});
    set(L.Head, Id + ' · ' + (Q.type || '?') + (Q.instructions ? ' - ' + Q.instructions : ''));
    // The chips are rebuilt only when the option set itself changes; a new
    // probability is a text write into a node that is already there.
    const Sig = Options.join('|');
    if (L.Sig !== Sig) {
      L.Sig = Sig; L.Track.textContent = ''; L.Chips = {};
      Options.forEach((O) => {
        const Chip = S(document.createElement('span'), CHIP);
        Chip.id = 'hw-chip-' + Id + '-' + O;
        L.Track.appendChild(Chip);
        L.Chips[O] = Chip;
      });
      if (!Options.length) { const N = S(document.createElement('span'), DIM); N.textContent = 'no options'; L.Track.appendChild(N) }
    }
    const Chosen = A && A.choice !== undefined && A.choice !== null ? String(A.choice) : null;
    const Prob = (A && A.probabilities) || {};
    const Position = A && typeof A.score === 'number' ? A.score : null;
    Options.forEach((O, At) => {
      const Chip = L.Chips[O]; if (!Chip) return;
      // A choice is answered by a word and a score by a position in its legend,
      // so the lit chip is whichever of the two the reply actually carried.
      const Lit = (Chosen !== null && Chosen === String(O)) || (Position !== null && Math.round(Position) === At);
      set(Chip, String(O) + (Prob[O] !== undefined ? ' ' + Number(Prob[O]).toFixed(2) : ''));
      const K = Chip.id + ':on';
      if (D.cache[K] !== Lit) { D.cache[K] = Lit; S(Chip, Lit ? CHIP_ON : CHIP) }
    });

    // VERDICT - one line per question. A choice and a score answer differently,
    // so the line says which it was instead of flattening both into "confidence".
    let Vd = Pool.verdicts[Id];
    if (!Vd) {
      const Row = S(document.createElement('div'), 'display:flex;justify-content:space-between;gap:6px');
      const K = S(document.createElement('span'), 'color:#5f7d95'); K.textContent = Id;
      const V = S(document.createElement('span'), 'color:#ffd9d9;text-align:right');
      V.id = 'hw-verdict-' + Id;
      Row.appendChild(K); Row.appendChild(V);
      verdictBox.Body.appendChild(Row);
      Vd = Pool.verdicts[Id] = { Row, V };
    }
    set(Vd.V, A
      ? (A.choice !== undefined && A.choice !== null
        ? String(A.choice) + ' ' + (A.probabilities && A.probabilities[A.choice] !== undefined ? Number(A.probabilities[A.choice]).toFixed(2) : '')
        : (typeof A.score === 'number' ? 'score ' + A.score.toFixed(2) : '-'))
      : '-');
  }

  // HISTORY - a column per request, oldest dropped. The lane and the answer
  // together are what a run reads as: a lane that never wins and an answer that
  // never clears the gate look identical in one tick and different here.
  if (Wire && Wire.lane) {
    const Picked = Answers && Object.keys(Answers).map((K) => Answers[K] && Answers[K].choice).filter(Boolean)[0];
    const Line = (Wire.lane || '?') + ' · ' + (Picked || '-') + ' · ' + (Wire.ms === undefined ? '?' : Math.round(Wire.ms)) + 'ms';
    if (Pool.history[Pool.history.length - 1] !== Line) {
      Pool.history.push(Line);
      if (Pool.history.length > 8) Pool.history.shift();
      historyBox.Body.textContent = '';
      for (const H of Pool.history) { const E = S(document.createElement('div'), 'color:#8fb8cf;white-space:nowrap;overflow:hidden;text-overflow:ellipsis'); E.textContent = H; historyBox.Body.appendChild(E) }
    }
  }
}

function paint() {
  const n = window.__nfsu2, s = n && n.session, i = s && s.input, st = i && i.status ? i.status() : null;
  let stats = {}; try { stats = s && s.stats ? s.stats() : {} } catch (e) {}
  const a = D.arrowData || {}, d = D.decision || {}, au = D.auto || {};
  // Live read first: the panel observes the game whether or not a driver is
  // running, and falls back to the last published state only if memory is gone.
  let car = {};
  let live = false;
  try { const read = window.__readCar(); if (read) { car = read; live = true; } else car = D.car || {}; }
  catch (e) { car = D.car || {}; }

  // The mode is the driver's answer, read from the game, so it is a lit cell in
  // the set rather than a button: nothing in this panel sets it. The sense is a
  // different question and a different variable, which is why the two no longer
  // share one.
  const Drive = d.mode || 'cruise';
  if (D.cache.__mode !== Drive) {
    D.cache.__mode = Drive;
    for (const M of MODES) {
      const Lit = Drive === M;
      S(modeCells[M], 'text-align:center;padding:2px 0;border:1px solid ' + (Lit ? 'rgba(120,220,255,.85)' : 'rgba(120,220,255,.14)') +
        ';border-radius:4px;' + (Lit ? 'background:rgba(120,220,255,.2);color:#eaf7ff' : 'color:#4f6b80'));
    }
  }
  // The observations header keeps its one-word badge: the panel above says what
  // is being read, and this says which conversation it is being read for.
  set(modeEl, D.mode + (au.on ? ' · live' : ' · idle'));

  // Every switch is read from the page on each paint, so a click in another tab
  // or a driver flipping one is reflected without a message passing system.
  const Power = window.__power || 'agentic';
  const Pipe = window.__pipe || 'on';
  const Sense = window.__mode || 'memory';
  const Key = Power + '|' + Pipe + '|' + Sense;
  if (D.cache.__switchKey !== Key) {
    D.cache.__switchKey = Key;
    for (const p of POWERS) S(powerBtns[p.id], Power === p.id ? BTN_ON : BTN);
    for (const q of SENSES) S(senseBtns[q.id], Sense === q.id ? BTN_ON : BTN);
    // The two halves the DRIVER choice already turns on, named rather than
    // offered: three buttons here would read as three independent things.
    halves.textContent = Power === 'user' ? 'rules only · no request'
      : Power === 'model' ? 'model only · rules fill the gaps'
      : 'rules + model · gated at 0.60';
    pipeBtn.textContent = Pipe === 'on' ? 'pipe open · click to break' : 'pipe broken · click to open';
    S(pipeBtn, Pipe === 'on' ? BTN_ON : BTN_OFF);
  }

  set(hud,
    'HUD ' + ((n && n.phase) || '?') + '  tick ' + (au.ticks || 0) + '\\n' +
    'mode  ' + D.mode + (au.on ? ' ON' : ' OFF') + '\\n' +
    'driver ' + Power + ' · sense ' + Sense + ' · pipe ' + Pipe + '\\n' +
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
    // Restyling the row is a cssText write, so it is cached like any other:
    // seven rows times four paints a second is churn the panel does not need.
    if (D.cache['row:' + act] !== chosen) {
      D.cache['row:' + act] = chosen;
      S(r.row, GRID + ';' + (chosen ? 'background:rgba(120,220,255,.18);color:#eaf7ff' : 'color:#8fb8cf'));
    }
    set(r.ms, chosen ? (d.ms ? d.ms + 'ms' : 'now') : '-');
    // The model answers one question per tick, about the action it chose, so
    // the probability belongs on that row and nowhere else: the same number
    // repeated down every candidate reads as if all of them had been weighed.
    set(r.prob, chosen && d.probability != null ? (+d.probability).toFixed(2) : '-');
    set(r.conf, chosen && d.confidence != null ? (+d.confidence).toFixed(1) : '-');
  }
  // publish() assigns its payload onto this same debug object, so the wire
  // arrives beside the decision it explains; a driver that nests it under
  // decision is read the same way rather than needing a second hook.
  const w = D.wire || (d && d.wire) || null;
  const wErr = w && w.error ? redact(w.error) : '';
  const wMs = w && typeof w.ms === 'number' ? Math.round(w.ms) : null;
  const wLane = w && w.lane ? redact(w.lane) : '';
  if (D.cache.__wireErr !== !!wErr) {
    D.cache.__wireErr = !!wErr;
    S(wireHdr, WIRE_HDR + (wErr ? ';color:#ffb3b3' : ''));
  }
  set(wireHdr, w
    ? 'WIRE · ' + (wLane || 'lane ?') + ' · ' + (wMs === null ? '?ms' : wMs + 'ms') + (wErr ? ' · ERROR ' + wErr : ' · ok')
    : 'WIRE · idle');
  drawWire(w ? redact(w.send) : '', w ? redact(w.recv) : '', w);
  wireWrite(wireRecv, (w && redact(pretty(w.recv))) || NOTHING);

  set(driving, 'driving: ' + Drive + ' · ' + (car.kmh === undefined ? '?' : car.kmh) + ' km/h · ' +
    ((a.cx === null || a.cx === undefined) ? 'no cue' : 'cue ' + a.cx.toFixed(2)) + ' · ' + (d.action || '-'));
}

D.paint = paint;
paint();
// One timer per page, not one per install: a second install must replace the
// first rather than run beside it, or two writers race and the panels disagree.
if (D._paintTimer) clearInterval(D._paintTimer);
D._paintTimer = setInterval(paint, 250);
return { installed: true, obs: !!document.getElementById('hermes-obs'), dec: !!document.getElementById('hermes-dec'), wire: !!document.getElementById('hermes-wire'), mode: D.mode };
})()`;

const out = await ev(INSTALL);
console.log(
	JSON.stringify({ page: page.url, target: TARGET, ...out }, null, 2),
);
ws.close();
