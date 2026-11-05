chars: 14809
---
New
Computer
Automations
Artifacts
Plugins
Projects
Recents
Nikola Hristov
Add your team
DELO
Answer
Links
Images
Share
Today 2:26 AM
2:26 AM

New question, following from your earlier answers.

The target is a web-based game served at a public URL and executed entirely in the browser: a recompiled PC game as WebAssembly, with a WebGPU canvas. There is no native binary and no server-side state. Everything I want to do happens in the page's own realm.

Given that, answer concretely:

If I want a third-party pack (reader + protocol + actuator) to run inside that page without the page's cooperation, what are the realistic vectors in 2026, ordered by what they can reach: userscript manager, bookmarklet, extension content script in the main world, extension with chrome.debugger, CDP. For each, say whether it can read the game's WebAssembly linear memory and whether it can inject input the game treats as trusted.

A second WebAssembly module cannot read another module's linear memory unless that memory is handed to it. Given the game's memory is exported on a JavaScript object, what is the minimal handoff that lets a Rust wasm module read absolute addresses — passing the Memory as an import, copying a window, or a SharedArrayBuffer view? What breaks with each when the game's memory grows?

If the same pack must also run under SpacetimeDB as the coordination layer, which parts belong in the database (authority, ordering, budget) and which must stay in the page (physical reads, physical writes)? Give the boundary rule in one sentence.

What is the smallest viable conformance suite for such a pack — the tests that would catch a regression in memory layout, in the input path, and in the suggestion protocol?

Read less
Finished
5s

For your target, the simplest architecture is a main-world JavaScript loader that obtains the game’s exported WebAssembly.Memory, instantiates a build-matched Rust reader against that memory, and applies validated modifiers through the game’s existing input API. Use debugger privileges only when you need browser-generated input or more reliable instrumentation—not merely to share memory.

The key distinction is access: executing in the page’s realm gives you access to reachable JavaScript objects; debugger attachment additionally provides browser-level instrumentation and input.

1. Realistic injection vectors, ordered by reach

This ordering describes increasing instrumentation privilege, not necessarily better access to an already exposed memory object.

Vector	Can read the game’s Wasm memory?	Can generate trusted keyboard/pointer input?	Practical conditions
Bookmarklet	Yes, if the memory object is reachable and execution is permitted	No, not through DOM event dispatch	Manual invocation; page/browser restrictions may prevent loading or compiling the pack
Userscript manager	Yes, if configured for page/main-world access or using a suitable bridge	No, not through ordinary userscript APIs	Manager-specific execution worlds; automatic reinjection can improve reliability
Extension main-world content script	Yes, if the exported object is reachable in that frame	No, not through ordinary script execution	Requires "scripting" plus host permission or "activeTab"; use world: "MAIN"
Extension with chrome.debugger	Yes, through page-context evaluation and a reachable memory reference	Yes, through supported CDP Input.* methods	Requires debugger permission and attachment; ordinary dispatchEvent() remains untrusted
External CDP host	Yes, through the relevant execution context and memory reference	Yes, through CDP Input.*	Requires an enabled debugger connection and selection of the correct target/frame

Chrome’s scripting API explicitly distinguishes MAIN, which shares the host page’s JavaScript environment, from the extension’s ISOLATED world. An isolated content script cannot simply access the page’s game globals.
developer.chrome

chrome.debugger is an alternative transport for the remote debugging protocol. For this use case, it belongs in the same instrumentation class as external CDP, although the extension API exposes a restricted set of protocol domains.
developer.chrome

Two qualifications matter

First, “exported” must mean reachable. A game may export memory from its Wasm instance but keep that instance inside a private JavaScript closure. That is different from exposing something such as:

js
globalThis.game.instance.exports.memory

For the object you describe as accessible, the first three vectors do not need debugger privileges to obtain it.

Second, trusted input and accepted input are different:

dispatchEvent(new KeyboardEvent(...)) does not become trusted merely because it runs in the main world.

Calling the game’s own input function may work perfectly without generating any DOM event.

CDP Input.dispatchKeyEvent uses browser input synthesis; wrapping dispatchEvent() in CDP evaluation does not. The browser controls Event.isTrusted, not the injecting script.
developer.mozilla
+1

If the game checks isTrusted in its keyboard handler, ordinary synthetic events can fail. If your actuator calls the game API downstream of that handler, that event check may be irrelevant. Verify the actual accepted path rather than requiring trusted input universally.

All five vectors still need a host call or scheduling mechanism to execute the pack. Installing a Wasm instance beside the game does not attach it to the game’s tick.

2. Minimal memory handoff: import the actual Memory
Recommended live reader

Assuming the exported object is reachable:

js
const gameMemory = globalThis.game.instance.exports.memory;

if (!(gameMemory instanceof WebAssembly.Memory)) {
  throw new Error("Game memory is unavailable");
}

const { instance: reader } = await WebAssembly.instantiate(
  readerBytes,
  {
    env: {
      memory: gameMemory
    }
  }
);

// Illustrative export: reads a game-memory byte offset.
const speed = reader.exports.read_f32(speedAddress);

Use the import module/name actually declared by the reader. The imported memory’s sharedness, address type and limits must be compatible with the module’s declaration.

The module now reads byte offsets in the same linear memory. It does not need JavaScript to copy each field. Host-supplied memory and callable exports are standard Wasm integration mechanisms.
developer.mozilla

Compare the three handoffs
Handoff	What the reader sees	What growth breaks
Import the actual WebAssembly.Memory	Live game memory, at the same byte offsets	Wasm reads continue using the same memory; cached JavaScript views may become invalid
Copy a memory window into reader-owned memory	A snapshot, usually rebased to offset zero	Existing snapshot stays valid, but becomes stale; the next copy must refresh its source view and bounds
Supply a SharedArrayBuffer/view	Shared bytes only if the game already uses compatible shared memory and the bridge is designed for them	Old shared-buffer views retain their old length; refresh to access newly grown regions

For ordinary, non-shared memory, memory.grow() detaches the previous ArrayBuffer, including on grow(0). Recreate Uint8Array, DataView and other JavaScript views from memory.buffer after growth. For shared memory, old buffers are not detached, but their length is not updated; obtaining memory.buffer again exposes the expanded range.
developer.mozilla

Why a SharedArrayBuffer is not a substitute for the memory object

A core Wasm memory import expects a WebAssembly.Memory, not a typed-array view. You cannot construct a new WebAssembly.Memory around an arbitrary existing SharedArrayBuffer.

Consequently:

If the game uses ordinary memory, you cannot turn it into shared memory after the fact.

If it already uses shared memory, pass the memory object where the runtime permits, rather than treating a buffer view as a Wasm memory import.

Same-thread sharing of ordinary memory between instances does not require cross-origin isolation.

Cross-worker shared-memory operation has additional shared-memory and isolation requirements.
developer.mozilla
+1

Two implementation hazards beyond growth

Bounds are not layout compatibility. An address can remain in bounds after a game update but contain a different field. Pin the reader to a host build and memory-map hash.

Importing memory grants writing capability too. A Rust reader can accidentally write stack frames, initialised data or allocator metadata into game memory. Check the final Wasm artifact for stores, active data segments, startup code and growth—not just the Rust source or --import-memory.

For the smallest auditable reader, use scalar exports, no allocation and no linear-memory writes. Keep protocol buffers and actuator bookkeeping out of game-owned memory.

3. SpacetimeDB boundary

Boundary rule: SpacetimeDB authorises, orders and accounts for expiring modifier targets; the page reads physical state, revalidates applicability and performs physical input writes without waiting for a database round trip.

Database	Page
Component/provider registration	Locate the exported memory and input API
Authority leases and fencing epochs	Read memory and capture/derive frame observations
Policy and question-pack revisions	Maintain local monotonic time and observation sequence
Suggestion admission and arbitration	Check current state, freshness and modifier preconditions
Per-channel ordering and deduplication	Smooth, clamp and rate-limit modifiers
Global budget reservation/accounting	Enforce local hard limits and watchdog expiry
Audit records and acknowledgement storage	Write through the existing game input boundary
Model work dispatch	Report what was actually observed or written

Keep a local budget ceiling as well as central accounting: a disconnected page must not become unlimited.

Local and remote model workers can subscribe to observations/work items and submit suggestions. Do not make outbound model calls from reducers; reducers cannot perform those network requests.
spacetimedb

A database transaction can record that a modifier was authorised. It cannot atomically commit a change into the browser’s running game. Report written_to_input_boundary separately from consumption_observed; if consumption is not observable, do not claim it.

4. Smallest viable conformance suite

Use one synthetic Wasm host fixture for deterministic tests, plus one short test against the real page. A mock alone cannot catch changes in the real game’s exported object or accepted input API.

Test 1 — Host identity and layout rejection

Fixtures:

Supported host build and memory-map hash.

Different host build.

Truncated memory.

Invalid base pointer or out-of-range field offset.

Assertions:

Supported fixture returns known field values.

Unsupported identity fails closed before reader activation.

Bounds and pointer failures produce an explicit invalid observation, not a plausible default zero.

Also test the production pack’s artifact hashes against its manifest. Never derive host identity solely from bytes currently stored in mutable game memory.

Test 2 — Live reads, growth and non-mutation

Have the fixture:

Write known values at representative addresses.

Instantiate the real reader against its memory.

Change the values and confirm the reader sees the changes.

Grow memory.

Read an old address and a newly valid address.

Compare memory before and after reader invocation.

Assertions:

Reads are live, not an accidental initial copy.

The JavaScript bridge refreshes detached or undersized views.

The reader does not alter game memory.

Static artifact validation rejects forbidden writing/initialisation instructions.

The growth expectations should explicitly match ordinary versus shared memory behaviour.
developer.mozilla

Test 3 — Real input-path smoke test

On the actual target page:

Find the documented/exported input entry point.

Apply a small, bounded modifier.

Release it to neutral.

Observe the strongest available evidence of acceptance.

Run a separate event-path check if supported:

text
DOM synthetic event → recorded as untrusted
debugger input      → inspect received event trust and game acceptance
page API call       → verify state change without requiring an event

Do not use a passing dispatchEvent() test as proof the game accepts the input. Assert an input-state change or another observable result.

Include preservation of base control: a trim must bias the base value rather than silently replace it.

Test 4 — Suggestion ordering and expiry

Feed the actual protocol engine this short sequence:

text
valid current suggestion
duplicate
older channel sequence
expired suggestion
wrong authority epoch
wrong policy revision
synthetic-stream suggestion targeting live actuation

Assertions:

Only the valid suggestion installs a target.

Duplicate installation is idempotent.

Rejections carry stable reason codes.

Expiry is checked using the actuator’s clock domain, not an unrelated server timestamp.

Rejected suggestions do not modify the active target.

Run database admission tests and page-side enforcement tests: either side can receive stale information.

Test 5 — Modifier safety and disconnect

Drive a deterministic local clock through:

text
neutral → maximum positive target → maximum negative target
→ lost connection → lease/TTL expiry → neutral

Assertions:

𝑚
min
⁡
≤
𝑚
(
𝑡
)
≤
𝑚
max
⁡
m
min
	​

≤m(t)≤m
max
	​

∣
𝑚
𝑘
−
𝑚
𝑘
−
1
∣
≤
𝑣
max
⁡
Δ
𝑡
∣m
k
	​

−m
k−1
	​

∣≤v
max
	​

Δt

Also assert that:

No queued old target is replayed after reconnect.

A new session/generation invalidates old modifiers.

Release behaviour follows the declared policy.

Disconnect does not stall the game loop.

Test 6 — Injection-host compatibility

Run the same pack handshake under each supported host:

text
userscript
bookmarklet
extension MAIN
extension debugger
external CDP

Record capabilities rather than expecting identical privileges:

text
memory reference available?
page input API available?
browser input available?
frame source available?
host build compatible?

The pack must either enable the supported path or explicitly degrade to observation-only/advice-only. Navigation must invalidate the old session and require a fresh handshake.

These six test groups cover the three regression classes you named: memory/layout, accepted input and suggestion semantics. Keep the deterministic suite browser-independent where possible, but make the real-page smoke test mandatory for any change to the game build, loader or input adapter.

18 sources
Show me how to map the WebAssembly memory layout
Detail the CDP input generation vs event dispatch approach
Write the Rust reader component for that Wasm memory
How do reducers handle desyncs in this layout
Draft the extension manifest and main-world bridge




Ask a follow-up
Search
Computer
GPT-6.1 Sol
