# FlowGuard architecture

## Ownership

Tauri manages windows and trusted commands. Rust `SharedState` owns the persisted document, preferences, selected connection, observation snapshots, bounded history and scene description. Each WebView mirrors a revisioned snapshot and sends commands. Local form drafts and search text are presentation state; no native window owns the document. Browser preview is an isolated demo adapter with a different storage namespace.

Mutations are serialized through a mutex. Document changes are written to a sibling temporary file, flushed, then atomically renamed. A failed write restores the in-memory document and selection. Events broadcast after a successful commit. Clients reject older snapshot revisions. The observer runs outside the UI thread and outside the state lock; tool execution has a five-second deadline and bounded output.

The inspector can dock left/right or become another Tauri WebView. Closing its floating window restores the docked state. Restart resets the panel to a docked position. Current docking uses explicit placement controls; Adobe-style drag-to-dock tab groups and arbitrary panels are not implemented.

## Observation is not enforcement

`socketfilterfw --getglobalstate` reads the standard macOS firewall state. `lsof -nP -iTCP -sTCP:ESTABLISHED -FpcnPt` reads sockets visible to the current account. Arguments and absolute tool paths are fixed. Neither command writes settings, invokes a shell, requests root privileges, resolves domains, or captures packet payloads. macOS `proc_pidpath` resolves an executable path when permitted.

This observes a subset of established TCP sockets, not every packet, every application or short-lived connection. Socket direction is `unknown`: local/remote endpoints alone cannot determine who initiated a connection. History records first detection; polling is not continuous capture. Permission errors and unsupported platforms must not be reported as complete observation coverage.

A saved rule is an intention only. Matching includes data source, executable, exact destination, port and direction. Specific-direction rules take precedence over a both-direction fallback, then the newest rule. Unknown observed direction only matches a both-direction rule. Demo rules cannot enter the Swift enforcement policy. Native enforcement is always `unavailable` in this bundle.

## Network Extension candidate

`native/FlowGuardFilter` contains a Swift policy library and an `NEFilterDataProvider` subclass. It compiles against the local macOS SDK. Candidate policy evaluation uses an audit token and Security framework to identify the executable, exact remote endpoint and actual extension-provided traffic direction. An explicit TCP rule dispatches new flows to the provider. The implementation supports allow/drop; ask requires an external decision transport and otherwise passes traffic with the future host required to report ask as unsupported. Paused requests are bounded to 128 and time out after 15 seconds to allow. This timeout policy needs product review and signed integration testing before activation.

The library has **no executable extension target**, activation manager, authenticated host IPC, or rule acknowledgement connection. Packaging templates are reference inputs only, not installed entitlements. The desktop app does not call this provider. No claim of active protection is made.

Before deployment, add the executable system-extension target, approved signing profile and identifiers; authenticate host callers using their audit/signing identity; make rule updates transactional and versioned; acknowledge the exact applied generation; coordinate initial policy before enabling filtering; provide decision request/resume messages; and expose disconnected/error/stale generations in the UI. Do not enable an extension just because its source compiles.

Apple references: [NEFilterDataProvider](https://developer.apple.com/documentation/networkextension/nefilterdataprovider), [flow pause semantics](https://developer.apple.com/documentation/networkextension/nefilternewflowverdict/pause%28%29), [Network Extensions for the modern Mac](https://developer.apple.com/videos/play/wwdc2019/714/).

## GPU boundary

The compact communication diagram is currently a bounded code-native UI graphic in the WebView. Rust owns the renderer module and capped scene descriptions. The optional `gpu` feature implements a wgpu/Metal route pipeline, bounded mesh tessellation, resize handling and a GPU-owned render target. The native compositor connection remains unfinished; the default build does not initialize this renderer.

GPU commands, geometry, buffers and textures remain in Rust/wgpu. JavaScript receives application state, identifiers and small scene metadata, never image buffers or rendered frames. There is intentionally no GPU readback or frame IPC API. The Metal smoke check submits a route to the GPU-owned texture without exporting pixels.

## Local data

Rules and preferences are stored under Tauri's application data directory. JSON backups contain executable paths and destinations. History is in-memory only (up to 500 first-seen entries per session) and clears at exit. No remote backend, analytics, telemetry, credential storage, reverse DNS or threat service is connected. Restore validates schema, enum values, bounds and duplicate IDs; the UI offers one-step undo. Concurrent independent processes and stale undo conflicts require the next coordination milestone.
