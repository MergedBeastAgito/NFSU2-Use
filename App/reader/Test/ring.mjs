/**
 * Measure the game's input mailbox, so the ring layout is documented from the
 * running process rather than from a reading of the source alone.
 *
 *   node Test/ring.mjs [--port 9222] [--target <id>]
 *
 * The mailbox is the block the game registers for `HostKind.Input` (3), and its
 * base is not a constant - it is whatever the game's allocator gave it, which is
 * why this asks `session.proc.registered(3)` for it instead of guessing.
 *
 * What it establishes, and why it matters to the card:
 *
 *   - which ring is written by the page and which by the game (they run in
 *     opposite directions, and the two are easy to swap);
 *   - that the page -> game ring advances when `session.input.key` is used, so
 *     the JS path is the one that works today;
 *   - that the game -> page ring stays still while nothing asks for synthetic
 *     input, so a second writer there would be joining a quiet queue rather
 *     than proving it is safe.
 *
 * The report goes to Test/ring.json.
 */
import fs from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const Here = path.dirname(fileURLToPath(import.meta.url));
const Arg = (Name, Fallback) => {
	const i = process.argv.indexOf(`--${Name}`);
	return i > 0 && process.argv[i + 1] ? process.argv[i + 1] : Fallback;
};
const Port = Arg("port", "9222");
const Target = Arg("target", "");

const list = await (await fetch(`http://127.0.0.1:${Port}/json/list`)).json();
const Page = Target
	? list.find((t) => t.id === Target)
	: list.find(
			(t) => t.type === "page" && /^https:\/\/nfsu2\.xyz/.test(t.url),
		);
if (!Page) throw new Error(`no NFSU2 page on 127.0.0.1:${Port}`);

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
const Eval = async (expression) => {
	const m = await Send("Runtime.evaluate", {
		expression,
		returnByValue: true,
		awaitPromise: true,
	});
	if (m.result?.exceptionDetails)
		throw new Error(
			m.result.exceptionDetails.exception?.description ??
				m.result.exceptionDetails.text,
		);
	return m.result?.result?.value;
};

const Out = await Eval(`(async () => {
	const S = window.__nfsu2.session;
	const Reg = await S.proc.registered(3);
	const Words = new Int32Array(S.proc.memory.buffer, Reg.ptr, 64);
	const U32 = (w) => Words[w] >>> 0;
	const W = {
		magic: U32(0), version: U32(1), size: U32(2), ev_n: U32(3),
		ev_w: U32(4), ev_r: U32(5), ev_drop: U32(6), attached: U32(7),
		syn_w: U32(8), syn_r: U32(9), syn_drop: U32(10),
		pad_seq: U32(11), keys_held: U32(25), events: U32(26), quit: U32(28),
	};
	const Before = { ...W };
	const Sample = () => ({ ev_w: U32(4), ev_r: U32(5), ev_drop: U32(6), syn_w: U32(8), syn_r: U32(9) });
	// One key, held the way the driver holds it: a single call is a tap.
	S.input.key("ArrowUp", true);
	await new Promise((r) => setTimeout(r, 300));
	const Held = Sample();
	const Hold = setInterval(() => S.input.key("ArrowUp", true), 16);
	await new Promise((r) => setTimeout(r, 700));
	clearInterval(Hold);
	S.input.key("ArrowUp", false);
	await new Promise((r) => setTimeout(r, 300));
	const Released = Sample();
	// The layout, checked against the block's own declared size.
	const Layout = {
		header_words: 64, header_bytes: 256,
		events: { offset: 4416, slots: W.ev_n, stride: 16, bytes: W.ev_n * 16 },
		synthetic: { offset: 20800, slots: 256, stride: 16, bytes: 256 * 16 },
		local_pads: { offset: 24896, pads: 6, words: 32, bytes: 6 * 32 * 4 },
		declared_size: W.size,
	};
	Layout.arithmetic = {
		events_end: Layout.events.offset + Layout.events.bytes,
		synthetic_end: Layout.synthetic.offset + Layout.synthetic.bytes,
		pads_end: Layout.local_pads.offset + Layout.local_pads.bytes,
		matches_declared: Layout.local_pads.offset + Layout.local_pads.bytes === W.size &&
			4416 + W.ev_n * 16 === 20800 && 20800 + 256 * 16 === 24896,
	};
	return { base: Reg.ptr, size: Reg.size, header: Before, held: Held, released: Released, layout: Layout };
})()`);

const Layout = Out.layout;
const Checks = [
	[
		Out.header.magic === 0x4e494e50 && Out.header.version === 2,
		`mailbox at 0x${Out.base.toString(16)} (${Out.size} bytes): magic 0x${Out.header.magic.toString(16)}, version ${Out.header.version}`,
	],
	[
		Layout.arithmetic.matches_declared,
		`layout adds up: events ${Layout.events.offset}+${Layout.events.bytes}, synthetic ${Layout.synthetic.offset}+${Layout.synthetic.bytes}, pads ${Layout.local_pads.offset}+${Layout.local_pads.bytes} = ${Layout.declared_size}`,
	],
	[
		Out.released.ev_w > Out.header.ev_w &&
			Out.released.ev_r >= Out.released.ev_w - Layout.events.slots,
		`page -> game ring advanced: w ${Out.header.ev_w} -> ${Out.held.ev_w} -> ${Out.released.ev_w}, r ${Out.header.ev_r} -> ${Out.released.ev_r} (dropped ${Out.released.ev_drop})`,
	],
	[
		Out.released.syn_w === Out.header.syn_w &&
			Out.released.syn_r === Out.header.syn_r,
		`game -> page ring still: w ${Out.header.syn_w} -> ${Out.released.syn_w}, r ${Out.header.syn_r} -> ${Out.released.syn_r}`,
	],
];

fs.writeFileSync(
	path.join(Here, "ring.json"),
	JSON.stringify({ at: new Date().toISOString(), ...Out }, null, 2),
);
for (const [Ok, Text] of Checks)
	console.log(`${Ok ? "PASS" : "FAIL"}  ${Text}`);
console.log(`\nreport: ${path.join(Here, "ring.json")}`);
Ws.close();
process.exit(Checks.every(([Ok]) => Ok) ? 0 : 1);
