# The input mailbox

What a second writer would have to join if injection ever moved off
`session.input.key`, and why it has not.

Everything here is the port's own: `runtime/web/input_proto.h` is the struct the
game compiles, `web/src/input/proto.ts` is the page's copy of the same numbers
(`web/tools/check-proto.mjs` compares the two files), and the measurements are
from the running game.

---

## Where it is

The mailbox is **not at a fixed address**. The game allocates it and registers
it as `HostKind.Input` (3), so a reader asks for the address:

```js
const { ptr, size } = await window.__nfsu2.session.proc.registered(3);
const words = new Int32Array(
	window.__nfsu2.session.proc.memory.buffer,
	ptr,
	64,
);
```

Measured in one session: `ptr = 0x29e25c80`, `size = 25664`, magic `0x4e494e50`
(`'NINP'`), version `2`. The magic and version are the check that the block is
the mailbox and not a stale registration.

---

## The block

`HostMailbox`, 25664 bytes, `_Static_assert`ed by the game itself against the
offsets below.

- **`+0` .. `+255`** - 64 header words (`uint32`, atomic).
- **`+256`** - the controller's name, 64 bytes, under `IM_PAD_SEQ`.
- **`+320`** - key names by DIK code, 256 × 16 bytes.
- **`+4416`** - the event ring, 1024 × 16 bytes: **page → game**.
- **`+20800`** - the synthetic ring, 256 × 16 bytes: **game → page**.
- **`+24896`** - six local-player pad records, 32 words each.

The arithmetic closes exactly, which is how the layout is checked rather than
assumed: `4416 + 1024×16 = 20800`, `20800 + 256×16 = 24896`,
`24896 + 6×128 = 25664`.

---

## The header words that matter

- **`IM_MAGIC = 0`**, **`IM_VERSION = 1`**, **`IM_SIZE = 2`** - `0x4E494E50`,
  `2`, `25664`.
- **`IM_EV_W = 4`** - events written, _by the page_. The game's input thread
  waits on this word (`__builtin_wasm_memory_atomic_wait32`).
- **`IM_EV_R = 5`** - events consumed, _by the game_.
- **`IM_EV_DROP = 6`** - events the page dropped because the ring was full.
- **`IM_ATTACHED = 7`** - who owns the event ring: `0` nobody, `1` the page, `2`
  the game's own autotest translation. Claimed by compare-exchange from 0.
- **`IM_SYN_W = 8`** - synthetic requests written, _by the game_.
- **`IM_SYN_R = 9`** - synthetic requests consumed, _by the page_.
- **`IM_KEYS_HELD = 25`**, **`IM_EVENTS = 26`** - what the game says it has.

---

## The two rings run in opposite directions

This is the part that is easy to get backwards, and the card had it backwards:
`main.js` does not write the ring at `d+20800`.

**Page → game, at `d+4416`.** The page pushes a 16-byte `InEv` and publishes it
with `Atomics.store(IM_EV_W, w+1)` followed by `Atomics.notify`
(`web/src/input/index.ts`, `push`). The game's input thread copies
`ev[r & 1023]`, advances `IM_EV_R`, and handles the record. This is the ring
`session.input.key(code, down)` writes into, and the only ring that moves during
normal play.

**Game → page, at `d+20800`.** The game writes a `SynReq` and publishes it with
`Atomics.store(IM_SYN_W, w+1)` (`runtime/web/input_web.c`, its `cs_push`
counterpart). The page drains it once per animation frame
(`web/src/input/index.ts`, `drainSynth`) and turns each request into a synthetic
DOM key event, which then re-enters the page→game ring through the ordinary
listener. It is the game asking the page to type, not the page typing.

Measured, holding `ArrowUp` for a second:

```
page -> game   w 10488 -> 10489 -> 10534, r 10488 -> 10534, dropped 0
game -> page   w 0 -> 0, r 0 -> 0
```

The event ring advanced 46 records and the game consumed every one. The
synthetic ring never moved: nothing in the session asked the page for a
synthetic event.

---

## The records

`InEv`, 16 bytes (`_Static_assert(sizeof(InEv) == 16)`):

- **`+0`** - type (`IEV_KEY_DOWN = 1`, `IEV_KEY_UP = 2`, `IEV_KEY_TAP = 3`,
  `IEV_MOUSE_*`, `IEV_FOCUS`, `IEV_RELEASE_ALL`, `IEV_TEXT`, ...)
- **`+1`** - DIK scan code
- **`+2`** - virtual key code
- **`+3`** - flags (`IKF_ALT`, `IKF_CTRL`, `IKF_SHIFT`, `IKF_SYNTH`)
- **`+4`** - character count, at most 3; **`+5` .. `+7`** - those characters
  (cp1252)
- **`+8`** - `i32 a`; **`+12`** - `i32 b` (the type decides what they mean)

`SynReq`, 16 bytes, the same shape with `type`, `dik`, `mods`, a `ch` word at
`+4`, and two `f32` at `+8` and `+12`.

---

## Why a second writer is not proven safe

The ring has **one writer by contract**, and the game enforces the contract with
the `IM_ATTACHED` compare-exchange. Two mechanisms make a second producer unsafe
rather than merely unusual:

- **The producer is not a CAS loop.** Both `push` implementations do
  `w = load(IM_EV_W)`, write the slot at `w & 1023`, then `store(IM_EV_W, w+1)`.
  Two producers that read the same `w` write the _same slot_ and one event is
  lost - silently, because the second `store` also succeeds.
- **The consumer copies without a seqlock.** The input thread reads the record
  as a plain struct copy and only then advances `IM_EV_R`. A producer that is
  halfway through writing that slot - and `cs_push` zeroes the record with
  `memset` before filling it - gives the game a half-written event.

Neither is a bug: the design is a single-producer queue, and it is correct as
one. Joining it would mean either taking the `IM_ATTACHED` claim away from the
page (and losing `session.input.key` with it), or adding a sequence number the
consumer does not read - which is a change to the game, not to the caller.

So injection stays on `session.input.key`: one call per frame per key, the
proven path, with the game's own producer doing the publishing.
