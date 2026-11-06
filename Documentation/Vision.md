# Vision: one schema, many targets

An **Agito schema** describes what can be observed and what can be decided. A
**custom library** - written against one target's own source - turns that schema
into **insertions** that change the target. The schema is the contract, the
library is the adapter, and the addresses live in the library.

---

## NFSU2

Small, closed, fast, fully understood. One loop at 120 Hz, one car, one memory
space, one visual cue. The loop is observe, decide, insert, observe the effect.

The insertions are keys and memory writes, and the library is written from the
port's own C source: it knows `0x00d00000` is the player array and `car+0x430`
is speed. None of that belongs in the schema. All of it belongs in the adapter.

## TimeLine

Large, open, slow, mostly human. An Agito-and-human publishing operation: news
extracted from Twitter and Facebook, modelled as interactions, hosted on
SpacetimeDB. It has its own network layer that is not Agito at all. An insertion
there is a draft, a queue entry or a published item.

Nothing about it is a frame: no frame rate, no memory map, no single loop.

---

## The split

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
