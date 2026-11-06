# The memory reader

A `cdylib` compiled to `wasm32-unknown-unknown` that the page instantiates
beside the game's own instance. It imports the game's memory and returns the car
state as numbers, so a tick stops being an `Evaluate` round trip through a
string of JavaScript.

---

## The import contract

The module imports exactly one thing: `env.memory`.

```
Import[1]:
 - memory[0] pages: initial=16 max=65536 shared <- env.memory
```

**The memory must be shared.** The port builds it with
`new WebAssembly.Memory({ shared: true })` (`web/src/host/memory.ts`,
`allocMemory`), because the game's threads are workers over one
`SharedArrayBuffer`. An import that does not declare `shared` is refused when
that memory is offered to it - which is why the build carries `--shared-memory`
and `+atomics`, and why `--max-memory` is 4 GB: a declared maximum smaller than
the game's own cannot be matched.

The declared minimum of 16 pages is the linker's default and is satisfied by any
running game; the memory it is given is 32768 pages (2 GB) on a machine that
accepts the guest heap's window.

---

## What it may never do

**It may not allocate.** The imported memory _is_ the game's heap: an allocator
here would hand out blocks the game is still using. The crate is `#![no_std]`
with no `#[global_allocator]`, and the artefact is checked for the consequences:

- no `memory.grow` - the module cannot enlarge the game's memory;
- no `__rust_alloc`, `__rust_dealloc`, `dlmalloc` or `malloc` symbol;
- **no data segment.** This is the one that is easy to miss: wasm applies data
  segments _at instantiation_, at an address the linker chose, so a single
  string literal or initialised static would overwrite the game's low memory
  before the first reading. The module has no data section and no globals at
  all, which also means no `__stack_pointer`: the functions do not spill.

**It may not write.** There are zero store instructions in the module. A reading
is returned, never stored - see the transport note below for what that costs.

**It may not trap on a stale pointer.** Every address is checked against the
memory's current size before it is read (`memory_size(0)`, one instruction), and
the pointer chain is bounded at each step, so a world that has not given a car
answers "no car" rather than reading someone else's memory or throwing into the
page's tick loop.

---

## The offsets

All from the port's own runtime, so this is a read of a known layout rather than
a search through two gigabytes.

- **`0x00d00000`** - `PlayersByNumber[0]`, the `Player*` (`runtime/career.c`,
  `G_PLAYER1`).
- **`Player + 4`** - that player's `Car*`.
- **`Car + 0x60`** - `float[3]` position, metres, world space.
- **`Car + 0x40c`** - heading as a `u16`: 65536 to a turn, so degrees are
  `u16 * 360 / 65536` (`runtime/roam_game.c`, `local_state`).
- **`Car + 0x430`** - the physics' speed, m/s, signed. `+0x42c` is the one the
  dashboard shows; the two are written together but are not the same field.
- **`Car + 0x434`** - gear (`i8`); **`+0x440`** - engine speed (`f32`).
- **`Car` block size `0x9e0`** - `local_state` refuses to read a car it cannot
  see that much of (`ok(car, 0x9e0)`), and so does this module.

A reading is **NaN** when there is no car. A car parked at the origin and a
world that has not given one are different answers, and only one of them is a
number.

---

## The transport, and why there are several calls

A returned value is the whole transport budget, and that budget is **64 bits per
call**. It cannot be widened:

- Rust's wasm ABI returns a struct through a hidden pointer - measured: a
  `#[repr(C)]` struct of five `f32` and a `u128` both compile to `(i32) -> nil`,
  i.e. the caller passes an address, which is a write into the game's memory;
- the one 128-bit value the ABI does have, `v128`, is refused by the JavaScript
  API at the call site:
  `TypeError: type incompatibility when transforming from/to JS`.

So the surface is one value per call:

- `read_state() -> i64` - position x and y, as two `f32` bit patterns in one
  value (low half x, high half y). The two that change on every frame.
- `read_position_z() -> f32`
- `read_heading() -> f32` - degrees
- `read_speed() -> f32` - m/s, signed
- `read_rpm() -> f32`
- `read_gear() -> i32`
- `read_car() -> u32` - the validity test; 0 means the rest is NaN

Heading is exact, not approximate: `360 / 65536` is representable in `f32`
(65536 is a power of two) and the product is at most 22 bits, so the module and
a `f64` page compute the identical number from the same `u16`.

**The consequence for a caller:** a full reading is six calls, and the game's
physics runs on other threads, so the calls do not see one instant. A caller
that needs a consistent snapshot reads the game twice and accepts only the ticks
where both readings agree.

---

## A panic is contained

`#[panic_handler]` calls `unreachable()`, and the crate is built with
`panic = "abort"`. A panic is therefore a wasm trap: the page catches a
`RunTimeError`, the module's other exports still work, and the game's own
instance - a different module - never sees it. `panic_probe()` exists so that
the containment can be watched on a live game rather than taken on faith.

---

## Build

```sh
cargo build --release          # .cargo/config.toml carries the link flags
```

The build writes `target/wasm32-unknown-unknown/release/NfsuReader.wasm` (516
bytes), which is the path the driver resolves by default: relative to
`App/reader`, with `NFSU2_READER` naming another module and
`/usr/local/share/nfsu2/NfsuReader.wasm` as the last resort.

---

## What it does not do

It does not inject input. Input stays on `session.input.key`, the path that is
proven to move the car; the ring a second writer would have to join is described
in [Input-Ring.md](Input-Ring.md), together with the reason it is not safe to
join yet.

---

## Reading the artefact

The properties above are visible in the module itself, which is what keeps them
claims about this build rather than about the source: `wasm-objdump -x` shows one
import and no globals, `-h` shows no `Data` section, and
`wasm-objdump -d | grep -cE '\.store|memory.grow'` is 0. `Committed()` is the
runtime half of the same statement - every read is bounded against
`memory_size(0)` and the pointer chain is bounded at each step.
