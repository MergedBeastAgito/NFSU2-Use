// Driver v4: the same policy, driven smoothly.
//
// v3 was bang-bang: hold a key for a fixed 120-600 ms, then release, on a 320 ms
// tick with throttle either fully on or fully off. That reads as surging and
// sawing. This version replaces each on/off decision with a duty cycle:
//
//   - steering: a PWM pulse inside a fixed 110 ms window, duty proportional to
//     the smoothed error, so small corrections are short presses rather than
//     nothing-or-everything;
//   - throttle: PWM too, duty from (target - speed), so it eases toward the
//     target instead of arriving at it and cutting;
//   - the cue is low-pass filtered (EMA 0.35) and held for up to 3 ticks when a
//     frame misses it, so detector jitter does not reach the wheel;
//   - the tick is 150 ms instead of 320 ms, so each correction is smaller;
//   - roam turns ramp over three ticks instead of one long hold.
//
// Usage: node nfsu-drive4.mjs <targetId> [port] [ticks] [intervalMs]
//
// The pixel modes shell out to analyze_frame.py, so `PYTHON` names the
// interpreter that has its imports installed; the default is whatever
// `python3` is on PATH.
import { spawnSync } from "node:child_process";
import fs from "node:fs";

const [TARGET, PORT = "9222", TICKS = "0", INTERVAL = "150"] =
	process.argv.slice(2);
const PY = process.env.PYTHON || "python3";
const DIR = new URL(".", import.meta.url).pathname;
const CRUISE = Number(process.env.HERMES_CRUISE || "12");
const TURN_SPEED = Number(process.env.HERMES_TURN_SPEED || "7");
const EMA = 0.35;
const DEADZONE = 0.02;
const STEER_WINDOW = 110;
const CUE_HOLD_TICKS = 3;
const G_RACETYPE = 0x0089e7b0;

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
let latestFrame = null;
ws.onmessage = (e) => {
	const m = JSON.parse(e.data);
	if (m.method === "Page.screencastFrame") {
		latestFrame = m.params.data;
		ws.send(
			JSON.stringify({
				id: ++id,
				method: "Page.screencastFrameAck",
				params: { sessionId: m.params.sessionId },
			}),
		);
		return;
	}
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
const sleep = (ms) => new Promise((r) => setTimeout(r, Math.max(0, ms)));
const VK = { ArrowLeft: 37, ArrowUp: 38, ArrowRight: 39, ArrowDown: 40 };
const key = (type, name) =>
	send("Input.dispatchKeyEvent", {
		type,
		key: name,
		code: name,
		windowsVirtualKeyCode: VK[name],
		nativeVirtualKeyCode: VK[name],
	});

await send("Page.enable");
await send("Page.startScreencast", {
	format: "jpeg",
	quality: 55,
	maxWidth: 1058,
	maxHeight: 470,
	everyNthFrame: 1,
});

await ev(`(()=>{
  window.__car = () => {
    const dv = new DataView(window.__nfsu2.session.proc.memory.buffer);
    const u32 = a => dv.getUint32(a, true), f32 = a => dv.getFloat32(a, true), u16 = a => dv.getUint16(a, true);
    const player = u32(0x00d00000); const car = player ? u32(player + 4) : 0;
    if (!car) return null;
    return { car, pos: [+f32(car+0x60).toFixed(1), +f32(car+0x64).toFixed(1), +f32(car+0x68).toFixed(1)],
      vel: [+f32(car+0x70).toFixed(2), +f32(car+0x74).toFixed(2), +f32(car+0x78).toFixed(2)],
      yawRate: +f32(car+0x110).toFixed(3),
      steer: +(dv.getInt16(u32(car+0x30) + 0x1e0, true) / 32767).toFixed(3),
      surface: [0,1,2,3].map(k => dv.getUint32(car+0x394 + 4*k, true)),
      slip: [0,1,2,3,4,5,6,7].map(k => +f32(car+0x3b4 + 4*k).toFixed(2)),
      grounded: [0,1,2,3].map(k => !!dv.getUint8(car+0x390 + k)),
      heading: +(u16(car+0x40c)/65536*360).toFixed(1), speed: +f32(car+0x430).toFixed(2),
      rpm: Math.round(f32(car+0x440)),
      kmh: Math.round(Math.abs(f32(car+0x430))*3.6), gear: dv.getInt8(car+0x434), racetype: u32(${G_RACETYPE}) };
  };
  return 1;
})()`);

const publish = async (payload) => {
	// One evaluate carries the overlay update and the bus rows, so a tick costs
	// one round trip and the tables are the record - no .jsonl anywhere.
	const expression = `(()=>{
    const D=window.__hermesDebug; if(D){Object.assign(D,${JSON.stringify(payload)}); if(D.push)D.push('out',${JSON.stringify(payload._label || "hold")},${JSON.stringify(payload._note || "")});}
    const B=window.__bus; if(B && ${JSON.stringify(Boolean(payload.car))}){B.publish('observation',${JSON.stringify(payload.car || {})});B.publish('decision',${JSON.stringify(payload.decision || {})});}
    return 1;
  })()`;
	await send("Runtime.evaluate", { expression, returnByValue: true });
};

// No trace file: the bus is the record now (see nfsu-bus.mjs).
let prevFrame = null;
let cue = null;
let cueAge = 99;
let lost = 0;
let stalled = 0;
let backUntil = 0;
let ramp = 0;
let rampDir = 1;
let backLeft = false;
let ticks = 0;
let maxSpeed = 0;
let stopped = false;

const release = async () => {
	stopped = true;
	for (const k of ["ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight"])
		await key("keyUp", k);
	await send("Page.stopScreencast").catch(() => {});
};
for (const sig of ["SIGINT", "SIGTERM"]) {
	process.on(sig, async () => {
		await release();
		console.log(
			JSON.stringify({
				stopped: sig,
				ticks,
				maxSpeed: +maxSpeed.toFixed(1),
			}),
		);
		process.exit(0);
	});
}

const started = Date.now();
while (!stopped && (Number(TICKS) === 0 || ticks < Number(TICKS))) {
	const t0 = Date.now();
	ticks++;
	const mode = await ev(
		"window.__mode || (window.__hermesDebug && window.__hermesDebug.mode) || 'memory'",
	);
	const car = JSON.parse(await ev("JSON.stringify(window.__car())"));
	const speed = car ? car.speed : 0;
	maxSpeed = Math.max(maxSpeed, Math.abs(speed));

	const needsPixels = mode === "gps" || mode === "road" || mode === "race";
	let rawCx = null;
	let seenPixels = 0;
	if (needsPixels && latestFrame) {
		const frame = `${DIR}/tick-${ticks % 2}.jpg`;
		fs.writeFileSync(frame, Buffer.from(latestFrame, "base64"));
		const args = [`${DIR}/analyze_frame.py`, frame, "--json"];
		if (prevFrame) args.push("--prev", prevFrame);
		prevFrame = frame;
		const seen = JSON.parse(
			spawnSync(PY, args, { encoding: "utf8" }).stdout || "{}",
		);
		if (seen.arrow) {
			rawCx = seen.arrow.cx;
			seenPixels = seen.arrow.pixels || 0;
		}
	}
	// Low-pass the cue and ride out a missing frame, so jitter never reaches the wheel.
	if (rawCx !== null) {
		cue = cue === null ? rawCx : EMA * rawCx + (1 - EMA) * cue;
		cueAge = 0;
	} else {
		cueAge++;
	}
	const usable =
		cue !== null &&
		cueAge <= CUE_HOLD_TICKS &&
		(mode === "gps" || mode === "race");
	lost = usable ? 0 : lost + 1;

	let label = "coast";
	let steerDuty = 0;
	let steerKey = null;
	let throttleDuty = 0;
	let brakeMs = 0;
	const targetSpeed = usable
		? rawCx !== null && cue !== null && cue < 0.35
			? 15
			: 9
		: CRUISE;

	if (mode === "ui") {
		label = "unsupported";
	} else if (Date.now() < backUntil) {
		await key("keyDown", "ArrowDown");
		// Back out AND peel off: reversing straight leaves it against the same
		// wall, which is why it was recovering eleven times a run.
		steerKey = backLeft ? "ArrowLeft" : "ArrowRight";
		steerDuty = 0.8;
		label = "reverse";
	} else {
		await key("keyUp", "ArrowDown");
		stalled = Math.abs(speed) < 1.0 ? stalled + 1 : 0;
		if (stalled >= 3) {
			stalled = 0;
			backUntil = Date.now() + 800;
			backLeft = !backLeft;
			await key("keyUp", "ArrowUp");
			await key("keyDown", "ArrowDown");
			label = "reverse";
		} else if (usable) {
			const err = cue - 0.5;
			if (Math.abs(err) > DEADZONE) {
				steerKey = err < 0 ? "ArrowLeft" : "ArrowRight";
				steerDuty = Math.min(1, Math.abs(err) * 7);
				label = err < 0 ? "steer left" : "steer right";
			} else label = "centred";
		} else if (mode === "race" && car && car.racetype === 0) {
			label = "not racing";
		} else {
			// Roam: ramp the turn over three ticks instead of one long hold.
			if (ramp > 0) {
				ramp--;
				steerKey = rampDir > 0 ? "ArrowRight" : "ArrowLeft";
				steerDuty = 0.35 + (0.25 * (3 - ramp)) / 3;
				label = "roam turn";
			} else if (lost % 10 === 0) {
				ramp = 3;
				rampDir = -rampDir;
				steerKey = rampDir > 0 ? "ArrowRight" : "ArrowLeft";
				steerDuty = 0.35;
				label = "roam turn";
			} else label = "cruise";
		}

		// Throttle with hysteresis rather than PWM: pulsing it lost too much
		// thrust and the car stalled nine times in a minute. Steering stays PWM
		// because that is where the visible jerk came from.
		if (label !== "unsupported" && label !== "not racing") {
			if (Math.abs(speed) < targetSpeed - 1) {
				throttleDuty = 1;
			} else if (Math.abs(speed) > targetSpeed + 1.5) {
				throttleDuty = 0;
				if (Math.abs(speed) > targetSpeed + 5) brakeMs = 90;
			} else {
				throttleDuty = -1; // hold: leave the key exactly as it is
			}
		}
	}

	// Apply: steering first in its own window, then throttle for what is left.
	if (steerKey && steerDuty > 0.05) await key("keyDown", steerKey);
	await sleep(steerDuty > 0.05 ? STEER_WINDOW * steerDuty : 0);
	if (steerKey) await key("keyUp", steerKey);

	if (throttleDuty === 1) await key("keyDown", "ArrowUp");
	else if (throttleDuty === 0) await key("keyUp", "ArrowUp");
	// -1 means hold: the key is left exactly as it was.
	if (brakeMs) {
		await key("keyUp", "ArrowUp");
		await key("keyDown", "ArrowDown");
		await sleep(brakeMs);
		await key("keyUp", "ArrowDown");
	}

	const elapsed = Date.now() - t0;
	await sleep(Number(INTERVAL) - elapsed);

	await publish({
		car: car || {},
		auto: {
			on: true,
			ticks,
			steering: 0,
			reversing: 0,
			braking: 0,
			searching: 0,
			mode,
		},
		arrowData: {
			cx: cue === null ? null : +cue.toFixed(3),
			cy: null,
			none: cue === null,
			pixels: seenPixels,
		},
		decision: {
			tick: ticks,
			action: label,
			ms: Math.round(steerDuty * STEER_WINDOW),
			cx: cue === null ? null : +cue.toFixed(3),
			cy: null,
			band: null,
			reason: `duty ${steerDuty.toFixed(2)} · thr ${throttleDuty.toFixed(2)} · ${Math.round(Math.abs(speed) * 3.6)} km/h`,
			held: true,
		},
		_label: label,
		_note: `${Math.round(throttleDuty * 100)}%`,
	});
	// The tick is already on the bus (observation + decision); no file, by design.
}

await release();
await publish({
	auto: {
		on: false,
		ticks,
		steering: 0,
		reversing: 0,
		braking: 0,
		searching: 0,
		mode: "idle",
	},
});
const status = JSON.parse(
	(
		await send("Runtime.evaluate", {
			expression: "JSON.stringify(window.__nfsu2.session.input.status())",
			returnByValue: true,
		})
	).result.value,
);
console.log(
	JSON.stringify(
		{
			ticks,
			maxSpeed: +maxSpeed.toFixed(1),
			seconds: Math.round((Date.now() - started) / 1000),
			status,
		},
		null,
		2,
	),
);
ws.close();
