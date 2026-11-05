chars: 26700
mode: Search
model: GPT-6.1 Sol
question: Documentation/Research/Acknowledgement.question.txt
captured: 2026-11-05T01:44:28.448Z
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
Answer
Links
Images
Share
3:42 AM

The acknowledgement contract: what an adapter may claim about an insertion, and what only the target can confirm.

CONTEXT FIRST, then the questions.

Repositories involved

NikolaRHristov/NFSU2 (public) - the example adapter: a Rust crate that drives a browser WebAssembly game through Agentic's Browse port, plus a wasm reader crate, a JavaScript harness and JSON schema templates.

Agentic - the framework workspace: Crate/{Agentic,BrowserCDP,ChatDriver,DesignVerification,Runtime,SkillContract,SpacetimeCatalog,Workflow}, each with a module.json declaring provides/requires ports.

NikolaRHristov/TimeLine - an Astro site for bookmark ingestion, where an insertion is a draft, a queue entry or a published item rather than a key press.

House conventions (non-negotiable)

Rust identifiers and JSON keys are PascalCase; acronyms are full-cased (URL, URI, SDK, ID, JSON, WASM, CDP).

Crates: Crate/<Name>/{Source/{Library.rs,Fn.rs,Fn/<Verb>.rs,Struct.rs,Struct/<Noun>.rs,Script/Expressions/*.ts}, build.rs, module.json, Test/}.

module.json vocabulary: name @agentic/<kebab>, kind capability|adapter, provides {port, errors}, requires {ports}, artefact {entry,hash,runtime}, state, cost {coldMs,warmMs,network}, verify.

The schema never names a memory address, a selector or a platform. An adapter knows those; it never defines the shape.

What exists today, and what it measures

Observation: a 16-field envelope, each field sourced from WASM memory (absolute address) or from a captured frame.

Decision: Cloudflare Clef decision models, POST /ai/run/@cf/cloudflare/clef-flash with {model,state,questions}, returning a choice and a probability per option. No output tokens, about 375-499 input tokens per call.

Insertion paths already measured in the NFSU2 example:

memory slot: the game's shared WebAssembly.Memory, imported by a wasm reader built with --import-memory, so reads and writes are one typed array view at a known address;

synthetic input: session.input.key(code, true) injects exactly one frame of a key; re-asserted every 16 ms it holds the key. Measured: one call moved the car 0.2 m, the interval moved it 11 m in 4 s. The page's own loop runs at 120 Hz;

the observed effect: position at car+0x60, signed speed at car+0x430, heading at car+0x40c (u16, 65536 a turn), read back through the same memory.

What is missing

The framework can describe an insertion and dispatch it, but it cannot say what happened to it. Every adapter currently reports "sent". A key that the game consumed, a key the page swallowed while unfocused, and a key that landed during a loading screen are indistinguishable in that report. On the TimeLine side the same hole exists for a draft: a queued item that was rejected by a rule and an item that is simply not published yet look identical.

The ladder we are considering, which you should attack rather than adopt: Requested -> Dispatched -> Accepted -> EffectObserved, where only the last rung is a claim about the target's state.

QUESTIONS

For each of the four actuators (memory slot write, synthetic key or pointer input, DOM property set, draft or queue entry in a database), which rungs of that ladder are observable by the adapter itself, and which require a target-side read? Be specific about what distinguishes Accepted from EffectObserved in each case.

For each actuator, what is the minimal measurable evidence that the effect occurred - not that the call returned? Give concrete APIs, functions or queries, the sampling window, and the false-positive mode of each (the case where the evidence appears without the effect).

Distinguish "dispatched but not consumed" from "consumed but no observable effect yet". Give a state machine with named states, the transition that proves each, and the timeout and retry policy per state, including what to do when the effect arrives late.

If an insertion must not be applied twice (a held key that repeats, a draft that re-publishes after a reconnect), what token or sequence scheme detects the duplicate at the target rather than at the sender? Name the field, where it is checked, and what the check costs per insertion.

Which parts of this belong in the observation schema's envelope (field names, state names, evidence shape) and which must stay in the adapter? Keep the house rule: the schema may not name an address, a selector or a platform.

Then argue against your own answer: where is this contract worse than a plain "sent" report, what does it cost in round trips per insertion, and what would make you drop the EffectObserved rung entirely?

Prefer API names, header names, limits, version numbers and concrete numbers over general advice.

Read more
Finished
24s

The contract should describe evidence, not promote every insertion through a presumed success ladder.

Requested and Dispatched are adapter-side facts. Accepted must be a receipt from a named target boundary. EffectObserved must identify a particular postcondition observed at that boundary or beyond it. Neither acceptance nor an observed change automatically proves that the insertion caused the intended outcome.

Most importantly: without a consumer receipt, “dispatched but not consumed” and “consumed but no effect yet” are indistinguishable. A timeout cannot resolve that ambiguity.

Context and scope

I checked the connected GitHub repositories before making this proposal:

The NFSU2 repository lookup returned HTTP 404 through the connector. I therefore treat your WASM addresses, input measurements and 120 Hz loop as supplied experimental results, not independently verified implementation details.

Agentic’s crate directory is accessible. Its SkillContract/module.json currently uses the manifest vocabulary you specified, including lowercase name, provides, artefact and cost.

TimeLine’s root contains Firestore rules and indexes, and its package manifest declares Firebase 13.1.0. The database examples below therefore lead with Firestore rather than silently assuming PostgreSQL. I have not verified TimeLine’s actual ingestion handler or publication pipeline.

All proposed names, timings and policies below are a contract design, not claims that these repositories already implement them. Third-party API names retain their published spelling; proposed Rust identifiers and JSON keys use PascalCase.

1. Define the boundary before defining the rung

The proposed ladder mixes three things:

Progress through a transport.

Progress through a target’s processing pipeline.

Knowledge acquired by an observer.

Those need separate treatment. An effect can be observed after an acceptance receipt was lost; an insertion can be accepted and then rejected by a business rule; an observed effect can subsequently disappear.

Use these definitions:

Claim	Exact meaning
Requested	The adapter has recorded a specific operation and its intended postcondition.
Dispatched	The adapter handed the operation to the configured transport or invocation boundary. It does not imply receipt or consumption.
Accepted	The named target boundary recorded responsibility for this operation, with a correlated receipt.
Consumed	The semantic consumer examined or applied this operation. This is distinct from admission to a queue.
EffectObserved	An observer measured the declared postcondition, with scope, revision and operation correlation where available.

Add Consumed; otherwise Accepted must ambiguously mean both “queue committed” and “worker processed.”

Observability by actuator
Actuator	Adapter can establish itself	Accepted requires	Consumed versus EffectObserved
Memory slot write	Request recorded; store executed without a trap or exception.	If the boundary is the storage cell, immediate readback establishes the stored value. If the boundary is the game consumer, a consumer-written receipt is required.	Consumer records that it sampled the command. Effect is the declared state predicate after processing—not merely the command bytes still being present.
Synthetic key or pointer	Input call issued; transport response received.	Receipt from the intended application input boundary, not merely the browser transport.	Input handler/loop records consumption; effect is a later simulation or application-state change. A keydown listener alone is not proof that the physics loop used the key.
DOM property set	Assignment executed; setter returned.	Readback establishes property acceptance only at the DOM-object boundary. Application acceptance requires a model/reducer receipt.	Model consumed the update; effect is the declared model, rendered or persisted state. These are different scopes.
Database draft/queue insertion	Write submitted.	Authoritative commit receipt establishes admission to the database boundary.	Worker records processing or rule rejection. Effect depends on intent: draft creation, queue admission and publication are three different postconditions.

An adapter can perform a target-side read itself. The distinction is the origin of the evidence, not which process executes the read.

For browser input, Input.dispatchKeyEvent and Input.dispatchMouseEvent dispatch input; they do not supply a game-consumption receipt. Likewise, dispatchEvent() returning true means the event was not canceled under its return-value rules—not that the intended application action succeeded.
chromedevtools.github
+1

2. Minimum measurable effect evidence

First declare the effect. “Write this value,” “accelerate,” “create a draft” and “publish this bookmark” cannot share one success predicate.

The following windows are proposed initial budgets. They need validation against each adapter’s measured latency distribution; none is a browser or database guarantee.

Memory slot write

Concrete operations:

Store/read through the reader’s typed view, or DataView.setFloat32() / DataView.getFloat32() with the correct endianness.

Capture a baseline and subsequent position, signed speed and heading from your existing reader.

Prefer a target hook that emits ConsumedInsertionID and a simulation tick after command sampling.

Sampling:

Immediate readback for the storage postcondition.

For simulation consequences, sample after each completed simulation tick for 12 ticks: nominally 100 ms at your stated 120 Hz.

Take the fields in one coherent snapshot. Sequential reads without a tick/revision guard can combine different simulation states.

Evidence and false positives:

Predicate	Minimum evidence	False-positive mode
“The slot contains V”	Readback of V from the current memory instance and expected region.	V was already present; the address points to unrelated storage; the game overwrites it before consumption.
“The command was consumed”	Consumer-written operation receipt after sampling.	A hook logs before actual sampling or logs a different consumer’s receipt.
“The car moved as intended”	Baseline plus post-consumption state satisfying a declared direction/displacement predicate.	Momentum, another controller, collision or scripted movement produces the same displacement.

Your measured 0.2 m single-call displacement is useful calibration. It is not a universal acceptance threshold or causal proof.

Sharing one WebAssembly.Memory between modules does not by itself establish cross-thread synchronization. If workers access genuinely shared memory concurrently, design an explicit atomic command-publication/receipt protocol; Atomics.store(), Atomics.load() and Atomics.compareExchange() provide the relevant primitives.
developer.mozilla

Synthetic key or pointer input

Concrete operations:

Dispatch with Input.dispatchKeyEvent / Input.dispatchMouseEvent, or the existing Browse input function.

Instrument the application’s actual input consumer to record the insertion’s sequence and the simulation tick.

Read game state through the existing WASM reader after that tick.

Sampling:

Consumption budget: 12 simulation ticks, nominally 100 ms.

Effect budget: initially another 12 ticks for a pulse; calibrate per action.

Held input: one bounded command, with periodic observations during its declared duration—not a separate success claim for every 16 ms refresh.

False positives:

Event delivery is mistaken for simulation consumption.

Movement comes from an earlier held key or momentum.

A pointer event reaches an overlay or a different element with a coincidentally similar visible effect.

A requestAnimationFrame() callback runs before repaint; it is not proof of a simulation tick or a completed paint. Do not substitute “two animation frames elapsed” for a target-state measurement.
developer.mozilla

DOM property set

Concrete operations:

Set the property in the page execution context.

Read the property back immediately.

If the promised effect is application adoption, read the application’s model/revision after its update cycle.

If the promise is persistence, read the authoritative persisted record.

For a form control, setting .value and dispatching an input event are separate operations. Even synchronous listener execution does not prove that an asynchronous save or downstream update completed.
developer.mozilla

Sampling:

Immediate property readback.

Model observation until its revision advances, with a proposed 250 ms foreground-page budget.

Persistence uses its own database budget, not the DOM budget.

False positives:

The property already had the value.

A controlled component later restores its model value.

The DOM changes but submission reads another model.

A rendered success message appears optimistically before persistence.

If the requested effect is only “property equals V,” readback is enough. Do not invent a business-level effect.

Database draft, queue entry or published item

For a proposed Firestore implementation:

Write to an operation-correlated document.

Use getDocFromServer() for explicit reconciliation, or onSnapshot() with metadata updates.

For listener evidence, require the expected ID/hash/state and reject local-pending or cache-only evidence.

waitForPendingWrites() concerns backend acknowledgement of pending writes; it does not establish that a publication worker processed them. Firestore’s API explicitly distinguishes listener synchronization from server synchronization.
firebase.google

Suggested predicates:

text
Draft:
  Record.InsertionID == RequestedInsertionID
  Record.PayloadHash == RequestedPayloadHash
  Record.State == "Draft"

Publication:
  Record.PublicationInsertionID == RequestedInsertionID
  Record.State == "Published"
  Record.Revision >= ExpectedRevision

Sampling:

Proposed server-admission budget: 5 seconds.

Proposed automated-publication budget: 30 seconds.

A human-reviewed draft has no useful 30-second publication deadline. Observe draft creation and track publication separately.

False positives:

A local optimistic snapshot is mistaken for committed state.

An old published record matches only by URL.

Database state says Published, but the static build or public delivery layer has not updated.

If the promise is “visible on the public site,” verify the served item and matching content revision too. Database publication and public availability are separate effects.

For a PostgreSQL alternative, use INSERT ... RETURNING plus successful transaction commit; RETURNING alone inside an uncommitted transaction is insufficient. PostgreSQL 18 returns only rows actually inserted or updated, and ON CONFLICT DO NOTHING does not return the conflicting row.
postgresql

3. State machine: uncertainty must remain representable

Use an append-only receipt history and a derived current status:

text
Requested
  ├─ local validation failure ──────────────> Rejected
  └─ transport invocation ─────────────────> Dispatched
       ├─ correlated admission receipt ────> Accepted
       │    ├─ consumer receipt ───────────> Consumed
       │    │    ├─ matched postcondition ─> EffectObserved
       │    │    └─ observation deadline ──> EffectUnknown
       │    └─ acceptance deadline ────────> ConsumptionUnknown
       ├─ authoritative refusal ───────────> Rejected
       └─ receipt deadline ────────────────> DeliveryUnknown

Allow Dispatched → EffectObserved when valid effect evidence arrives without intermediate receipts. Do not manufacture the missing receipts.

What actually proves the distinction?

Accepted plus no consumer receipt means consumption is unknown.

A durable queue record still marked pending shows pending state at that revision, not that no worker ever inspected it.

NotConsumed requires an authoritative terminal receipt: expired before execution, rejected before consumption, or canceled atomically before consumption.

Consumed requires a receipt emitted by the semantic consumer.

Consumed plus no matching effect means “consumed, effect not observed,” not necessarily “no effect occurred.”

Timeout and retry policy
State	Timeout policy	Retry policy
Requested	Local validation budget, e.g. 100 ms.	Safe before dispatch.
Dispatched / DeliveryUnknown	Browser receipt budget initially 1 second; database admission 5 seconds.	Reconcile first. Replay only with target deduplication, using the same InsertionID.
Accepted / ConsumptionUnknown	Input: 12 simulation ticks. Queue: workflow-specific deadline.	Do not create a second operation. Query the original receipt.
Consumed / EffectUnknown	Apply the declared effect window.	Never automatically replay a non-idempotent action.
Rejected / Expired	Terminal for that operation.	A corrected request gets a new ID. A transient replay retains the original ID only under the defined protocol.
EffectObserved	Stop active polling; retain evidence.	No replay. Later disappearance is a subsequent state change, not proof the observation was false.

On a late effect, append a new receipt with Late: true, preserve the earlier timeout and resolve the original operation. Notify the workflow that previously saw uncertainty.

A timeout must not authorize a conflicting replacement action. If the caller must move on, require target-enforced expiry, cancellation or supersession first.

4. Duplicate detection must execute with the effect

Use two identifiers:

InsertionID: stable logical operation identity across reconnects and retries.

AttemptID: identity of a transport attempt, never the deduplication key.

Also carry:

PayloadHash: reject reuse of one ID with different content.

TargetEpoch: prevents receipts from an old target instance being treated as current.

Sequence: ordered control-stream version where ordering matters.

Memory and input

For an instrumented single-consumer command stream, check (TargetEpoch, Sequence) at the consumer immediately before application.

Same sequence and hash: return the prior receipt.

Lower sequence: reject as stale.

Same sequence with different hash: Conflict.

Next valid sequence: apply and advance the stored sequence in one serialized consumer step.

For ordered, single-consumer processing, this costs O(1) comparison and state update per command. Multiple consumers require a properly synchronized claim protocol; one atomic claim alone does not make a multi-step business effect crash-atomic.

For held input, do not replay “keydown for another frame” indefinitely. Propose a target-side desired-state command:

json
{
  "InsertionID": "opaque-operation-ID",
  "TargetEpoch": "opaque-epoch",
  "Sequence": "42",
  "State": "Held",
  "DurationTicks": 480
}

At your stated 120 Hz, 480 ticks is four seconds. Duplicates must not restart or extend the duration. A newer release command supersedes the hold; an old hold arriving afterward is rejected.

Raw browser input has no application-level InsertionID deduplication facility. A cooperative harness or target bridge must carry and check it. Sender-side suppression alone cannot provide this guarantee.
chromedevtools.github

DOM

Check the ID in the application command handler/reducer, not merely in an adapter-owned map.

Repeated assignment of one property to one value may be idempotent at the storage level. Repeated setters, events and resulting saves need not be. Persist deduplication state if the effect survives reload.

Database

For Firestore, propose a receipt document keyed by tenant and InsertionID. In one runTransaction():

Read the receipt.

If present, compare PayloadHash and return its recorded result.

Otherwise write the domain mutation and receipt together.

For one new operation touching one domain document, the minimum is one receipt read and two document writes, plus any reads needed for business validation. Contention can repeat transaction reads. Firestore transaction callbacks can run again; never send email, dispatch input or perform another nontransactional effect inside that callback. The JavaScript reference documents a default maximum of five commit attempts.
firebase.google

Deduplicate publication separately at the publication boundary. Deduplicating draft creation does not prevent a reconnecting worker from publishing twice.

For PostgreSQL, use a unique key on ("TenantID", "InsertionID"), with the receipt and domain mutation in the same transaction. The unique-index conflict check arbitrates concurrent requests; on conflict, read and return the original result rather than treating “nothing inserted” as success.
postgresql

Retain deduplication records for the entire accepted replay horizon. If receipts expire after seven days, reject older requests rather than quietly making them executable again.

5. Schema versus adapter

Do not add acknowledgement semantics to each of the existing 16 sensor fields. Add an optional, orthogonal Acknowledgements collection to the observation envelope.

A proposed receipt:

json
{
  "ContractVersion": "1.0",
  "InsertionID": "opaque-operation-ID",
  "AttemptID": "opaque-attempt-ID",
  "TargetEpoch": "opaque-epoch",
  "ReceiptSequence": "7",
  "State": "EffectObserved",
  "Boundary": "opaque-boundary-ID",
  "ObservedAt": "2026-11-05T01:42:00Z",
  "Late": false,
  "Evidence": {
    "Kind": "Postcondition",
    "PredicateID": "opaque-predicate-ID",
    "ObservationID": "opaque-observation-ID",
    "TargetRevision": "812",
    "Correlation": "InsertionID",
    "Before": {
      "Revision": "811",
      "Value": 0
    },
    "After": {
      "Revision": "812",
      "Value": 1
    }
  }
}

Use decimal strings for potentially 64-bit counters crossing JavaScript/JSON boundaries. Specify units and comparison semantics in the predicate contract.

Shared schema responsibilities

IDs, state names and legal evidence kinds.

Receipt ordering and target epochs.

DeadlineExceeded versus authoritative Expired.

Error classification: Rejected, Conflict, Unsupported, Unknown.

Correlation strength: InsertionID, Sequence or TemporalOnly.

Declared predicate, observation window, target revision and evidence reference.

Optional capability declarations such as SupportsConsumptionReceipt and SupportsTargetDeduplication.

A temporally correlated position change must remain visibly weaker than a target receipt correlated by insertion ID.

Adapter responsibilities

Memory addresses, offsets, types and endianness.

Selectors, execution contexts and element identity.

Platform API calls and transport routing.

Concrete database paths and queries.

Frame/tick synchronization and snapshot consistency.

Predicate implementation, calibration and sampling schedule.

Mapping native errors into shared classifications.

Keep your prescribed module.json vocabulary unchanged; it is the explicit manifest exception to PascalCase JSON keys. Put the receipt types in the shared contract layer and implementations under each adapter’s existing crate layout—for example Source/Struct/InsertionReceipt.rs and Source/Fn/ObserveEffect.rs.

Do not make every receipt trigger another Clef decision. Acknowledgement tracking should be deterministic; a receipt enters the decision state only when it changes the workflow’s choices.

6. The case against this contract
It can be worse than “sent”

The main danger is false precision:

An Accepted badge hides which boundary accepted it.

A movement predicate turns correlation into a causal claim.

Instrumentation emits a receipt before the real consumer acts.

Polling misses a transient effect and encourages a dangerous retry.

A synchronous success requirement makes an asynchronous workflow brittle.

Added receipt storage, subscriptions and deduplication records become another system requiring recovery.

A truthful Dispatched report is better than fabricated EffectObserved.

Round-trip cost
Implementation	Additional cost beyond dispatch
Memory write plus local readback in one page expression	Zero extra browser round trips; only storage evidence.
Instrumented consumer with pushed receipt	Zero additional request round trips; an asynchronous receipt message.
One batched post-effect browser read	One extra round trip.
Poll browser state every 16 ms for 100 ms	About seven extra reads if each poll crosses the transport. Avoid this.
Firestore write plus explicit server reconciliation read	One additional server read request.
Existing Firestore listener	No per-insertion polling request, but listener traffic and billed reads remain.
Database receipt transaction	Extra receipt read/write work; transaction retries can increase it.

Run high-frequency sampling inside the target context and return a bounded batch. Piggyback receipts onto the existing observation stream where possible. Do not block the 120 Hz game loop on network acknowledgement.

When I would drop EffectObserved

Drop it as a required rung when:

No measurable postcondition separates the operation’s effect from background behavior.

The promised effect is already exactly the committed mutation—such as “create this draft”—so acceptance and effect coincide.

Human review or an external system controls the eventual outcome.

Observing every effect costs more than the action’s value.

Instrumentation materially perturbs timing or requires an unacceptable target modification.

In those cases, expose Dispatched, authoritative receipts where available, and an explicit Unsupported or Unknown effect status. Keep downstream state observations separate.

The essential guarantee is narrower than “the insertion worked”: every reported claim states its boundary, its evidence and what remains unknown.

28 sources
Show how to structure the state boundary schema
Draft the Rust types for adapter acknowledgements
How to handle consumer timeouts without ambiguity
Compare this contract with transactional outbox patterns
Map these rules to the TimeLine Firestore rules
⠦




Ask a follow-up
Search
Computer
GPT-6.1 Sol