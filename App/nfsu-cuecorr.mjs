// Try to find the route arrow's data in the emulated memory.
//
// Samples the arrow's screen position (from captured frames) and a window of
// WebAssembly.Memory at the same instant, repeatedly, then correlates every
// float in that window against the arrow's x and y. A float that tracks the
// arrow strongly across samples is a candidate for its world position; anything
// near zero is noise. Reports the best candidates rather than asserting one.
import { spawnSync } from "node:child_process";
import fs from "node:fs";

const [TARGET, PORT = "9222", SAMPLES = "8", OFF_MB = "819"] =
	process.argv.slice(2);
const DIR = ".";
const PY = "python3";
const WIN = 8 * 1024 * 1024;
const OFF = Number(OFF_MB) * 1024 * 1024;

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
const key2 = (type, name) =>
	send("Input.dispatchKeyEvent", {
		type,
		key: name,
		code: name,
		windowsVirtualKeyCode: name === "ArrowLeft" ? 37 : 39,
		nativeVirtualKeyCode: name === "ArrowLeft" ? 37 : 39,
	});

await send("Page.enable");
const m = await send("Page.getLayoutMetrics");
const vp = m.cssVisualViewport || m.visualViewport;
const clip = {
	x: 0,
	y: 0,
	width: vp.clientWidth,
	height: Math.round(vp.clientHeight * 0.63),
	scale: 0.5,
};

await key("keyDown");
const rows = [];
// Alternate the steering between samples so the cue's screen x goes down, then
// up, then down. A real position tracks that; a monotone counter does not.
let pulseLeft = true;
for (let i = 0; i < Number(SAMPLES); i++) {
	const { data } = await send("Page.captureScreenshot", {
		format: "jpeg",
		quality: 55,
		clip,
	});
	const frame = `${DIR}/corr-${i}.jpg`;
	fs.writeFileSync(frame, Buffer.from(data, "base64"));
	const seen = JSON.parse(
		spawnSync(PY, [`${DIR}/analyze_frame.py`, frame, "--json"], {
			encoding: "utf8",
		}).stdout,
	);
	const snap = await ev(
		`(()=>{const m=window.__nfsu2.session.proc.memory;if(${OFF}+${WIN}>m.buffer.byteLength)return null;return JSON.stringify(Array.from(new Float32Array(m.buffer,${OFF},${WIN / 4})));})()`,
	);
	if (!snap) break;
	rows.push({
		cx: seen.arrow ? seen.arrow.cx : null,
		cy: seen.arrow ? seen.arrow.cy : null,
		mem: JSON.parse(snap),
	});
	if (i < Number(SAMPLES) - 1) {
		await key2("keyDown", pulseLeft ? "ArrowLeft" : "ArrowRight");
		await sleep(450);
		await key2("keyUp", pulseLeft ? "ArrowLeft" : "ArrowRight");
		pulseLeft = !pulseLeft;
		await sleep(260);
	}
}
await key("keyUp");

const usable = rows.filter((r) => r.cx !== null);
const report = {
	offsetMb: Number(OFF_MB),
	samples: rows.length,
	withCue: usable.length,
	candidates: [],
};
if (usable.length >= 4) {
	const n = usable.length;
	const xs = usable.map((r) => r.cx);
	const ys = usable.map((r) => r.cy);
	const mean = (a) => a.reduce((p, c) => p + c, 0) / a.length;
	const corr = (a, b) => {
		const ma = mean(a);
		const mb = mean(b);
		let num = 0;
		let da = 0;
		let db = 0;
		for (let i = 0; i < a.length; i++) {
			num += (a[i] - ma) * (b[i] - mb);
			da += (a[i] - ma) ** 2;
			db += (b[i] - mb) ** 2;
		}
		return da === 0 || db === 0 ? 0 : num / Math.sqrt(da * db);
	};
	const width = usable[0].mem.length;
	for (let k = 0; k < width; k++) {
		const col = usable.map((r) => r.mem[k]);
		if (col.some((v) => !Number.isFinite(v) || Math.abs(v) > 100000))
			continue;
		if (new Set(col).size < 3) continue;
		const rx = corr(col, xs);
		const ry = corr(col, ys);
		if (Math.abs(rx) > 0.8 || Math.abs(ry) > 0.8) {
			report.candidates.push({
				byteOffset: OFF + k * 4,
				rX: +rx.toFixed(2),
				rY: +ry.toFixed(2),
				values: col.map((v) => +v.toFixed(2)),
			});
		}
	}
	report.candidates.sort(
		(a, b) =>
			Math.max(Math.abs(b.rX), Math.abs(b.rY)) -
			Math.max(Math.abs(a.rX), Math.abs(a.rY)),
	);
	report.candidates = report.candidates.slice(0, 8);
}

fs.writeFileSync(`${DIR}/cuecorr.json`, JSON.stringify(report, null, 2));
console.log(
	JSON.stringify(
		{ ...report, candidates: report.candidates.slice(0, 5) },
		null,
		2,
	),
);
ws.close();
