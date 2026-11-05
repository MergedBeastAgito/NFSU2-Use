chars: 13728
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
Today 3:53 AM
3:53 AM

CONTEXT, then one question.

A Rust workspace has these crates, each with a manifest that declares a port and what it requires:

@agentic/adapter  (kind: capability) provides "agentic.adapter"; it defines the trait a third party implements to supply a substrate.

@agentic/sdk      (kind: facade)     provides nothing; requires agentic.runtime, agentic.browse, agentic.chat.ask, agentic.verify.design, agentic.skill.validate, agentic.adapter.

@agentic/lane     (kind: capability) provides "agentic.lane.decide".

plus capabilities: browse, chat.ask, verify.design, skill.validate, and a runtime.

House conventions: every capability is a verb (browse, chat.ask, lane.decide); the facade is the only crate a consumer imports; adapters are consumed by path and never modify the workspace.

Now a concrete case. Someone wants to build a "car simulator driver" for a specific game — a crate that reads that game's memory, decides, and injects input. Call it a driver product. Today they would path-depend on the sdk, implement the adapter trait, and get browse/chat/lane from the workspace.

Questions:

What is the canonical dependency direction when there is both a facade (sdk) and a seam (adapter)? Should the sdk depend on adapter, adapter depend on sdk, or neither — and where does the third party's crate attach? Name the pattern (ports and adapters / hexagonal, facade, plugin registry, inversion of control) and say which one this actually is.

Is "framework" a crate, a workspace, or a third concept? If someone says "we want to build a framework on top of this", what should they actually produce — a crate that implements the adapter trait, a crate that composes the sdk, or a separate distribution?

The adapter crate is currently kind: capability but provides a trait, not a verb. What should its kind be, and what does that change in the manifest vocabulary?

Give the exact Cargo dependency arrows for: workspace crates, the sdk, a third-party driver, and the adapter crate. Mark which arrows would create a cycle and how to avoid it.

If a third party wants their driver to be also usable as a library by other drivers (a car-simulator framework), what is the correct layering so it is both a consumer of the workspace and a provider to others — without becoming a fork of the workspace?

What naming would you give these three roles so the names alone tell a reader which one they implement and which one they import?

Prefer concrete arrows, trait signatures and names over prose.

Read more
Finished
5s

The canonical direction is SDK → Adapter contract, with the third-party driver depending on SDK and implementing the contract re-exported by it. The contract must not depend on the facade, the concrete driver or the application composition root.

Your system is ports-and-adapters with a facade and composition-root dependency injection. A plugin registry is optional machinery, not the defining pattern.

1. Facade versus seam: dependency direction

Use A → B to mean “A declares a normal Cargo dependency on B.”

text
ThirdPartyDriver → SDK → AdapterContract
                        ↑
                 FrameworkCapabilities

The SDK re-exports the actual trait, not a duplicate definition:

rust
// Crate/Sdk/Source/Library.rs
// Assuming this is the existing library name and root export:
pub use AgenticAdapter::Adapter;

Rust re-exports expose an existing item through another public path; they do not create a second trait identity.
doc.rust-lang

Therefore:

text
Dependency declared by consumer:  agentic-sdk
Trait implemented by consumer:    SDK's re-export of Adapter
Trait originally defined in:     Crate/Adapter

The third party does not need a direct Cargo dependency on Adapter.

Which pattern is which?
Mechanism	Role here
Ports and adapters / hexagonal architecture	Framework code uses contracts; external implementations adapt substrates to those contracts
Facade	SDK is the supported consumer import surface
Dependency injection	A composition root supplies a concrete adapter to the framework
Inversion of control	Framework execution invokes the supplied adapter
Plugin registry	Optional lookup/selection mechanism; not required for linked path dependencies

Ports and adapters separates contracts from concrete integrations; dependency injection supplies those integrations at the application boundary.
wikipedia
+1

One qualification: implementing Adapter does not automatically prove that the whole driver product is merely a substrate adapter. It can contain both a substrate implementation and application-level orchestration.

2. “Framework” is an architectural role, not a Cargo container

A framework can be distributed as one crate, several crates in a workspace, or a larger distribution. Calling something a framework should describe its reusable execution model and extension points—not its directory layout.

For your concrete case:

Intended result	What to produce
Drive one particular game	A driver product that consumes SDK and supplies its concrete adapter
Reuse driving policies and state models across games	A domain library consuming SDK
Let other developers implement new game integrations under your execution model	A car-simulator framework with its own game-facing trait and runner
Bundle browser, models, assets and deployment tooling	A distribution; optional, separate from whether the library is a framework

A crate that implements Agentic’s adapter trait is an Agentic integration. It becomes a car-simulator framework only when it offers a reusable domain API and extension seam of its own.

“Framework on top” should therefore normally mean:

text
CarSimulatorSDK → AgenticSDK
        ↑
GameSpecificDriver

Not:

text
CopiedAgenticWorkspace + game-specific modifications
3. The adapter contract should be kind: "contract"

A crate defining the adapter trait is not itself a runnable verb capability. Nor is it a concrete kind: "adapter" implementation.

Recommended classification:

json
{
  "name": "@agentic/adapter",
  "kind": "contract"
}

This is a proposed extension to your manifest vocabulary, not a currently established kind.

What changes in the vocabulary?

Make provides describe executable ports only:

json
{
  "name": "@agentic/adapter",
  "kind": "contract",
  "version": "0.1.4",
  "provides": {
    "port": null,
    "errors": []
  },
  "requires": {},
  "artefact": {
    "entry": "Target/debug/libAgenticAdapter.rlib",
    "hash": null,
    "runtime": "rust>=1.88"
  },
  "state": "stateless",
  "cost": {
    "coldMs": 0,
    "warmMs": 0,
    "network": false
  },
  "verify": "cargo test -p agentic-adapter"
}

The package name in verify must match the real Cargo.toml; it is illustrative here.

Consequences:

Add contract to the manifest schema’s kind enum.

Permit provides.port: null for contracts, as you already do for the facade.

Remove agentic.adapter from SDK’s runtime requires.ports.

Retain SDK’s Cargo dependency on the contract crate.

Do not register a contract as an executable provider or wait for it to become ready.

If you need manifest-level build dependency discovery, add a separate dependencies field. Do not overload requires.ports to represent both Rust library dependencies and runtime capability requirements.

There is no need to invent an adapter.define verb to make a type declaration fit the capability category.

4. Cargo arrows and cycle hazards

The following is the recommended target graph. Your capability manifests alone do not prove the current Cargo graph.

Framework and facade
text
AdapterContract
    → foundational dependencies only
    → NOT SDK
    → NOT concrete providers

Runtime
    → AdapterContract       [where its public execution API uses Adapter]

Browse / Chat / Verify / Skill / Lane implementations
    → their required lower-level contracts and libraries
    → NOT SDK

SDK
    → AdapterContract
    → Runtime
    → Browse API/implementation exposed by SDK
    → Chat
    → Verify
    → Skill
    → Lane                  [required if consumers get Lane through SDK]

Agentic application / composition root
    → SDK
    → chosen concrete providers, if not constructed through SDK

Do not infer ChatDriver → BrowserCDP just because ChatDriver requires agentic.browse. It needs the Browse contract; depending on a particular provider is a separate implementation choice.

Your listed SDK manifest does not currently require agentic.lane.decide. If SDK exposes Lane as part of the product, add the appropriate library dependency/re-export; add the runtime port requirement only if SDK’s manifest means that Lane must be available in its runtime composition.

External product
text
nfsu-driver library
    → agentic-sdk

nfsu-driver binary
    → its own library
    → agentic-sdk           [only if the binary uses SDK directly]

Source/Library.rs and Source/Main.rs remain unchanged as Cargo entry paths.

Forbidden reverse arrows
Arrow	Result
AdapterContract → SDK	Cycle with SDK → AdapterContract
Lane → SDK	Cycle if SDK → Lane
Runtime → SDK	Cycle if SDK → Runtime
SDK → nfsu-driver	Cycle with nfsu-driver → SDK
AdapterContract → nfsu-driver	Cycle through nfsu-driver → SDK → AdapterContract
CarSimulatorSDK → GameSpecificDriver	Cycle if that driver depends on CarSimulatorSDK

Cargo normally disallows dependency cycles; its limited development-dependency exception is not a solution for the production architecture.
doc.rust-lang

A separately owned application binary may depend on both SDK and the concrete driver. That is the correct place to connect them.

5. A driver that is also reusable: introduce a second, domain-specific seam

Do not force Agentic’s substrate trait to carry every car-simulator concept.

Use two contracts:

text
Agentic Adapter
    Framework-facing substrate contract.

CarSimulator GamePort
    Domain-facing contract implemented by each game integration.

Recommended layering:

text
CarSimulatorApplication
    ├── CarSimulatorSDK
    │       └── AgenticSDK
    │               └── AdapterContract
    └── NFSU2Driver
            └── CarSimulatorSDK

CarSimulatorSDK must not depend on NFSU2Driver. Its runner receives a game implementation from the composition root.

For a small initial product, the domain contract and runner can live in one crate. Split them only when dependency size or independent versioning justifies it.

Illustrative domain trait

This is a proposed car-simulator API, not a reconstruction of your existing Agentic Adapter signature:

rust
use std::{
    future::Future,
    pin::Pin,
};

pub type GameFuture<'a, T> =
    Pin<Box<
        dyn Future<Output = Result<T, GameError>>
            + 'a
    >>;

pub trait GamePort {
    fn Observe<'a>(
        &'a self,
    ) -> GameFuture<'a, Observation>;

    fn ApplyModifier<'a>(
        &'a self,
        Modifier: Modifier,
    ) -> GameFuture<'a, ApplyOutcome>;

    fn Release<'a>(
        &'a self,
    ) -> GameFuture<'a, ApplyOutcome>;
}

This local-compatible shape deliberately does not impose Send + Sync. A native multithreaded runner can define stronger requirements where needed.

Then:

rust
pub struct NFSU2Game {
    // SDK-provided Browse handle and game-specific bindings.
}

impl GamePort for NFSU2Game {
    // Memory reads and game API calls through Browse/Evaluate.
}

The domain runner owns the reusable flow:

text
Observe
  → project state
  → ask Lane
  → apply policy/gates
  → request bounded modifier
  → record outcome

The game implementation owns:

text
memory layout
page expressions
cue interpretation
input API binding
game-specific action semantics

The runner’s Agentic-facing adapter wrapper can implement the existing Adapter trait if the framework needs to invoke the product that way. That wrapper should delegate to the domain runner; it should not require every game implementation to duplicate the Agentic integration.

All game-specific page expressions remain individual .ts files compiled by build.rs and included through SDK’s script! export.

6. Names that tell developers what to implement and import

Use role names rather than calling every layer “adapter” or “framework.”

Role	Package/module name	Public name	Developer instruction
Framework extension contract	Existing agentic-adapter, optionally renamed later to agentic-port-contract	Existing Adapter, or clearer future SubstratePort	Implement this to supply a substrate
Framework consumer facade	agentic-sdk	SDK import surface	Import this to consume Agentic
Concrete game integration	nfsu2-driver	NFSU2Game / NFSU2Adapter	Construct this for NFSU2

For the reusable domain layer:

Role	Name
Domain facade/framework crate	car-simulator-sdk
Domain extension trait	GamePort
Reusable orchestration type	Driver
Concrete game implementation	NFSU2Game
Executable composition root	nfsu2-drive

The resulting instructions are unambiguous:

text
Consume Agentic:
    depend on agentic-sdk

Extend Agentic with a substrate:
    implement Adapter, imported through agentic-sdk

Consume the car-simulator framework:
    depend on car-simulator-sdk

Add a game to that framework:
    implement GamePort, imported through car-simulator-sdk

Keep the existing names initially if renaming would disrupt working consumers. The architectural fix is dependency direction and contract classification—not a mandatory rename or workspace fork.

15 sources
Show me how the workspace Cargo.toml handles this
Show me the manifest for the adapter crate
What does the composition root look like in code
Show me how capabilities resolve at runtime
How do other crates declare their requirements




Ask a follow-up
Search
Computer
GPT-6.1 Sol
