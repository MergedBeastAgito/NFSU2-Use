# What the research settled

What this repository concluded about observing a closed system, deciding about it, and
inserting a bounded influence back into it. The game is where every boundary can be measured;
the design must hold where none can. A record says what was observed and how, never where it
lives; a capability is negotiated, never inferred from the route the code took; and no claim
outruns the evidence at the boundary that would confirm it.

---

## The envelope

One record per semantic field, not one per snapshot. A snapshot is a consumer's assembly
of records sharing a target, a stream and an epoch, so no producer has to claim an atomic
read it cannot perform.

| part | carries | why it is separate |
| --- | --- | --- |
| field | the semantic name a decision reads | the only identifier a model should see |
| value | number, text, boolean or artifact reference | typed, so nothing is parsed out of prose |
| status | valid, missing, unsupported, permission denied, read error | absence is a result, not a zero |
| source | kind, binding identity and revision, layout identity | how the value was acquired |
| provenance | measured, replay, simulation, generated, mixed, plus lineage | where the data came from |
| quality | method, and an optional scored confidence with a declared meaning | how far it can be leaned on |
| time | clock, sample start and end, validity, clock uncertainty | whether it is still true |
| identity | target, stream, producer, epoch, sequence | which document, which run, which order |

The source discriminator lets one envelope serve a memory read and a DOM read: memory needs
a layout identity, DOM a read operation and sometimes an attribute, derived an extractor and
its inputs. Derived values inherit the provenance of their inputs: measured plus synthetic
inputs produce `Mixed`, never `Measured`, and a synthetic stream may drive a simulation
target but never a live one.

**Missing is not zero.** The route cue is normalised with `0` meaning far left, and an
absent cue encoded as `0` is a lie about the road. **Time is an interval on a named
clock**: two sample times are more honest than one when memory and a frame are acquired
separately, and a consumer accepts only while validity, minus uncertainty, still covers
now. An absolute address, likewise, is a resolution rather than an observation, and it
changes with the target's build: the record carries a field name and a binding, the adapter
carries the offsets, build hash and selector, and immutable binding revisions keep the
record explainable after the adapter moves on.

---

## Tiers and reachability

Four deployment tiers describe how the integration was installed, not what it may do: an
extension without a main-world bridge can see less than a bookmarklet that found an
exposed game object.

| tier | name | baseline |
| --- | --- | --- |
| 0 | uninstrumented page | rendered output, ordinary input, whatever the application publishes |
| 1 | page-context integration | authorised code in the page, ordinary web-platform privileges |
| 2 | extension-mediated | permissions, isolated and main worlds, extension storage and messaging |
| 3 | debugger-mediated | CDP, externally or through an extension debugger permission |

Tier 0 is not blind - a person reads the screen, and an independently authorised capture is
an observation channel, not memory access granted by the tier. Tier 2 with a debugger
permission is tier 3 and is reported as such. Degradation is declared: no memory means
frame-only, no frames memory-only, no actuation advice-only.

**The critical resource is a reachable memory object**, not an address, not the compiled
module, not execution in the same tab. Same-realm code does not reach closure-private
objects, and nothing standard enumerates every wasm memory in a document. Importing gives
live bytes but cached views detach on growth; a copied window goes stale; and a
`SharedArrayBuffer` cannot become a `WebAssembly.Memory` afterwards.

**Input trust is a separate capability.** `Event.isTrusted` is browser-controlled, so
`dispatchEvent()` and `click()` are untrusted in every world, including the main world.
The page's own input API sidesteps the question by generating no event - the path this
application uses. Browser-synthesised input through the debugger is the trusted route.

An extension adds a persistent installation, an origin, storage and injection into
permitted frames, not access to page objects; its service worker dies after roughly thirty
seconds idle, so the fast actuator cannot live there. A debugger adds capture without the
page's cooperation and browser input, but no universal read-by-address command. Substituting
key events for a trim API is forbidden: the semantics differ.

---

## Where the driver ends and the port begins

The dependency direction is driver to SDK to contract, with the SDK re-exporting the trait
rather than defining a second one. A contract that depends on the facade, a concrete driver
or the composition root is a cycle waiting for its second consumer. The trait definition is
a contract, not a capability, and nothing waits for it to be ready.

Two seams, not one. The framework-facing substrate contract says only "evaluate this
expression in this document"; memory layout is not a substrate concern. The domain-facing
contract - observe, apply, release - is what a game integration implements:

```
Application ── DomainSDK ── FrameworkSDK ── SubstrateContract
     └── GameDriver ── DomainSDK
```

The tempting arrow, domain SDK depending on a concrete driver, is the cycle; the inversion
is that the SDK defines the trait, the driver implements it, and a thin binary constructs
the driver and hands it to the runner. The contract serves a 120 Hz loop and a slow
publishing pipeline by separating **admission from execution**: `Apply` returns a receipt
for having admitted a request, with a mode of replaceable target or durable queue. A game
replaces the target; a pipeline enqueues a draft. A tick is 8.33 ms and a decision call
measures 268-451 ms, so the runner is asynchronous and the page-side actuator is local,
checking epoch, expiry and preconditions before it writes. A timeout is not a rollback:
uncertainty is resolved by inspecting the identity already issued.

---

## Suggestion, lease, modifier

A decision layer above a closed loop must not drive it. It advises; an arbiter authorises a
bounded influence; a local actuator applies it, and the roles are separate because they fail
differently.

| message | direction | carries |
| --- | --- | --- |
| observation | observer to store to workers | fields, capture interval, provenance, build and schema identity |
| suggestion | worker to arbiter | channel, target, option probabilities, the observation read, applicability, age and hold limits |
| modifier | arbiter to actuator | authorised target, bounds, slew, expiry, authority epoch, channel sequence, policy revision |
| outcome | actuator and arbiter to store | stage, status, reason, and the evidence that stage rests on |

A suggestion names a state-validity window, not the tick it was computed for, and a
modifier's limits come from policy rather than from what the model asked for. The
distinction that keeps this honest: a modifier biases an input that already exists, so an
adapter that cannot preserve the base value cannot implement an additive trim - replacing
the whole value is direct control whatever the message is called.

Applying low-rate advice to a 120 Hz loop is a local filter: clamp to bounds, exponential
smoothing with `α = 1 − e^(−Δt/τ)`, then a slew clamp, and a neutral target at a declared
release rate on expiry. That bounds discontinuity, not stability, and a key-press-only
boundary needs a discrete policy with dwell times. Conflicts resolve by rule: latest valid
within one authority epoch, one lease per channel with a fencing epoch, and only the
configured authority promotes. Negotiation happens once, before any call carrying
authority: the pack declares required interfaces and a schema hash, the host answers with
typed, scoped, revocable handles rather than permission flags, and the pack refuses to start
if an interface is absent, the hash differs, or a handle lacks a needed constraint.

---

## What an acknowledgement is worth

"Sent" is the report this design exists to replace: a key the game consumed, a key swallowed
while the page was unfocused, and a key that landed during a loading screen are the same
event to a sender.

| claim | what it means |
| --- | --- |
| requested | the adapter recorded an operation and its intended postcondition |
| dispatched | it handed the operation to the configured transport |
| accepted | a named target boundary recorded responsibility, with a correlated receipt |
| consumed | the semantic consumer examined or applied it |
| effect observed | an observer measured the declared postcondition, with scope, revision and correlation |

Accepted without consumed means consumption is unknown, and no timeout resolves it. That
is the central result: "dispatched but not consumed" and "consumed with no effect yet" are
indistinguishable without a consumer receipt. Every rung has a false positive - a slot that
already held the value, momentum from an earlier hold, a controlled component restoring its
model value, an optimistic snapshot read as committed.

Duplicates are detected where the effect happens, not at the sender. A logical insertion
identity survives reconnects, an attempt identity is never the deduplication key, a payload
hash rejects reuse with different content, and a target epoch stops an old instance's
receipt being read as current. Held input is a desired-state command with a duration in
ticks - 480 ticks is four seconds at 120 Hz - and a duplicate must not extend it. One call
moves the car 0.2 m; the same key re-asserted every 16 ms moves it 11 m in four seconds,
which is why holding is a page-side interval and not a stream of events.

Cost decides whether the last rung is worth having. A local readback inside one page
expression costs no extra round trip; a pushed receipt costs an asynchronous message; one
batched post-effect read costs one round trip; polling every 16 ms for 100 ms costs about
seven and should not be done. The rung is dropped when no measurable postcondition
separates the effect from background behaviour, when the promised effect is exactly the
committed mutation, or when observing costs more than the action is worth. A truthful
"dispatched" beats a fabricated "effect observed".

---

## Telemetry and the composition graph

A trace is a shared envelope per event plus explicit links: trace identity, span identity
and parent, component, schema version, occurrence and reception time, typed payload. The
link table carries the relation - caused, forwarded, triggered, observed - because a parent
pointer cannot express one decision that consumed three observations. Ordering is producer
sequence within a stream and epoch; timestamps are timing evidence, not proof of causality.
Reconstruction is done by whoever reads the rows, because subscriptions return whole rows
from one table, join at most two tables on indexed columns, and offer no projections,
ordering or grouping - which makes narrow rows and out-of-line detail a consequence. The
shape of a running system can be rows in the same way: what each component offers, what it
requires, what was verified, and what the resolver bound.

| table | holds |
| --- | --- |
| component | identity, kind, version, tier, lease, declaration revision |
| offer and requirement | port, version range, contract hash, scope, cardinality, phase, access mode |
| claim and fact | what a component says about itself, and what a verifier attested with an expiry and an evidence hash |
| binding and resolution | which offer satisfied which requirement, in which plan revision, at which startup wave |
| intent and runtime | what a host was told to start, and what it observed |

**Claims are not facts.** Registration can say a component exists; it cannot say it deserves
a privilege. Only a trusted verifier or an approved handshake establishes a capability used
for privileged binding.

Resolution is inference under explicit policy: match offers to requirements, check version,
contract, scope and verified predicates, enforce cardinality and capacity, apply a selection
rule, build startup edges, diagnose cycles, and publish bindings and waves under a revision
that becomes active atomically. Statuses name the reasons - missing, ambiguous, capability,
capacity, cycle, dependency. Satisfiable describes a plan; only an observation says ready.

Startup is dispatched as guarded intents: an intent exists only for a component whose bound
dependencies are ready, and the claim checks plan revision, generation, provider readiness
and lease validity. Generations are fencing tokens, so a restarted instance cannot declare
the new one ready. Only hard startup edges are topologically sorted - a feedback cycle of
loop, observer, model, modifier, loop is a legal dataflow.

| limit | workaround |
| --- | --- |
| no DDL from reducers | fixed typed core, publication for specialised storage |
| no installation of arbitrary reducers | precompiled handlers, bounded interpreters, external clients |
| no cross-database join | local summaries, asynchronous messaging, client-side join |
| no nested transaction | one coordinating reducer, propagate failure |
| no exactly-once external effect | idempotent installation, local expiry, evidence levels |

Representation follows the same discipline: indexed deadlines are integer microseconds,
because timestamps, floats and vectors are not indexable; topology lives in typed columns,
with opaque JSON for configuration only; "latest" is a materialised head row, not an
ordered subscription; and sensitive tables stay private behind scoped views.

---

## The web-game pack

One versioned pack, three logical components, three artifact hashes: a reader that acquires
observations, a protocol component that validates, sequences and expires, and an actuator
that applies bounded influence. The split that matters is privilege: the reader gets read
access, the protocol owns its scratch state, the actuator gets only approved operations, and
browser privileges stay in the host.

Bounds are not layout: an address can stay inside the memory after an update and hold a
different field, so the reader is pinned to a host build and memory-map identity, and a
mismatch fails before the first read. Importing memory also grants write access, so the
smallest auditable reader has scalar exports, no allocation and no linear-memory writes.

The pack's own state lives outside game memory: a scratch region with an ABI version,
transmit and receive pointers and capacities, a poll entry point, a monotonic clock and a
notify import. Length-prefixed JSON is enough for a first version; a single-producer,
single-consumer ring for concurrency. The store authorises, orders and accounts for expiring
targets; the page reads, revalidates and writes without waiting for it.

---

## Interaction beyond memory

Modelling browser interaction as "wasm access plus fallbacks" is the mistake that makes a
model fit one target. A debugger can capture pixels without a reachable application object,
and an ordinary page integration can have a rich application API and no browser input.

| axis | levels |
| --- | --- |
| operation | observe, actuate |
| representation | application state, wasm memory, DOM, accessibility, canvas, WebGL, WebGPU, composited pixels |
| execution context | page main, extension isolated, worker, extension host, debugger host |
| reach | current document, same-origin frame, permitted frame, debugger target |
| actuation path | application API, memory slot, DOM property, DOM event, browser input |
| event trust | not applicable, synthetic, browser-generated |
| cooperation | explicit API, reachable objects, instrumentable, uncooperative |

Some combinations do not exist: a worker does not touch the document without a bridge, a
synthetic event is never browser-generated, and an animation frame is not proof of an
application tick. Availability is also not authority: a probe can show an API works without
granting the right to use it.

The observers worth remembering are ordinary: text and attributes for a node, `innerText`
and geometry for layout, value and checked state for a control, a mutation observer for
change, an accessibility tree for semantics, a DOM snapshot for structure, `getImageData` for
a 2D canvas, `readPixels` for WebGL, a texture-to-buffer copy for WebGPU, and a screenshot
for the composited result - the last at about two calls per second.

The accessibility tree earns its place because the browser has computed roles, names and
states without every layout container; it is the wrong reader for canvas contents, pixel
geometry, exact source text and hidden state, and its node identities are ephemeral, so a
semantic locator is resolved afresh.

Graphics readback is a hierarchy of access, not a menu of equivalents: canvas export needs an
origin-clean canvas, `readPixels` reads whichever framebuffer is bound at that moment, a
WebGPU copy needs the texture and usage flags, and a screenshot needs a debugger or a capture
permission and works on a page that cooperates in nothing. For an uncooperative page the
order inverts: capture for pixels, debugger inspection for structure, browser input for
actuation, and page integration only when a reference is obtainable.

Actuation keeps its distinctions too - setting a DOM property, dispatching synthetic events,
calling the application's API and sending browser input are four different promises. What
belongs in the reusable layer is small: negotiation, the envelope, source discriminators,
document and frame identity, units, absence statuses, action semantics, evidence levels and
navigation invalidation. Everything naming a transport, selector, address map or perception
heuristic stays in an adapter.

---

## Where the design is uncertain

**Tiers rank reach; the design says rank nothing.** The table orders vectors by
instrumentation privilege and then insists privilege is not access: a tier number must never
select a code path - the negotiated capability set must.

**The envelope is per field; the tick is per lane.** A record per semantic field is right for
provenance, staleness and absence, and verbose for a loop that sends five numbers every
150 ms. Whether the envelope is the wire format or the store's is unresolved.

**Modifier semantics assume an analogue input.** Smoothing, slew and additive trim are
well-defined for a channel that accepts a value, and this application's boundary is a key. A
discrete policy with dwell times is a different design, not a tuning of this one.

**The last acknowledgement rung is usually unavailable here.** There is no consumer hook,
so the chain stops at "written to the input boundary", with an effect inferred from
position and speed. Promoting that inference is the false precision the rung prevents.

**Exactly-once is not available end to end.** The store can commit an authorisation and the
actuator can install it idempotently, but nothing proves the loop consumed it. Every stronger
claim in this file is bounded by that sentence.
