# Vision: one schema, many targets

NFSU2 is not the product. It is the smallest complete proof that the schema works.

---

## What is actually being built

An **Agentic schema** describes what can be observed and what can be decided. A
**custom library** - written against one target's own source - turns that schema
into **insertions** that change the target. The schema is the contract; the
library is the adapter; the target is an example.

For NFSU2 the insertions are keys and memory writes, and the library is written
from the port's own C source: it knows `0x00d00000` is the player array and
`car+0x430` is speed. None of that belongs in the schema. All of it belongs in
the adapter.

---

## The two examples, and why both matter

**NFSU2** - small, closed, fast, fully understood. One loop at 120 Hz, one car,
one memory space, one visual cue. It proves the loop: observe, decide, insert,
observe the effect. It is the unit test.

**TimeLine** - large, open, slow, mostly human. An agentic-and-human publishing
operation: news extracted from Twitter and Facebook, modelled as interactions,
hosted on SpacetimeDB. It has its own network layer that is not Agentic at all -
and could be. It proves the schema survives a target with no frame rate, no
memory map and no single loop: a publishing pipeline where "insertion" means a
draft, a queue entry or a published item.

If the schema only fits NFSU2 it is a game harness. If it fits both, it is a
contract.

---

## The rule that keeps them apart

- **Schema** - observation envelope, typed questions, lanes, decisions, budgets.
  No addresses, no selectors, no platform names.
- **Adapter** - one per target, written against that target's source, declaring
  what it provides and requires. It knows the addresses; it never defines the
  shape.
- **Insertion** - the only thing an adapter may do: a bounded influence the
  target consumes on its own terms. A key, a memory slot, a DOM event, a draft.

A schema that names a memory address is an adapter wearing the schema's clothes.

---

## Why SpacetimeDB

Every component registers what it provides and requires as rows; reducers infer
what is satisfiable, what is blocked, and in what order things may start. The
plan is a materialised explanation of what is possible; guarded intents are the
authority for what may happen next. Declarations cannot establish trust,
executable code or physical effects - only acknowledgements from the target can.
