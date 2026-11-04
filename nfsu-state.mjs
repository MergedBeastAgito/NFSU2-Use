// Read the live car state out of the emulated memory, using the addresses the
// port's own runtime uses (runtime/roam_game.c local_state()):
//
//   G_PLAYER1 0x00d00000 (wasm build) -> Player*, +4 = Car*
//   Car +0x60  float[3] position (m)        Car +0x70  float[3] velocity
//   Car +0x90  float[3] forward basis       Car +0xa0  left, Car +0xb0 up
//   Car +0x42c float speed (m/s)            Car +0x440 rpm, +0x434 gear
//   Car +0x30  -> control block: +0x1e0 steer (i16), +0x20c throttle, +0x214 brake
//
// Prints the state, drives briefly, prints it again so the reader is proven
// against real motion rather than assumed.
import fs from "node:fs";

const [TARGET, PORT = "9222"] = process.argv.slice(2);
const DIR = ".";

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
const ev = async (x) => (await send("Runtime.evaluate", { expression: x, returnByValue: true, awaitPromise: true })).result.value;
const sleep = (ms) => new Promise((r) => setTimeout(r, ms));
const key = (type, name) =>
	send("Input.dispatchKeyEvent", {
		type,
		key: name,
		code: name,
		windowsVirtualKeyCode: name === "ArrowLeft" ? 37 : name === "ArrowUp" ? 38 : 39,
		nativeVirtualKeyCode: name === "ArrowLeft" ? 37 : name === "ArrowUp" ? 38 : 39,
	});

await send("Page.enable");

// Install the reader once, so each sample is a single call.
await ev(`(()=>{
  window.__car = () => {
    const mem = window.__nfsu2.session.proc.memory;
    const dv = new DataView(mem.buffer);
    const u32 = a => dv.getUint32(a, true);
    const f32 = a => dv.getFloat32(a, true);
    const i16 = a => dv.getInt16(a, true);
    const i8  = a => dv.getInt8(a, true);
    const player = u32(0x00d00000);
    const car = player ? u32(player + 4) : 0;
    if (!car) return { car: 0 };
    const ctl = u32(car + 0x30);
    const f = [f32(car + 0x90), f32(car + 0x94), f32(car + 0x98)];
    const heading = Math.atan2(f[0], f[1]) * 180 / Math.PI;   // degrees, 0 = +y
    return {
      player, car,
      pos: [f32(car + 0x60), f32(car + 0x64), f32(car + 0x68)].map(v => +v.toFixed(2)),
      vel: [f32(car + 0x70), f32(car + 0x74), f32(car + 0x78)].map(v => +v.toFixed(2)),
      yawRate: +f32(car + 0x110).toFixed(3),
      speed: +f32(car + 0x42c).toFixed(2),
      kmh: Math.round(f32(car + 0x42c) * 3.6),
      rpm: Math.round(f32(car + 0x440)),
      gear: i8(car + 0x434),
      heading: +heading.toFixed(1),
      forward: f.map(v => +v.toFixed(3)),
      ctl: ctl ? { steer: +(i16(ctl + 0x1e0) / 32767).toFixed(3), throttle: +f32(ctl + 0x20c).toFixed(2), brake: +f32(ctl + 0x214).toFixed(2) } : null,
    };
  };
  return "installed";
})()`);

const sample = async () => JSON.parse(await ev("JSON.stringify(window.__car())"));

const idle = await sample();
await key("keyDown", "ArrowUp");
await sleep(1800);
const driving = await sample();
await key("keyDown", "ArrowLeft");
await sleep(700);
const turning = await sample();
await key("keyUp", "ArrowLeft");
await key("keyUp", "ArrowUp");
await sleep(400);
const after = await sample();

const report = { idle, driving, turning, after };
fs.writeFileSync(`${DIR}/state.json`, JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
ws.close();
