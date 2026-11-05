---
title: "Observation envelope across WASM and DOM"
thread: 6a2759f0-264f-4c78-b5a4-b4cf69754626
project: DELO
kind: research
date: 2026-11-05
status: captured
source: https://www.perplexity.ai/search/6a2759f0-264f-4c78-b5a4-b4cf69754626
captured: 2026-11-05T05:22:43
segments: 1
collapsed: false
steps: 7
sources: 15
---

# Observation envelope across WASM and DOM

| Field         | Value |
| ------------- | ----- |
| Thread        | `6a2759f0-264f-4c78-b5a4-b4cf69754626` |
| Project       | DELO |
| Source        | https://www.perplexity.ai/search/6a2759f0-264f-4c78-b5a4-b4cf69754626 |
| Captured      | 2026-11-05T05:22:43 |
| Segments      | 1 |
| Trace steps   | 7 |
| Cited sources | 15 |
| Characters    | 22569 |

## Transcript

DELO
Today 5:19 AM

Next question, same system. This is the crux of generalising beyond WebAssembly.

Two targets must share ONE observation envelope:
(a) a WebAssembly game: fields read at absolute addresses in the module's linear memory, plus a cue found in captured frames.
(b) an ordinary web application: a DOM node's text, an input's value, a button's state, an accessibility-tree node, a canvas region.

Questions:

Design the observation envelope. Give the JSON Schema (2020-12) for one observation record that can carry both, with a discriminator on source kind. Show two concrete instances: one memory field, one DOM text node.

What belongs in the envelope versus the adapter? Specifically: does an address belong in the record, in the adapter's config, or nowhere? Same for a CSS selector.

Provenance: an observation must say how it was obtained and how much to trust it (memory read vs OCR vs accessibility tree vs pixel heuristic). Give the field(s) and the vocabulary for confidence.

Time: a 120 Hz loop and a slow pipeline produce observations at different rates. What time fields does the envelope need (sample time, sequence, validity window) so a consumer can tell a stale observation from a fresh one without knowing the producer?

Synthetic data must flow through the same envelope as measured data. Where does that flag live and what does it change downstream?

Give the equivalent SpacetimeDB table definition for this envelope, with the indexes a subscriber would need to ask "latest observation of kind X for target Y".

What is the smallest conformance suite for the envelope — the tests that catch a producer writing a record a consumer cannot read?

Prefer schema and table definitions over prose.

Read more

Use one record per semantic field, with a typed value, a source discriminator, explicit provenance, and an expiring time interval. The record says what was observed and how; adapter configuration says how to locate it.

The central rule is separation: source location, measurement quality, producer authority and freshness are different concepts. Do not collapse them into one “confidence” number.

1. Observation envelope: JSON Schema 2020-12

This schema covers memory, DOM, accessibility, canvas pixels, compositor pixels and derived observations. A detected cue or OCR result is Derived, with references to its input observations.

Application keys follow your PascalCase convention. JSON Schema keywords retain their standard spelling.

json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "urn:agentic:observation:1",
  "title": "Agentic Observation",
  "type": "object",
  "additionalProperties": false,
  "required": [
    "SchemaVersion",
    "ObservationID",
    "TargetID",
    "StreamID",
    "ProducerID",
    "EpochID",
    "Sequence",
    "FieldID",
    "Status",
    "Value",
    "Source",
    "Provenance",
    "Quality",
    "Time"
  ],
  "properties": {
    "SchemaVersion": { "const": 1 },
    "ObservationID": { "$ref": "#/$defs/ID" },
    "TargetID": { "$ref": "#/$defs/ID" },
    "StreamID": { "$ref": "#/$defs/ID" },
    "ProducerID": { "$ref": "#/$defs/ID" },
    "EpochID": { "$ref": "#/$defs/ID" },
    "Sequence": { "$ref": "#/$defs/UInt64" },
    "FieldID": { "$ref": "#/$defs/ID" },
    "Unit": { "type": "string", "minLength": 1 },
    "Status": {
      "enum": [
        "Valid",
        "Missing",
        "Unsupported",
        "PermissionDenied",
        "ReadError"
      ]
    },
    "Value": { "$ref": "#/$defs/Value" },
    "Source": { "$ref": "#/$defs/Source" },
    "Provenance": { "$ref": "#/$defs/Provenance" },
    "Quality": { "$ref": "#/$defs/Quality" },
    "Time": { "$ref": "#/$defs/Time" }
  },
  "allOf": [
    {
      "if": {
        "properties": { "Status": { "const": "Valid" } }
      },
      "then": {
        "properties": { "Value": { "not": { "type": "null" } } }
      },
      "else": {
        "properties": { "Value": { "type": "null" } }
      }
    }
  ],
  "$defs": {
    "ID": {
      "type": "string",
      "minLength": 1,
      "maxLength": 256
    },
    "UInt64": {
      "type": "string",
      "pattern": "^(0|[1-9][0-9]{0,19})$"
    },
    "Value": {
      "oneOf": [
        { "type": "null" },
        {
          "type": "object",
          "additionalProperties": false,
          "required": ["Type", "Data"],
          "properties": {
            "Type": { "const": "Number" },
            "Data": { "type": "number" }
          }
        },
        {
          "type": "object",
          "additionalProperties": false,
          "required": ["Type", "Data"],
          "properties": {
            "Type": { "const": "Text" },
            "Data": { "type": "string" }
          }
        },
        {
          "type": "object",
          "additionalProperties": false,
          "required": ["Type", "Data"],
          "properties": {
            "Type": { "const": "Boolean" },
            "Data": { "type": "boolean" }
          }
        },
        {
          "type": "object",
          "additionalProperties": false,
          "required": ["Type", "ArtifactID", "MediaType"],
          "properties": {
            "Type": { "const": "Artifact" },
            "ArtifactID": { "$ref": "#/$defs/ID" },
            "MediaType": { "type": "string", "minLength": 1 }
          }
        }
      ]
    },
    "Source": {
      "type": "object",
      "additionalProperties": false,
      "required": ["Kind", "BindingID", "BindingRevision"],
      "properties": {
        "Kind": {
          "enum": [
            "WASMMemory",
            "DOM",
            "Accessibility",
            "CanvasPixels",
            "CompositorPixels",
            "Derived"
          ]
        },
        "BindingID": { "$ref": "#/$defs/ID" },
        "BindingRevision": { "$ref": "#/$defs/ID" },
        "LayoutID": { "$ref": "#/$defs/ID" },
        "Read": {
          "enum": [
            "TextContent",
            "InnerText",
            "Value",
            "Checked",
            "Disabled",
            "Attribute"
          ]
        },
        "Property": { "type": "string", "minLength": 1 },
        "ExtractorID": { "$ref": "#/$defs/ID" },
        "Inputs": {
          "type": "array",
          "minItems": 1,
          "uniqueItems": true,
          "items": { "$ref": "#/$defs/ID" }
        }
      },
      "oneOf": [
        {
          "properties": { "Kind": { "const": "WASMMemory" } },
          "required": ["LayoutID"],
          "not": {
            "anyOf": [
              { "required": ["Read"] },
              { "required": ["Property"] },
              { "required": ["ExtractorID"] },
              { "required": ["Inputs"] }
            ]
          }
        },
        {
          "properties": { "Kind": { "const": "DOM" } },
          "required": ["Read"],
          "not": {
            "anyOf": [
              { "required": ["LayoutID"] },
              { "required": ["ExtractorID"] },
              { "required": ["Inputs"] }
            ]
          },
          "if": {
            "properties": { "Read": { "const": "Attribute" } }
          },
          "then": { "required": ["Property"] },
          "else": { "not": { "required": ["Property"] } }
        },
        {
          "properties": { "Kind": { "const": "Accessibility" } },
          "required": ["Property"],
          "not": {
            "anyOf": [
              { "required": ["LayoutID"] },
              { "required": ["Read"] },
              { "required": ["ExtractorID"] },
              { "required": ["Inputs"] }
            ]
          }
        },
        {
          "properties": {
            "Kind": {
              "enum": ["CanvasPixels", "CompositorPixels"]
            }
          },
          "not": {
            "anyOf": [
              { "required": ["LayoutID"] },
              { "required": ["Read"] },
              { "required": ["Property"] },
              { "required": ["ExtractorID"] },
              { "required": ["Inputs"] }
            ]
          }
        },
        {
          "properties": { "Kind": { "const": "Derived" } },
          "required": ["ExtractorID", "Inputs"],
          "not": {
            "anyOf": [
              { "required": ["LayoutID"] },
              { "required": ["Read"] },
              { "required": ["Property"] }
            ]
          }
        }
      ]
    },
    "Provenance": {
      "type": "object",
      "additionalProperties": false,
      "required": ["Origin", "Lineage"],
      "properties": {
        "Origin": {
          "enum": [
            "Measured",
            "Replay",
            "Simulation",
            "Generated",
            "Mixed"
          ]
        },
        "Lineage": {
          "type": "array",
          "uniqueItems": true,
          "items": { "$ref": "#/$defs/ID" }
        }
      }
    },
    "Quality": {
      "type": "object",
      "additionalProperties": false,
      "required": ["Method", "Confidence"],
      "properties": {
        "Method": {
          "enum": [
            "DirectRead",
            "BrowserComputed",
            "PixelHeuristic",
            "OCR",
            "ModelInference",
            "DeterministicTransform"
          ]
        },
        "Confidence": {
          "oneOf": [
            { "type": "null" },
            {
              "type": "object",
              "additionalProperties": false,
              "required": ["Score", "Meaning", "ProfileID"],
              "properties": {
                "Score": {
                  "type": "number",
                  "minimum": 0,
                  "maximum": 1
                },
                "Meaning": {
                  "enum": [
                    "HeuristicScore",
                    "UncalibratedProbability",
                    "CalibratedProbability"
                  ]
                },
                "ProfileID": { "$ref": "#/$defs/ID" }
              }
            }
          ]
        }
      }
    },
    "Time": {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "ClockID",
        "SampleStartUS",
        "SampleEndUS",
        "ValidForUS",
        "ClockUncertaintyUS"
      ],
      "properties": {
        "ClockID": { "$ref": "#/$defs/ID" },
        "SampleStartUS": { "$ref": "#/$defs/UInt64" },
        "SampleEndUS": { "$ref": "#/$defs/UInt64" },
        "ValidForUS": { "$ref": "#/$defs/UInt64" },
        "ClockUncertaintyUS": { "$ref": "#/$defs/UInt64" }
      }
    }
  }
}

JSON Schema validates structure. A semantic validator must additionally enforce:

All decimal strings fit u64.

SampleStartUS <= SampleEndUS.

Time arithmetic does not overflow.

FieldID has the declared value type/unit in the field catalogue.

Pixel-source fields use artifact values.

Derived input references and binding revisions exist.

Layout identity is compatible with the target.

Provenance cannot be downgraded from synthetic or mixed to measured.

Draft 2020-12 separates format annotation from format assertion; this design deliberately uses explicit numeric timestamps rather than depending on a validator treating format: "date-time" as an assertion.
json-schema

Instance A: game memory field
json
{
  "SchemaVersion": 1,
  "ObservationID": "Observation-9001",
  "TargetID": "NFSU2-Tab-1",
  "StreamID": "VehicleState",
  "ProducerID": "NFSU2Reader",
  "EpochID": "Document-17",
  "Sequence": "9001",
  "FieldID": "Vehicle.Speed",
  "Unit": "km/h",
  "Status": "Valid",
  "Value": {
    "Type": "Number",
    "Data": 76.4
  },
  "Source": {
    "Kind": "WASMMemory",
    "BindingID": "Vehicle.Speed",
    "BindingRevision": "MemoryMap-3",
    "LayoutID": "HostWASM-SHA256-ABC"
  },
  "Provenance": {
    "Origin": "Measured",
    "Lineage": []
  },
  "Quality": {
    "Method": "DirectRead",
    "Confidence": null
  },
  "Time": {
    "ClockID": "ObserverClock-1",
    "SampleStartUS": "81000000",
    "SampleEndUS": "81000020",
    "ValidForUS": "50000",
    "ClockUncertaintyUS": "100"
  }
}
Instance B: DOM text
json
{
  "SchemaVersion": 1,
  "ObservationID": "Observation-120",
  "TargetID": "Checkout-Tab-2",
  "StreamID": "CheckoutState",
  "ProducerID": "DOMObserver",
  "EpochID": "Document-8",
  "Sequence": "120",
  "FieldID": "Checkout.TotalText",
  "Status": "Valid",
  "Value": {
    "Type": "Text",
    "Data": "€42.00"
  },
  "Source": {
    "Kind": "DOM",
    "BindingID": "Checkout.Total",
    "BindingRevision": "CheckoutBindings-4",
    "Read": "TextContent"
  },
  "Provenance": {
    "Origin": "Measured",
    "Lineage": []
  },
  "Quality": {
    "Method": "DirectRead",
    "Confidence": null
  },
  "Time": {
    "ClockID": "ObserverClock-2",
    "SampleStartUS": "93000000",
    "SampleEndUS": "93000050",
    "ValidForUS": "5000000",
    "ClockUncertaintyUS": "100"
  }
}
2. Addresses and selectors belong in versioned bindings

Neither is a semantic field identifier.

Item	Canonical location
Absolute address or base-plus-offset expression	Adapter binding configuration
WASM build/layout hash	Binding configuration and observation’s LayoutID
CSS selector, shadow-root traversal, frame locator	Adapter binding configuration
Resolved address/node reference	Optional diagnostic evidence
Semantic field name	Observation’s FieldID
Binding identity/version	Observation’s Source

For example:

json
{
  "BindingID": "Vehicle.Speed",
  "Revision": "MemoryMap-3",
  "Kind": "WASMMemory",
  "LayoutID": "HostWASM-SHA256-ABC",
  "Address": "123456",
  "StorageType": "F32LE"
}
json
{
  "BindingID": "Checkout.Total",
  "Revision": "CheckoutBindings-4",
  "Kind": "DOM",
  "Selector": "[data-testid='total']",
  "Read": "TextContent"
}

Persist immutable binding revisions so an observation remains explainable later. If locators are sensitive, authorise access separately from observation access.

Do not send addresses and selectors to a decision model unless its task specifically requires them.

3. Provenance and confidence

Use three independent concepts:

Question	Field
Where did this data originate?	Provenance.Origin
How was this value obtained?	Source and Quality.Method
What does an uncertainty score mean?	Quality.Confidence

Recommended mappings:

text
WASM field read       → DirectRead
DOM text/value read   → DirectRead
Computed AX property  → BrowserComputed
Cyan cue detector     → PixelHeuristic
Recognised text       → OCR
Learned interpretation→ ModelInference
Unit conversion       → DeterministicTransform

Confidence: null means “no score is supplied,” not “untrusted.”

Do not assign 1.0 to memory merely because the bytes were read successfully. The address map might be wrong. Similarly, browser-computed accessibility semantics describe the browser’s interpretation of the page, not proof that the page’s business meaning is correct.

Producer authorisation and attestation belong in authenticated producer/binding registries. A producer cannot establish its own trustworthiness by writing "Confidence": 1.

4. Time: comparable clocks, not assumed clocks

The required fields serve different purposes:

EpochID: resets identity after navigation, restart or replay restart.

Sequence: ordering within (ProducerID, StreamID, EpochID).

SampleStartUS / SampleEndUS: acquisition interval.

ClockID: identifies the clock domain.

ClockUncertaintyUS: uncertainty of capture timing within that domain.

ValidForUS: producer’s maximum validity policy after acquisition.

A consumer needs a mapping from ClockID to its own clock. That mapping is infrastructure, not knowledge of whether the producer is a game or a pipeline.

With mapped capture-end time 
𝑡
^
𝑒
t
^
e
	​

, combined mapping/sample uncertainty 
𝜖
ϵ, and current time 
𝑡
t, accept conservatively only when:

𝑡
≤
𝑡
^
𝑒
+
ValidForUS
⁡
−
𝜖
.
t≤
t
^
e
	​

+ValidForUS−ϵ.

Consumers may impose a shorter maximum age. If the clock is unknown or its mapping expired, classify freshness as Unknown; do not call the observation fresh merely because it just arrived.

Server reception time is useful for transport monitoring, but a replayed old observation can arrive now. Reception time cannot replace sample time.

A sample can be structurally Valid yet currently stale. Staleness is evaluated by consumers; it is not an immutable measurement status.

5. Synthetic data

Put it in Provenance.Origin, not a source-kind branch.

This permits:

text
Measured DOM
Replayed DOM
Simulated WASM memory
Generated pixels
Measured pixels → derived OCR
Mixed measured/synthetic inputs → derived value

Rules:

Replay preserves historical lineage but uses the current execution’s epoch and timing semantics.

Derived results inherit input provenance; mixed inputs produce Mixed.

Live actuation accepts only authorised streams/provenance.

Synthetic streams may target simulation actuators, not live targets.

Database reducers derive or validate provenance against registered stream permissions.

A caller-supplied origin string alone is not a safety boundary.

6. SpacetimeDB representation and “latest” lookup

Use one validated canonical envelope plus indexed routing columns. This avoids duplicating the entire JSON union in a second independently evolving type system.

The database reducer must parse and validate the envelope and derive every indexed column itself. Never accept independent caller-supplied index fields.

Table definitions
rust
#![allow(non_snake_case)]

use spacetimedb::Identity;

#[spacetimedb::table(
    accessor = observation,
    index(
        accessor = TargetKind,
        btree(columns = [TargetID, SourceKind])
    ),
    index(
        accessor = TargetField,
        btree(columns = [TargetID, FieldID])
    ),
    index(
        accessor = StreamSequence,
        btree(columns = [StreamID, EpochID, Sequence])
    )
)]
pub struct Observation {
    #[primary_key]
    pub ObservationID: String,

    pub TargetID: String,
    pub StreamID: String,
    pub Producer: Identity,
    pub ProducerID: String,
    pub EpochID: String,
    pub Sequence: u64,

    pub FieldID: String,
    pub SourceKind: String,
    pub Origin: String,
    pub Status: String,

    pub ClockID: String,
    pub SampleStartUS: u64,
    pub SampleEndUS: u64,
    pub ValidForUS: u64,

    pub ReceivedAtUS: u64,

    // Validated, canonical UTF-8 JSON conforming to SchemaVersion 1.
    pub EnvelopeJSON: String,
}

#[spacetimedb::table(
    accessor = observation_head,
    index(
        accessor = TargetKind,
        btree(columns = [TargetID, SourceKind])
    )
)]
pub struct ObservationHead {
    #[primary_key]
    pub HeadID: String,

    pub TargetID: String,
    pub SourceKind: String,
    pub FieldID: String,
    pub StreamID: String,
    pub EpochID: String,

    pub ObservationID: String,
    pub Sequence: u64,

    // Whole-row subscriptions receive the current envelope directly.
    pub EnvelopeJSON: String,
}

Use an unambiguous encoding or hash of this tuple for HeadID:

text
(TargetID, StreamID, EpochID, FieldID, SourceKind)

The macro shape targets SpacetimeDB 2.x; pin the actual module SDK version. Keep these tables private and expose authorised views, or mark them public only within a deliberately public trust boundary.

Why a head table?

Current subscription documentation says subscriptions cannot use ORDER BY or LIMIT. A materialised head avoids relying on unsupported “latest row” subscription SQL.
spacetimedb

The authorised subscription equivalent is:

sql
SELECT * FROM ObservationHead
WHERE TargetID = 'Checkout-Tab-2'
  AND SourceKind = 'DOM'

This returns the latest observation for each matching field/stream/epoch. Subscribe only to active epochs, or remove old head rows when an epoch retires.

Do not define “latest across all producers” by comparing unrelated sequence numbers. Cross-stream arbitration needs a selected authoritative stream or comparable mapped sample times.

Ingestion reducer
text
PublishObservation(EnvelopeJSON)
  1. Authenticate producer.
  2. Validate JSON Schema and semantic constraints.
  3. Validate target, stream, epoch, binding and provenance.
  4. Reject conflicting duplicate ObservationID.
  5. Insert immutable Observation.
  6. Advance its head only if Sequence increases.
  7. Commit atomically.

Identical retries are idempotent. Out-of-order records may be retained for audit without moving the head.

The EnvelopeJSON choice sacrifices nested database querying. If a value needs indexed numeric/text queries, materialise approved typed value columns or child tables during ingestion. SpacetimeDB supports typed composite columns as an alternative, but those still require an explicit JSON-to-database mapping.
spacetimedb

7. Smallest conformance suite

Use one shared fixture corpus against every producer and consumer implementation.

Test	Required assertion
Test	Required assertion
Valid source variants	Memory, DOM, AX, pixel artifact and derived records validate and decode
Discriminator rejection	Unknown source kind, wrong branch fields and missing required fields fail
Typed-value round trip	Number, Unicode text, boolean, artifact and missing value survive JSON → database → client
Numeric/time boundaries	u64 overflow, reversed intervals and expiry overflow fail
Missing versus actual values	Missing is not decoded as zero, false or empty text
Ordering/idempotency	Duplicate is stable; lower sequence cannot replace a head; new epoch is handled separately
Freshness	Expired, unknown-clock and excessive-uncertainty observations cannot be treated as fresh
Provenance	Replay/synthetic/mixed cannot be relabelled as measured or routed to unauthorised live actuation
Binding compatibility	Unsupported layout/binding revision fails or becomes explicitly unavailable
Subscription contract	Subscriber receives and decodes the actual whole head row, including the current envelope

Include negative fixtures for NaN/infinity in language-side producers: they are not valid JSON numbers and must not silently serialize into a different value.

The decisive compatibility test is not merely “the producer emits valid JSON.” It is:

text
Producer fixture
  → schema + semantic validation
  → database ingestion
  → subscribed head row
  → consumer decode
  → same value, provenance and freshness interpretation

That tests the actual shared envelope rather than three superficially similar representations.

Show the Rust structs for this shared envelope
How do native targets populate absolute address fields
Define the provenance and freshness rules for both
Show how consumer decode validates this schema
Walk through a concrete multi-target sync loop
⠋
