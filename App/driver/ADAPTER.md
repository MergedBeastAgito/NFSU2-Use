# Agito Adapter

`agito-adapter` (`Crate/Adapter` in the Agito workspace) is the port a substrate
that workspace did not write implements. `agito-sdk` re-exports it, so a crate
outside the tree can implement it without naming a second Agito crate, and this
crate does: `Source/Struct/Driver.rs` implements `Adapter` for `Driver`,
`module.json` declares `"Kind": "Adapter"`,
`agito verify --module @agito/nfsu-driver` reports it ready, and the composition
root holds the value as `Arc<dyn Adapter>`. `Serves` answers whether the port it
reports agrees with the port that file names.

## The port

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

Three methods, and every one takes a receiver and borrows for `'a`, returning a
future bounded by that borrow. Object safety is why `Port` takes `&self`: a
trait method without a receiver cannot be called on `dyn Adapter`, so the trait
could not be held as `Arc<dyn Adapter>` - which is the one thing the trait
exists for.

`Dispose` is part of the port because an adapter that owns a session, a socket
or a device has to be able to let go of it, and a release that can fail is a
release a caller skips. It never fails.

`Error` is the adapter's own: a code - the word this crate's own `module.json`
declares - plus a message and what was observed. Agito does not enumerate the
failures of a substrate it has never seen, which is why `Crate/Adapter`'s own
manifest names no errors and this one names four.

## An adapter crate declares itself in its own `module.json`

```json
{
	"Name": "@someone/thing",
	"Kind": "Adapter",
	"Provide": { "Port": "agito.thing.drive", "Error": ["timeout"] },
	"Require": { "Port": ["agito.browse"] },
	"Artefact": { "Entry": "Target/release/thing", "RunTime": "rust>=1.88" },
	"State": "HoldTransport"
}
```

`Kind: "Adapter"` is the only new vocabulary. Everything else - `Provide`,
`Require`, `Artefact`, `State` - is the manifest Agito already reads.

## Why a trait and not a registry

A registry is a global that makes load order observable; a trait is a value the
composition root already knows how to pass. Agito's own rule - a capability
never names a concrete adapter - is what this preserves: the adapter is
constructed once, in the composition root, and handed on as `Arc<dyn Adapter>`.
`Serves` compares the port the value reports with the port the manifest
declares, at the one place both are in hand.

## How a third-party manifest is found

Nothing is compiled into Agito. `agito manifests --adapter <dir>` and
`agito verify --module <name>` read a `module.json` from a directory named by
`--adapter`, or by `AGITO_ADAPTER_PATH` (a path list) when the flag is absent.
The listing prints the directory each row was read from, so a row this binary
carries and a row it read are never confused.

`Documentation/Adapter.md` in the Agito repository is the full statement: what
the reader accepts, what a foreign readiness report checks, and what it cannot.

## What this crate does with it

`Source/Struct/Driver.rs` is the value a composition root holds:

- `Port()` answers `agito.nfsu.drive` - the string `module.json` declares, from
  one constant rather than three literals;
- `Call(Request, Call)` performs one tick: bootstrap the page on the first call,
  read the car, decide by the rules, ask `agito.lane.decide` when the policy
  says to - the lane selected from the corpus, the body composed there, the
  poster this crate supplies - hold the four keys, publish, and answer the state
  it read as
  `{ "tick", "car", "scene", "lane", "asked", "modelAdvice", "decision" }`;
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
as `Arc<dyn Adapter>`, and drives one call per tick. It names no expression, so
the six the crate sends - bootstrap, state, hold, cruise, publish and release -
live in `Source/Fn/Expressions.rs` and the loop cannot send a seventh.

The model is a port the adapter holds, not code it carries: `Driver::New` takes
an `Option<Arc<dyn DecidePort>>`, and `Source/Main.rs` builds it from the corpus
(`Source/Fn/Corpus.rs`: the directory and the run's ceiling - the lanes, their
conditions and their templates are the corpus's), the transport
(`Source/Fn/Poster.rs`: the `Poster` `agito-lane` never implements) and nothing
else. A run without a model - or with a corpus that does not load - passes
`None` and drives on the rules alone; the startup line says which of the two
happened.

## What this crate is

`@agito/nfsu-driver` is an adapter in everything including the `kind` field: it
requires `agito.browse` and `agito.lane.decide`, provides `agito.nfsu.drive`,
keeps its JavaScript in `Source/Script/Expressions/*.ts` compiled by `build.rs`,
and is consumed by path so Agito is never modified.
