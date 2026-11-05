chars: 9564
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

Design a capability negotiation protocol for one WebAssembly pack that must run in-page, in a browser extension, and behind CDP, with identical schemas.

Finished
3s

Design the pack as a single WIT-defined WebAssembly component with a strict, capability-only boundary. The same component binary and schema run in-page, in an extension worker, and under CDP; only the host adapter changes. This follows WASI’s no-ambient-authority model: the guest starts with no access and receives only explicitly granted, typed capabilities.
wasi

Core principle

Do not negotiate “permissions” as strings or booleans inside the pack. Negotiate typed resource handles at instantiation time, then expose every operation through an interface that requires one of those handles.

The pack declares required interfaces, versions, and constraints in one canonical WIT package.

Each host implements the same WIT world with a different backend.

The host grants opaque, unforgeable handles; the pack cannot invent or widen them.

The Component Model’s typed imports/exports provide the contract and static inspection surface.
component-model.bytecodealliance
+1

Canonical schema

Use one WIT package as the sole source of truth, versioned semantically and content-hashed into the pack manifest.

text
package example:pack@1.0.0;

interface core {
  record pack-info {
    name: string,
    version: string,
    schema-hash: string,
  }

  init: func(config: config) -> result<session, pack-error>;
  shutdown: func();
}

interface capabilities {
  /// Host-minted, opaque capability handles.
  resource dom-read;
  resource dom-write;
  resource storage;
  resource messaging;
  resource cdp-session;
  resource clock;
  resource random;
}

interface dom {
  use capabilities.{dom-read, dom-write};

  query: func(cap: borrow<dom-read>, selector: string) -> result<list<string>, pack-error>;
  mutate: func(cap: borrow<dom-write>, patch: dom-patch) -> result<_, pack-error>;
}

interface kv {
  use capabilities.{storage};

  get: func(cap: borrow<storage>, key: string) -> result<option<list<u8>>, pack-error>;
  set: func(cap: borrow<storage>, key: string, value: list<u8>) -> result<_, pack-error>;
}

interface transport {
  use capabilities.{messaging};

  send: func(cap: borrow<messaging>, envelope: message) -> result<receipt, pack-error>;
  subscribe: func(cap: borrow<messaging>, filter: string) -> result<subscription, pack-error>;
}

interface debug {
  use capabilities.{cdp-session};

  evaluate: func(cap: borrow<cdp-session>, expression: string) -> result<string, pack-error>;
  inspect: func(cap: borrow<cdp-session>, target: string) -> result<string, pack-error>;
}

world pack-host {
  import example:pack/core@1.0.0;
  import example:pack/dom@1.0.0;
  import example:pack/kv@1.0.0;
  import example:pack/transport@1.0.0;
  import example:pack/debug@1.0.0;

  export example:pack/core@1.0.0;
}

WIT is appropriate because it defines language-neutral imports, exports, types, and resources, enabling typechecked composition across hosts.
component-model.bytecodealliance
+2

Negotiation handshake

Use a deterministic three-phase handshake before any capability-bearing call.

1. Manifest offer

The host reads the pack manifest, which contains:

json
{
  "pack": "example.pack",
  "version": "1.0.0",
  "wit": {
    "package": "example:pack@1.0.0",
    "world": "pack-host",
    "schemaHash": "sha256:..."
  },
  "requires": [
    { "interface": "example:pack/dom@1.0.0", "mode": "read" },
    { "interface": "example:pack/kv@1.0.0", "scope": "namespace:cart" },
    { "interface": "example:pack/transport@1.0.0", "channels": ["ui", "background"] }
  ],
  "optional": [
    { "interface": "example:pack/debug@1.0.0", "when": "cdp" }
  ]
}

The manifest is descriptive, not authoritative: the host still validates it against the component’s actual imports.

2. Host grant

The host returns a capability grant, not a permission flag:

json
{
  "protocol": "example.pack.capability-negotiation",
  "version": 1,
  "session": "sess_01J...",
  "granted": [
    {
      "interface": "example:pack/dom@1.0.0",
      "capability": "dom-read",
      "handle": "cap_dom_read_7f2...",
      "constraints": { "origins": ["https://shop.example"], "selectors": ["#cart *"] }
    },
    {
      "interface": "example:pack/kv@1.0.0",
      "capability": "storage",
      "handle": "cap_storage_91b...",
      "constraints": { "namespace": "cart", "maxBytes": 65536 }
    }
  ],
  "denied": [
    {
      "interface": "example:pack/debug@1.0.0",
      "reason": "no-cdp-session"
    }
  ],
  "limits": {
    "callsPerSecond": 50,
    "memoryBytes": 16777216,
    "executionMsPerCall": 10
  }
}

Capabilities should be scoped, revocable, rate-limited, and resource-budgeted; capability possession alone does not eliminate misuse risk.
wasi
+1

3. Session acknowledgment

The pack calls init(config) with a canonical, host-generated session and a normalized view of grants. It must reject startup if:

A required interface is absent.

The schema hash differs.

A granted handle lacks a required constraint.

The host protocol version is unsupported.

After init, all calls carry a borrowed handle. The host rejects calls using an expired, revoked, mismatched, or forged handle.

Host adapters
Runtime	Capability source	DOM	Storage	Messaging	CDP
In-page	Page-origin policy plus embedded host shim	Real DOM, selector-scoped	IndexedDB / origin storage	postMessage / BroadcastChannel	Usually denied
Extension	Extension permissions plus per-tab policy	chrome.scripting / content-script bridge	chrome.storage namespaced	Extension message port	Extension debugger permission, if present
CDP	Automation policy plus target attachment	Runtime.evaluate, DOM domain	Host-side virtual store or mapped profile storage	CDP events / bindings	Native cdp-session capability

The key is that these adapters implement the same WIT interfaces, so the pack has no if (isExtension) branches in its business logic. Chrome DevTools can debug WebAssembly with source-level breakpoints and inspection, but CDP should be treated as a separately granted debug capability rather than an implicit ambient channel.
developer.chrome

Transport and framing

Use one envelope schema for all runtimes:

json
{
  "v": 1,
  "session": "sess_01J...",
  "cap": "cap_storage_91b...",
  "op": "kv.set",
  "nonce": "01J...",
  "payload": {
    "key": "items",
    "value": "base64..."
  },
  "constraints": {
    "namespace": "cart"
  }
}

Rules:

cap is an opaque token; the host resolves it to a validated capability object.

op must map to an exact WIT function.

The host validates the operation against the capability’s constraints before invoking the component.

Responses include ok, err, revoked, or limit-exceeded.

Every request carries a nonce and monotonic sequence number to prevent replay within a session.

Security rules

No ambient APIs. The component imports no global fetch, DOM, storage, or extension API directly; it receives handles only. This mirrors WASI’s explicit grant model.
wasi

Least privilege. Grant dom-read without dom-write where possible; grant one storage namespace, not global storage.

Capability confinement. A DOM capability should encode origin, frame, selector scope, and mutation policy. A messaging capability should encode permitted channels and peer identities.

Separate debug authority. cdp-session must never be bundled with ordinary page capabilities; CDP can inspect and control runtime state, so it deserves its own grant and audit trail.

Revocation. Hosts must be able to revoke a capability immediately. The pack should treat subsequent use as a recoverable revoked error, not a crash.

Deterministic fallback. If an optional capability is denied, the pack must run in a defined reduced mode or fail cleanly; it must not probe for undocumented host APIs.

Versioning

Use strict compatibility:

Patch: bug fixes only; schema hash unchanged.

Minor: additive optional interfaces, new optional fields, backward-compatible result variants.

Major: changed function signatures, removed interfaces, changed resource semantics, or changed constraint meaning.

The host must compare the WIT package version and schemaHash; if either mismatches, negotiation fails before instantiation. The Component Model’s interface contracts are intended to make such composition decisions explicit and typechecked.
component-model.bytecodealliance
+1

Minimal negotiation flow
text
Host -> Pack: load component + manifest
Pack -> Host: describe() { protocol, world, schemaHash, required, optional }
Host -> Pack: grant(session, capabilities[], limits[])
Pack -> Host: init(config)
Host -> Pack: ready(session)
Pack -> Host: capability-scoped calls
Host -> Pack: revoke(capabilityId) / expire(session)
Pack -> Host: shutdown()

This gives you one pack, one schema, and three enforcement boundaries—without letting the browser page, extension, or CDP controller become an implicit source of authority.

15 sources
Wasm capability negotiation protocol: WIT schemas, host-binding maps, and CDP message-passing specs for in-page vs extension vs DevTools runtimes
Computer
Build a WebAssembly component specification test suite with mock host runtimes for DOM, extension background, and CDP targets
Computer
Show me the rest of the WIT file
Detail the in-page host adapter implementation
Detail the CDP host adapter implementation
⠋




Ask a follow-up
Search
Computer
Model
