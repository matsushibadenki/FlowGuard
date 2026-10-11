# FlowGuard architecture

## Ownership

Tauri manages windows and trusted commands. Rust `SharedState` owns the persisted document, preferences, selected connection, observation snapshots, bounded history and scene description. Each WebView mirrors a revisioned snapshot and sends commands. Local form drafts and search text are presentation state; no native window owns the document. Browser preview is an isolated demo adapter with a different storage namespace.

Mutations are serialized through a mutex. Document changes are written to a sibling temporary file, flushed, then atomically renamed. A failed write restores the in-memory document and selection. Events broadcast after a successful commit. Clients reject older snapshot revisions. The observer runs outside the UI thread and outside the state lock; tool execution has a five-second deadline and bounded output.

The inspector can dock left/right or become another Tauri WebView. Closing its floating window restores the docked state. Restart resets the panel to a docked position. Current docking uses explicit placement controls; Adobe-style drag-to-dock tab groups and arbitrary panels are not implemented.

## Observation is not enforcement

`socketfilterfw --getglobalstate` reads the standard macOS firewall state. `lsof -nP -iTCP -sTCP:ESTABLISHED -FpcnPt` reads sockets visible to the current account. Arguments and absolute tool paths are fixed. Neither command writes settings, invokes a shell, requests root privileges, resolves domains, or captures packet payloads. macOS `proc_pidpath` resolves an executable path when permitted.

This observes a subset of established TCP sockets, not every packet, every application or short-lived connection. Socket direction is `unknown`: local/remote endpoints alone cannot determine who initiated a connection. History records first detection; polling is not continuous capture. Permission errors and unsupported platforms must not be reported as complete observation coverage.

A saved rule is an intention only. Matching includes data source, executable, exact destination, port and direction. Specific-direction rules take precedence over a both-direction fallback, then the newest rule. Unknown observed direction only matches a both-direction rule. Demo rules cannot enter the Swift enforcement policy. Unsigned enforcement remains unavailable. A signed runtime can report a correlated provider acknowledgement separately from stored intentions; it does not prove a particular flow was blocked.

## Network Extension candidate

`native/FlowGuardFilter` contains a Swift policy library and an `NEFilterDataProvider` subclass. It compiles against the local macOS SDK. Candidate policy evaluation uses an audit token and Security framework to identify the executable, exact remote endpoint and actual extension-provided traffic direction. An explicit TCP rule dispatches new flows to the provider. The implementation supports allow/drop; ask requires an external decision transport and otherwise passes traffic with the future host required to report ask as unsupported. Paused requests are bounded to 128 and time out after 15 seconds to allow. This timeout policy needs product review and signed integration testing before activation.

The package now includes an executable extension target, a compiled activation/configuration API, signature-gated host IPC and a versioned acknowledgement contract. The unsigned staging script embeds the extension and helper but does not install entitlements or activate them. The desktop calls a bundled Swift runtime through a versioned C ABI for activation and uses fixed-helper status/sync commands. Rust owns lifecycle and policy receipt state; all WebViews display the same revisioned snapshot. The unsigned runtime rejects activation before calling OS configuration APIs.

Before deployment, configure approved signing profiles and verify IPC identities in a real signed environment; provide decision request/resume messages; implement automatic resync, startup-policy refresh for subsequent edits and uncertain-timeout recovery. Do not enable an extension just because its source compiles.

Apple references: [NEFilterDataProvider](https://developer.apple.com/documentation/networkextension/nefilterdataprovider), [flow pause semantics](https://developer.apple.com/documentation/networkextension/nefilternewflowverdict/pause%28%29), [Network Extensions for the modern Mac](https://developer.apple.com/videos/play/wwdc2019/714/).

## GPU boundary

The compact communication diagram is currently a bounded code-native UI graphic in the WebView. Rust owns the renderer module and capped scene descriptions. The optional `gpu` feature implements a wgpu/Metal route pipeline, bounded mesh tessellation, resize handling and a GPU-owned render target. The native compositor connection remains unfinished; the default build does not initialize this renderer.

GPU commands, geometry, buffers and textures remain in Rust/wgpu. JavaScript receives application state, identifiers and small scene metadata, never image buffers or rendered frames. There is intentionally no GPU readback or frame IPC API. The Metal smoke check submits a route to the GPU-owned texture without exporting pixels.

## Local data

Rules and preferences are stored under Tauri's application data directory. JSON backups contain executable paths and destinations. History is in-memory only (up to 500 first-seen entries per session) and clears at exit. No remote backend, analytics, telemetry, credential storage, reverse DNS or threat service is connected. Restore validates schema, enum values, bounds and duplicate IDs; the UI offers one-step undo. A separate stable `.lock` file is held with an OS advisory file lock for the document lifetime; another compliant FlowGuard process is rejected before loading or writing. Rule mutation requests carry the rule list they were based on, compared inside the session mutex before persistence. Conflicts emit the latest snapshot and reject the stale request. Undo carries the post-operation rules as its baseline, so later edits cannot be overwritten. Legacy builds that do not use this lock must be closed before running this version. Native application groups are projected by Rust using executable paths; unidentified processes stay separate. Signature identity remains unfinished.


## Unsigned native packaging increment

The Swift package now builds `FlowGuardSystemExtension` and `FlowGuardBridgeCLI`. `npm run native:package` stages an unsigned app copy with the executable extension under `Contents/Library/SystemExtensions` its helper under `Contents/MacOS`, and `libFlowGuardHostRuntime.dylib` under `Contents/Frameworks`. Provider registration uses the explicit Objective-C name `FlowGuardFilterDataProvider`. This native package requires macOS 13 for public XPC signing requirement APIs.

The shared bridge contract uses protocol v1, request UUID, per-process provider instance UUID and expected revision. The running provider's receiver serializes validation and replacement, rejects demo/ask policies and preserves its previous revision when application fails. The signed helper authenticates the provider, the provider authenticates the helper, and the helper authenticates its signed desktop parent. Anonymous XPC tests exercise the same contract without starting the production listener or weakening its signature requirements.

Rust loads the fixed bundled runtime, checks ABI version 2 and retains it for asynchronous callback lifetime. Swift dispatches signed activation onto the main queue. Activation callbacks are correlated to the current operation; approval and configuration persistence never imply policy application.

Sync records the exact live-rule snapshot sent to the provider. Only a correlated acknowledgement whose rules still match the current document yields `applied` and per-rule receipt badges. Concurrent edits yield `stale`; disconnect or a different provider instance/revision clears confirmation. Status refresh polls every five seconds when signed and idle, and never restores a lost receipt merely from a revision number. Receipts are session-local. Failures may leave an older policy active, which the UI explicitly explains. No automatic resync or uncertain-timeout reconciliation is implemented yet. All signed sandbox/IPC and OS-filter behavior remains Pending. See `native/FlowGuardFilter/README.md` for reproduction commands.

## Initial policy and removal

Activation captures validated live rules under the Rust document lock before any OS request. C ABI v2 copies bounded JSON into Swift. The host saves it as Data in `NEFilterProviderConfiguration.vendorConfiguration`; the provider validates and loads it before applying network settings or announcing startup. Missing, duplicate, demo, ask and oversized initial policies fail startup rather than silently substituting an empty policy. This configuration persists locally in macOS preferences. See [Apple vendorConfiguration](https://developer.apple.com/documentation/networkextension/nefilterproviderconfiguration/vendorconfiguration).

After configuration save, Rust keeps the activation operation busy, probes provider readiness up to ten times, and sends one correlated policy update. An uncertain update is never automatically repeated. The acknowledgement is compared with the current document, so edits during approval or synchronization produce stale state. A reboot-required activation does not initiate sync; resumed activation remains explicit.

Disable loads only our configuration, saves `isEnabled=false`, then reloads and verifies before reporting `disabled`. Removal follows disable, deletes our preferences and submits System Extension deactivation; OS failure, approval and reboot requirements remain distinct from completion. An absent disabled configuration skips the unnecessary save. Another provider or unidentified enabled configuration is rejected. Rust clears receipts and suppresses provider polls after confirmed disable/removal; saved document rules remain intact. A disabled setting is not proof that an existing connection has resumed.

Subsequent manual rule sync currently changes the running in-memory policy only. On provider restart the bootstrap rules from the last activation may return, not the last manual sync. Keeping OS bootstrap configuration current and reconciling after restart are high-priority next work; signed production use is not validated.
