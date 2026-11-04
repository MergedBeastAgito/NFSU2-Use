// Scan several windows across the emulated memory while the car drives, looking
// for vector-shaped runs of changing values (a position, a transform). A probe,
// not a claim: a run is a hypothesis until it correlates with movement.
import fs from "node:fs";

const [TARGET, PORT = "9222"] = process.argv.slice(2);
const DIR = ".";
const WIN = 8 * 1024 * 1024;

const list = await (await fetch(`http://127.0.0.1:${PORT}/json/list`)).json();
const page = list.find((t) => t.id === TARGET);
if (!page) throw new Error("no target");
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
const ev = async (x) =>
	(
		await send("Runtime.evaluate", {
			expression: x,
			returnByValue: true,
			awaitPromise: true,
		})
	).result.value;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const key = (type) =>
	send("Input.dispatchKeyEvent", {
		type,
		key: "ArrowUp",
		code: "ArrowUp",
		windowsVirtualKeyCode: 38,
		nativeVirtualKeyCode: 38,
	});

await send("Page.enable");

const size = await ev("window.__nfsu2.session.proc.memory.buffer.byteLength");
const offsets = [];
// Float32Array views must start on a 4-byte boundary, so the stride is aligned.
const step = Math.max(WIN, Math.floor(size / 10 / 4) * 4);
for (let o = 0; o + WIN <= size && offsets.length < 10; o += step)
	offsets.push(o);

await key("keyDown");
await sleep(500);

const results = [];
for (const off of offsets) {
	const summary = await ev(`(()=>{
    const m = window.__nfsu2.session.proc.memory;
    const a = new Float32Array(m.buffer, ${off}, ${WIN / 4}).slice(0);
    return new Promise(res => setTimeout(() => {
      const b = new Float32Array(m.buffer, ${off}, ${WIN / 4}).slice(0);
      const hits = [];
      for (let i = 0; i < a.length; i++) {
        const x = a[i], y = b[i];
        if (!Number.isFinite(x) || !Number.isFinite(y)) continue;
        if (Math.abs(x) > 20000 || Math.abs(y) > 20000 || x === y) continue;
        const d = y - x;
        if (Math.abs(d) < 1e-3 || Math.abs(d) > 400) continue;
        hits.push(i);
        if (hits.length > 3000) break;
      }
      let best = 0, runEnd = -1, cur = 0, prev = -9;
      for (const i of hits) {
        if (i === prev + 1) cur++; else cur = 1;
        if (cur > best) { best = cur; runEnd = i }
        prev = i;
      }
      res(JSON.stringify({ changed: hits.length, longestRun: best, runAt: runEnd >= 0 ? ${off} + (runEnd - best + 1) * 4 : null }));
    }, 900));
  })()`);
	results.push({
		offsetMb: Math.round(off / 1048576),
		...JSON.parse(summary),
	});
}
await key("keyUp");

const report = { bufferMb: Math.round(size / 1048576), windows: results };
fs.writeFileSync(`${DIR}/memscan.json`, JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
ws.close();
