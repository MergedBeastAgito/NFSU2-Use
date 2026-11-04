chars: 25055
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
1:33 AM

Research brief: capability tiers for injecting a WebAssembly pack into a live page, with and without CDP.

I have a working system that drives a closed-loop game in a browser: state read from the page's WebAssembly memory at absolute addresses, input injected through the page's own JavaScript API, perception from captured frames, and suggestions from a decision model applied as modifiers. Today the integration runs over the Chrome DevTools Protocol from an external process. I want to model the same thing across capability tiers, because CDP is only one avenue and not always available.

Design for four tiers and tell me what each can and cannot do:

TIER 0 - NO INJECTION AT ALL. The page as served, no extension, no debugger. What is possible? (Nothing in memory, presumably. Is anything observable at all?)

TIER 1 - IN-PAGE WASM PACK. Modules inserted into the page's own realm by some means that does not require a debugger: a userscript manager, a bookmarklet, an import map, a service worker, a SharedWorker, or a <script> injected by another page. For each insertion vector say: whether it can reach the host page's WebAssembly memory, whether it survives navigation, whether it can inject input, and what the page can do to prevent it.

TIER 2 - BROWSER EXTENSION. A content script plus a background service worker. What does it add over Tier 1: cross-origin frames, storage, a stable origin, chrome.debugger as an optional upgrade? What are the Manifest V3 constraints that matter here (service worker lifetime, no remote code, web_accessible_resources)?

TIER 3 - CDP. An external process attached to the browser. What does it add that a page cannot do to itself (frame capture without the page's cooperation, input events that are trusted, attaching to a tab that has no extension)?

Questions:

MEMORY ACCESS BY TIER. To read another WebAssembly module's linear memory, what exactly is required at each tier? Can an in-page module ever read a host module's memory without the host exposing it? What about SharedArrayBuffer, cross-origin isolation (COOP/COEP), and whether a second module can be given the host's memory by reference.

INPUT INJECTION BY TIER. Which tiers can produce input the page treats as trusted? Where does isTrusted come from, and which vectors can never set it? What is the practical consequence for a game that ignores untrusted events?

THE PACK. If I ship "WASM + WASM + WASM" as one pack - reader, protocol, actuator - what is the correct packaging so it works across tiers? One module with imports, three modules with a shared ABI, or one module with feature flags? How should the pack declare which tier it is running in and degrade?

THE PROTOCOL ACROSS TIERS. The same suggestion protocol must run whether the pack is in-page, in an extension, or behind CDP. What changes in the message transport (postMessage, BroadcastChannel, SharedWorker port, native messaging, WebSocket) and what must stay identical for the schema to be one schema?

SHIPPING IT AS A PRODUCT. This pack is also intended to be shipped by a product whose users install it themselves - a browser extension that ingests their private bookmarks. What is the accepted pattern for one extension that both does its own job and hosts an optional, versioned capability pack? What are the review and permission implications?

WHAT I AM MISSING. Name the tiers, vectors and constraints I have not listed, including ones that are current in 2026 and ones that are deprecated but still deployed. Prefer specifics: API names, header names, manifest keys, limits.

Assume the reader is implementing this across all tiers, not choosing one.

Read less
Finished
13s

Model these as four deployment tiers, but negotiate individual capabilities rather than treating the tier number as proof of access. An extension without a main-world bridge may see less game state than a bookmarklet with access to an exposed game object; CDP attached after initialisation may still lack a convenient reference to an unexported memory.

The critical resource is reachability: to import the game’s memory into another Wasm module, you need the actual WebAssembly.Memory object—not its addresses, its compiled module, or merely execution in the same tab.

1. Tier model and memory access
Recommended tier names
Tier	Name	Baseline
0	Uninstrumented page	Only the application’s existing UI, APIs and user-visible outputs
1	Page-context integration	Authorised code executing in the page, with ordinary web-platform privileges
2	Extension-mediated integration	Extension permissions, isolated/main-world scripts, extension storage and messaging
3	Debugger-mediated integration	CDP instrumentation through an external debugger or extension chrome.debugger

Tier 2 plus chrome.debugger crosses into debugger-mediated capabilities. It should be reported as such, not hidden behind “ordinary extension.”

Tier 0: no injected code does not mean nothing is observable

The user can see rendered output and use ordinary input. The page may already expose telemetry, downloadable replays or a documented control interface.

An external observer could also capture the display through an independently authorised OS facility. That is an additional capability, not memory access granted by Tier 0.

Without an existing interface or some separate capture mechanism, there is no programmatic observation channel for your pack: the pack is not running.

What is required to share Wasm memory?

The ordinary implementation is:

js
const reader = await WebAssembly.instantiate(readerBytes, {
  env: {
    memory: gameMemory
  }
});

Here gameMemory must be the actual memory object, and its type and limits must satisfy the reader’s import declaration. The object’s .buffer is an ArrayBuffer for ordinary memory or SharedArrayBuffer for shared memory.
developer.mozilla
+1

There are three useful cases:

The game exports or otherwise exposes its memory object.

Your integration runs early enough to observe the game’s memory construction/import or instantiation and retain a reference.

A debugger helps recover a reference or inspect runtime state.

Same-realm execution alone does not grant access to closure-private objects. If an existing module’s memory was never exported, imported through a reachable object, or captured during construction, ordinary page code has no standard “enumerate all Wasm memories” API.

Early interception is also conditional: it misses initialisation that happened before installation, in another realm, or through references captured before your wrapper was installed.

SharedArrayBuffer and COOP/COEP

Do not confuse these two arrangements:

Two instances on the same JavaScript thread importing the same ordinary WebAssembly.Memory: does not inherently require shared memory or cross-origin isolation.

Concurrent worker access to shared Wasm memory: requires shared memory and the browser’s shared-memory security conditions.
developer.mozilla
+1

For the normal web deployment, plan around:

text
Cross-Origin-Opener-Policy: same-origin
Cross-Origin-Embedder-Policy: require-corp

Alternatively, use the supported COEP: credentialless deployment where appropriate. Check crossOriginIsolated at runtime rather than assuming headers were effective. Cross-origin isolation enables shared-memory facilities; it does not grant access to someone else’s memory object.
developer.mozilla

An injected script cannot simply convert the already-loaded page into a properly isolated deployment by setting a JavaScript flag.

2. Tier 1 insertion vectors: several are not insertion mechanisms

Your list mixes code-insertion mechanisms, module-resolution configuration and execution hosts.

Vector matrix

“Survives” below distinguishes persistence of installation from survival of one running instance.

Vector	Can obtain game memory?	Navigation behaviour	Input	Prevention and limits
Userscript manager	Conditionally: main-world bridge or early capture required	Manager can reinject; old instance dies	Page API or untrusted DOM events	Manager/world settings, page CSP where applicable, private references, restricted URLs
Bookmarklet	Conditionally, if execution is allowed and memory is reachable	Must generally be invoked again	Page API or untrusted DOM events	CSP/browser restrictions and inaccessible game objects
Import map	Not by itself; only changes how module specifiers resolve	Document-scoped	None by itself	Page must load executable code that uses the mapping
Web service worker	No direct DOM or page-memory access	Registration can persist; worker execution is intermittent	Must message a cooperating page	Secure-context, origin/scope and worker-script requirements
SharedWorker	Only memory explicitly supplied through a suitable bridge	May outlive one document while other owners remain; not durable installation	Must message a page-side actuator	Same-origin construction rules, CSP and worker lifetime
<script> inserted by another page	Only if the other page already has authority to manipulate the target document	Script instance dies on document navigation	Same as ordinary page code	Same-origin policy, CSP and Trusted Types

Main-world scripts share the page environment and are subject to the page’s CSP. Isolated scripts do not automatically share page globals. Chrome explicitly documents both the separation and the risks of main-world execution.
developer.chrome
+1

Important corrections

A cross-origin page cannot simply insert a <script> into your game tab. It needs same-origin access, a cooperating receiver or an extension/debugger privilege.

A service worker does not run inside the window’s JavaScript realm. It can support a designed integration—for example by serving controlled application resources—but is not a general-purpose “attach to this live page” mechanism.

A SharedWorker is likewise a separate execution context. Its constructor normally requires a same-origin worker script, and any access to game state must be supplied explicitly.
developer.mozilla

An import map is not permission to execute code or access memory. It is useful packaging infrastructure in a page you control, not a general attachment vector.

CSP can independently restrict scripts, workers, connections and Wasm compilation. In particular, a restrictive script-src/default-src can block Wasm compilation unless permitted with 'wasm-unsafe-eval' or a broader applicable allowance.
developer.mozilla

3. Tier 2: what an extension actually adds

An extension adds a persistent installation and privileged mediation, not automatic access to every page object.

Architecture
text
extension service worker
    ├── permissions, storage, orchestration, network
    └── content script: ISOLATED
            └── narrowly scoped bridge
                    └── page integration: MAIN
                            └── game API / reachable memory

Content scripts normally run in an isolated world: they can interact with the DOM but cannot directly read page-created JavaScript variables. Use an explicit main-world integration for reachable game objects.
developer.chrome

Capabilities added

Injection into permitted documents and frames.

Extension-owned storage and an extension origin.

Packaged assets independent of the game server.

Browser APIs such as bookmarks.

Background coordination and optional native messaging.

Optional debugger-mediated capture and input through chrome.debugger. Permissions produce user-facing warnings; "debugger" is substantially broader than ordinary page scripting.
developer.chrome

For cross-origin frames, inject a separate content script into each permitted frame and coordinate through extension messaging. Do not claim that all_frames removes origin restrictions or that a top-frame script gains direct access to a cross-origin child’s globals.

Manifest V3 constraints
Service worker lifetime

Do not host the 120 Hz actuator in the background worker. Chrome normally terminates extension workers after 30 seconds of inactivity; long operations have additional limits. Active debugger sessions and WebSocket traffic have documented lifecycle effects, but neither should replace recovery logic.
developer.chrome

Persist configuration and authority state; keep time-critical application in the page or an appropriate local host.

Wasm CSP

Extension contexts that compile Wasm need an appropriate policy, for example:

json
{
  "content_security_policy": {
    "extension_pages": "script-src 'self' 'wasm-unsafe-eval'; object-src 'self';"
  }
}

Chrome’s default extension policy does not enable Wasm compilation. Main-world code remains subject to the page’s policy, not merely this extension policy.
developer.chrome
+1

web_accessible_resources

Declare only assets the page actually needs:

json
{
  "web_accessible_resources": [
    {
      "resources": [
        "packs/game-v1/page-bridge.js",
        "packs/game-v1/reader.wasm"
      ],
      "matches": [
        "https://approved-game.example/*"
      ]
    }
  ]
}

This makes those resources accessible; it does not grant memory access or automatically bypass page CSP. Resources are not web-accessible by default, partly to limit fingerprinting and exposure.
developer.chrome

No remotely hosted executable pack updates

Bundle executable JavaScript and Wasm with the extension and update them through the extension distribution channel. Chrome Web Store treats remotely loaded Wasm as remotely hosted code, just like JavaScript.
developer.chrome

Remote JSON configuration and model responses can be data. Do not use that distinction to smuggle an arbitrary executable program into a configuration interpreter. Chrome permits remote configuration and service calls, while requiring the extension’s logic to be packaged.
developer.chrome

4. Input: trusted events are a separate capability

Event.isTrusted is a browser-controlled, read-only property. Events sent with dispatchEvent() are untrusted; HTMLElement.click() also produces an untrusted click. Some browser-generated events caused by programmatic APIs can be trusted, so “trusted” is not simply synonymous with “a human physically did it.”
developer.mozilla

Path	Trusted keyboard/pointer injection?	Practical result
Tier 0	Real user input only	Existing controls work
Tier 1 dispatchEvent()	No	Games filtering isTrusted can ignore it
Tier 1 page input API	Not an event-trust question	Can work if the game API accepts the call
Tier 2 ordinary content/main-world script	No additional trust	Same DOM-event restriction
Tier 2 with chrome.debugger	Debugger-mediated input	Treat as Tier 3 capability
Tier 3 CDP Input.*	Browser input synthesis rather than DOM dispatch	Appropriate route for trusted browser-dispatched input; verify exact event path in your target Chrome

CDP exposes Input.dispatchKeyEvent and related browser input methods; use those rather than wrapping a dispatchEvent() inside Runtime.evaluate.
chromedevtools.github

For your game, the page’s own input API is preferable when available. It bypasses the need to simulate a keyboard event—not the browser’s security model.

Do not treat trusted input as a universal bypass for user activation, pointer lock, fullscreen, permissions or application-specific checks. Test those capabilities independently.

5. Tier 3: CDP adds instrumentation, not omniscience

Useful CDP capabilities include:

Attaching to debuggable targets without installing an extension.

Evaluating code in selected execution contexts.

Registering early document instrumentation.

Inspecting targets and worker contexts.

Capturing the rendered page through Page.captureScreenshot.

Browser-mediated input through the Input domain.
chromedevtools.github
+2

CDP screenshot capture does not require the page to export a canvas or cooperate with a canvas readback API. It still should not be advertised as unrestricted OS or protected-content capture.

Memory access remains an implementation task. CDP gives debugging and evaluation tools; it does not create a universal stable “read arbitrary Wasm linear memory by absolute address” command for your adapter. Retaining a reachable memory reference is still the simplest path.

Deployment changes relevant in 2026

Since Chrome 136, --remote-debugging-port and --remote-debugging-pipe are not honoured for the default Chrome data directory; use a non-standard --user-data-dir. Chrome recommends Chrome for Testing for automation.
developer.chrome

There is also a newer consent-mediated attachment path: Chrome’s documentation describes Chrome 144+ remote-debugging enablement through chrome://inspect/#remote-debugging, with an explicit user permission dialog for automatic agent connection. Do not assume every existing session requires launching a browser with an open debugging port.
developer.chrome

Secure debugger access as a privileged control channel, not as an ordinary unauthenticated local service.

6. Pack design: three components, one manifest, host-specific bindings

Recommend three logical modules:

text
reader       → observation acquisition
protocol     → validation, sequencing, expiry and modifier state
actuator     → bounded application through an authorised input interface

Ship them as one versioned pack with independent artifact hashes. Do not require three Wasm binaries when linking some together is operationally simpler.

The important boundary is privilege:

Reader receives read access where enforceable.

Protocol owns its own scratch state.

Actuator receives only approved input operations or explicitly allowed memory slots.

Network and browser privileges remain in the host bridge.

Proposed pack.json
json
{
  "pack_format": 1,
  "pack_id": "org.example.game-modifiers",
  "version": "0.1.0",
  "protocol": "slmp/1",
  "abi": "slmp-host/1",
  "host_compatibility": {
    "wasm_sha256": "..."
  },
  "components": {
    "reader": {
      "path": "reader.wasm",
      "sha256": "..."
    },
    "protocol": {
      "path": "protocol.wasm",
      "sha256": "..."
    },
    "actuator": {
      "path": "actuator.wasm",
      "sha256": "..."
    }
  },
  "requirements": {
    "observe_memory": ["game.memory.reference"],
    "observe_frames": ["frame.source"],
    "apply_trim": ["game.input.trim"]
  }
}
Negotiate capabilities; do not trust a tier claim

Add a host handshake:

json
{
  "kind": "host_capabilities",
  "protocol_version": 1,
  "deployment_tier": 2,
  "capabilities": {
    "game.memory.reference": false,
    "game.input.trim": true,
    "input.dom.synthetic": true,
    "input.browser.debugger": false,
    "frame.source": "tab-capture",
    "memory.concurrent_shared": false
  }
}

Then degrade explicitly:

text
memory unavailable → frame-only observation
frame unavailable → memory-only observation
actuation unavailable → advice-only
all observation unavailable → configuration/status only
host hash mismatch → disable absolute-address reader

Do not silently switch to key events when a trim API disappears: those have different semantics.

For Wasm-to-Wasm imports, version exported functions and data layouts. Use separate scratch memory or host-copy operations rather than writing protocol buffers into supposedly unused game memory. Browser hosts can supply imported memory/functions and call module exports.
developer.mozilla

7. One protocol across all transports

Keep Observation, Suggestion, Modifier, Outcome, provenance, authority epochs, policy revisions and expiry semantics identical.

Change only the transport adapter:

Boundary	Suitable transport
Page ↔ isolated content script	Narrow postMessage handshake, preferably transferring a dedicated MessagePort
Page ↔ worker	MessagePort; optional shared ring buffer
Same-origin collaborating contexts	BroadcastChannel, where appropriate
Content script ↔ extension worker	chrome.runtime.sendMessage / connect
Extension ↔ installed local process	Native messaging
Page/extension/local host ↔ remote service	WebSocket or request/response transport
External debugger ↔ page	CDP evaluation/bindings carrying protocol envelopes

Do not assume BroadcastChannel crosses page and extension origins. Do not assume a transferred MessagePort makes the main world trustworthy.

The bridge must:

Bind a session to a tab, frame and document instance.

Validate every message.

Enforce payload/rate limits.

Authenticate privileged operations outside the page.

Invalidate old document sessions after navigation.

Re-negotiate capabilities after reconnection.

A nonce can prevent accidental cross-talk; it cannot secure privileged secrets against malicious code sharing the same main world. Chrome explicitly warns that the page can interfere with main-world scripts.
developer.chrome

In particular, never expose a generic “call any extension API” bridge.

8. Shipping alongside private-bookmark ingestion

A defensible product pattern is a bundled, versioned capability host:

text
extension core
  ├── bookmark ingestion
  ├── permission and consent controller
  ├── signed/distributed packaged capability assets
  └── narrow per-capability brokers

But there is a product-policy constraint: “bookmark ingestion” and “arbitrary game automation” may be two unrelated purposes. Chrome Web Store requires a narrow, understandable single purpose. Calling one feature an optional pack does not resolve that issue. A separate companion extension may be the cleaner distribution.
developer.chrome

Permissions

Use:

"bookmarks" only for the bookmark feature.

"storage" for necessary persisted settings/state.

"scripting" and narrowly scoped host access for injection.

"activeTab" for user-invoked temporary access where sufficient.

Optional host permissions for ongoing access to user-selected sites.

"debugger" only for an explicitly disclosed debugger upgrade.

"nativeMessaging" only when a separately installed host is required.

The bookmarks permission warns users that the extension can read and change bookmarks; debugger has much broader warnings.
developer.chrome

Keep bookmark records, URLs, titles and private browsing-derived data out of page bridges and game/model telemetry. Disclose collection, transmission, retention and remote processing consistently in the privacy policy and store disclosures.
developer.chrome

A remote agent must not be able to request raw bookmarks through the capability bus merely because the extension possesses that permission.

9. Missing vectors and constraints
Additional execution and capture paths

chrome.scripting.executeScript() and registered content scripts: the ordinary extension attachment mechanisms.

chrome.userScripts: a separate user-script capability, not permission to evade normal extension packaging policy. Current Chrome documentation requires "userScripts", host permissions and explicit user enablement.
developer.chrome
+1

WebDriver/ChromeDriver: browser automation sessions, including attachment through a debugger address; a different product integration path from your raw CDP client.
developer.chrome

Android remote debugging through ADB forwarding: useful when desktop extension assumptions do not apply.
developer.chrome

First-party integration: serving the pack as part of the application is stronger and simpler than opportunistic injection.

User-consented screen/tab capture: an independent observation capability; it need not imply memory access or input authority.

Constraints worth putting in your capability checklist

script-src, worker-src, connect-src, 'wasm-unsafe-eval'.

Trusted Types restrictions on injection sinks.

Cross-origin frame and worker boundaries.

Navigation, frame replacement and back/forward lifecycle changes.

Memory growth and stale JavaScript views.

Main-thread scheduling and background-tab throttling.

Game build identity and address-map compatibility.

Browser-controlled input trust versus application input APIs.

Extension worker restart and reconnect recovery.

Browser-specific API support: the tables above target Chromium, not a universal extension platform.
developer.mozilla
+2

Deprecated techniques

Do not build the standard around Manifest V2 persistent background pages, legacy tabs.executeScript, or legacy event construction such as initEvent(). Legacy material and deployed software may still use them, but new implementations should use current extension APIs and event constructors. initEvent() is explicitly deprecated.
developer.chrome
+1

The resulting architecture is one protocol and one pack contract, with multiple host adapters. The tier records how the host was installed; the negotiated capability set determines what the pack may actually do.

44 sources
Compare memory export patterns across CDP vs main-world injection
Draft the capability negotiation protocol for the Wasm pack
Show how Tier 2 bridges isolated worlds to raw WebAssembly memory
Detail the failure modes when CDP attaches mid-execution
Map how SpacetimeDB consumes telemetry from these tiers




Ask a follow-up
Search
Computer
GPT-6.1 Sol
