# `nfsu-reader`

The game's car state, read from **inside** the game's own WebAssembly memory.

A 516-byte `cdylib` the page instantiates beside the game's instance. It imports
`env.memory` - the game's memory, which is a shared `WebAssembly.Memory` - and
returns position, heading, speed, rpm and gear as numbers, so a tick is a
handful of wasm calls instead of an `Evaluate` round trip through a string of
JavaScript.

- `Source/Library.rs` - the module. `#![no_std]`, no allocator, no stores, no
  data segment: the imported memory _is_ the game's heap.
- `.cargo/config.toml` - the link flags (`--import-memory`, `--shared-memory`,
  `--max-memory`, `--no-entry`) and why each is required.
- `Documentation/Reader.md` - the import contract, the offsets, the 64-bit
  transport limit, and panic containment.
- `Documentation/Input-Ring.md` - the input mailbox, measured: which ring the
  page writes, which the game writes, and why a second producer is not safe.
- `Test/attach.mjs` - attaches to the live game, proves twenty consecutive ticks
  equal to the page's own reader, proves a panic is contained, soaks 60 s.
- `Test/ring.mjs` - measures the input mailbox against the running game.

```sh
cargo build --release
node Test/attach.mjs --soak 60
node Test/ring.mjs
```

`Test/attach.mjs` writes `Test/report.json` and exits non-zero if any claim
fails. It needs `wasm-objdump` (`brew install wabt`) and the game running in a
tab reachable over CDP (default `127.0.0.1:9222`).
