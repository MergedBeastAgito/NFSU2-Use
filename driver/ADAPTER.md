# Agentic Adapter

A proposal for the package this crate needed and Agentic does not have.

**Landed.** `Crate/Adapter` (`agentic-adapter`) exists in the Agentic workspace,
this crate's `module.json` declares `"kind": "adapter"`, and
`agentic verify --module @agentic/nfsu-driver` reports it ready. What follows is
the proposal, with the two places it was wrong corrected against the
implementation: a specification that disagrees with the code it produced is worse
than no specification.

## The gap

Agentic ships capabilities (`browse`, `chat`, `design`, `skills`) and a
composition root that names the one concrete browser adapter. A third party with
a *new substrate* - another game, another protocol, another device - can copy
this crate, but there is nowhere to register it: `agentic manifests` lists what
the workspace compiles in, and nothing outside that workspace could say "I
provide this port" and be believed.

## The shape

One crate, `Crate/Adapter`, holding the trait and the one check a registry-free
composition root needs:

```rust
pub trait Adapter: Send + Sync {
    /// The port this adapter provides, as its own `module.json` names it.
    fn Port(&self) -> &'static str;
    /// One request, bounded by the caller's deadline.
    fn Call<'a>(&'a self, Request:&'a str, Call:Call) -> FutureBox<'a, Result<Value, Error>>;
    /// Release everything this adapter holds. Never fails.
    fn Dispose<'a>(&'a self) -> FutureBox<'a, ()>;
}
```

Two corrections to the first draft:

- **`fn Port()` took no receiver.** A trait method without one cannot be called
  on `dyn Adapter`, so the trait was not object-safe and could not be held as
  `Arc<dyn Adapter>` - which is the one thing the trait exists for. It takes
  `&self`.
- **The draft had no `Dispose`.** An adapter that owns a session, a socket or a
  device has to be able to let go of it, and a release that can fail is a
  release a caller skips. It is there, and it never fails.

`Error` is the adapter's own: a code - the word this crate's own `module.json`
declares - plus a message and what was observed. Agentic does not enumerate the
failures of a substrate it has never seen, which is why `Crate/Adapter`'s own
manifest names no errors and this one names three.

## An adapter crate declares itself in its own `module.json`

```json
{ "name": "@someone/thing", "kind": "adapter",
  "provides": { "port": "agentic.thing.drive", "errors": ["timeout"] },
  "requires": { "ports": ["agentic.browse"] },
  "artefact": { "entry": "Target/release/thing", "runtime": "rust>=1.88" },
  "state": "holds-transport" }
```

`kind: "adapter"` is the only new vocabulary. Everything else - `provides`,
`requires`, `artefact`, `state` - is the manifest Agentic already reads.

## Why a trait and not a registry

A registry is a global that makes load order observable; a trait is a value the
composition root already knows how to pass. Agentic's own rule - a capability
never names a concrete adapter - is what this preserves: the adapter is
constructed once, in the composition root, and handed on as `Arc<dyn Adapter>`.
The port the value reports is compared with the port the manifest declares by
`AgenticAdapter::Serves`, at the one place both are in hand.

## How a third-party manifest is found

Nothing is compiled into Agentic. `agentic manifests --adapter <dir>` and
`agentic verify --module <name>` read a `module.json` from a directory named by
`--adapter`, or by `AGENTIC_ADAPTER_PATH` (a path list) when the flag is absent.
The listing prints the directory each row was read from, so a row this binary
carries and a row it read are never confused.

`Documentation/Adapter.md` in the Agentic repository is the full statement: what
the reader accepts, what a foreign readiness report checks, and what it cannot.

## What this crate demonstrates

`@agentic/nfsu-driver` is an adapter in everything including the `kind` field: it
requires `agentic.browse`, provides `agentic.nfsu.drive`, keeps its JavaScript in
`Source/Script/Expressions/*.ts` compiled by `build.rs`, and is consumed by path
so Agentic is never modified. The migration was the one field.
