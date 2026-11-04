chars: 25711
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
1:25 AM

Research brief: a communication protocol for suggestions above closed game loops, designed to transduce into SpacetimeDB.

Setting aside any particular repository, I need a design for a system with these four layers, and the schema and protocol that connect them.

LAYER 1 - the closed game loop. A compiled game runs at 60-144 Hz with its own tick, its own memory, and no hooks. It cannot be paused, and any modification must arrive as an input (a key, a value in a memory slot) that the loop consumes on its next tick.

LAYER 2 - the observer. State is read from that loop's memory (absolute addresses) and, separately, from rendered frames. Two kinds of data: real (measured) and synthetic (generated, replayed, or simulated for testing). Both must be first-class.

LAYER 3 - the decision layer. Local models (rule engines, small on-device models) and remote models (a hosted decision model that takes a state plus typed questions and returns a probability per option). Either can be authoritative; the other advises.

LAYER 4 - the agent. An LLM agent that reasons over the whole thing, can run tooling locally, and can change the pipeline while it runs.

The core idea to design for: the decision layer does not drive the loop directly. It produces SUGGESTIONS that are applied as modifiers to a closed loop that keeps running - a pitch shift, a cue bias, a steering trim - so a bad suggestion degrades the drive rather than breaking it.

Questions:

PROTOCOL. What should the message protocol between these four layers be so that a suggestion is applied as a modifier rather than a command? Name the message types, the direction of each, and what each carries. Consider: observation, suggestion, modifier, acknowledgement, rejection, staleness marker. What is the minimum set that is still sufficient?

SCHEMA -> SPACETIME. The whole thing must run on SpacetimeDB: tables, reducers, subscriptions, and clients that subscribe. Give a concrete table schema for the layers above (table names, columns, primary keys, indexes), the reducers that write them, and which clients subscribe to which tables. How should a suggestion that arrives too late for the tick it was computed for be handled - a column, a separate table, or a reducer that refuses?

REAL AND SYNTHETIC. Both measured and synthetic data must flow through the same tables. Should synthetic rows be marked by a column, a separate table, or a separate database? What breaks when a synthetic row is consumed as if it were real, and how does the schema prevent it?

WASM DUPLEX. Two integration styles must both work: (a) an agent running locally that prompts the JavaScript side of a browser tab over CDP, and (b) a WebAssembly module sharing the game's memory that speaks the protocol directly. What is the same and what is different between them? Can one schema serve both, and what must the WASM side expose (exports, imports, shared buffers) to participate?

SMOOTHNESS. Suggestions arrive at a different rate than the loop runs (say 5 Hz vs 120 Hz). What is the correct way to apply a low-rate suggestion to a high-rate loop without introducing jitter - interpolation, exponential smoothing, a rate-limited modifier, a slew limiter? Give the formula and the failure mode of each.

USE CASES. Expand beyond driving: name at least six other domains where a closed loop consumes low-rate suggestions as modifiers (audio, rendering, robotics, trading, live video, accessibility). For each, say which of the four layers changes and which stays identical.

FAILURE MODES. What must the protocol guarantee when the remote model is slow, when a suggestion contradicts the loop's own state, and when two agents suggest conflicting modifiers? Name the resolution rule.

Prefer concrete schema, concrete message shapes and named algorithms over general advice. Assume the reader is implementing this, not evaluating it.

Read more
Finished
13s

Use a bounded, leased modifier protocol with two distinct authorities: SpacetimeDB authorises a target modifier; a local actuator decides whether that modifier is still applicable and moves toward it at the loop’s available input boundary. The game continues running if the observer, model, database or agent disappears.

The essential distinction is between a suggestion, which is evidence-backed advice, and a modifier, which is a constrained, expiring authorisation to bias an existing input. Neither should mean “execute this arbitrary command.”

Call the proposed protocol SLMP/1: Suggestion–Lease–Modifier Protocol, version 1. This is a proposed design, not an existing standard.

1. Protocol: separate advice, authorisation and physical application
First establish the integration limit

If the game exposes no callable tick hook, an external observer cannot guarantee execution on a particular game tick. It can write an existing input slot or submit an input event; the game consumes that input whenever its own implementation does so.

Likewise, sharing game memory does not automatically give a helper Wasm module execution on every game tick. It still needs to be called by a host, worker or existing execution path.

Therefore:

Treat game ticks as exact only when reliably observable.

Otherwise use a local monotonic observation sequence and explicitly label it as such.

Never make database acknowledgement synonymous with game consumption.

Run interpolation and expiry locally—not in a database subscription callback.

Minimum message set

Four message types are sufficient; acknowledgement, rejection and staleness can be variants of one outcome type.

Message	Direction	Contents
Observation	Observer → database → decision workers/agent	Snapshot, provenance, timing, validity, build and schema identity
Suggestion	Local/remote decision worker → database arbiter	Proposed channel target, evidence, probabilities, applicability conditions and expiry
Modifier	Database arbiter → local actuator	Authorised bounded target, authority epoch, sequence, validity interval and policy revision
Outcome	Arbiter/actuator → database → interested clients	Accepted, rejected, stale, installed, superseded or revoked; reason and actual application evidence

Use a fifth, control-plane message, PolicyRevision, for agent-proposed pipeline changes. It should not be necessary to carry every normal observation or suggestion.

Common envelope

Use JSON for diagnostics and generated typed SDK values for SpacetimeDB. Represent u64 values as decimal strings in JSON to avoid JavaScript integer precision loss.

json
{
  "protocol_version": 1,
  "session_id": "session-7",
  "stream_id": "measured-main",
  "message_id": "producer-a:203",
  "producer_id": "producer-a",
  "producer_seq": "203",
  "policy_revision": "12"
}

Bind authenticated producer identity on the server; do not trust a caller-supplied identity string.

Observation
json
{
  "kind": "observation",
  "observation_id": "obs-9001",
  "capture_seq": "9001",
  "game_tick": null,
  "capture_start_mono_us": "81000000",
  "capture_end_mono_us": "81006000",
  "schema_id": "vehicle-state/1",
  "host_build_hash": "sha256:...",
  "fields": [
    {
      "path": "/speed_kmh",
      "value": 76.4,
      "origin": "measured",
      "source": "memory",
      "valid": true
    },
    {
      "path": "/cue/x",
      "value": 0.62,
      "origin": "measured",
      "source": "frame",
      "valid": true
    }
  ]
}

A capture interval is more honest than one timestamp when memory and frame observations are acquired separately. Include per-field capture times if their skew matters. Do not describe an unsynchronised memory/frame read as an atomic snapshot.

Suggestion

A suggestion names a state-validity window, not just “the tick I was computed for.”

json
{
  "kind": "suggestion",
  "suggestion_id": "suggestion-44",
  "based_on_observation": "obs-9001",
  "channel": "steering_trim",
  "target": 0.06,
  "unit": "normalized",
  "confidence": 0.83,
  "question_pack_hash": "sha256:...",
  "max_observation_age_ms": 300,
  "max_hold_ms": 400,
  "precondition_id": "grounded-and-forward/1"
}

Store the actual option probabilities separately from any policy-derived confidence. A model’s probability is not automatically calibrated confidence.

Modifier

The arbiter derives limits from policy, not from whatever limits a model requests:

json
{
  "kind": "modifier",
  "modifier_id": "modifier-45",
  "suggestion_id": "suggestion-44",
  "channel": "steering_trim",
  "authority_epoch": "8",
  "channel_seq": "91",
  "target": 0.06,
  "minimum": -0.12,
  "maximum": 0.12,
  "slew_per_second": 0.3,
  "smoothing_tau_ms": 100,
  "expires_observer_mono_us": "81700000",
  "expiry_target": 0.0
}

The expiry is expressed in the observer/actuator clock domain. This assumes those roles share a defined monotonic clock; otherwise use a clock-domain identifier and an explicit mapping with uncertainty.

For an additive channel:

𝑢
applied
(
𝑡
)
=
clamp
⁡
(
𝑢
base
(
𝑡
)
+
𝑚
(
𝑡
)
,
𝑢
min
⁡
,
𝑢
max
⁡
)
.
u
applied
	​

(t)=clamp(u
base
	​

(t)+m(t),u
min
	​

,u
max
	​

).

For multiplicative channels, neutral is usually 
1
1; for additive ones, usually 
0
0. Define composition, units, bounds and neutral value per channel.

Crucially, if the adapter cannot access or preserve the base input, it cannot honestly implement an additive trim. Replacing the entire steering value is still direct control, regardless of the message name.

2. SpacetimeDB: transactional arbitration, local application

Target the SpacetimeDB 2.x API family and pin the exact CLI, SDK and module versions together. In 2.x Rust, caller identity is accessed through ctx.sender(), rather than the older ctx.sender field.
spacetimedb

The schema below is a concrete logical schema, not SQL DDL. Implement it with SpacetimeDB table declarations and generated client bindings.

Tables

Use application-generated string IDs for cross-system message identity. Use native Identity, Timestamp, integer and enum types where appropriate.

Table	Columns	Primary key and indexes
session	id, host_build_hash, active_policy_revision, actuator_identity, status	PK id
stream	id, session_id, mode, clock_domain, observer_identity, actuation_allowed	PK id; index session_id
producer_grant	id, session_id, identity, role, stream_id, enabled	PK id; index (session_id, identity)
observation	id, session_id, stream_id, capture_seq, game_tick?, capture_start_us, capture_end_us, received_at, schema_id, payload, provenance_class, lineage	PK id; index (stream_id, capture_seq)
stream_head	stream_id, observation_id, capture_seq, capture_end_us, received_at	PK stream_id
channel_policy	id, session_id, revision, channel, composition, unit, neutral, minimum, maximum, slew_rate, tau_ms, max_age_ms, max_hold_ms, precondition_id	PK id; index (session_id, revision)
authority_lease	id, session_id, channel, owner_identity, epoch, expires_at	PK deterministic session/channel ID; index session_id
suggestion	id, session_id, stream_id, producer_identity, based_on_observation, policy_revision, channel, target, probabilities, received_at, status, reason?	PK id; indexes (session_id, channel), based_on_observation
modifier_head	id, session_id, stream_id, channel, suggestion_id, authority_epoch, channel_seq, policy_revision, target, expires_observer_us, issued_at	PK deterministic session/channel ID; index session_id
outcome	id, session_id, message_id, reporter_identity, stage, status, reason, local_seq?, local_time_us?, actual_value?, evidence_level, received_at	PK id; indexes message_id, session_id
pipeline_revision	id, session_id, revision, parent_revision, schema_hash, question_pack_hash, policy_hash, artifact_hashes, status, author_identity	PK id; index (session_id, revision)

Define payload as a versioned typed observation value structure, or bytes plus a schema identifier if heterogeneous observations demand it. Do not make opaque payloads the source of authority, freshness or provenance checks; those need explicit columns.

Use modifier_head for efficient subscriptions and suggestion/outcome for audit. Updating a head row means a slow subscriber may miss intermediate targets, which is desirable here: modifiers are replaceable targets, not a command queue.

Reducers
Reducer	Writes and checks
open_session	Authorised session creation
register_stream	Stream provenance and actuation permission
publish_observation	Authenticate observer; reject sequence regression; validate schema/provenance; update head
submit_suggestion	Authenticate worker; validate observation reference, revision, freshness and authority; insert suggestion; possibly update modifier head
acquire_authority / renew_authority	Atomically manage per-channel owner and fencing epoch
report_outcome	Authenticate reporter; append actuator evidence; never imply game consumption without evidence
revoke_modifier	Publish neutral/revoked head with a new sequence
stage_pipeline_revision	Validate and store a candidate immutable revision
activate_pipeline_revision	Authorised revision switch; invalidate incompatible old modifiers
expire_and_prune	Cleanup old leases/history; not a real-time actuator timer

Run local and remote models as clients. Reducers are transactional and cannot make outbound network requests; keep model calls outside them.
spacetimedb

Late suggestions: record the rejection, do not just throw

Use all three mechanisms, for different purposes:

Columns carry observation identity, age limits and status.

The reducer refuses promotion into modifier_head.

The actuator independently refuses expired application.

For a validly authenticated but stale suggestion, insert it with status = rejected and reason = stale_observation, then complete the reducer successfully.

Returning an error after inserting a rejection would roll the insertion back: SpacetimeDB reducers are atomic transactions.
spacetimedb

Compare observation time to the newest known observer clock for that stream. Also enforce server-side observer liveness. Since the server cannot know the exact current game state, the actuator performs the final age and precondition check locally.

Subscriptions and access
Client	Subscribes to
Observer	Active session/revision and its ingestion configuration
Decision workers	Authorised observation/head views, question configuration, authority state
Actuator	Its authorised modifier heads, active policy and lease epochs
Agent	Scoped observations, outcomes, policies and revision state
Test runner	Synthetic stream equivalents

Subscriptions are not access control. Keep underlying sensitive tables private and expose appropriately scoped views, or use separate databases for stronger tenancy boundaries. Public tables are readable by clients; private tables are accessible through server-side logic and views rather than direct client subscriptions.
spacetimedb

3. Real and synthetic: same structures, enforced provenance boundaries

Use the same tables, with separate streams and mandatory provenance. A boolean synthetic is insufficient.

Define:

text
StreamMode:
  Measured
  Replay
  Simulation
  Generated
  Mixed

Also preserve field-level lineage. A replay of a measured frame is still a replay in the current execution, not a fresh measurement of the live game.

Rules:

A stream’s mode is immutable.

Provenance follows authenticated ingestion permissions.

Derived data inherits the union of its input provenance.

Mixing synthetic and measured inputs produces Mixed, not Measured.

Live authority requires an actuation-enabled measured stream.

A simulation stream can actuate only its own simulated target.

No reducer may “promote” a synthetic stream into a measured stream.

The key check is not merely a label:

text
suggestion.stream
  == observation.stream
  == authorised actuator stream

Synthetic-as-real failure can produce steering toward a nonexistent cue, a model responding to replayed hazards, or a supposedly fresh modifier based on an old scene. Field-level contamination can be subtler: fifteen measured fields plus one generated cue must not silently count as an entirely measured observation.

Use a separate database for high-risk live deployments where credentials must make test-to-live crossover impossible. Same tables and protocol can still be used in both databases.

4. Wasm duplex: one semantic protocol, two transports
What stays identical

Both CDP and Wasm paths use:

Session/stream identities.

Observation and suggestion schemas.

Authority epochs and channel sequences.

Policy revisions and expiry semantics.

Outcome evidence levels.

Neither transport receives special permission to bypass the actuator gate.

What differs
Concern	CDP/local bridge	Wasm helper
Memory/frame acquisition	Page evaluation and frame capture	Direct memory loads; frame features usually supplied by host
Execution	Local process and browser callbacks	Only when invoked by host or scheduled worker
Networking	Local SpacetimeDB SDK client	Browser/native host bridge
Input delivery	Page API, input event or documented slot	Documented input slot or imported host function
Main risk	Browser scheduling and round-trip delays	Memory corruption, concurrency and unwarranted tick assumptions

Browser Wasm interacts with Web APIs through host integration; it does not become an independent DOM/network client merely by importing memory.
developer.mozilla

A concrete ABI

Provide a protocol scratch region separate from game-owned data:

text
Exports:
  slmp_abi_version() -> u32
  slmp_tx_ptr() -> u32
  slmp_tx_capacity() -> u32
  slmp_rx_ptr() -> u32
  slmp_rx_capacity() -> u32
  slmp_poll(now_mono_us: u64) -> u32

Imports:
  host.monotonic_us() -> u64
  host.notify_tx()

Use length-prefixed UTF-8 JSON for ABI version 1. The host drains TX messages, uses the SpacetimeDB SDK, writes RX messages and invokes slmp_poll. Define buffer ownership explicitly; never allow simultaneous host/module reuse of a message slot.

A helper can import game memory and use separate scratch memory where the runtime supports that arrangement. Otherwise use host-copy functions for protocol data. Do not place a ring buffer into apparently unused game memory.

For actual cross-worker concurrency, use shared memory and atomic publication. Imported memory alone does not mean concurrent shared-memory semantics; WebAssembly.Memory must have the appropriate shared backing to be shared across workers.
developer.mozilla

A single-producer/single-consumer ring is sufficient:

Producer writes payload.

Producer publishes the write sequence atomically.

Consumer reads only published entries.

Consumer advances the read sequence.

Full buffer coalesces replaceable observations/modifier targets; critical outcomes use a bounded retry path.

An input-writing Wasm helper is not a read-only reader. Keep those capabilities separate, even if one module packages both.

5. Smoothness: local target tracking, not subscription-driven stepping

At 5 Hz advice and 120 Hz local application, apply the target through a local filter each available application step using measured 
Δ
𝑡
Δt. Do not assume 24 identical substeps if scheduling varies.

Algorithms and their failure modes
Algorithm	Formula	Failure mode
Hold latest target	
𝑚
𝑘
=
𝑟
latest
m
k
	​

=r
latest
	​

	Steps and chatter
Linear interpolation	
𝑚
(
𝑡
)
=
(
1
−
𝜆
)
𝑚
0
+
𝜆
𝑟
,
𝜆
=
clamp
⁡
(
(
𝑡
−
𝑡
0
)
/
𝑇
,
0
,
1
)
m(t)=(1−λ)m
0
	​

+λr,λ=clamp((t−t
0
	​

)/T,0,1)	Repeated retargeting changes slope; interpolation between future samples requires added delay
Exponential smoothing	
𝑚
𝑘
=
𝑚
𝑘
−
1
+
𝛼
𝑘
(
𝑟
𝑘
−
𝑚
𝑘
−
1
)
,
𝛼
𝑘
=
1
−
𝑒
−
Δ
𝑡
/
𝜏
m
k
	​

=m
k−1
	​

+α
k
	​

(r
k
	​

−m
k−1
	​

),α
k
	​

=1−e
−Δt/τ
	Adds lag; does not itself guarantee a derivative bound
Slew/rate limiter	
𝑚
𝑘
=
𝑚
𝑘
−
1
+
clamp
⁡
(
𝑟
𝑘
−
𝑚
𝑘
−
1
,
−
𝑣
−
Δ
𝑡
,
𝑣
+
Δ
𝑡
)
m
k
	​

=m
k−1
	​

+clamp(r
k
	​

−m
k−1
	​

,−v
−
	​

Δt,v
+
	​

Δt)	Can react too slowly to urgent changes; slope changes remain abrupt
Acceleration-limited ramp	Bound both modifier velocity and its change	More state and tuning; reversal/overshoot handling must be explicit

Exponential smoothing trades responsiveness against suppression of changes; a slew limiter specifically bounds the first derivative, with separate rising and falling limits.
itl.nist
+1

Recommended default:

𝑟
𝑘
←
clamp
⁡
(
𝑟
𝑘
,
𝑚
min
⁡
,
𝑚
max
⁡
)
r
k
	​

←clamp(r
k
	​

,m
min
	​

,m
max
	​

)
𝑚
~
𝑘
=
𝑚
𝑘
−
1
+
(
1
−
𝑒
−
Δ
𝑡
/
𝜏
)
(
𝑟
𝑘
−
𝑚
𝑘
−
1
)
m
~
k
	​

=m
k−1
	​

+(1−e
−Δt/τ
)(r
k
	​

−m
k−1
	​

)
𝑚
𝑘
=
𝑚
𝑘
−
1
+
clamp
⁡
(
𝑚
~
𝑘
−
𝑚
𝑘
−
1
,
−
𝑣
−
Δ
𝑡
,
𝑣
+
Δ
𝑡
)
.
m
k
	​

=m
k−1
	​

+clamp(
m
~
k
	​

−m
k−1
	​

,−v
−
	​

Δt,v
+
	​

Δt).

On expiry, set 
𝑟
𝑘
r
k
	​

 to neutral and use a specified release rate. Safety-critical hard limits may override smoothing.

This limits discontinuity; it does not prove the combined loop is stable. Modifier magnitude, delay and coupling to the original controller still require testing.

If the only available interface is key presses, continuous smoothing is not directly implementable. Define a discrete input policy with minimum dwell times; do not pretend repeated key toggles are equivalent to an analogue trim.

6. Six other domains

These are applications of the proposed architecture, not claims that every existing product exposes the required modification boundary. In each, protocol, provenance, leases and outcome tracking remain the same.

Domain	Layer 1	Layer 2	Layer 3	Layer 4
Audio	Audio callback consumes gain/pitch targets; ramp within audio processing	Level, pitch and clipping measurements	Proposes gain, cents or filter bias	Changes analysis/policy within bounded controls
Rendering	Render loop consumes exposure/LOD/detail bias	Frame time, luminance, GPU metrics	Proposes bounded quality adjustments	Selects optimisation goals and budgets
Robotics	Existing servo controller consumes limited setpoint offsets	Encoders, force and vision	Proposes trim or trajectory bias	Plans tasks; cannot bypass safety controller
Trading	Order/risk engine consumes risk multipliers and limits	Positions, fills and market data	Proposes bounded exposure bias	Changes strategy policy under approval rules
Live video	Frame pipeline consumes crop, colour or stabilisation targets	Motion, faces and image statistics	Proposes framing/colour offsets	Changes scene policy and priorities
Accessibility	Existing UI/input loop consumes pointer gain or assistance bias	Pointer motion, gaze or interaction state	Proposes assistance strength	Configures user-specific policies and tools

Layer 1 and the observation schema change in every domain. Decision questions and channel units change in Layer 3. Layer 4 retains its orchestration role, but its permissions and approval requirements are domain-specific.

For trading, a modifier expiry cannot undo already executed trades. For robotics, a softened bad suggestion can still be unsafe. “Modifier, not command” reduces coupling; it is not a universal safety guarantee.

7. Resolution rules and guarantees
Failure	Resolution rule
Remote model is slow	Latest-valid-wins within one authority epoch; reject expired evidence and never queue a backlog of old advice
Newer observation arrives during inference	Revalidate age and explicit predicates; do not demand exact snapshot equality unless the channel requires it
Suggestion contradicts current state	Local precondition veto; neutral/release behaviour; report precondition_failed
Two agents conflict	Single authoritative lease per channel; fencing epoch rejects former owners
Local and remote disagree	Only the configured authority can promote; the other remains advisory
Authority fails	Explicit lease expiry and takeover with a new epoch; never implicit racing
Duplicate/reordered delivery	Message IDs, monotonic producer sequence and channel sequence; duplicate requests are idempotent
Database disconnects	Local watchdog expires the modifier and returns toward neutral
Pipeline changes during inference	Reject incompatible old revisions; activate immutable revisions at a local boundary
Agent proposes unsafe configuration	Stage, validate and authorise separately; immutable actuator ceilings remain outside ordinary agent edits

Do not resolve disagreement by comparing raw probability numbers from unrelated models. They may not be comparable. Do not blend competing steering suggestions unless policy explicitly defines a tested blend.

Acknowledgement must state its evidence level

Use distinct outcome stages:

text
accepted_by_arbiter
installed_by_actuator
written_to_input_boundary
consumption_observed
effect_observed

If no hook or observable consumption marker exists, stop at written_to_input_boundary. A successful memory store proves a store, not that the game consumed the intended value on the intended tick.

Finally, distinguish transactional and physical guarantees:

SpacetimeDB can atomically select an authorised modifier and record that decision.

The actuator can enforce local bounds, expiry and idempotent target installation.

Neither can guarantee exactly-once physical consumption by an uninstrumented game loop.

That boundary is the central implementation rule: database state authorises a bounded influence; only the local integration can apply it, and only observable evidence can establish what the closed loop actually did.

28 sources
Show me the protocol message sequence
Draft the Rust client struct for SLMP 1
How does SpacetimeDB handle lease expiry
Design the local fallback when the database drops
Map the observation schema to reducer calls




Ask a follow-up
Search
Computer
GPT-6.1 Sol
