// The pipeline lane: one full-width strip showing the round trip.
//
//   SEND    what we hand the decision model, as the exact JSON body
//   RECEIVE what came back (action, probabilities, confidence)
//   APPLIED what the driver actually did with it
//   TRACE   the last few ticks, so a run reads as a sequence
//
// Toggles sit at the right edge (SEND | RECV | APPLY | TRACE | ALL) and decide
// which columns are visible, so the strip can be narrowed to one question at a
// time. Everything is inline-styled because this page's CSP blocks <style>, and
// every write is cached so a tick that changes nothing touches no DOM.
//
// Usage: node nfsu-lane.mjs <targetId> [port] [--off]
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
	console.log(
		JSON.stringify({
			removed: await ev(
				`(()=>{const e=document.getElementById('hermes-lane');if(!e)return 0;e.remove();return 1})()`,
			),
		}),
	);
	ws.close();
	process.exit(0);
}

const INSTALL = `(()=>{
const Old = document.getElementById('hermes-lane'); if (Old) Old.remove();
const D = window.__hermesDebug = window.__hermesDebug || {};
D.lane = D.lane || { show: 'all', trace: [], seen: -1 };

const BOX = 'position:fixed;left:0;right:0;bottom:0;z-index:2147483646;font:10px/1.35 ui-monospace,SFMono-Regular,Menlo,monospace;color:#9fe8ff;background:rgba(3,7,12,.92);border-top:1px solid rgba(120,220,255,.35);padding:4px 6px;pointer-events:none';
const S = (el, css) => { el.style.cssText = css; return el };

const lane = document.createElement('div');
lane.id = 'hermes-lane';
S(lane, BOX);
document.documentElement.appendChild(lane);

const head = S(document.createElement('div'), 'display:flex;justify-content:space-between;align-items:center;gap:8px');
lane.appendChild(head);
const title = S(document.createElement('div'), 'color:#cfeeff;letter-spacing:.08em');
title.textContent = 'PIPELINE  observation → model → applied';
head.appendChild(title);

const toggles = S(document.createElement('div'), 'display:flex;gap:3px');
head.appendChild(toggles);
const LANES = [
  {id:'send',   label:'SEND'},
  {id:'recv',   label:'RECV'},
  {id:'apply',  label:'APPLY'},
  {id:'trace',  label:'TRACE'},
  {id:'all',    label:'ALL'},
];
const BTN = 'font:inherit;color:#b9dcf2;background:rgba(255,255,255,.03);border:1px solid rgba(120,220,255,.25);border-radius:3px;padding:1px 5px;cursor:pointer;pointer-events:auto';
const BTN_ON = BTN + ';border-color:rgba(120,220,255,.85);background:rgba(120,220,255,.22);color:#eaf7ff';
const btns = {};
for (const l of LANES) {
  const b = document.createElement('button');
  b.textContent = l.label; b.dataset.lane = l.id; S(b, BTN);
  b.addEventListener('click', () => { D.lane.show = l.id; paint(true) });
  toggles.appendChild(b); btns[l.id] = b;
}

const grid = S(document.createElement('div'), 'display:grid;grid-template-columns:repeat(4,1fr);gap:6px;margin-top:3px');
lane.appendChild(grid);
const COLS = [
  {id:'send',  title:'SEND (to model)'},
  {id:'recv',  title:'RECEIVE (from model)'},
  {id:'apply', title:'APPLIED'},
  {id:'trace', title:'TRACE'},
];
const cols = {};
for (const c of COLS) {
  const box = S(document.createElement('div'), 'border:1px solid rgba(120,220,255,.18);border-radius:4px;padding:3px 5px;min-height:64px;overflow:hidden');
  const h = S(document.createElement('div'), 'color:#5f7d95;letter-spacing:.06em;font-size:8px');
  h.textContent = c.title;
  const pre = S(document.createElement('pre'), 'margin:2px 0 0;white-space:pre-wrap;word-break:break-word;font:inherit;color:#9fe8ff;max-height:56px;overflow:hidden');
  pre.id = 'hl-' + c.id;
  box.appendChild(h); box.appendChild(pre);
  grid.appendChild(box); cols[c.id] = {box, pre};
}

const set = (node, text) => { if (D.cache['lane:' + node.id] !== text) { node.textContent = text; D.cache['lane:' + node.id] = text } };

// The body this pipeline sends, built here so the strip shows the real shape
// rather than a description of it. It mirrors Source/Fn/Cloudflare.rs.
function request(car, cue) {
  return {
    model: 'clef-flash',
    state: {
      speed_kmh: Math.round(car.kmh || 0),
      gear: car.gear,
      heading_degrees: car.heading,
      route_cue_x: (cue === undefined ? null : cue),
      situation: 'free roam on a city road; ghost mode on, so other cars cannot be hit',
    },
    questions: {
      action: {
        type: 'choice',
        instructions: 'What should the driver do next?',
        criteria: {
          steer_left: 'Turn toward the left of the road',
          steer_right: 'Turn toward the right of the road',
          throttle: 'Hold the line and keep the throttle down',
          brake: 'Slow down',
          hold: 'Do nothing and coast',
        },
      },
      confidence: {
        type: 'score',
        instructions: 'How sure are you of that action?',
        criteria: ['guessing', 'unsure', 'fairly sure', 'certain'],
      },
    },
  };
}

function paint(force) {
  const a = D.arrowData || {}, d = D.decision || {}, car = D.car || {}, au = D.auto || {};

  const shown = D.lane.show;
  for (const c of COLS) cols[c.id].box.style.display = (shown === 'all' || shown === c.id) ? '' : 'none';
  grid.style.gridTemplateColumns = shown === 'all' ? 'repeat(4,1fr)' : '1fr';
  for (const l of LANES) S(btns[l.id], D.lane.show === l.id ? BTN_ON : BTN);

  // SEND
  const cue = (a.cx === null || a.cx === undefined) ? null : a.cx;
  set(cols.send.pre, JSON.stringify(request(car, cue)));

  // RECEIVE - honest about the pipe and the power: a broken pipe or user power
  // means the model was never asked, and the column says which.
  const advice = D.modelAdvice;
  const pipe = window.__pipe || 'on';
  const power = window.__power || 'agentic';
  set(cols.recv.pre, advice
    ? JSON.stringify(advice)
    : pipe === 'off'
      ? 'pipe broken - rules only\\n(click PIPE to restore)'
      : power === 'user'
        ? 'user powered - no model\\n(rules only)'
        : 'not called yet - rules only\\n(power ' + power + ')');

  // APPLIED
  set(cols.apply.pre, (d.action || '-') + (d.ms ? ' · ' + d.ms + 'ms' : '') + '\\n' + (d.reason || ''));

  // TRACE - one line per tick, newest last.
  if (d.tick !== undefined && d.tick !== D.lane.seen) {
    D.lane.seen = d.tick;
    D.lane.trace.push(d.tick + ' ' + (d.action || '-') + ' ' + (car.kmh === undefined ? '?' : car.kmh) + 'km/h');
    if (D.lane.trace.length > 6) D.lane.trace.shift();
  }
  set(cols.trace.pre, D.lane.trace.join('\\n') || '-');
}

D.paintLane = paint;
paint(true);
if (D._laneTimer) clearInterval(D._laneTimer);
D._laneTimer = setInterval(() => paint(false), 250);
return { installed: true, lane: !!document.getElementById('hermes-lane'), show: D.lane.show };
})()`;

const out = await ev(INSTALL);
console.log(
	JSON.stringify({ page: page.url, target: TARGET, ...out }, null, 2),
);
ws.close();
