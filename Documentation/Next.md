# Next

What is still open in this application, and what was learned the hard way. Every
line here is a thread that was started and deliberately left; nothing in this
file is running.

---

## The steering gap

**The car has no lateral input in free roam.** It holds a heading and a target
speed until geometry intervenes, then the recovery rules turn it out of the wall.

Measured over one five-minute run: 1,497 ticks, `cruise` on 1,383 of them, and
the cue was absent on 1,416. The only lateral observation in the corpus is
`route_cue_x`, which is the route arrow's screen position, and the arrow only
exists when a route is set. So no steering question can be asked before impact -
the 114 lateral actions in that run (`steer left` 48, `brake` 33,
`reverse left` 33) all fired after the car was already blocked, stalled or
creeping (35 non-moving ticks).

What would fix it: a lateral reading that exists without a route. Candidates are
the road band from a captured frame (already ingested as `road_band_motion`, but
it is a scalar and carries no direction), the car's own yaw rate against the
heading it is being held at, or the minimap. None of these has been measured yet.

---

## Lane selection favours a question that cannot matter

The corpus priority is `recover > race > corner > context > drive`. A nearby
ghost car matches `context` / `traffic.ghost`, and in ghost free roam contact
cannot happen, so the question has no consequence. Over one five-minute run the
split was `context` 150, `drive` 65; over another, `context` 168, `drive` 33,
`corner` 2.

The cost is not the money ($0.0033 per five minutes) but the attention: the
model is asked about traffic instead of about the road. This is a corpus change
in `Templates/Questions.json`, not a flag.

---

## The reader is a crate, not a reader

`App/reader` builds a 516-byte `cdylib` that reads the car from inside the game's
own shared memory, and nothing in the driving path uses it. The artifact is built
and its path is resolved at startup, but no run instantiates it.

The tick currently reads state through one `Evaluate` round trip of page-side
JavaScript (`__readCar`). Wiring the wasm module would make a tick a handful of
calls into the game's own heap instead of a string across CDP.

Build it with `cargo build --release` in `App/reader` - `.cargo/config.toml`
carries the link flags and the reason for each.

---

## Predicates and composable questions

The intent is that a predicate over the state selects a small typed question,
and the questions compose - rather than every tick asking one broad question
about everything.

That work is blocked on the state inspection above: a predicate can only be
written over a field that exists. `Templates/Observation.json` names 19 fields
today, of which 3 are frame-derived and 16 are memory reads at known offsets.
How much more the port exposes has not been surveyed.

---

## The harness is JavaScript; the project runs Rust

`nfsu-gui4.mjs`, `nfsu-drive4.mjs`, `nfsu-lane.mjs` and `nfsu-bus.mjs` are ESM.
Agentic does not run TypeScript - its `Test` script is `cargo test --workspace`,
and its only `.ts` files are page expressions baked into crates by `build.rs`.
Node 22.23.0 does execute `.ts` directly by type stripping, so a port needs no
build step, but it inherits nothing from the SDK.

---

## What the overlay now is

- `DECISIONS` is a three-column panel: candidates and the driving line, then
  `MODE` and `DRIVER`, then `SENSE`, `WIRE` and `TUNING`.
- `WIRE` is four typed boxes - state (green), questions (cyan), verdict (red),
  history (grey) - built once and written into, never rebuilt. A pooled node
  survives a request; that is the property the panel exists for.
- The `TUNING` knobs are the page harness's own and do not steer the Rust
  driver, which takes `--target`, `--every` and `--threshold`. The label says so.

---

## Traps worth keeping

- **The whole overlay is one template literal.** A backtick anywhere inside it -
  including in a `//` comment - ends the template early and breaks the file. A
  regex written `/\s+/` inside it arrives at the page as `/s+/`.
- **Never clear a container that holds pooled nodes.** `textContent = ''`
  detaches them, and a pool whose nodes are out of the document can never put
  them back. Toggle a placeholder node instead, and re-attach when
  `parentNode !== container`.
- **The gate's threshold belongs to the model's own distribution.** Measured
  probabilities clustered 0.43-0.65 with a median of 0.565, so a 0.60 gate
  discarded 63% of answers. At 0.50, 73-79% were obeyed.
- **A tick that ends releases its keys, and the car coasts.** After a run the
  car moves a further 28-51 m with `keysHeld: 0` - that is a stop, not a wall.
