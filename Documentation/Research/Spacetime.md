chars: 25266
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
1:39 AM

Research brief: modelling an entire composed system in SpacetimeDB, where reducers infer the ecosystem rather than being told it.

Prior context: a system of components (a game loop, an in-page WebAssembly pack, a browser extension, a CDP host, local and remote decision models, an agent) that today runs as separate processes and files, and must instead run on SpacetimeDB - tables, reducers, subscriptions, clients.

The hard part: I want the composition itself to be data. A component should register what it provides and requires as rows, and reducers should infer the topology, the order of execution and the capabilities available - rather than an operator wiring it by hand or a config file declaring it.

Constraints I already know from using SpacetimeDB: reducers cannot execute DDL, so the schema is fixed at publish; there is no GROUP BY, so aggregation happens client-side or in a reducer loop; subscriptions reject column projections, so a subscriber takes whole rows; scheduled reducers exist via ScheduleAt and a table.

Questions:

SELF-DESCRIBING COMPOSITION. Design the tables that let a component register itself (identity, provided ports, required ports, version, tier, capabilities) and let a reducer compute: which components are satisfiable, which are blocked, and in what order they must start. What columns, what primary keys, what indexes, and which reducers write them? Is a reducer-per-relationship better than one reducer that recomputes the whole graph?

INFERENCE WITHOUT DDL. Since a reducer cannot create tables, how does a system whose shape is discovered at runtime live inside a fixed schema? Options to compare: a generic edge table (from, to, kind), an entity-attribute-value table, a single typed payload column (JSON), or one table per known component kind. Which of these can still be indexed usefully in SpacetimeDB, and what does each cost in query complexity?

EXECUTION ORDER AS A REDUCER. If start order must be inferred, is it correct to compute a topological order inside a reducer and write it to a table, or should each component reducer check its own preconditions and refuse until they hold? Compare the failure modes: a stale order table vs a thundering herd of refusals.

TRACING END TO END. To observe what every component did from start to finish, what is the right event model: one append-only event table with a component column, one table per component, or a span table (trace id, span id, parent id, start, end)? Which survives a subscription that cannot project columns, and how do you reconstruct a causal chain without GROUP BY?

COMPOSITION OF PRODUCTS. Each product is a set of tables and reducers. Composing product A with product B: is the accepted pattern namespacing by table prefix, one database per product with a bridge, or a shared schema with a product column? What breaks in SpacetimeDB specifically for each?

LIMITS. What can SpacetimeDB genuinely not express here - dynamic schemas, cross-database joins, reducer-to-reducer transactions, exactly-once effects into an external loop - and what is the correct workaround for each?

Prefer concrete table definitions, reducer signatures and index choices over architectural prose. Assume the reader is implementing this and will hit the limits.

Read more
Finished
9s
Researching SpacetimeDB constraints for the composition model
site.spacetimedb.com docs subscriptions SQL joins select columns group by indexes views
site.spacetimedb.com docs inter database communication reducers transaction atomic
site.spacetimedb.com docs Rust tables indexes composite ScheduleAt reducer context
Reducer Context - SpacetimeDB docs
spacetimedb
Indexes - SpacetimeDB docs
spacetimedb
Overview | SpacetimeDB docs
spacetimedb
+12 more
Checking SpacetimeDB documentation for reducer limits and indexing tradeoffs
https://spacetimedb.com/docs/databases/inter-database-communication
https://spacetimedb.com/docs/tables/indexes/
Indexes | SpacetimeDB docs
spacetimedb
Inter-Database Communication | SpacetimeDB docs
spacetimedb

Use a fixed, typed composition graph whose rows describe components, offers, requirements, verified capabilities and inferred bindings. Reducers compute a versioned plan; clients perform external startup and report readiness. Do not make every component repeatedly attempt startup until something succeeds.

The important distinction is inference versus policy: reducers can infer compatibility and dependency order, but cannot infer whether an untrusted component deserves a privilege, whether two compatible providers are interchangeable, or whether “latest version” is the right selection policy. Those rules must be published as data or code, even when nobody wires individual edges manually.

The design below targets the current SpacetimeDB 2.0 documentation. Two relevant corrections to common assumptions:

Subscription SQL supports whole-row projections and constrained two-table joins; ordinary query SQL has broader projection support.
spacetimedb

Current documentation includes asynchronous inter-database communication with outbox tables. It provides exactly-once receiving-reducer invocation per outbox row, but not an atomic commit across both databases.
spacetimedb

1. Self-describing composition
Tables: declarations, evidence, inference and execution

Use a composition_id to scope one independently resolved ecosystem. Use single-column IDs as primary keys; enforce logical uniqueness in reducers where a separate unique constraint is not declared.

The following are logical table definitions. Id means u64; enum columns below are no-payload enums.

Table	Columns	Primary key; B-tree indexes
composition	id, owner: Identity, input_revision: u64, active_plan_revision: u64, policy_revision: u64, status	PK id
component	id, composition_id, instance_key: String, owner: Identity, kind, version: String, tier: u8, declaration_revision: u64, enabled: bool, lease_until_us: i64	PK id; (composition_id, enabled); (composition_id, instance_key); (composition_id, lease_until_us)
port_offer	id, composition_id, component_id, port: String, version: String, contract_hash: String, scope_key: String, capacity: u32, enabled: bool	PK id; (composition_id, port); component_id
port_requirement	id, composition_id, component_id, slot: String, port: String, version_range: String, contract_hash: String, scope_key: String, min_count: u32, max_count: u32, phase, access_mode	PK id; (composition_id, port); component_id
capability_claim	id, component_id, capability: String, value: String	PK id; (component_id, capability)
capability_fact	id, component_id, capability: String, value: String, verifier: Identity, expires_us: i64, evidence_hash: String	PK id; (component_id, capability)
requirement_capability	id, requirement_id, capability: String, operator, expected: String	PK id; requirement_id
selection_policy	id, composition_id, port: String, rule, allow_rebind: bool, revision: u64	PK id; (composition_id, port)
binding	id, composition_id, plan_revision, requirement_id, offer_id, consumer_id, provider_id	PK id; (composition_id, plan_revision); (plan_revision, consumer_id); (plan_revision, provider_id)
component_resolution	id, composition_id, plan_revision, component_id, status, start_wave: u32, reason_code, explanation: String	PK id; (composition_id, plan_revision); (plan_revision, status)
resolution_issue	id, composition_id, plan_revision, component_id, requirement_id?, code, related_component_id?, detail: String	PK id; (composition_id, plan_revision); component_id
component_runtime	component_id, composition_id, generation: u64, state, observed_plan_revision, last_report_seq, last_report_us	PK component_id; (composition_id, state)
start_intent	id, composition_id, component_id, plan_revision, generation, assigned_host: Identity, state, lease_until_us: i64	PK id; (assigned_host, state); (composition_id, component_id)

Use integer microseconds for indexed deadlines. Current index documentation lists Timestamp, ScheduleAt, floating-point values, nested structs and vectors as unsupported index keys; integers, strings, identities and no-payload enums are supported. Composite indexes work by their leftmost prefixes.
spacetimedb

Keep free-form capability values out of generic comparison logic unless their type is declared. For example, define memory.shared as boolean and protocol.slmp.major as integer in a fixed capability-definition catalogue.

Registration must be atomic

A component should register its declaration as one bounded unit, not leave half of its requirements visible between calls.

Proposed reducer interfaces:

rust
#[reducer]
pub fn register_component(
    ctx: &ReducerContext,
    composition_id: u64,
    expected_input_revision: u64,
    declaration: ComponentDeclaration,
) -> Result<(), String>;

#[reducer]
pub fn renew_component(
    ctx: &ReducerContext,
    component_id: u64,
    generation: u64,
) -> Result<(), String>;

#[reducer]
pub fn attest_capability(
    ctx: &ReducerContext,
    component_id: u64,
    fact: CapabilityFactInput,
) -> Result<(), String>;

#[reducer]
pub fn reconcile(
    ctx: &ReducerContext,
    composition_id: u64,
    expected_input_revision: u64,
) -> Result<(), String>;

#[reducer]
pub fn claim_start(
    ctx: &ReducerContext,
    intent_id: u64,
    generation: u64,
) -> Result<(), String>;

#[reducer]
pub fn report_runtime(
    ctx: &ReducerContext,
    component_id: u64,
    generation: u64,
    report_seq: u64,
    state: RuntimeState,
) -> Result<(), String>;

These are proposed signatures; the input structures must be declared as SpacetimeDB serialisable types.

Authentication rules:

A component can update only its own registration.

Claims do not become verified capabilities automatically.

A trusted verifier or approved handshake establishes capabilities used for privileged binding.

The registering client does not choose its own lease duration without a server policy cap.

A startup host can report only components assigned to it.

What reconcile computes

For each enabled, live component:

Find offers matching each requirement’s port and composition.

Check version range, contract identity, scope and verified capability predicates.

Enforce cardinality and capacity constraints.

Apply the selection policy.

Construct provider-to-consumer startup edges.

Compute satisfiability and diagnose cycles.

Write bindings, resolution rows and startup waves under a new plan revision.

Atomically move active_plan_revision to the completed revision.

Useful resolution statuses:

text
Satisfiable
BlockedMissing
BlockedAmbiguous
BlockedCapability
BlockedCapacity
BlockedCycle
BlockedDependency
Disabled

Do not confuse Satisfiable with Ready: the first describes the plan; the second is an observed runtime state.

Provider choice can be a constraint problem

Independent “pick the best provider for each requirement” decisions can fail when offers have limited capacity. A greedy resolver might allocate the only provider suitable for consumer B to consumer A, even though A has another candidate.

Start with this explicit boundary:

Unlimited, interchangeable providers: local deterministic selection.

Exclusive/capacity-limited providers: deterministic matching.

Rich alternatives, preferences and dependency-dependent availability: bounded constraint solving, with a declared search limit.

Return BlockedAmbiguous when policy does not establish a winner. Do not silently use row insertion order.

One reducer per relationship?

Use relationship-specific mutation reducers if useful for permissions, but one reconciliation implementation.

For a modest composition graph, recomputing the whole graph after a coalesced change is simpler and safer than maintaining many independently derived edge states. Registration can mark the composition dirty and insert one scheduled reconciliation task.

For larger graphs, recompute the affected dependency closure using reverse indexes. Keep a full-rebuild verifier for tests and repair.

Do not reconcile on every telemetry update: only on topology-affecting changes, capability expiry, policy changes and relevant runtime transitions.

2. Inference without DDL

Runtime discovery changes rows, not table definitions.

Compare the representations
Representation	Useful indexing	Cost	Recommendation
Generic edge table: from, to, kind	Source, destination, kind and composition	Each edge kind needs separate semantic validation; awkward requirement alternatives	Good for inferred links and supplementary relationships
EAV	Entity/key and typed scalar value columns	Many rows, repeated lookups, weak mandatory-field/type guarantees	Restrict to optional metadata
One JSON/string payload	Envelope columns only; no automatic index on nested JSON paths	Parsing/scanning for every semantic query; large whole-row updates	Use for opaque configuration, not topology
Table per known kind	Strong typed columns and direct indexes	New kinds require module/schema publication; duplicated common fields	Use for specialised domain state
Typed common tables plus specialised payloads	Indexed topology and extensible detail	Requires a stable core vocabulary	Best fit

SpacetimeDB indexes do not automatically index nested structures or vectors. If a payload field affects selection, materialise it into a supported scalar column or a normalised child table.
spacetimedb

Recommended split:

text
Common typed core:
  component, offer, requirement, capability, binding, runtime

Specialised domain data:
  observation, suggestion, modifier, bookmark_job, model_request, ...

Opaque extensions:
  configuration_blob(component_id, schema_id, bytes, hash)

Do not encode a port name only inside JSON and then expect subscription queries or indexes to find all providers efficiently.

This models new component instances and new declared ports without DDL. It does not enable registration of arbitrary new reducer implementations.

3. Execution order: infer a plan, then guard every transition

Use both approaches, but for different jobs:

Topological planning provides discovery, diagnosis and startup waves.

Preconditions at execution time prevent stale plans from authorising invalid work.

Distinguish dependency kinds

Not every relationship is a startup edge.

text
RequiredBeforeStart
RequiredWhileRunning
Optional
LateBound
DataFlow
Advisory

A game → observer → model → modifier → game feedback cycle is a valid runtime dataflow cycle. It must not automatically become an impossible startup cycle.

Run topological sorting only over the hard startup graph.

Algorithm

For the selected hard-dependency graph:

Run strongly connected component detection.

Mark nontrivial hard cycles, including self-loops, as blocked.

Propagate blockage to hard dependants.

Run Kahn’s algorithm over the remaining DAG.

Store start_wave = 0, 1, 2, …; components in a wave can start concurrently.

The post-selection graph algorithms are 
𝑂
(
𝑉
+
𝐸
)
O(V+E). Candidate matching and constraint solving are additional work.

If alternatives exist, a cycle in one candidate selection does not prove the ecosystem is unsatisfiable. Try permitted alternatives before declaring BlockedCycle.

Dispatch, do not broadcast “try now”

A reconciliation or advancement reducer creates start_intent rows only for components whose bound startup dependencies are currently ready.

The assigned client:

Claims the intent.

Starts the external component.

Reports readiness using the intent’s generation.

Causes advancement to release downstream work.

claim_start checks:

text
intent.plan_revision == composition.active_plan_revision
intent.generation == component_runtime.generation
all required providers are Ready
capability facts and leases remain valid
caller is assigned host
Failure comparison
Approach	Failure
Order table only	Stale dependencies or expired capabilities can authorise incorrect startup
Preconditions only	Repeated refusals, poor cycle diagnosis and thundering-herd retries
Versioned plan + guarded intents	More bookkeeping, but explicit diagnosis and controlled dispatch

Use per-component generations as fencing tokens. After a restart or rebind, reports from the previous generation cannot declare the new instance ready.

A single reducer can publish a complete plan atomically. If plan construction must span multiple transactions, stage its output and activate it only when its input revision still matches. Reducers provide atomic commit/rollback, but separately invoked reducers do not form one transaction.
spacetimedb

Scheduled reducers are suitable for lease expiry, retry backoff and reconciliation—not high-frequency game-loop scheduling.

4. Tracing: append-only events plus spans and causal links

Use one common event schema, not one table per component. Add a span table for current summaries and a link table for many-to-many causality.

Tables
Table	Columns	Keys/indexes
trace_event	id, composition_id, trace_id: String, span_id: String, component_id, component_generation, producer_seq, kind, message_id, caused_by_event_id?, server_time, source_time_us?, summary, detail_id?	PK id; (composition_id, trace_id); (component_id, component_generation, producer_seq); message_id
trace_span	id: String, composition_id, trace_id, parent_span_id?, component_id, operation, status, start_event_id, end_event_id?	PK id; (composition_id, trace_id); parent_span_id
trace_link	id, composition_id, trace_id, from_event_id, to_event_id, relation	PK id; (composition_id, trace_id); from_event_id; to_event_id
trace_detail	id, schema_id, payload, content_hash	PK id

A parent span is insufficient when one decision combines multiple observations, or one modifier combines several suggestions. Represent those causal relationships as links.

Whole-row subscriptions favour narrow rows

Subscribe to the small event and span rows:

sql
SELECT * FROM trace_event
WHERE composition_id = 7 AND trace_id = 'trace-123'

Fetch or subscribe to detail rows only when needed. Putting screenshots, prompts and full model responses in every event row would make whole-row replication expensive.

Subscription SQL cannot project a few event columns, but a dedicated narrow table—or an authorised view with a declared smaller result shape—provides the intended boundary. Current documentation supports subscribable views.
spacetimedb
+1

Reconstruct without GROUP BY

The client builds:

text
events_by_id
spans_by_id
events_by_trace
outgoing_links_by_event

Then follows parent relationships and explicit causal links.

Use producer sequence numbers for local ordering. Treat timestamps as timing evidence, not proof of cross-process causality. An arrival timestamp cannot establish that two independently produced operations caused one another.

Persist expected lifecycle events:

text
Registered → Resolved → StartIssued → StartClaimed
→ Ready → WorkStarted → WorkCompleted → Stopped

Also retain rejection and timeout events. Configure retention and archival: an append-only application convention is not a reason to retain unlimited telemetry in the live database.

5. Composing products

There is no universal winner. Choose based on transactional and deployment boundaries.

A. One database, prefixed tables

Example:

text
comp_component
comp_binding
video_observation
video_modifier
bookmarks_ingest_job

Advantages:

One transactional boundary.

Direct table access from reducers.

Shared composition catalogue.

No bridge required for local coordination.

Costs:

Products must be integrated into one published module/schema.

Table and reducer names are not isolated merely because source libraries are separate.

Releases, migrations and operational failures are coupled.

Prefix reducer names as well as tables. A prefix is collision avoidance, not an authorisation boundary.

Use this for one tightly integrated product.

B. One database per product, with communication

Advantages:

Independent publication, ownership and lifecycle.

Clearer private-data boundaries.

Independent failure and scaling domains.

Costs:

No transparent cross-database SQL join.

Remote readiness is a replicated fact, not a local instantaneous truth.

Calls and acknowledgements need correlation, expiry and compensation.

Current async IDC requires compatible generated receiver bindings.

SpacetimeDB’s documented async IDC sends calls through typed outbox tables after the sender commits. Receiving calls run later in separate transactions; use on_result and compensation for cross-product workflows.
spacetimedb

Important current limitations include:

Same server/cluster only.

Outbox declarations supported for Rust and TypeScript.

Pending outbox calls cannot be cancelled.

An unreachable target can hold up later calls through that outbox to that target.

Receiver signature changes require coordinated binding/version management.
spacetimedb

For runtime-discovered products, publish a stable receiver such as:

text
receive_protocol_message(
  protocol_version,
  message_id,
  composition_id,
  message_kind,
  payload
)

Its signature is fixed; the discovered destination identity and message contents are data. New arbitrary remote reducer signatures still require new bindings/publication.

C. Shared tables with product_id

Advantages:

Reuse one generic composition implementation.

No table proliferation.

Efficient product-scoped indexes.

Costs:

All reducers must enforce product/tenant scope.

Public table access can expose other products if access is designed poorly.

Every subscription receives the complete matching row.

One product’s specialised fields tend to leak into generic payloads.

Use product_id for logical partitioning within one trust and deployment boundary—not as a substitute for database isolation.

Recommended arrangement

Keep the composition catalogue and orchestration state together. Place specialised products in the same database when atomic coordination is essential; otherwise represent their capabilities through authenticated remote registrations and versioned protocol endpoints.

6. What SpacetimeDB cannot express—and the workaround
Limit	Consequence	Workaround
Runtime DDL from reducers	New component kinds cannot create new physical tables	Generic fixed core; publish schema changes for new specialised storage
Runtime installation of arbitrary reducers	Registering a component does not install executable server logic	Precompiled handlers, bounded data interpreters, or external clients
Transparent cross-database SQL joins	No single query over arbitrary product databases	Materialised local summaries, async IDC or a client-side join
One transaction across separately invoked same-database reducers	Partial workflow progress can exist between calls	One coordinating reducer calling shared helper functions
Automatic rollback of a caught nested error	Earlier mutations can remain if the outer reducer succeeds	Validate first; propagate failure to roll back the entire transaction
Atomic sender/receiver commit through async IDC	Receiver can fail after sender commits	Saga/state machine and on_result compensation
Exactly-once external game input	Database commit cannot prove input consumption	Idempotent target installation, generations, local expiry and evidence-level acknowledgements
Discovery of real privileges from self-report alone	Malicious registrations can lie	Trusted attestation and capability probes
Exact hard-real-time scheduling	Database scheduling cannot replace a game/audio/servo tick	Local real-time actuator; database supplies bounded targets
Arbitrarily large reconciliation in one cheap transaction	Solver work and writes can exceed practical execution budgets	Bounded graph sizes, incremental closure, staged revisioned computation

Same-database reducer composition deserves emphasis: current architecture documentation explicitly says a direct call into another reducer does not create a child transaction. If the called code mutates rows and returns an error that the outer function catches, those mutations can still commit when the outer reducer succeeds. Use ordinary helpers with clear mutation discipline; do not assume nested rollback.
spacetimedb

Async IDC’s exactly-once guarantee is narrower and stronger than “everything is at least once”: it applies to the receiving reducer invocation for each outbox row. It does not extend to an external browser, a physical actuator or an uninstrumented game loop.
spacetimedb

Implementation rule

Make the inferred plan a materialised explanation of what is possible, and make guarded intents the authority for what may happen next. Components declare facts and requirements; reducers resolve them under explicit policy; clients execute; acknowledgements update observed reality.

That keeps the composition discoverable without pretending that declarations alone can establish trust, executable code or physical effects.

15 sources
Show me how reducers infer the dependency graph
Detail the client-side state machine for the plan
Write the SpacetimeDB table definitions in Rust
How do reducers handle conflicting component offers
Show the SQL queries for subscription and sync




Ask a follow-up
Search
Computer
GPT-6.1 Sol
