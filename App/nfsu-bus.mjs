// The in-page bus: bounded tables in memory, with subscriptions.
//
// This replaces the `.jsonl` traces as the source of truth. A file is a log you
// read afterwards; a table is state a consumer can subscribe to and act on now,
// which is the shape the Agentic toolkit already uses (SpacetimeDB: tables,
// subscriptions, one channel between modules). Any adapter - a JS driver, a Rust
// crate through Evaluate, the overlay - reads and writes the same tables.
//
// Three tables, each fine-grained for its own consumer:
//   observation  what the game reported this tick
//   decision     what the driver chose, and why
//   exchange     what was sent to a decision model and what came back
//
// Bounded on purpose: a bus that grows without limit is a memory leak wearing a
// ring buffer's clothes. The default depth is 600 rows per table.
//
// Usage: node nfsu-bus.mjs <targetId> [port] [--off] [--depth N]
import fs from "node:fs";

const [TARGET, PORT = "9222", FLAG = "", DEPTH_ARG] = process.argv.slice(2);
const DEPTH = Number(DEPTH_ARG || 600);

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
	const res = await send("Runtime.evaluate", { expression: x, returnByValue: true, awaitPromise: true });
	if (res.exceptionDetails) throw new Error(res.exceptionDetails.text);
	return res.result.value;
};

if (FLAG === "--off") {
	console.log(JSON.stringify({ removed: await ev("(()=>{const had=!!window.__bus;delete window.__bus;return had?1:0})()") }));
	ws.close();
	process.exit(0);
}

const out = await ev(`(()=>{
  if (window.__bus) return JSON.stringify({ installed: 'already', tables: Object.keys(window.__bus.tables) });
  const DEPTH = ${DEPTH};
  const Subs = { observation: new Set(), decision: new Set(), exchange: new Set() };
  const Tables = { observation: [], decision: [], exchange: [] };
  const Counts = { observation: 0, decision: 0, exchange: 0 };

  window.__bus = {
    tables: Tables,
    publish(table, row) {
      if (!Tables[table]) return 0;
      Tables[table].push(row);
      if (Tables[table].length > DEPTH) Tables[table].shift();
      Counts[table]++;
      for (const Fn of Subs[table]) { try { Fn(row) } catch (e) {} }
      return Tables[table].length;
    },
    subscribe(table, Fn) {
      if (!Subs[table]) return () => {};
      Subs[table].add(Fn);
      return () => Subs[table].delete(Fn);
    },
    latest(table) { const T = Tables[table]; return T && T.length ? T[T.length - 1] : null },
    stats() { return { depth: DEPTH, counts: { ...Counts }, sizes: { observation: Tables.observation.length, decision: Tables.decision.length, exchange: Tables.exchange.length }, subscribers: { observation: Subs.observation.size, decision: Subs.decision.size, exchange: Subs.exchange.size } } },
  };
  return JSON.stringify({ installed: true, tables: Object.keys(Tables), depth: DEPTH });
})()`);

console.log(JSON.stringify({ page: page.url, target: TARGET, ...JSON.parse(out) }, null, 2));
ws.close();
