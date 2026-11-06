# NFSU2 Agito

Drives the browser port of Need for Speed: Underground 2 from the game's own
memory, through Agito's browser port. Two Rust crates do the work - a driver
that holds the port, and a wasm reader that reads the game's memory from inside
it - and a small JavaScript harness does the seeing and the showing.

---

## What this is

The port (`https://nfsu2.xyz/`) runs the original PC game recompiled to
WebAssembly. Three facts make it drivable, and each was measured rather than
assumed:

- **The state is addressable.** The port's own source names the addresses:
  `PlayersByNumber[0]` at `0x00d00000` in the wasm build, `car = [player+4]`,
  position at `car+0x60`, heading `+0x40c` (u16, 65536 a turn), signed speed
  `+0x430`, rpm `+0x440`, gear `+0x434`.
- **Input can be injected from the page.** `session.input.key(code, true)` is a
  single frame; re-asserted every 16 ms it holds a key. Measured: one call moved
  the car 0.2 m, the interval moved it 11 m in 4 s.
- **The route cue is visual.** The arrow is drawn, not published, so its screen
  position is read from a captured frame - one cyan cluster, filtered against
  the minimap corner and edge-hugging signage.

Because the page injects its own input, Agito needs no input verb: every step
here is one `Evaluate`.

---

## Layout

```
App/                        the application: what runs
  driver/                   the Rust crate (an extension beside Agito)
    Source/Library.rs       crate doc, modules, re-exports
    Source/Main.rs          the composition root: one adapter, one loop
    Source/Fn/              Decide, Cloudflare, Expressions, Gate
    Source/Struct/          State, Decision, Policy, Driver, Mode, Situation
    Source/Script/Expressions  one .ts file per expression the page runs
    build.rs                compiles those scripts into OUT_DIR/Script
    module.json             the manifest Agito's vocabulary already reads
    ADAPTER.md              the adapter surface, and what this crate does with it
  reader/                   the wasm reader: the car state, from inside the game
    Source/Library.rs       the module - no_std, no allocator, no stores
    .cargo/config.toml      the link flags, and why each is required
    Documentation/          the import contract, the offsets, the input mailbox
  nfsu-drive4.mjs           the JS driver: duty-cycle steering, hysteretic throttle
  nfsu-gui4.mjs             HUD, observations, decisions, power and pipe switches
  nfsu-lane.mjs             the pipeline lane: SEND → RECEIVE → APPLIED → TRACE
  nfsu-bus.mjs              in-page tables and subscriptions (no .jsonl)
  analyze_frame.py          arrow detection and road-band motion

Templates/                  data, read at runtime - never compiled in
  Observation.json          every field we can send, with its source address
  Questions.json            five lanes and seven typed question templates
  README.md                 how a template composes into a request

Documentation/
  StateCharts.md            the tick, the send/receive contract, the pipe into the game
  Vision.md                 why the schema is the product and NFSU2 is the unit test
  Design.md                 what the research settled: the envelope, the tiers, the
                            protocol, and where the design is still uncertain
  Memory.md                 the memory map, every address named by the port's source
  Workflow/                 how a card asks rather than guesses: Research.md is the
                            procedure and ask.mjs the driver
```

---

## Running it

The browser must be listening with the game tab open. Bind by target id:

```sh
WS=$(curl -s http://127.0.0.1:9222/json/version | python3 -c "import json,sys;print(json.load(sys.stdin)['webSocketDebuggerUrl'])")
cd driver && cargo build --release
./target/release/nfsu-driver --attach "$WS" --bind <target-id> --ticks 40 --target 45
```

The reader is a `cdylib` and nothing else: it builds to a `.wasm` module the page
instantiates beside the game's own, importing the game's shared memory.

```sh
cd reader && cargo build --release          # .cargo/config.toml carries the link flags
```

Three paths a run reads are resolved the same way - relative to the crate, then
the environment, then one absolute location: the corpus (`NFSU2_CORPUS`), the
dotenv file (`CLOUDFLARE_ENV_FILE`) and the reader's module (`NFSU2_READER`).

The JavaScript harness is installed into the page, not run by it:

```sh
node nfsu-bus.mjs < target-id > 9222
node nfsu-gui4.mjs < target-id > 9222
node nfsu-lane.mjs < target-id > 9222
node nfsu-drive4.mjs 600 150 < target-id > 9222
```

---

## The decision model

Cloudflare's Clef models are decision models, not chat models: they take a
`state` and a schema of typed `questions` and return a probability for every
allowed option. The chat-completions route rejects that shape with a 400, so the
request goes to `/ai/run/@cf/cloudflare/<name>` with
`{model, state, questions}`.

| alias        | model                       | price            |
| ------------ | --------------------------- | ---------------- |
| `clef-flash` | `@cf/cloudflare/clef-flash` | $0.038 / M input |
| `clef`       | `@cf/cloudflare/clef`       | $0.24 / M input  |

One measured call: `action: throttle` at 0.5348, confidence 2.12 of 3, 343 input
tokens and no output tokens - about $0.000013, which is why advice is throttled
by `--every` rather than asked for on every tick.

The token is read from `CLOUDFLARE_API_TOKEN`, or from a dotenv file -
`App/.env` beside the crate by default, `CLOUDFLARE_ENV_FILE` for another one -
and is never printed. The account id has no default either:
`CLOUDFLARE_ACCOUNT_ID` is required, because a hard-coded one would ship
somebody's account in a public repository.

---

## Power and pipe

Two switches live on the page, and any driver reads the same ones the person
clicks:

- **Power** - `user` (rules only, no model), `agito` (rules drive, the model
  argues), `model` (the model decides).
- **Pipe** - `on` / `off`. Broken, no request is made at all, and the lane says
  `pipe broken` rather than implying the model was consulted.

---

## Notes

- The overlays are DOM, never in-game drawing. Each panel takes the mouse over
  its own box and the text in it is selectable, so a reading can be copied; a
  click outside every panel still reaches the canvas, which is what keeps the
  game drivable while the panels are up.
- This page's CSP blocks `<style>` elements outright, so every panel is styled
  through `cssText`; a stylesheet silently does nothing here.
- Screenshots flash a WebGPU canvas, so frames come from a screencast stream.
- State and decisions live in in-page tables with subscriptions, not in a log
  file.
