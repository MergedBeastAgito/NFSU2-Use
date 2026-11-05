/**
 * Attach the wasm reader to the running game and prove the card's four claims.
 *
 *   node Test/attach.mjs [--port 9222] [--target <id>] [--ticks 20] [--soak 60]
 *
 * What it checks, in order:
 *
 *   1. Shape, offline (`wasm-objdump` on the artefact): one import, and it is a
 *      shared memory; no data segment, no globals, no defined memory, no start
 *      function; no `memory.grow`, no store of any width, no allocation symbol.
 *      A data segment is not a detail: wasm applies it at instantiation, at an
 *      address the linker chose, which would overwrite the game's low memory.
 *   2. Attach: the module instantiates against `session.proc.memory` itself.
 *   3. Twenty consecutive ticks of equality against the page's own reader. A
 *      tick counts only when two full wasm reads and two full JavaScript reads
 *      all agree - the game's physics runs on other threads, so a single read
 *      of each cannot prove the two saw the same instant. The car is driven
 *      during the run, so the values are moving ones, not a frozen snapshot.
 *   4. A panic in the module is caught as a RuntimeError and the game survives.
 *   5. The game is still running `--soak` seconds after attach.
 *
 * The report goes to Test/report.json; the exit code is 0 only when every
 * claim held.
 */
import { execFileSync } from "node:child_process";
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const Here = path.dirname(fileURLToPath(import.meta.url));
const Root = path.resolve(Here, "..");
const Wasm = path.join(
	Root,
	"target/wasm32-unknown-unknown/release/NfsuReader.wasm",
);

// The report is written inside the repository and read by a person diffing one
// run against the next, so a path in it is named from the repository root. An
// absolute path would publish the layout of whichever machine ran the harness. A
// checkout with no `.git` above it falls back to the working directory, which is
// still relative.
const Named = (Path) => {
	let Directory = path.resolve(Path);
	for (;;) {
		if (fs.existsSync(path.join(Directory, ".git"))) {
			return path.relative(Directory, Path).split(path.sep).join("/");
		}
		const Parent = path.dirname(Directory);
		if (Parent === Directory) return path.relative(process.cwd(), Path);
		Directory = Parent;
	}
};

const Arg = (Name, Fallback) => {
	const i = process.argv.indexOf(`--${Name}`);
	return i > 0 && process.argv[i + 1] ? process.argv[i + 1] : Fallback;
};
const Port = Arg("port", "9222");
const Ticks = Number(Arg("ticks", "20"));
const Soak = Number(Arg("soak", "60"));
const Target = Arg("target", "");

const sleep = (ms) => new Promise((r) => setTimeout(r, ms));

// ---- 1. shape, offline ------------------------------------------------------

function Shape() {
	const run = (...a) =>
		execFileSync("wasm-objdump", [...a, Wasm], { encoding: "utf8" });
	const head = run("-h");
	const x = run("-x");
	const d = run("-d");
	const Fail = [];
	if (!/Import start/.test(head)) Fail.push("no import section");
	const Imports = (x.match(/<- env\.memory/g) ?? []).length;
	if (Imports !== 1)
		Fail.push(`expected one memory import, found ${Imports}`);
	if (!/pages: initial=16 max=65536 shared/.test(x))
		Fail.push("the import is not a shared memory with a 4 GB maximum");
	// A data segment writes into the imported memory at instantiation.
	if (/\bData start/.test(head))
		Fail.push(
			"the module carries a data segment: it would write into the game's memory at instantiation",
		);
	if (/\bGlobal start/.test(head))
		Fail.push("the module carries globals (a stack pointer or statics)");
	if (/\bMemory start/.test(head))
		Fail.push("the module defines a memory instead of importing one");
	if (/\bStart start/.test(head))
		Fail.push(
			"the module has a start function: something runs before a caller asks",
		);
	if (/\bElem start|\bTable start/.test(head))
		Fail.push("the module carries a table");
	const Grow = (d.match(/memory\.grow/g) ?? []).length;
	const Stores = (
		d.match(/\b(?:i32|i64|f32|f64|v128|i8|i16)\.store(?:8|16|32)?\b/g) ?? []
	).length;
	const Copies = (d.match(/memory\.(?:copy|fill|init)/g) ?? []).length;
	const Sizes = (d.match(/memory\.size/g) ?? []).length;
	if (Grow !== 0) Fail.push(`memory.grow appears ${Grow} time(s)`);
	if (Stores !== 0)
		Fail.push(
			`${Stores} store instruction(s): the module writes the game's memory`,
		);
	if (Copies !== 0) Fail.push("memory.copy/fill/init present");
	if (
		/\b(?:__rust_alloc|__rust_dealloc|__rust_realloc|dlmalloc|malloc)\b/.test(
			x,
		)
	)
		Fail.push("an allocation symbol is present");
	// The transport: a v128 return compiles, and is then refused at the call
	// site by the JavaScript API, so the surface is checked by name.
	const Exports = [...x.matchAll(/- func\[\d+\] <[^>]*> -> "([^"]+)"/g)].map(
		(m) => m[1],
	);
	const Want = [
		"read_state",
		"read_position_z",
		"read_heading",
		"read_speed",
		"read_rpm",
		"read_gear",
		"read_car",
		"panic_probe",
	];
	for (const W of Want)
		if (!Exports.includes(W)) Fail.push(`missing export ${W}`);
	for (const E of Exports)
		if (!Want.includes(E)) Fail.push(`unexpected export ${E}`);
	if (/-> v128/.test(x))
		Fail.push("an export returns v128, which the JavaScript API refuses");
	return {
		bytes: fs.statSync(Wasm).size,
		imports: Imports,
		exports: Exports,
		memory_grow: Grow,
		stores: Stores,
		memory_copy: Copies,
		memory_size: Sizes,
		data_segment: /\bData start/.test(head),
		globals: /\bGlobal start/.test(head),
		failures: Fail,
	};
}

// ---- 2. the page ------------------------------------------------------------

const list = await (await fetch(`http://127.0.0.1:${Port}/json/list`)).json();
const Page = Target
	? list.find((t) => t.id === Target)
	: list.find(
			(t) => t.type === "page" && /^https:\/\/nfsu2\.xyz/.test(t.url),
		);
if (!Page)
	throw new Error(
		`no NFSU2 page on 127.0.0.1:${Port}; open https://nfsu2.xyz/ or pass --target <id>`,
	);

const Ws = new WebSocket(Page.webSocketDebuggerUrl);
await new Promise((r, j) => {
	Ws.onopen = r;
	Ws.onerror = j;
});
let Next = 0;
const Pending = new Map();
Ws.onmessage = (e) => {
	const m = JSON.parse(e.data);
	const s = Pending.get(m.id);
	if (s) {
		Pending.delete(m.id);
		s(m);
	}
};
const Send = (method, params = {}) =>
	new Promise((r) => {
		const id = ++Next;
		Pending.set(id, r);
		Ws.send(JSON.stringify({ id, method, params }));
	});
const Eval = async (expression, timeout = 120_000) => {
	const m = await Send("Runtime.evaluate", {
		expression,
		returnByValue: true,
		awaitPromise: true,
		timeout,
	});
	if (m.result?.exceptionDetails) {
		const e = m.result.exceptionDetails;
		throw new Error(e.exception?.description ?? e.text);
	}
	return m.result?.result?.value;
};

// The game has to be running: the reader reads a process that exists.
const Phase = await Eval(
	`({ phase: document.body.dataset.phase, session: !!window.__nfsu2.session })`,
);
if (!Phase.session) {
	throw new Error(
		`the game is not running (phase ${Phase.phase}); press Play in the tab and re-run`,
	);
}

const Bytes = fs.readFileSync(Wasm).toString("base64");
const Attach = await Eval(`(async () => {
	const bin = Uint8Array.from(atob(${JSON.stringify(Bytes)}), (c) => c.charCodeAt(0));
	const Mod = await WebAssembly.compile(bin);
	const Mem = window.__nfsu2.session.proc.memory;
	const Inst = await WebAssembly.instantiate(Mod, { env: { memory: Mem } });
	const X = Inst.exports;
	// The page's own reader, unrounded: the same addresses Bootstrap.ts uses, so
	// "equal" means equal to the reader the driver already trusts.
	const View = new DataView(Mem.buffer);
	const U32 = (a) => View.getUint32(a, true), F32 = (a) => View.getFloat32(a, true), U16 = (a) => View.getUint16(a, true);
	const Page = () => {
		const P = U32(0x00d00000), Car = P ? U32(P + 4) : 0;
		if (!Car) return { car: 0 };
		return { car: Car, pos: [F32(Car + 0x60), F32(Car + 0x64), F32(Car + 0x68)],
			heading: (U16(Car + 0x40c) / 65536) * 360, speed: F32(Car + 0x430),
			rpm: F32(Car + 0x440), gear: View.getInt8(Car + 0x434) };
	};
	const Bits = new Uint32Array(2), Lanes = new Float32Array(Bits.buffer);
	// read_state is one i64: x in the low half, y in the high half, both the
	// game's own f32 bit patterns.
	const Pair = (v) => { Bits[0] = Number(v & 0xffffffffn); Bits[1] = Number((v >> 32n) & 0xffffffffn); return [Lanes[0], Lanes[1]]; };
	const Wasm = () => {
		const Car = X.read_car();
		if (!Car) return { car: 0 };
		const [x, y] = Pair(X.read_state());
		return { car: Car, pos: [x, y, X.read_position_z()], speed: X.read_speed(), heading: X.read_heading(), rpm: X.read_rpm(), gear: X.read_gear() };
	};
	window.__wasmReader = { X, Mem, Page, Wasm };
	return { exports: Object.keys(X), memory: { pages: Mem.buffer.byteLength / 65536, shared: Mem.buffer instanceof SharedArrayBuffer },
		first: Wasm(), page_first: Page(), memory_is_the_game: Mem === window.__nfsu2.session.proc.memory };
})()`);

if (!Attach.memory_is_the_game)
	throw new Error("the module was not given the game's own memory object");
if (!Attach.first.car)
	throw new Error(
		"the reader resolved no car; the game is running but the world has none",
	);

// ---- 3. twenty consecutive ticks -------------------------------------------

const Tick = await Eval(`(async () => {
	const R = window.__wasmReader, S = window.__nfsu2.session;
	const Key = (c, on) => { try { S.input.key(c, on); } catch (e) {} };
	const Eq = (a, b) => a.car === b.car && a.speed === b.speed && a.heading === b.heading && a.rpm === b.rpm && a.gear === b.gear &&
		a.pos[0] === b.pos[0] && a.pos[1] === b.pos[1] && a.pos[2] === b.pos[2];
	const rows = [];
	let accepted = 0, attempts = 0, rejected = 0, moved = 0;
	const Throttle = setInterval(() => Key("ArrowUp", true), 16);
	const Turn = setInterval(() => { if (accepted >= Math.floor(${Ticks} / 2)) Key("ArrowLeft", true); }, 16);
	try {
		while (accepted < ${Ticks} && attempts < ${Ticks} * 20) {
			attempts++;
			// Two reads of each, in one synchronous block: a tick counts only when
			// the game did not move between any two of the four reads.
			const J1 = R.Page(), W1 = R.Wasm(), W2 = R.Wasm(), J2 = R.Page();
			const Stable = Eq(J1, J2) && Eq(W1, W2);
			const Equal = Eq(W1, J1);
			if (Stable && Equal) {
				if (rows.length) { const p = rows[rows.length - 1]; if (p.speed !== W1.speed || p.pos[0] !== W1.pos[0] || p.pos[1] !== W1.pos[1] || p.heading !== W1.heading) moved++; }
				accepted++;
				rows.push({ tick: accepted, attempt: attempts, pos: W1.pos, heading: W1.heading, speed: W1.speed, rpm: W1.rpm, gear: W1.gear });
			} else rejected++;
			await new Promise((r) => setTimeout(r, 16));
		}
	} finally {
		clearInterval(Throttle); clearInterval(Turn);
		Key("ArrowLeft", false); Key("ArrowUp", false);
	}
	return { rows, accepted, attempts, rejected, moved };
})()`);

// ---- 4. a panic must not take the game down --------------------------------

const Panic = await Eval(`(() => {
	const S = window.__nfsu2.session;
	const Before = { fps: window.__nfsu2.fps.game, car: window.__wasmReader.X.read_car() };
	let Caught = null, Name = null;
	try { window.__wasmReader.X.panic_probe(); } catch (e) { Caught = String(e); Name = e && e.name; }
	// The instance is a trap site now; the module's other exports must still work.
	let After = null, AfterError = null;
	try { After = window.__wasmReader.X.read_car(); } catch (e) { AfterError = String(e); }
	return { caught: Caught, name: Name, before: Before, after: After, after_error: AfterError,
		session_alive: !!window.__nfsu2.session, phase: document.body.dataset.phase };
})()`);

// ---- 5. the game still runs a minute later ---------------------------------

const AttachedAt = Date.now();
await sleep(Soak * 1000);
const Soak_ = await Eval(`(() => {
	const S = window.__nfsu2.session, R = window.__wasmReader;
	const J = R.Page(), W = R.Wasm();
	const Eq = (a, b) => a.car === b.car && a.speed === b.speed && a.heading === b.heading && a.rpm === b.rpm && a.gear === b.gear &&
		a.pos[0] === b.pos[0] && a.pos[1] === b.pos[1] && a.pos[2] === b.pos[2];
	return { session_alive: !!S, phase: document.body.dataset.phase, exit_code: S ? S.exitCode : null,
		fps: window.__nfsu2.fps.game, same_memory: R.Mem === S.proc.memory,
		car: W.car, equal_now: Eq(W, J), pos: W.pos, speed: W.speed };
})()`);

// ---- report ----------------------------------------------------------------

const Shape_ = Shape();
const Report = {
	at: new Date().toISOString(),
	wasm: { path: Named(Wasm), ...Shape_ },
	attach: { ...Attach, page_first: undefined },
	ticks: { required: Ticks, ...Tick },
	panic: Panic,
	soak: { seconds: Math.round((Date.now() - AttachedAt) / 1000), ...Soak_ },
};

const Verdict = [];
Verdict.push([
	Shape_.failures.length === 0,
	`artefact: ${Shape_.bytes} B, 1 shared memory import, ${Shape_.stores} stores, ${Shape_.memory_grow} memory.grow, ${Shape_.memory_size} memory.size, data segment: ${Shape_.data_segment}, globals: ${Shape_.globals}`,
]);
Verdict.push([
	Tick.accepted === Ticks,
	`${Tick.accepted}/${Ticks} consecutive ticks equal (${Tick.rejected} rejected for a mid-read game update, ${Tick.moved} ticks differed from the one before)`,
]);
Verdict.push([
	!!Panic.caught && Panic.session_alive && Panic.after !== null,
	`panic caught as ${Panic.name}; the module still answers (car ${Panic.after}); the session is ${Panic.session_alive ? "alive" : "gone"}`,
]);
Verdict.push([
	Soak_.session_alive &&
		Soak_.fps > 0 &&
		Soak_.same_memory &&
		Soak_.equal_now &&
		Soak_.exit_code === null,
	`${Report.soak.seconds}s after attach: session ${Soak_.session_alive ? "alive" : "gone"}, ${Soak_.fps?.toFixed?.(1)} fps, same memory: ${Soak_.same_memory}, equal now: ${Soak_.equal_now}`,
]);

fs.writeFileSync(
	path.join(Here, "report.json"),
	JSON.stringify(Report, null, 2),
);
for (const [Ok, Text] of Verdict)
	console.log(`${Ok ? "PASS" : "FAIL"}  ${Text}`);
console.log(`\nreport: ${path.join(Here, "report.json")}`);
Ws.close();
process.exit(Verdict.every(([Ok]) => Ok) ? 0 : 1);
