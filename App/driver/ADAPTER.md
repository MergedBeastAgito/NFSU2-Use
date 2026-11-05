# Agentic Adapter

A proposal for the package this crate needed and Agentic does not have.

**Landed, in two halves.** `Crate/Adapter` (`agentic-adapter`) exists in the
Agentic workspace, `agentic-sdk` re-exports its port so a crate outside the tree
can implement it, and this crate's `Driver` does implement it: `module.json`
declares `"kind": "adapter"`, `agentic verify --module @agentic/nfsu-driver`
reports it ready, and the value is held as `Arc<dyn Adapter>`, so `Serves`
answers whether it agrees with that file. What follows is the
proposal, with the two places it was wrong corrected against the implementation:
a specification that disagrees with the code it produced is worse than no
specification.

## The gap

Agentic ships capabilities (`browse`, `chat`, `design`, `skills`) and a
composition root that names the one concrete browser adapter. A third party with
a _new substrate_ - another game, another protocol, another device - can copy
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
{
	"name": "@someone/thing",
	"kind": "adapter",
	"provides": { "port": "agentic.thing.drive", "errors": ["timeout"] },
	"requires": { "ports": ["agentic.browse"] },
	"artefact": { "entry": "Target/release/thing", "runtime": "rust>=1.88" },
	"state": "holds-transport"
}
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

## What this crate does with it

`Source/Struct/Driver.rs` is the value a composition root holds:

- `Port()` answers `agentic.nfsu.drive` - the string `module.json` declares,
  from one constant rather than three literals;
- `Call(Request, Call)` performs one tick: bootstrap the page on the first call,
  read the car, decide by the rules, ask `agentic.lane.decide` when the policy
  says to - the lane selected from the corpus, the body composed there, the
  poster this crate supplies - hold the four keys, publish, and answer the state
  it read as `{ "tick", "car", "scene", "lane", "asked", "modelAdvice",
  "decision" }`;
- `Dispose()` releases the keys and then the page, once, and never fails - a
  held throttle survives the process that set it, so the release is the one step
  that must not be skipped. Its deadline is built when it runs, not when the
  adapter was composed: a call fixed at construction has already expired by the
  end of a long run, and the release it carried would be refused by a page that
  is still answering.

`Request` is this crate's own encoding, in `Source/Struct/Policy.rs`:
`{"cue":0.42,"target":45,"model":null,"every":10,"threshold":0.6}`. A key the
policy does not know is refused with `request-invalid` - the one code in
`provides.errors` that is not the browser's - rather than silently ignored; an
absent key takes its default; empty text means "the policy this adapter was
constructed with".

`Source/Main.rs` is the composition root: it constructs `Driver` once, holds it
as `Arc<dyn Adapter>`, and drives one call per tick. It names no expression any
more, so the five the crate sends live in one place and the loop cannot send a
sixth.

The model is a port the adapter holds, not code it carries: `Driver::New` takes
an `Option<Arc<dyn DecidePort>>`, and `Source/Main.rs` builds it from the corpus
(`Source/Fn/Corpus.rs`: the directory and the run's ceiling - the lanes, their
conditions and their templates are the corpus's), the transport
(`Source/Fn/Poster.rs`: the `Poster` `agentic-lane` never implements) and nothing
else. A run without a model - or with a corpus that does not load - passes `None`
and drives on the rules alone; the startup line says which of the two happened.

## What this crate demonstrates

`@agentic/nfsu-driver` is an adapter in everything including the `kind` field:
it requires `agentic.browse`, provides `agentic.nfsu.drive`, keeps its
JavaScript in `Source/Script/Expressions/*.ts` compiled by `build.rs`, and is
consumed by path so Agentic is never modified. The migration was the one field.
