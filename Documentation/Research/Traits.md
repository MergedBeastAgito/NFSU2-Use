chars: 30445
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
Today 4:44 AM
4:44 AM

Follow-up on your layering answer. Now give the implementable detail.

You proposed: agentic-adapter (implement to supply a substrate), agentic-sdk (import to consume Agentic), a domain crate car-simulator-sdk with trait GamePort, a concrete NFSU2Game, and a thin nfsu2-drive binary.

Questions:

Write the GamePort trait in full, as Rust, with the associated types and lifetimes, such that it works for both a 120 Hz game loop (where Apply must complete within a tick budget) and a slow publishing pipeline (where Apply is a queued draft). Show how one trait serves both without becoming lowest-common-denominator.

Write the SubstratePort (the agentic-adapter trait) in full, and show how NFSU2Game uses it: which calls go through the substrate (page evaluate) and which are pure Rust (memory maths).

Give the exact Cargo dependency arrows for these five crates plus the workspace, and mark the one arrow that would create a cycle if drawn the obvious way, with the inversion that avoids it.

The domain crate owns the flow: observe, project, ask Lane, gate, request modifier, record. Where does the async boundary sit? Should GamePort be sync (called from the game's thread) with the runner async, or the reverse? What is the failure mode of each?

Manifest kinds: what should each of the five declare (kind, provides, requires) so that agentic manifests tells a reader which one to implement and which to import?

Testing: what can be tested without the game, and what requires it? Give the seam that makes the memory layout testable without a browser.

Prefer full code over description. Assume the reader will paste this into a crate.

Read more
Finished
7s

Use an asynchronous admission interface with explicit receipts, and keep the actual tick-sensitive modifier application local to the page. A Rust call over Browse/Evaluate cannot guarantee completion within a 120 Hz game tick; it can submit an expiring target that the page validates and applies at its next available input boundary.

The reusable distinction is admission versus execution: the same interface can admit a modifier or enqueue a draft, without pretending those effects have identical timing or completion semantics.

All APIs below are proposed contracts, not reconstructions of the unpublished existing Adapter trait. Replacing that trait requires an explicit compatibility migration.

1. Full GamePort contract

GamePort can technically serve both examples, but a publishing pipeline is not a game. Name the shared contract WorkPort; re-export it as GamePort in car-simulator-sdk.

The associated types preserve domain-specific observations, requests and results. The common contract defines lifecycle, deadlines and evidence—not a universal “action” structure.

Place this in the domain contract initially. It uses only std.

rust
#![allow(non_snake_case)]

use std::{
    error::Error,
    future::Future,
    pin::Pin,
    time::Instant,
};

pub type PortFuture<'a, T, E> =
    Pin<Box<dyn Future<Output = Result<T, E>> + 'a>>;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct RequestID(pub u128);

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct OperationID(pub u128);

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AdmissionMode {
    ReplaceableTarget,
    DurableQueue,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Evidence {
    Admitted,
    WrittenToBoundary,
    ConsumptionObserved,
    EffectObserved,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OperationState {
    Pending,
    Running,
    Completed,
    Rejected,
    Expired,
    Superseded,
    Cancelled,
}

#[derive(Clone, Copy, Debug)]
pub struct ApplyContext {
    pub RequestID: RequestID,

    // Local host monotonic deadline for admitting the request.
    // This is not a remotely transferable timestamp.
    pub AdmitBefore: Instant,

    pub AuthorityEpoch: u64,
    pub PolicyRevision: u64,
}

#[derive(Clone, Debug)]
pub struct Receipt<R> {
    pub RequestID: RequestID,
    pub OperationID: OperationID,
    pub Mode: AdmissionMode,
    pub Evidence: Evidence,
    pub Result: Option<R>,
}

#[derive(Clone, Debug)]
pub struct Progress<R> {
    pub OperationID: OperationID,
    pub State: OperationState,
    pub Evidence: Evidence,
    pub Result: Option<R>,
}

pub trait WorkPort {
    type Observation;
    type Request;
    type Result;
    type Error: Error + 'static;

    fn Mode(&self) -> AdmissionMode;

    fn Observe<'a>(
        &'a self,
    ) -> PortFuture<'a, Self::Observation, Self::Error>;

    fn Apply<'a>(
        &'a self,
        Context: ApplyContext,
        Request: Self::Request,
    ) -> PortFuture<'a, Receipt<Self::Result>, Self::Error>;

    fn Inspect<'a>(
        &'a self,
        OperationID: OperationID,
    ) -> PortFuture<'a, Progress<Self::Result>, Self::Error>;

    // Must be idempotent. Returns true if cancellation was installed.
    // Completed irreversible effects need not be cancellable.
    fn Cancel<'a>(
        &'a self,
        OperationID: OperationID,
    ) -> PortFuture<'a, bool, Self::Error>;
}

pub use WorkPort as GamePort;

The boxed futures borrow self through 'a. They deliberately omit Send, allowing browser-local implementations. A native multithreaded variant can require Send futures, but that should be an explicit API choice.

Two concrete specialisations
text
NFSU2Game
  Observation = VehicleObservation
  Request     = SteeringModifier
  Result      = ModifierEvidence
  Mode        = ReplaceableTarget

PublishingPipeline
  Observation = EditorialState
  Request     = DraftSubmission
  Result      = DraftResult
  Mode        = DurableQueue

The modifier request includes expiry, channel sequence and neutralisation policy. The draft request includes content identity, revision and publishing destination. Those remain strongly typed domain data.

Required contract semantics

Apply acknowledges admission, not necessarily execution.

RequestID deduplicates retries within an authority/session scope.

Inspect resolves uncertain outcomes.

A dropped Apply future does not imply cancellation.

Game targets replace older targets; drafts enter a durable queue.

Admission mode is stable for an implementation. Use two implementations if a product offers both behaviours.

At 120 Hz, a tick is approximately 8.33 ms. The page-side actuator—not a remote asynchronous call—must enforce any next-tick deadline.

2. Full SubstratePort, and how the game uses it

Keep this seam focused on the substrate operation actually required: page evaluation. Do not add WASM memory methods to the substrate contract; memory layout belongs to the target adapter.

The following uses serde_json = "1".

Contract owned by agentic-adapter
rust
#![allow(non_snake_case)]

use std::{
    error::Error,
    future::Future,
    pin::Pin,
    time::Instant,
};

use serde_json::Value;

pub type SubstrateFuture<'a, T> =
    Pin<
        Box<
            dyn Future<
                    Output = Result<T, SubstrateError>
                > + 'a,
        >,
    >;

#[derive(Clone, Debug)]
pub struct SubstrateError {
    pub Code: &'static str,
    pub Message: String,
}

impl std::fmt::Display for SubstrateError {
    fn fmt(
        &self,
        F: &mut std::fmt::Formatter<'_>,
    ) -> std::fmt::Result {
        write!(F, "{}: {}", self.Code, self.Message)
    }
}

impl Error for SubstrateError {}

#[derive(Clone, Debug)]
pub struct EvaluateRequest {
    pub DocumentID: String,

    // A baked JavaScript function expression accepting one JSON argument.
    pub Expression: &'static str,
    pub Argument: Value,

    // Local transport deadline, not a hard execution guarantee.
    pub AdmitBefore: Instant,
}

#[derive(Clone, Debug)]
pub struct EvaluateResponse {
    pub DocumentID: String,
    pub Value: Value,
}

pub trait SubstratePort {
    fn Evaluate<'a>(
        &'a self,
        Request: EvaluateRequest,
    ) -> SubstrateFuture<'a, EvaluateResponse>;
}

SDK re-exports these exact items. The Browse implementation or a small wrapper implements SubstratePort.

Do not create two separately defined traits called Adapter and SubstratePort. Either:

Retain existing Adapter and adapt this design to it; or

Introduce SubstratePort as a versioned successor and migrate deliberately.

How NFSU2Game holds it
rust
pub struct NFSU2Game<S: SubstratePort> {
    pub Substrate: S,
    pub DocumentID: String,
    pub Layout: MemoryLayout,
    pub Expressions: Expressions,
}

pub struct Expressions {
    pub ReadWindow: &'static str,
    pub ApplyModifier: &'static str,
    pub Inspect: &'static str,
    pub Cancel: &'static str,
}

Populate Expressions from your existing script! exports. The exact macro syntax was not supplied, so it should not be fabricated.

Every expression remains a .ts file:

text
Source/Script/Expressions/
  ReadWindow.ts
  ApplyModifier.ts
  Inspect.ts
  Cancel.ts
Observation path
text
Substrate.Evaluate(ReadWindow)
    → bytes captured from reachable game memory
    → Rust bounds checking and address arithmetic
    → Rust decoding
    → typed VehicleObservation

A minimal ReadWindow.ts expression could be:

typescript
(Request: { Address: number; Length: number }) => {
    const Memory =
        (globalThis as any).GameMemory as WebAssembly.Memory;

    if (!(Memory instanceof WebAssembly.Memory)) {
        throw new Error("MemoryUnavailable");
    }

    const { Address, Length } = Request;
    const Buffer = Memory.buffer;

    if (
        !Number.isSafeInteger(Address) ||
        !Number.isSafeInteger(Length) ||
        Address < 0 ||
        Length < 0 ||
        Address > Buffer.byteLength ||
        Length > Buffer.byteLength - Address
    ) {
        throw new Error("MemoryBounds");
    }

    return Array.from(new Uint8Array(Buffer, Address, Length));
}

GameMemory is an illustrative binding: substitute the target’s actual exported object. A shared-memory concurrent reader additionally needs an explicit consistency strategy.

Actuation path
text
Pure Rust policy
    → typed SteeringModifier
    → Substrate.Evaluate(ApplyModifier)
    → page-side admission checks
    → page-local smoothing/application
    → receipt

The expression must call a real page-side actuator API. It must not claim that returning from Evaluate proves game consumption.

Also note the boundary: a native Rust process cannot directly dereference browser WASM memory. It computes addresses and decodes captured bytes; actual loads happen in the page or an imported-memory reader.

3. Exact target dependency arrows

These are the recommended arrows, not verified current Cargo metadata.

text
agentic-adapter
    → serde_json                    if JSON is part of its public contract
    → no SDK dependency

workspace Browse implementation
    → agentic-adapter
    → its transport dependencies

workspace Runtime / Lane / other capabilities
    → lower-level contracts as required
    → NOT agentic-sdk

agentic-sdk
    → agentic-adapter
    → workspace Runtime
    → workspace Browse
    → workspace Lane
    → other exposed capabilities

car-simulator-sdk
    → agentic-sdk

nfsu2-driver
    → car-simulator-sdk

nfsu2-drive
    → car-simulator-sdk
    → nfsu2-driver

Have car-simulator-sdk re-export the Agentic substrate types needed to implement its game contract. That lets the game driver use one supported domain facade instead of reaching into framework internals.

Alternatively, the driver may explicitly depend on agentic-sdk too. That does not create a cycle, but it exposes two consumer surfaces.

The tempting cycle
text
car-simulator-sdk → nfsu2-driver → car-simulator-sdk
                  ✗

Avoid it by inversion:

text
car-simulator-sdk defines GamePort
nfsu2-driver implements GamePort
nfsu2-drive constructs NFSU2Game and passes it to Driver

SDK must likewise never depend on its external concrete drivers.

The thin binary is the composition root. It owns concrete construction; reusable libraries own contracts and execution.

4. Async boundary: runner and transport async, local actuator bounded

Neither “make everything sync” nor “make everything async” solves timing.

Use this split:

text
Async Rust runner:
  Observe
  Project                 pure Rust
  Ask Lane                async if necessary
  Gate                    pure Rust
  Apply                   async admission
  Record                  async

Page-local actuator:
  Read latest admitted target
  Check epoch / expiry / current preconditions
  Clamp and smooth
  Write input boundary
Why not sync GamePort?

A synchronous method is appropriate for a genuinely local memory reader or input slot. It is not appropriate if its implementation blocks waiting for Browse/CDP or a model.

Failure: a nominally fast synchronous call blocks the executor or rendering thread and overruns its budget.

Why not async application on every game tick?

Failure: every awaited round trip introduces latency, overlap and stale targets. A remote model call cannot sit in the 120 Hz critical path.

Async is appropriate for admitting a target. The target must then be enforced locally.

Timeout is not rollback

Tokio timeout drops the inner future on timeout; it does not undo an input write or a remotely admitted draft. A future that does not yield can also overrun the timeout without being pre-empted.
docs
+1

Therefore:

text
timeout
  → outcome unknown
  → Inspect using the known RequestID/OperationID
  → retry only with the same idempotency identity

For uncertain admission before an operation ID is known, derive the operation ID from the request identity or provide lookup by RequestID. Do not make reconciliation depend on an ID lost with the response.

Instant remains local to the Rust process. The page must receive a page-clock expiry or a deadline translated through an explicit clock mapping—not a serialized Rust Instant.

5. Manifest kinds and ports

The manifest distinguishes executable ports from libraries. Introduce kind: "contract" explicitly; do not pretend a trait definition is a runnable capability.

The existing kind, provides and requires fields can express this:

Crate	kind	provides.port	requires.ports
agentic-adapter	contract — new kind	null	[]
agentic-sdk	facade	null	Existing exposed/runtime-required ports
car-simulator-sdk	facade	null	["agentic.browse", "agentic.lane.decide"], if mandatory for its runner
nfsu2-driver	adapter	"agentic.nfsu.drive"	["agentic.browse", "agentic.lane.decide"], if it owns both calls
nfsu2-drive	composition	"agentic.nfsu.run" if independently callable	["agentic.nfsu.drive"]

Choose one owner for Lane:

If car-simulator-sdk owns the decision runner, the game adapter normally requires only Browse.

If nfsu2-driver still owns decision orchestration, its manifest requires Lane too.

For the proposed layering, prefer:

text
NFSU2Game adapter:
  requires Browse
  provides NFSU drive operations

CarSimulator runner:
  consumes GamePort and Lane

If the domain crate’s runner itself needs discovery as an executable capability, split or separately declare a car-simulator-driver capability providing car.simulator.drive. A facade should not claim an executable port merely because it re-exports a runner type.

For agentic manifests to literally say “implement” versus “import,” add presentation rules:

text
contract → Implement
facade   → Import
adapter  → Construct / supply
composition → Run

These labels need not become new JSON fields.

6. Testing: isolate memory decoding from capture

The seam is a read-only byte-address space, not a browser.

This code can be pasted into a testable module. It uses checked arithmetic, explicit little-endian decoding and no unsafe. Rust provides f32::from_le_bytes for exactly this representation conversion.
doc.rust-lang

rust
#![allow(non_snake_case)]

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MemoryError {
    AddressOverflow,
    OutOfBounds,
    InvalidValue,
}

pub trait MemoryRead {
    fn Read(
        &self,
        Address: u64,
        Output: &mut [u8],
    ) -> Result<(), MemoryError>;
}

pub struct MemoryWindow<'a> {
    pub Base: u64,
    pub Bytes: &'a [u8],
}

impl MemoryRead for MemoryWindow<'_> {
    fn Read(
        &self,
        Address: u64,
        Output: &mut [u8],
    ) -> Result<(), MemoryError> {
        let Offset = Address
            .checked_sub(self.Base)
            .ok_or(MemoryError::OutOfBounds)?;

        let Start = usize::try_from(Offset)
            .map_err(|_| MemoryError::AddressOverflow)?;

        let End = Start
            .checked_add(Output.len())
            .ok_or(MemoryError::AddressOverflow)?;

        let Source = self.Bytes
            .get(Start..End)
            .ok_or(MemoryError::OutOfBounds)?;

        Output.copy_from_slice(Source);
        Ok(())
    }
}

pub struct MemoryLayout {
    pub SpeedOffset: u64,
}

pub fn ReadSpeed<M: MemoryRead + ?Sized>(
    Memory: &M,
    CarAddress: u64,
    Layout: &MemoryLayout,
) -> Result<f32, MemoryError> {
    let Address = CarAddress
        .checked_add(Layout.SpeedOffset)
        .ok_or(MemoryError::AddressOverflow)?;

    let mut Bytes = [0u8; 4];
    Memory.Read(Address, &mut Bytes)?;

    let Speed = f32::from_le_bytes(Bytes);

    if !Speed.is_finite() || !(0.0..=400.0).contains(&Speed) {
        return Err(MemoryError::InvalidValue);
    }

    Ok(Speed)
}

#[cfg(test)]
mod Test {
    use super::*;

    #[test]
    fn ReadsSpeedWithoutBrowser() {
        let Bytes = 76.4_f32.to_le_bytes();
        let Memory = MemoryWindow {
            Base: 0x1430,
            Bytes: &Bytes,
        };

        let Layout = MemoryLayout {
            SpeedOffset: 0x430,
        };

        assert_eq!(
            ReadSpeed(&Memory, 0x1000, &Layout),
            Ok(76.4)
        );
    }

    #[test]
    fn RejectsMissingMemory() {
        let Memory = MemoryWindow {
            Base: 0,
            Bytes: &[],
        };

        let Layout = MemoryLayout {
            SpeedOffset: 0x430,
        };

        assert_eq!(
            ReadSpeed(&Memory, 0x1000, &Layout),
            Err(MemoryError::OutOfBounds)
        );
    }
}

The storage type and range above are illustrative; use the game port’s actual layout. A passing range check does not establish layout compatibility.

Minimum test split
Without the game	Requires the real target
Byte decoding and pointer arithmetic	Actual exported memory binding
Layout fixtures and out-of-bounds handling	Layout agreement with the supported game build
Projection, predicates and gates	Input API acceptance and release
Scripted SubstratePort responses	Page-local scheduling behaviour
Duplicate requests and stale epochs	Observable game consumption/effect
Draft queue/idempotency semantics	Navigation/reload recovery
Expiry and local smoothing with fake clocks	Browser privilege and capture behaviour

Preserve your existing structure:

text
Source/Fn/ReadState.rs
Source/Struct/MemoryLayout.rs
Source/Struct/MemoryWindow.rs
Source/Script/Expressions/ReadWindow.ts
Test/Memory.rs
Test/Adapter.rs
Test/Decision.rs

The contract unifies observation and admission. Domain-specific types preserve meaning, and the local actuator—not the trait’s use of async—provides the timing boundary.

15 sources
Show me the adapter implementation crate next
Show me how car-simulator implements WorkPort
How does the admission queue handle dropped ticks
Show me the rest of the code for adapter.rs
How do you structure the error types for WorkPort
⠹




Ask a follow-up
Search
Computer
GPT-6.1 Sol
