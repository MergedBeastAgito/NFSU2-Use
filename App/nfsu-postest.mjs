// Validate the coordinate candidates: a position must hold still while parked and
// move while driving. Reads the same floats twice under each condition.
import fs from "node:fs";

const [TARGET, PORT = "9222"] = process.argv.slice(2);
const DIR = ".";
const SPOTS = [865739868, 865739884, 865739932, 865842484, 865842724];

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

const read = () =>
	ev(
		`(()=>{const m=window.__nfsu2.session.proc.memory;const out={};for(const off of ${JSON.stringify(SPOTS)}){out[off]=Array.from(new Float32Array(m.buffer,off,4)).map(v=>+v.toFixed(2))}return JSON.stringify(out)})()`,
	);

// Parked: no input at all.
const parkedA = JSON.parse(await read());
await sleep(1200);
const parkedB = JSON.parse(await read());

// Driving: throttle held.
await key("keyDown");
await sleep(1500);
const driveA = JSON.parse(await read());
await sleep(1500);
const driveB = JSON.parse(await read());
await key("keyUp");

const verdict = SPOTS.map((off) => {
	const delta = (a, b) => a.map((v, i) => +Math.abs(v - b[i]).toFixed(2));
	const parkedDelta = delta(parkedA[off], parkedB[off]);
	const driveDelta = delta(driveA[off], driveB[off]);
	const stillParked = Math.max(...parkedDelta) < 0.5;
	const movedDriving = Math.max(...driveDelta) > 1.0;
	return {
		offset: off,
		parkedA: parkedA[off],
		parkedB: parkedB[off],
		driveA: driveA[off],
		driveB: driveB[off],
		maxParkedDelta: Math.max(...parkedDelta),
		maxDriveDelta: Math.max(...driveDelta),
		looksLikePosition: stillParked && movedDriving,
	};
});

fs.writeFileSync(`${DIR}/postest.json`, JSON.stringify(verdict, null, 2));
console.log(JSON.stringify(verdict, null, 2));
ws.close();
