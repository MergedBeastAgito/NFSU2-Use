chars: 22975
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
1:05 AM

Follow-up: here are our real files. Critique them against the four crates you proposed (agentic-port-api, agentic-decision-contract, agentic-decision-runtime, agentic-decision-clef) and say exactly what to rename, split, or add.

A crate manifest, as it exists today:

{
"name": "@agentic/nfsu-driver",
"kind": "capability",
"version": "0.1.0",
"provides": { "port": "agentic.nfsu.drive", "errors": ["browser-unavailable", "navigation-failed", "timeout"] },
"requires": { "ports": ["agentic.browse"] },
"artefact": { "entry": "Target/release/nfsu-driver", "hash": null, "runtime": "rust>=1.88" },
"state": "holds-transport",
"cost": { "coldMs": 40, "warmMs": 3, "network": false },
"verify": "cargo run --release -- --attach <ws> --bind <target> --ticks 1"
}

The observation schema (the ingestion), one field shown:

"speed_kmh": { "type": "number", "source": "memory", "address": "car+0x430", "unit": "km/h", "range": [0, 400] }
"route_cue_x": { "type": "number", "source": "frame", "range": [0, 1] }

There are 16 such fields, each with a source of "memory" (an absolute address in the host's WebAssembly memory) or "frame" (a value derived from a captured screenshot), plus a "derived" block of predicates expressed in prose: "stuck": "speed_kmh < 2 for three consecutive ticks while the throttle is held".

The question schema. Five lanes, each naming the fields it sends and the templates it serves:

"drive":   { "fields": ["speed_kmh","gear","route_cue_x"], "templates": ["freeroam.track_cue","freeroam.cruise"], "cadence": "every tick (150-250 ms)" },
"recover": { "fields": ["speed_kmh","gear","grounded","surface"], "templates": ["recovery.stuck","surface.loss"] },
"corner":  { "fields": ["speed_kmh","heading_degrees","route_cue_x","yaw_rate"], "templates": ["corner.entry"] },
"race":    { "fields": ["speed_kmh","gear","racetype","track","route_cue_x"], "templates": ["race.start"] },
"context": { "fields": [sixteen fields], "templates": ["traffic.ghost"] }

One template, in full:

"freeroam.track_cue": {
"when": "a route cue is on screen",
"questions": {
"action": { "type": "choice", "instructions": "The route arrow is on screen at route_cue_x, where 0.0 is the far LEFT of the view, 1.0 is the far RIGHT and 0.5 is dead ahead. ...", "criteria": { "steer_left": "...", "steer_right": "...", "throttle": "...", "brake": "..." } },
"urgency": { "type": "score", "instructions": "How urgently should this be applied?", "criteria": ["whenever","soon","now","immediately"] }
} }

The repository layout:

App/driver/Source/{Library.rs, Main.rs, Fn/{Decide.rs, Cloudflare.rs, Expressions.rs, Gate.rs}, Struct/{State.rs, Decision.rs, Budget.rs}, Script/Expressions/*.ts}
App/reader/  (cdylib compiled with --import-memory, reads the same addresses, no allocation)
App/{nfsu-drive4.mjs, nfsu-gui4.mjs, nfsu-lane.mjs, nfsu-bus.mjs, analyze_frame.py}
Templates/{Observation.json, Questions.json, README.md}
Documentation/StateCharts.md

Measured facts to respect: clef-flash costs 343-499 input tokens per call depending on lane width and returns no output tokens; a full-state call was 499 tokens, the drive lane 375; latency was network-dominated (268-451 ms) so lanes buy tokens, not speed.

Questions:
A. Where does the manifest belong in your four-crate split - is kind: capability still right for something that provides a port and requires another?
B. The "derived" predicates are prose. What should they be instead so a validator can check a lane against them?
C. Should the lane definition live in the contract crate or stay data? What does compiling it in buy?
D. Our adapter provides agentic.nfsu.drive and requires agentic.browse. Under your collision rule, what should happen if a second game adapter also requires agentic.browse - and what if two provide agentic.nfsu.drive?
E. Give one concrete JSON Schema (draft 2020-12) for a single choice question, so we can adopt it as the template standard.

Read less
Finished
5s

Your files already separate observation, questions and execution conceptually, but they mix three boundaries: portable contracts, NFSU-specific data, and deployment metadata. The main correction is separation, not turning every existing file into a crate.

Keep the NFSU adapter as a fifth, example-specific package that uses the four reusable crates. Its manifest does not belong inside any of those four.

A. Manifest: keep it with the adapter, and distinguish capability from loading mechanism
kind: "capability" is still correct

Providing agentic.nfsu.drive while requiring agentic.browse makes this a composed capability. A dependency on another port does not change that classification.

However, your manifest currently conflates a Rust package, an executable artifact and a capability declaration. Rename it to agentic-capability.json, keep it beside the example’s Cargo.toml, and define its schema in agentic-port-api.

Recommended ownership:

Item	Owner
Manifest schema, port identifiers, provider identities, binding rules	agentic-port-api
This particular manifest	examples/nfsu-driver/agentic-capability.json
Manifest loading and application composition	Host/bootstrap code
Executable launch protocol, if supported	An explicitly versioned transport contract

A concrete revised shape would be:

json
{
  "manifest_version": 1,
  "name": "@agentic/nfsu-driver",
  "kind": "capability",
  "version": "0.1.0",
  "provider_id": "org.agentic.nfsu-driver",
  "provides": [
    {
      "port": "agentic.nfsu.drive",
      "api_version": "0.1.0",
      "errors": [
        "browser-unavailable",
        "navigation-failed",
        "timeout"
      ]
    }
  ],
  "requires": [
    {
      "binding": "browser",
      "port": "agentic.browse",
      "api_requirement": "^0.1",
      "sharing": "exclusive-session"
    }
  ],
  "implementation": {
    "loading": "linked",
    "cargo_package": "nfsu-driver"
  },
  "lifecycle": {
    "scope": "session",
    "holds_transport": true
  }
}

The version numbers above are proposed initial contracts, not assertions about your existing Browse API.

Specific changes to your current fields
Current field	Change
name	Keep as the logical capability-package name; separately identify the Cargo package
provides.port	Change to an array of provided-port declarations with API versions
requires.ports	Change to named dependency slots with compatibility and session-sharing requirements
artefact.entry	Remove for linked-crate loading; retain only for an explicitly executable deployment
artefact.hash: null	Permit only in development manifests; require a real digest in release artifact metadata
runtime: "rust>=1.88"	Move to Cargo as rust-version = "1.88"
state: "holds-transport"	Replace with structured lifecycle and ownership metadata
cost	Move measured timings to benchmark data; keep execution/network policy separate
verify	Move to a smoke-test script with explicit arguments and expected results

Cargo’s rust-version specifies the supported Rust toolchain version; it is not an executable runtime requirement. Cargo also already defines library and binary targets and their paths.
doc.rust-lang
+1

If you actually distribute nfsu-driver as a subprocess, keep an artifact descriptor, but add a transport protocol such as a versioned framed-JSON protocol over stdio. A binary path alone does not explain how the host invokes agentic.nfsu.drive.

Do not silently convert Target/release to target/release until checking your Cargo configuration: it might be intentional. Either way, a development build path should not be the release artifact locator.

Correct the network claim

network: false needs a scope. Your decision path uses Cloudflare, so it cannot describe the whole composed capability as network-free.

Separate:

Permission: outbound network required or permitted.

Scope: adapter-local, browser transport, or decision backend.

Measurements: observed cold/warm time, model latency and token counts.

Your measured 268–451 ms decision latency and 343–499 input tokens belong in benchmark fixtures or reports, not normative promises in the capability manifest.

B. Derived predicates: use a typed expression plus an explicit temporal operator

The existing Observation.json is not yet an ingestion schema in the JSON Schema sense. It is a field catalogue mixing validation, acquisition and derivation.

Split it into:

text
observation.schema.json
observation-sources.json
derived-features.json
1. Pure observation validation
json
{
  "speed_kmh": {
    "type": "number",
    "minimum": 0,
    "maximum": 400,
    "description": "Vehicle speed in kilometres per hour."
  }
}

range is not the JSON Schema numeric constraint. Use minimum and maximum; put acquisition-specific metadata elsewhere.

For route_cue_x, specify what happens when the cue is absent: omit the field, make it nullable, or use an explicit presence flag. Do not encode “not visible” as 0, because 0 already means far left.

2. Game-specific acquisition map
json
{
  "speed_kmh": {
    "source": "memory",
    "base": "car",
    "offset_bytes": 1072,
    "storage_type": "f32",
    "unit": "km/h"
  },
  "route_cue_x": {
    "source": "frame",
    "extractor": "cyan-route-cue-v1",
    "coordinate_frame": "viewport-normalized"
  }
}

Here f32 is illustrative: use the actual storage type from your port.

Also, car+0x430 is base-relative addressing, not a complete absolute address. Document how car is resolved and bind that resolution to the host build. This map stays in the NFSU example.

3. Machine-readable temporal derivation

For this release, I recommend a small, versioned expression AST rather than prose or arbitrary TypeScript:

json
{
  "format_version": 1,
  "features": {
    "stuck": {
      "result_type": "boolean",
      "expression": {
        "op": "consecutive",
        "samples": 3,
        "input": {
          "op": "and",
          "args": [
            {
              "op": "lt",
              "args": [
                { "field": "/speed_kmh" },
                { "literal": 2 }
              ]
            },
            {
              "op": "eq",
              "args": [
                { "field": "/controls/throttle_held" },
                { "literal": true }
              ]
            }
          ]
        }
      },
      "history": {
        "scope": "session",
        "warmup_result": "unknown",
        "reset_on": [
          "session-change",
          "missing-sample",
          "out-of-order-sample"
        ]
      }
    }
  }
}

Define consecutive precisely: true only when the input is true on each of the last three accepted ticks; false when contradicted; unknown during insufficient history or missing inputs.

This resolves an existing ambiguity: does “while the throttle is held” mean held now, or held throughout all three ticks? The example above chooses throughout.

At 150–250 ms per tick, three samples do not represent a fixed elapsed duration. Preserve sample-based semantics if that is what you intended; otherwise introduce a separate duration operator.

CEL is a reasonable later alternative for scalar predicates because it supports checking expressions against declared variables and types. Temporal history would still need your own explicitly defined operators or runtime wrapper.
chromium.googlesource
+1

What the validator should check

The AST makes dependencies discoverable without trusting a handwritten requires list:

text
stuck
  ├── speed_kmh
  └── controls.throttle_held
      + three-sample history

Your current recover lane lists neither throttle state nor stuck. Whether it is invalid depends on the missing recovery.stuck template:

If the template consumes /stuck, add /stuck to the lane and ensure upstream ingestion supplies its dependencies.

If a runtime activation predicate uses /stuck, keep it in the runtime evaluation context; it need not be sent to Clef.

If the model is expected to infer stuckness itself, the current four fields do not establish the declared temporal condition.

Distinguish the feature-computation inputs from the fields transmitted to the model. A lane may send one derived boolean without sending all of its raw history.

C. Lane definitions: keep them as data; compile them into validated execution plans

The contract crate should contain lane types, schemas and semantic validation—not your five lane instances.

Layer	Responsibility
agentic-decision-contract	LaneSpec, question types, expression AST, dependency checks, deterministic plan construction
NFSU data files	drive, recover, corner, race, context, their projections and template assignments
agentic-decision-runtime	Scheduling, history, snapshot selection, budgets, gates, stale-result rejection and fallback
agentic-decision-clef	Encode the selected state/questions for Clef and normalise its response
Rename the lane fields
json
{
  "id": "drive",
  "projection": [
    "/speed_kmh",
    "/gear",
    "/route_cue_x"
  ],
  "templates": [
    "freeroam.track_cue",
    "freeroam.cruise"
  ],
  "schedule": {
    "trigger": "tick",
    "min_interval_ms": 150,
    "max_in_flight": 1
  }
}

Rename fields to projection, use JSON Pointer paths consistently, and replace prose cadence with a scheduling policy. Record the observed 150–250 ms interval separately unless you can enforce both bounds.

Add per-lane response-age and overlap policies. Your network-dominated 268–451 ms calls can outlast a driving tick. The runtime must explicitly choose whether to coalesce requests, skip ticks, permit overlap, or discard late answers.

“Compiling” has two meanings

Compile into the Rust binary: catches bundled-data errors during a build, avoids external-file deployment failures, but requires rebuilding to change lanes.

Compile JSON into an execution plan: resolves paths, type-checks expressions, computes dependencies, generates response schemas and prepares projections.

Choose the second as mandatory, at startup. Optionally run exactly the same compiler in CI or build.rs for embedded defaults.

Compilation cannot prove that prose instructions have no undeclared semantic dependency. Require explicit dependencies for instructions and criteria, and test them through replay.

Your measurements support token reduction, not a speed claim:

Full state: 499 input tokens.

Drive lane: 375 input tokens.

Difference: 124 tokens, approximately 25%.

Observed calls overall: 343–499 input tokens.

Observed latency: 268–451 ms, network-dominated.

Reported output tokens: zero.

Preserve zero output tokens as provider-reported usage. Do not infer that no structured result was returned. The Clef adapter should keep usage counters separate from decoded option probabilities.

D. Collisions: shared requirements are not provider collisions
Two game adapters require agentic.browse

This is not a collision. They are two consumers of the same port contract.

Resolve their named dependency slots independently:

text
nfsu-driver.browser  → chrome-provider / session A
other-game.browser   → chrome-provider / session B

One Browse provider may serve both if it supports multiple sessions.

The actual conflict is two adapters trying to control the same browser target. Your holds-transport field does not currently distinguish connection ownership from target-control ownership.

Add an exclusive session/target lease for mutating control. Allow shared read access only where explicitly supported. Do not allow two game controllers to race Evaluate calls against the same page merely because their Browse dependency resolved successfully.

Two adapters provide agentic.nfsu.drive

Register both by distinct provider identities. Then:

Reject duplicate provider-instance IDs.

Filter by port API compatibility.

Apply an explicit binding.

If more than one compatible provider remains for a singular binding, fail composition/startup.

Do not reject their mere installation, and do not choose by registration order.

If the second adapter drives a different game rather than implementing the NFSU contract, it should provide a different domain port. Extract a generic game-driving port only after defining shared semantics—not simply because both adapters drive games.

E. Adoptable JSON Schema for one choice question

Use kind, replacing your current question type, and make “one mutually exclusive outcome” explicit.

Your current options—steer_left, steer_right, throttle, brake—need a semantic decision first. Steering and throttle can be applied together. A single-choice question would prevent that unless these are intentionally mutually exclusive commands.

Prefer separate steering and longitudinal questions, or define complete action bundles. The schema below is for one mutually exclusive choice.

It uses standard Draft 2020-12 object constraints; required makes fields mandatory and additionalProperties: false closes the object.
json-schema
+1

json
{
  "$schema": "https://json-schema.org/draft/2020-12/schema",
  "$id": "urn:agentic:decision-contract:choice-question:1",
  "title": "Agentic choice question",
  "description": "A mutually exclusive choice with a probability for each option.",
  "type": "object",
  "additionalProperties": false,
  "required": [
    "format_version",
    "id",
    "kind",
    "requires",
    "instructions",
    "criteria",
    "response"
  ],
  "properties": {
    "format_version": {
      "const": 1
    },
    "id": {
      "type": "string",
      "pattern": "^[a-z][a-z0-9_]*(\\.[a-z][a-z0-9_]*)*$"
    },
    "kind": {
      "const": "choice"
    },
    "requires": {
      "type": "array",
      "uniqueItems": true,
      "items": {
        "type": "string",
        "description": "A non-root JSON Pointer into the projected state.",
        "pattern": "^(?:/(?:[^~/]|~[01])*)+$"
      }
    },
    "instructions": {
      "type": "string",
      "minLength": 1
    },
    "criteria": {
      "type": "object",
      "description": "Option IDs mapped to their decision criteria.",
      "minProperties": 2,
      "propertyNames": {
        "pattern": "^[a-z][a-z0-9_]*$"
      },
      "additionalProperties": {
        "type": "string",
        "minLength": 1
      }
    },
    "response": {
      "type": "object",
      "additionalProperties": false,
      "required": [
        "kind",
        "normalization",
        "sum_tolerance"
      ],
      "properties": {
        "kind": {
          "const": "option_probabilities"
        },
        "normalization": {
          "const": "sum_to_one"
        },
        "sum_tolerance": {
          "type": "number",
          "exclusiveMinimum": 0,
          "maximum": 0.01
        }
      }
    }
  }
}

An instance:

json
{
  "format_version": 1,
  "id": "freeroam.track_cue.steering",
  "kind": "choice",
  "requires": ["/route_cue_x"],
  "instructions": "Choose steering from the visible route cue. Its x coordinate is normalized: 0 is far left, 0.5 is centre, and 1 is far right.",
  "criteria": {
    "steer_left": "The route cue indicates a leftward correction.",
    "straight": "The route cue indicates no lateral correction.",
    "steer_right": "The route cue indicates a rightward correction."
  },
  "response": {
    "kind": "option_probabilities",
    "normalization": "sum_to_one",
    "sum_tolerance": 0.001
  }
}

The contract’s semantic validator must additionally:

Resolve every requires path against the lane’s guaranteed projected state.

Generate a response schema requiring exactly the option IDs in criteria.

Validate each returned probability in [0, 1].

Check the sum within sum_tolerance, without silently renormalising.

Verify that the backend supports this response interpretation.

Keep activation outside this single-question schema. Replace the template’s prose "when" with a typed predicate, such as the presence of /route_cue_x, evaluated before dispatch.

Your urgency question is not currently a numeric score: its four named levels make it an ordinal question. Rename it to kind: "ordinal" and specify the ordered levels, or keep score only after defining a numeric range and interpretation.

Exact file moves and splits

Use conventional src/lib.rs and src/main.rs; Cargo supports alternative paths, but its defaults already match this structure.
doc.rust-lang

Current file	Destination and split
Source/Library.rs	examples/nfsu-driver/src/lib.rs: adapter composition and public NFSU API
Source/Main.rs	examples/nfsu-driver/src/main.rs: CLI/bootstrap only
Fn/Decide.rs	Generic orchestration → runtime runner.rs; template selection and action mapping → example policy.rs
Fn/Cloudflare.rs	agentic-decision-clef/src/client.rs, codec.rs, usage.rs
Fn/Expressions.rs	AST/types/checking → contract expression.rs; evaluation/history → runtime expression.rs
Script/Expressions/*.ts	Replace decision predicates with versioned expression data; retain JS/TS only for genuinely browser-specific execution
Fn/Gate.rs	Generic gating → runtime gate.rs; thresholds → example policy data
Struct/State.rs	NFSU fields → example observation.rs; generic snapshot metadata → contract snapshot.rs
Struct/Decision.rs	Backend-neutral answers → contract decision.rs; game commands → example action.rs
Struct/Budget.rs	Serializable BudgetSpec → contract; counters/deadlines → runtime budget.rs; provider usage decoding → Clef
App/reader/	examples/nfsu-driver/reader/; retain as a separate build-bound artifact
Templates/Observation.json	Split into observation schema, acquisition map and derived features
Templates/Questions.json	Split into lanes.json, templates.json and questions.json
Documentation/StateCharts.md	docs/reference/nfsu-driver-state-machine.md; separate reusable runtime lifecycle if one exists

For the JavaScript files, the names alone do not establish their responsibilities. Move them under the example first and remove numeric revision suffixes once imports are updated:

text
examples/nfsu-driver/web/
  drive.mjs
  gui.mjs
  lane.mjs
  bus.mjs

examples/nfsu-driver/tools/
  analyze_frame.py

Do not extract bus.mjs or lane.mjs into framework components without inspecting their semantics.

Finally, add these conformance tests before extraction is considered complete:

Duplicate provider selection fails without a binding.

Shared Browse dependencies resolve, but conflicting target-control leases fail.

Missing derived-feature inputs and insufficient history produce explicit errors or unknown values.

Lane compilation rejects unresolved paths and missing question dependencies.

Late Clef results cannot act on an obsolete snapshot.

Clef response decoding works with reported zero output tokens.

Reader/host digest mismatch fails before memory inspection.

16 sources
Show me the Cargo workspace layout
Draft the agentic-port-api crate structure
Show the revised agentic-capability schema
Write the agentic-decision-contract types
How should the runtime handle stale snapshots




Ask a follow-up
Search
Computer
GPT-6.1 Sol
