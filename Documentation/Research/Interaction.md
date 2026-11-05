chars: 22578
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
2:33 AM

CONTEXT FIRST, then the question.

Repositories involved

NikolaRHristov/NFSU2 (public) - the example adapter: a Rust crate that drives a browser WebAssembly game through Agentic's Browse port (Evaluate only), plus a wasm reader crate, a JS harness (driver, GUI, pipeline lane, in-page bus) and JSON schema templates.

Agentic - the framework workspace: Crate/{Agentic,BrowserCDP,ChatDriver,DesignVerification,Runtime,SkillContract,SpacetimeCatalog,Workflow}, each with module.json declaring provides/requires ports.

NikolaRHristov/TimeLine - an Astro site for private bookmark ingestion (a product that installs as an extension).

Naming conventions (house style, non-negotiable)

Rust identifiers and JSON keys are PascalCase; acronyms are full-cased (URL, URI, SDK, ID, HTTP, JSON, WASM, CDP).

Crates: Crate/<Name>/{Source/{Library.rs,Fn.rs,Fn/<Verb>.rs,Struct.rs,Struct/<Noun>.rs,Script/Expressions/*.ts}, build.rs, module.json, Test/}.

JavaScript is never built with format! in Rust: one expression per .ts file, compiled by build.rs, baked in with a script! macro.

module.json vocabulary: name @agentic/<kebab>, kind capability|adapter, provides {port, errors}, requires {ports}, artefact {entry,hash,runtime}, state, cost {coldMs,warmMs,network}, verify.

File tree of the example

./.gitattributes
./.gitignore
./App/analyze_frame.py
./App/driver/ADAPTER.md
./App/driver/build.rs
./App/driver/Cargo.lock
./App/driver/Cargo.toml
./App/driver/module.json
./App/driver/Source/Fn.rs
./App/driver/Source/Fn/Cloudflare.rs
./App/driver/Source/Fn/Decide.rs
./App/driver/Source/Fn/Expressions.rs
./App/driver/Source/Fn/Gate.rs
./App/driver/Source/Library.rs
./App/driver/Source/Main.rs
./App/driver/Source/Script/Expressions/Bootstrap.ts
./App/driver/Source/Script/Expressions/Hold.ts
./App/driver/Source/Script/Expressions/Publish.ts
./App/driver/Source/Script/Expressions/Release.ts
./App/driver/Source/Script/Expressions/State.ts
./App/driver/Source/Struct.rs
./App/driver/Source/Struct/Budget.rs
./App/driver/Source/Struct/Decision.rs
./App/driver/Source/Struct/Driver.rs
./App/driver/Source/Struct/Policy.rs
./App/driver/Source/Struct/State.rs
./App/driver/Test/Adapter.rs
./App/driver/Test/Decision.rs
./App/nfsu-bus.mjs
./App/nfsu-drive4.mjs
./App/nfsu-gui4.mjs
./App/nfsu-lane.mjs
./App/reader/.cargo/config.toml
./App/reader/Cargo.lock
./App/reader/Cargo.toml
./App/reader/Documentation/Input-Ring.md
./App/reader/Documentation/Reader.md
./App/reader/README.md
./App/reader/Source/Library.rs
./App/reader/Test/attach.mjs
./App/reader/Test/report.json
./App/reader/Test/ring.mjs
./App/tick-0.jpg
./App/tick-1.jpg
./Documentation/Research/Agentic-Standard.md

What exists today

Observation schema: 16 fields, each sourced from memory (absolute wasm address) or frame (captured pixels).

Question schema: 5 lanes (drive/recover/corner/race/context) and 7 typed question templates (choice/score/noul).

A wasm reader compiled with --import-memory that reads the same addresses with no allocation.

A capability negotiation answer already exists (per-tier: no-injection, in-page pack, extension, CDP).

THE GAP

Every path modelled so far treats the target as a WebAssembly module with memory. The framework needs to model ordinary browser interaction too: DOM UI, forms, canvas 2D, WebGL/WebGPU graphics, accessibility trees, and graphical surfaces that are not games.

QUESTIONS

Give the capability taxonomy for browser interaction, independent of target: what are the orthogonal axes (read vs write; DOM vs canvas vs pixels; trusted vs synthetic input; same-origin vs cross-origin; page-cooperative vs page-hostile)? Name each axis and its levels.

For each axis combination, what is the minimal viable observer and actuator? Be concrete: APIs, not concepts.

How should one schema describe both a game memory field and a DOM node's text, so a decision model sees one observation shape? Propose the discriminator and the common envelope.

What does an accessibility-tree observation give that a DOM snapshot does not, and when is the a11y tree the wrong reader?

For graphics surfaces (WebGL/WebGPU canvas), what are the read options in 2026 and their costs: toDataURL, captureStream, drawImage into 2D, CDP screenshot, OffscreenCanvas transfer? Which work when the page is hostile?

Which parts of a browser-interaction model are stable enough to become a crate, and which must remain adapters?
Prefer API names, header names, limits and version numbers.

Read less
Finished
9s

The framework should model browser interaction as capabilities, not as “WASM access plus fallbacks.” Memory, DOM, accessibility semantics and rendered pixels are independent observation sources; application APIs, DOM mutation and browser-generated input are independent actuation paths.

Keep your existing module.json vocabulary unchanged. Introduce the richer interaction contract as separate, versioned data, using PascalCase for its keys and Rust identifiers. Standards-defined keys such as JSON Schema’s $schema, oneOf and properties must retain their standard spelling.

1. Capability taxonomy: orthogonal axes

Do not encode these as one ordered “power level.” A debugger can capture pixels without a reachable application object; an ordinary page integration may have a rich application API without browser input synthesis.

Proposed axes and levels
Axis	Levels	Meaning
Operation	Observe, Actuate	Reading and writing are independently granted
Representation	ApplicationState, WASMMemory, DOM, Accessibility, Canvas2D, WebGL, WebGPU, CompositedPixels	What is observed or modified
ExecutionContext	PageMain, ExtensionIsolated, Worker, ExtensionHost, DebuggerHost	Where the integration executes
Reach	CurrentDocument, SameOriginFrame, PermittedFrame, DebuggerTarget	Which document/target is accessible
ActuationPath	ApplicationAPI, MemorySlot, DOMProperty, DOMEvent, BrowserInput	How an action enters the application
EventTrust	NotApplicable, Synthetic, BrowserGenerated	Whether an action creates events and through which mechanism
Cooperation	ExplicitAPI, ReachableObjects, Instrumentable, Uncooperative	What the application exposes or tolerates
Readback	Semantic, OriginCleanSurface, GPUResource, CompositorCapture	Security and acquisition boundary for observations
Timing	OnDemand, EventDriven, AnimationFrame, ApplicationTick, BestEffortPolling	Available sampling/application cadence
Lifetime	Document, Reinjectable, HostSession	What survives navigation and reconnect
Consistency	SingleRead, CaptureInterval, ApplicationSnapshot	How strongly values can be treated as one observation

Some combinations are invalid:

Worker + DOM requires a bridge; workers do not directly manipulate the document.

DOMEvent + BrowserGenerated is not achieved through dispatchEvent().

Uncooperative + WASMMemory is unavailable without a reachable memory object or additional instrumentation.

ApplicationTick requires an actual execution boundary; requestAnimationFrame() is not proof of a game tick.

For extensions, MAIN shares the page environment; ISOLATED does not share its JavaScript globals. Main-world code is also exposed to page interference.
developer.chrome
+1

Separate authority from availability

A runtime probe may establish that an API works. It does not grant permission to use it.

Represent both:

json
{
  "Capability": "Browser.Observe.DOM",
  "Available": true,
  "Authorized": true,
  "Scope": {
    "DocumentID": "Document-17",
    "FrameID": "Frame-2"
  },
  "Evidence": "Probe-42"
}

A tier belongs in deployment metadata. Selection should use these scoped capabilities.

2. Minimal viable observers and actuators

A full Cartesian product would mostly enumerate impossible combinations. Instead, resolve each requested read/write through this matrix, then apply context, origin, timing and permission constraints.

Observer/actuator matrix
Target	Minimal observer	Minimal actuator	Important limitation
Text/content DOM	querySelector(), textContent, getAttribute()	textContent, setAttribute()	DOM mutation may not update application state
Visible DOM text/layout	innerText, getBoundingClientRect(), getComputedStyle()	Page API or browser input	Style/layout reads can incur rendering work
Form controls	.value, .checked, .selectedOptions, validity properties	Native property setter plus relevant events; preferably application API or browser input	Setting .value does not itself reproduce user editing
DOM changes	MutationObserver	Same paths as above	Does not observe arbitrary property changes, GPU draws or application memory
Semantic UI	CDP Accessibility.getFullAXTree / getPartialAXTree	Resolve semantic node to DOM target, then browser input	Accessibility observation is not a universal action API
DOM snapshot	CDP DOMSnapshot.captureSnapshot	DOM/CDP input operations	Snapshot IDs and nodes become stale
Canvas 2D pixels	Existing context’s getImageData(); canvas copy into a 2D observer canvas	Application API or pointer/keyboard input	Reading pixels does not reveal scene objects
WebGL pixels	readPixels() with the correct context/framebuffer	Application API or input	Requires context access and correct capture timing
WebGPU resource	copyTextureToBuffer() then buffer mapping	Application API or input	Requires resource/device access and compatible usage flags
Final visual output	CDP Page.captureScreenshot; extension tabs.captureVisibleTab()	Browser input	Pixel coordinates depend on viewport, zoom and scaling
WASM state	Imported WebAssembly.Memory or JavaScript views	Documented input slot/API	Build-dependent layout; reads and writes need separate permissions

CDP exposes both DOM snapshots and accessibility-tree retrieval. They are different readers and should remain separately selectable.
chromedevtools.github
+1

Origin/frame resolution

Apply this rule before selecting an observer:

Current document: use the available local integration.

Same-origin frame: obtain that document or execute inside it.

Cross-origin frame: inject a separately permitted extension integration or attach to its debugger target/context.

No suitable permission or attachment: treat the interior as unavailable; a visual capture may still observe its rendered output.

all_frames does not make cross-origin JavaScript objects accessible from the parent. It controls where eligible content scripts are installed.

Synthetic versus browser input

Ordinary scripts can dispatch synthetic events, including from an extension’s main-world script. They cannot make those events trusted by setting a property. Use browser input synthesis through CDP Input.* when that is required.
developer.mozilla
+1

For a framework operation such as SetValue, declare the implementation semantics:

text
DOMPropertyOnly
DOMPropertyAndSyntheticEvents
ApplicationAPI
BrowserKeyboardInput

Do not collapse them into one supposedly equivalent action. A controlled form, rich-text editor and ordinary HTML input may respond differently.

3. One observation schema for memory and DOM

Use two discriminators:

Source.Kind: how the value is acquired.

Value.Type: what the decision layer receives.

The decision model consumes stable semantic field IDs and values. It should not need to interpret selectors, memory offsets or debugger node IDs.

Common envelope
json
{
  "SchemaVersion": 1,
  "ObservationID": "Observation-9001",
  "SessionID": "Session-7",
  "DocumentID": "Document-17",
  "Sequence": "9001",
  "Capture": {
    "ClockID": "PageMonotonic",
    "StartUS": "81000000",
    "EndUS": "81006000",
    "Consistency": "CaptureInterval"
  },
  "Provenance": {
    "Kind": "Measured",
    "ProducerID": "Observer-1"
  },
  "Fields": [
    {
      "FieldID": "Vehicle.Speed",
      "Value": {
        "Type": "Number",
        "Number": 76.4
      },
      "Unit": "km/h",
      "Status": "Valid",
      "Source": {
        "Kind": "WASMMemory",
        "BindingID": "GameMemory",
        "LayoutID": "NFSU2-Build-Hash",
        "Address": "123456",
        "StorageType": "F32LE"
      }
    },
    {
      "FieldID": "Checkout.Total",
      "Value": {
        "Type": "Text",
        "Text": "€42.00"
      },
      "Status": "Valid",
      "Source": {
        "Kind": "DOM",
        "BindingID": "CheckoutDocument",
        "Selector": "[data-testid='total']",
        "Read": "TextContent"
      }
    }
  ]
}

For JSON transport, decimal strings avoid precision loss for large integer addresses, sequence numbers and timestamps.

Define source branches explicitly

Proposed Source.Kind values:

text
ApplicationAPI
WASMMemory
DOM
Accessibility
CanvasPixels
GPUReadback
CompositorPixels
Derived

Each branch has its own required fields. For example:

DOM: locator, read property, document binding.

Accessibility: role/name/property and an ephemeral node reference.

CanvasPixels: surface binding, region, pixel format and coordinate frame.

Derived: extractor ID/version and input observation references.

WASMMemory: memory binding, layout identity, address calculation and storage type.

Use JSON Schema Draft 2020-12 oneOf branches with a Kind property constrained by const. Keep the generated observation separate from its acquisition specification: a decision request normally needs the values and relevant quality metadata, not the complete source configuration.

Missing is not zero

Use explicit statuses:

text
Valid
Missing
Stale
Unsupported
PermissionDenied
ReadError

A missing route cue, absent DOM node and inaccessible cross-origin frame must not become 0, "" or false by default.

Also distinguish measured from derived quality. A DOM property read is not the same evidence as OCR, even when both yield "€42.00".

4. Accessibility tree versus DOM snapshot
What accessibility adds

The accessibility tree gives browser-computed semantics:

Roles such as button, heading, textbox and checkbox.

Accessible names derived from labels, contents and ARIA.

States and values relevant to assistive interaction.

A simplified semantic structure rather than every layout container.

Chromium derives this tree from the DOM and removes or ignores nodes without useful accessibility semantics. Some properties are computed by the browser rather than copied directly from one attribute.
developer.chrome
+1

For a decision model, “button named Submit, disabled” is often more useful than a subtree containing anonymous containers, styling classes and an SVG icon.

When it is the wrong reader

Use another reader when you need:

Canvas/WebGL/WebGPU scene contents without an accessible representation.

Pixel geometry, colour or visual alignment.

Exact source text, attributes or implementation structure.

Hidden application state.

Information omitted or incorrectly represented by the site’s accessibility semantics.

An inaccessible application does not become observable merely because the browser has an accessibility tree.

Do not assume a portable in-page API can retrieve the browser’s complete accessibility tree. In Chromium, CDP Accessibility.* is the concrete debugger path. getAttribute("aria-label") is not a substitute for the computed accessible name.
chromedevtools.github
+1

For interaction, retain the semantic locator and resolve a fresh actionable target. Do not treat an old accessibility node ID as a permanent application identifier.

5. Graphics read options in 2026

Distinguish surface readback from browser/compositor capture. Canvas APIs inspect a surface; screenshots inspect its rendered presentation, including surrounding DOM and overlays.

Readback matrix
Method	Output and cost	Conditions	Uncooperative page?
canvas.toDataURL()	Synchronous image encoding and a large base64 string	Accessible, origin-clean canvas	Conditional; not robust against page interference
canvas.toBlob()	Encoded image without constructing the large data URL	Accessible, origin-clean canvas	Same security boundary as canvas export
canvas.captureStream(FPS)	Continuous video track; frame-processing pipeline required	Accessible, origin-clean canvas	Conditional; no bypass of canvas security
drawImage(Canvas, …) into observer 2D canvas	Image copy; getImageData() adds CPU readback	Drawable source; destination inherits taint	Conditional
WebGL readPixels()	Raw pixels; GPU-to-CPU synchronisation can be expensive	Correct context/framebuffer and timing	Poor choice without reachable context/resources
WebGPU copyTextureToBuffer()	Explicit GPU copy and asynchronous mapping	Correct device/texture, copy flags and buffer layout	Usually requires cooperation or early instrumentation
CDP Page.captureScreenshot	Encoded browser-rendered image plus transport cost	Debugger attachment	Strongest listed option for an uncooperative page
tabs.captureVisibleTab()	Encoded visible-tab screenshot	Extension capture permission/user grant	Does not require page JavaScript cooperation
transferControlToOffscreen()	Transfers rendering ownership; not a screenshot	Must occur before a context has been established	Not a retrofit reader for an already-running renderer

toDataURL() encodes the entire image into an in-memory string; MDN recommends preferring toBlob() for larger images. Both remain subject to origin-clean restrictions.
developer.mozilla

captureStream(0) permits explicit capture through CanvasCaptureMediaStreamTrack.requestFrame(). It is useful when you control the capture schedule, but does not grant access to a tainted canvas.
developer.mozilla
+1

WebGL timing matters

readPixels() reads the currently selected framebuffer, not an abstract “latest game image.” A renderer can draw into intermediate framebuffers, resolve multisampling and present later. Capture at the wrong point can read the wrong surface.
developer.mozilla
+1

Do not assume the default framebuffer remains available indefinitely after presentation when preserveDrawingBuffer is false. Prefer capture aligned with rendering or browser screenshot capture when you do not control the renderer.

WebGPU requires resource access

For explicit readback:

text
GPUTexture with COPY_SRC
    → copyTextureToBuffer()
    → GPUBuffer with COPY_DST | MAP_READ
    → mapAsync()
    → read mapped bytes

Account for row padding and format interpretation. copyTextureToBuffer() requires the source texture to include GPUTextureUsage.COPY_SRC; it is not an arbitrary read of any hidden GPU allocation.
developer.mozilla

OffscreenCanvas is not an attachment trick

transferControlToOffscreen() throws InvalidStateError if the HTML canvas has already established a context or has already transferred control. That excludes the normal “attach to a live game after it started” case.
developer.mozilla

If you own an existing OffscreenCanvas, transferToImageBitmap() is useful, but it replaces the canvas image with a new blank image and requires lifecycle management of the bitmap. It is not a transparent, consequence-free read.
developer.mozilla

What “hostile” changes

For an uncooperative page, prefer:

Extension/browser capture for pixels.

Debugger DOM/accessibility inspection where authorised.

Browser input for actuation.

Page object/memory integration only when a reference is actually obtainable.

A main-world pack is not a protected enclave. The page can tamper with its environment and communication. An isolated extension observer has a stronger separation, while compositor capture avoids dependence on the page’s own export functions.
developer.chrome
+1

tabs.captureVisibleTab() captures the visible area of the active tab and requires <all_urls> or activeTab; Chrome limits it to two calls per second. It is suitable for occasional perception, not a 60 Hz frame feed.
developer.chrome

6. What becomes framework code, and what stays an adapter
Stable contract responsibilities

Within your house layout, the reusable layer should own:

text
Crate/Adapter/
  Source/
    Struct/
      Capability.rs
      Observation.rs
      Source.rs
      Target.rs
      Action.rs
      Outcome.rs
    Fn/
      Negotiate.rs
      Validate.rs

These are proposed additions, not claims about current source contents.

Stable concepts include:

Scoped capability negotiation.

Observation envelope and typed values.

Source discriminators.

Document/frame/session identity.

Coordinate systems and units.

Missing/stale/error statuses.

Action semantics and required capabilities.

Outcome evidence: requested, dispatched, accepted, effect observed.

Navigation invalidation and ephemeral-reference handling.

Keep action semantics distinct:

text
SetDOMValue
DispatchSyntheticEvent
InvokeApplicationAPI
SendBrowserInput
ApplyModifier

A common framework interface may expose these, but must not promise that they are interchangeable.

Browser implementations remain adapters
Responsibility	Owner
CDP context selection, screenshots, DOM/AX retrieval, browser input	BrowserCDP
Extension permissions, messaging, tab capture, main-world bridge	Extension adapter
Ordinary in-page DOM/API access	Page integration adapter
WASM address maps, build matching, game API invocation	NFSU2 example adapter
OCR, cue extraction, application-specific semantic interpretation	Perception adapter/example
Application-specific selectors and accessible-name expectations	Target configuration
Lane projection, typed question validation and decision policy	Decision layer, not browser transport

Keep browser expressions in individual .ts files and bake them through build.rs and script!, exactly as your convention requires:

text
Source/Script/Expressions/
  ReadDOM.ts
  ReadControl.ts
  ObserveMutations.ts
  InvokeInput.ts

Do not generate JavaScript with Rust format!.

Preserve manifest compatibility

Leave module.json using its existing vocabulary. Put detailed browser negotiation in a separate contract such as:

text
BrowserCapabilities.json
Observation.Schema.json
Action.Schema.json

The module manifest declares which port is provided. The negotiated capability document describes what that particular running provider can do in that document, frame and permission context.

That boundary generalises your working game adapter cleanly: WASM memory becomes one reader, the game API becomes one actuator, and neither defines the browser-interaction model for every other target.

34 sources
Show me the adapter code for NFSU2
How does the server script handle the Browse port
Walk through the Rust crate structure
How do observation and actuation map to the WASM memory
Show an example capability contract for this adapter




Ask a follow-up
Search
Computer
GPT-6.1 Sol
