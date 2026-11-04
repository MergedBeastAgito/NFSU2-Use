// Read the candidate structs found by the scan, three times while driving, and
// print them. A 16-float block with 1.0 on the diagonal is a transform matrix,
// whose translation column is a position; a float triple that drifts smoothly is
// a coordinate. Anything else is just churn.
import fs from "node:fs";

const [TARGET, PORT = "9222"] = process.argv.slice(2);
const DIR = ".";
const SPOTS = [865745488, 215242416, 755035404, 969800008];

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
await key("keyDown");

const read = () =>
	ev(`(()=>{
    const m = window.__nfsu2.session.proc.memory;
    const out = {};
    for (const off of ${JSON.stringify(SPOTS)}) {
      if (off + 260 > m.buffer.byteLength) { out[off] = null; continue }
      const f = new Float32Array(m.buffer, off, 65);
      out[off] = Array.from(f).map(v => Number.isFinite(v) ? +v.toFixed(3) : String(v));
    }
    return JSON.stringify(out);
  })()`);

const a = JSON.parse(await read());
await sleep(900);
const b = JSON.parse(await read());
await key("keyUp");

const summary = SPOTS.map((off) => {
	const va = a[off],
		vb = b[off];
	if (!va || !vb) return { offset: off, note: "out of range" };
	const changed = va
		.map((v, i) => (v !== vb[i] ? i : -1))
		.filter((i) => i >= 0);
	const first = va.slice(0, 16);
	const looksMatrix =
		Math.abs(va[0] - 1) < 0.01 &&
		Math.abs(va[5] - 1) < 0.01 &&
		Math.abs(va[10] - 1) < 0.01;
	return {
		offset: off,
		changedIndices: changed.slice(0, 12),
		changedCount: changed.length,
		first16: first,
		looksLikeIdentityMatrix: looksMatrix,
		sample: changed
			.slice(0, 4)
			.map((i) => ({ i, before: va[i], after: vb[i] })),
	};
});

fs.writeFileSync(`${DIR}/memread.json`, JSON.stringify(summary, null, 2));
console.log(JSON.stringify(summary, null, 2));
ws.close();
