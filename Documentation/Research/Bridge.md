chars: 6452
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

How can a Manifest V3 content script safely bridge an isolated world to a host page raw WebAssembly memory, and what does main-world injection cost?

Finished
6s

A safe MV3 bridge keeps the WebAssembly instance and its memory in the page’s MAIN world, then exposes only a narrow, validated message protocol to the extension’s ISOLATED-world content script. Do not try to hand the page’s WebAssembly.Memory object directly to isolated-world code: the worlds share the DOM, but not JavaScript objects or globals.
developer.chrome
+1

Recommended architecture

MAIN-world script: loads/instantiates WASM, owns WebAssembly.Memory, performs reads/writes, and listens for requests.

ISOLATED-world content script: validates messages, talks to your service worker with chrome.runtime, and never trusts page-supplied offsets, lengths, or function names.

Bridge: window.postMessage or CustomEvent on document; both cross the world boundary through structured clone rather than exposing page JS objects.
spaceraccoon
+1

js
// main-bridge.js — world: "MAIN"
let wasmMemory;

window.addEventListener("message", (event) => {
  if (event.source !== window) return;

  const msg = event.data;
  if (msg?.type !== "MY_EXT_WASM_REQUEST") return;

  // Validate strictly: never accept arbitrary function names or unbounded ranges.
  if (msg.op !== "checksum" || !Number.isSafeInteger(msg.offset) ||
      !Number.isSafeInteger(msg.length) || msg.length > 1_000_000) {
    window.postMessage({ type: "MY_EXT_WASM_RESPONSE", id: msg.id, error: "bad_request" }, "*");
    return;
  }

  const bytes = new Uint8Array(wasmMemory.buffer, msg.offset, msg.length);
  let sum = 0;
  for (const b of bytes) sum = (sum + b) >>> 0;

  window.postMessage({
    type: "MY_EXT_WASM_RESPONSE",
    id: msg.id,
    result: sum
  }, "*");
});
js
// isolated-content.js — default ISOLATED world
let nextId = 0;
const pending = new Map();

window.addEventListener("message", (event) => {
  if (event.source !== window) return;
  const msg = event.data;
  if (msg?.type !== "MY_EXT_WASM_RESPONSE") return;

  const resolve = pending.get(msg.id);
  if (resolve) {
    pending.delete(msg.id);
    resolve(msg.error ? Promise.reject(new Error(msg.error)) : msg.result);
  }
});

function requestWasmChecksum(offset, length) {
  const id = nextId++;
  return new Promise((resolve) => {
    pending.set(id, resolve);
    window.postMessage(
      { type: "MY_EXT_WASM_REQUEST", id, op: "checksum", offset, length },
      "*"
    );
  });
}

Use a unique message namespace, check event.source === window, validate every field, and prefer request IDs over implicit ordering. The isolated world is specifically designed so page JavaScript cannot read or overwrite extension globals; the shared DOM/postMessage channel is the intended boundary.
developer.chrome
+1

Moving raw memory

There are three practical choices:

Approach	What crosses the boundary	Cost	When to use
Return computed results	Small scalar/string/array result	Low; structured-clone copy of result only	Most extensions
Copy a byte range	ArrayBuffer or typed-array copy	Proportional to bytes copied; allocation + clone	Moderate, one-shot exports
Share a SharedArrayBuffer	Reference to the same backing bytes	Near-zero marginal copy cost; requires cross-origin isolation	Large, high-frequency, cooperative workloads

A non-shared WebAssembly.Memory.buffer is an ArrayBuffer; posting it normally produces a copy, while transferring it moves ownership and leaves the sender without access. For true zero-copy sharing, create the memory with shared: true; its backing buffer becomes a SharedArrayBuffer, and structured clone shares the backing store rather than copying bytes.
developer.mozilla
+3

However, SharedArrayBuffer requires a secure context and cross-origin isolation (COOP/COEP); check window.crossOriginIsolated before relying on it. That makes it unsuitable as a general solution for arbitrary host pages—you cannot force a third-party site to become cross-origin isolated.
developer.mozilla

What MAIN-world injection costs

Declaring "world": "MAIN"—either in content_scripts or via chrome.scripting.executeScript—runs your code in the page’s own JavaScript realm. You gain access to page globals and can intercept or call page-defined APIs, but you lose access to chrome.* extension APIs in that script.
developer.chrome
+2

The costs are:

Security: your MAIN-world code runs amid hostile, untrusted page JavaScript. The page can observe globals you create, tamper with prototypes or built-ins it controls, and attempt to impersonate your bridge. Keep the MAIN-world payload tiny and defensive.
mv3-extension
+2

No direct extension APIs: chrome.runtime.sendMessage, storage, and similar APIs are unavailable there; relay through the ISOLATED world.
extensionbooster
+1

Main-thread contention: the injected code executes on the page’s main thread, competing with rendering, layout, input handling, and the site’s own scripts. Heavy polling, large memory scans, or synchronous WASM work can cause jank.
mv3-extension

Injection/timing overhead: there is an additional script evaluation and message-channel setup cost. It is usually small compared with WASM compilation, but inject at document_start only if you must hook APIs before page scripts run.
stackoverflow

Operational fragility: hooks into page internals can break when the site changes its bundling, globals, or API shapes.

Practical rule

Use MAIN only for the minimum WASM ownership/interception surface; keep parsing, UI, permissions, storage, and extension communication in ISOLATED. For most cases, return small validated results across postMessage; reserve SharedArrayBuffer for pages you control or that are already cross-origin isolated.
developer.chrome
+2

27 sources
Build an IPC benchmark suite for MV3 isolated vs MAIN world bridging — measuring latency overhead for WebAssembly memory slicing vs postMessage and CustomEvents
Computer
Manifest V3 WebAssembly bridge architecture: comparison guide for custom element portals, DOM CustomEvents, and MAIN-world script injection patterns
Computer
Show me the ISOLATED world side of that postMessage bridge
How do I use SharedArrayBuffer safely with WASM across worlds
What are the security risks of world MAIN content scripts
⠸




Ask a follow-up
Search
Computer
Model
