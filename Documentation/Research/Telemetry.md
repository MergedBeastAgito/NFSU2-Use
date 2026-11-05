chars: 5777
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
1:42 AM

How should SpacetimeDB ingest telemetry from heterogeneous tiers and reconstruct causal chains without GROUP BY or column projections?

Finished
2s

Use SpacetimeDB as a transactional event store plus reducer-built projections, rather than trying to make subscription SQL do analytics. Ingest each tier’s telemetry as a normalized event row, link events explicitly by IDs, and reconstruct causal chains in reducers or client-side state—not through GROUP BY, projections, or multi-hop subscription joins. SpacetimeDB subscriptions return whole rows from one table, do not support column projections, and support at most a two-table indexed join.
spacetimedb

Ingest heterogeneous tiers

Give every tier a typed ingestion reducer, but preserve a common envelope:

event_id: globally unique ID (UUID or content hash).

trace_id: end-to-end causal trace.

span_id / parent_span_id: causal parent.

tier: edge, gateway, service, device, etc.

schema_version: lets each tier evolve independently.

occurred_at, received_at, clock_skew: for ordering and reconciliation.

payload: tier-specific structured data.

Each tier calls a reducer such as ingest_edge_event, ingest_gateway_event, or ingest_device_event. The reducer validates the envelope, assigns canonical IDs, writes the raw event, and updates only the small derived state needed for live operation. SpacetimeDB reducers run transactionally and push resulting changes to subscribed clients, which makes this a good fit for low-latency ingestion rather than batch analytics.
spacetimedb
+1

Model causality explicitly

Do not rely on SQL aggregation to discover causality. Store it:

rust
struct TelemetryEvent {
    #[primary_key]
    event_id: String,
    trace_id: String,
    span_id: String,
    parent_span_id: Option<String>,
    tier: String,
    schema_version: u32,
    occurred_at: Timestamp,
    received_at: Timestamp,
    payload: TierPayload,
}

Then maintain an explicit edge table:

rust
struct CausalEdge {
    #[primary_key]
    edge_id: String,
    trace_id: String,
    parent_event_id: String,
    child_event_id: String,
    relation: String, // "caused", "forwarded", "triggered", "observed"
}

The reducer resolves the parent using parent_span_id, a correlation ID, or a deterministic idempotency key. If a child arrives before its parent, insert it as pending and attach the edge when the parent arrives; use event_id uniqueness to make retries safe.

Reconstruct chains without SQL analytics

Use one of three reconstruction strategies:

Need	Where to do it	Why
Live chain for one trace	Reducer or client	Avoids expensive joins and keeps real-time latency low
Cross-tier lineage view	Materialized CausalEdge table	Subscriptions can replicate full rows efficiently
Historical rollups	Ad hoc SQL / external warehouse	SpacetimeDB is optimized for transactional, real-time workloads, not heavy analytical processing 
github

For a live trace, subscribe to the relevant tables and assemble the tree locally:

sql
SELECT * FROM TelemetryEvent WHERE trace_id = {trace_id}
sql
SELECT * FROM CausalEdge WHERE trace_id = {trace_id}

These are valid subscription-style queries because each returns complete rows from a single table and filters with an indexed column. The client or a reducer then builds the parent-child tree from parent_event_id → child_event_id.
spacetimedb

For a two-hop relationship, subscriptions may join two tables, but both join columns must be indexed:

sql
SELECT e.*
FROM CausalEdge edge
JOIN TelemetryEvent e
  ON edge.child_event_id = e.event_id
WHERE edge.trace_id = {trace_id}

This returns complete TelemetryEvent rows, not a projected “chain view.”
spacetimedb

Handle late, duplicate, and out-of-order events

Idempotency: make event_id the primary key; repeated ingestion updates or no-ops rather than duplicates.

Ordering: order by (trace_id, occurred_at, span_id), but treat occurred_at as logical time; retain received_at for ingestion diagnostics.

Late parents: store orphan events in a PendingEvent table and promote them when the parent reducer arrives.

Schema drift: keep schema_version in the event and normalize tier payloads in reducers, not in SQL.

Retention: archive cold events outside SpacetimeDB if you need long-horizon analytics; keep hot traces and current causal links in the live tables.

Avoid the SQL trap

The key rule is: subscriptions replicate state; reducers compute causality. SpacetimeDB’s subscription API intentionally restricts projections and complex joins because it is a real-time replication mechanism, while its ad hoc query API supports projections and COUNT but is not a substitute for a warehouse-style analytical engine.
spacetimedb

So the architecture is:

Each tier emits a typed event through its own reducer.

Reducers normalize, deduplicate, and write explicit causal edges.

Clients subscribe to full event and edge rows for a trace.

The causal chain is reconstructed in memory from the linked rows.

Aggregate or historical analysis is exported to an analytical store.

This gives you consistent, replayable causal chains without forcing GROUP BY, projections, or unsupported multi-table joins into SpacetimeDB’s subscription layer.
spacetimedb
+1

10 sources
Build a SpacetimeDB causal-chain engine: traverse event trees in Rust/TypeScript reducers without GROUP BY
Computer
SpacetimeDB telemetry ingestion design: handle heterogeneous tiers, stream updates, and bypass subscription limits
Computer
Show me how to write the Rust reducer code
How to handle clock drift between heterogeneous tiers
What analytical store should we export SpacetimeDB to
⠴




Ask a follow-up
Search
Computer
Model
