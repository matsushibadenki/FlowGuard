# FlowGuard native extension and host bridge

This package targets macOS 13+ for Foundation's public XPC code-signing requirements. The ordinary observation app remains compatible with macOS 12; the staged extension bundle requires 13.

## Unsigned validation

```sh
npm run tauri build -- --debug
npm run native:package
```

The second command builds two release executables and the host runtime dynamic library and copies the Tauri app to `native/build/FlowGuard.app`. It never signs, installs, activates, or changes a network filter. The default Mach service prefix is `UNSIGNED`; this is deliberately unusable in production. `-debug-info-format none` avoids dSYM creation during staging; this bundle is for validation, not release distribution.

Layout:

```text
FlowGuard.app/Contents/
  MacOS/flowguard                         # Rust / Tauri host
  MacOS/FlowGuardBridge                   # Swift XPC client
  Frameworks/libFlowGuardHostRuntime.dylib # Swift activation C ABI v3
  Library/SystemExtensions/
    com.flowguard.filter.systemextension/Contents/
      Info.plist
      MacOS/FlowGuardSystemExtension
```

Read-only unsigned preflight:

```sh
native/build/FlowGuard.app/Contents/MacOS/FlowGuardBridge --preflight \
  native/build/FlowGuard.app/Contents/Library/SystemExtensions/com.flowguard.filter.systemextension
```

Expected result: `packaged: true`, `signed: false`, `status: signature_required`. `signed` describes the helper's validated Team ID, not a verification of the entire containing bundle. Tauri's `extension_package_status` reports `packaged_unverified`; it never infers enforcement from package presence.

Swift tests use policy tests and an anonymous, test-only XPC endpoint with the production interface and state machine. No production Mach service or Network Extension is started. The fixture does not disable signing checks in production code.

## Host / provider protocol

- `--status` queries a running provider; `--sync` accepts `{"rules":[...]}` on stdin and performs status → update → correlated acknowledgement. `--push` accepts a preconstructed update envelope for diagnostics. All three require a valid signed `com.flowguard.desktop` parent of the same team; unsigned/manual callers are rejected. Preflight is read-only and has no such requirement.
- Native Tauri commands `extension_bridge_status` and `sync_extension_rules` call only the fixed bundled helper. Sync excludes demo and quarantined legacy rules; ask rules carry allow/block timeoutAction. Settings provides explicit activation, policy apply and status refresh actions. Unsigned policy apply is disabled.
- Protocol v3 updates carry a UUID request ID, the provider's per-process instance ID and expected revision. The receiver serializes validation and application. Invalid, stale, demo, duplicate, oversized or invalid policies never reach the provider. Failed application preserves the previous receiver state.
- Successful replies contain `accepted`, the exact next revision, request ID, instance ID and rule count. The host validates correlation. This means the running provider accepted that policy snapshot, not that any particular connection was blocked. Rust compares the exact acknowledged live rules with the current saved document. Matching rules get receipt badges; edits, disconnects or provider restarts remove confirmation. Unsaved inspector changes cannot inherit a previous receipt.
- The listener validates the signed helper's identifier and Team ID; the helper validates the signed provider's identifier and Team ID. The helper also validates its parent to avoid becoming a privileged proxy for arbitrary local callers.
- Each XPC transaction has a 5-second timeout and is invalidated after one completion. Reconnecting starts with status; the new instance ID prevents replay after provider restart. Startup requires a valid bootstrap policy from the OS configuration; missing or invalid bootstrap data fails startup. Later manual sync updates only in-memory rules, so restart can restore the last activation policy until bootstrap refresh is implemented. Receipts are session-local. Automatic resync and reconciliation after uncertain timeouts remain next work; an older policy may remain active after an uncertain failure.

## Signed integration boundary

`FlowGuardHost.ExtensionController` compiles the SystemExtensions activation delegate and NEFilterManager configuration lifecycle. It requires the signed containing desktop bundle and explicit user activation. It reports approval/reboot/failure/configuration-saved separately and refuses another provider's existing configuration. Rust loads the fixed bundled `FlowGuardHostRuntime` library and calls its C ABI from the signed desktop process. The CLI does not activate. Swift callbacks update Rust shared lifecycle state and the trilingual UI. Unsigned activation returns `signature_required` synchronously before any OS API is called. Configuration saved, approval required and restart required are distinct from a policy acknowledgement. Activation includes a validated initial policy in the OS provider configuration and automatically synchronizes after provider readiness. Disable and removal are connected through the same signature-gated runtime.

For future signed packaging, `scripts/package-native.py --team-id TEAMID` expands the actual Mach service prefix but still does not sign anything. `Packaging/Host.entitlements` and `Packaging/Extension.entitlements` are inputs for the future Developer ID signing pipeline. The helper must be signed with identifier `com.flowguard.bridge`, the extension with `com.flowguard.filter`, and the host with `com.flowguard.desktop`, all from the same team. The embedded runtime must also be signed for hardened-runtime library validation. Entitlements, provisioning, nested-code signing, hardened runtime and notarization require a real distribution configuration and verification. The app must be installed in the OS-supported location before activation testing.

Signed Mach-service registration, sandbox permissions, real peer-authentication rejection, System Extension approval, NEFilterManager permissions, actual allow/drop, crash recovery and uninstall remain pending. No signature bypass, SIP change, ad-hoc trust exception or network-filter installation is part of this validation.

References: [Apple's traffic-filtering sample](https://developer.apple.com/documentation/networkextension/filtering-network-traffic), [System Extension entry point](https://developer.apple.com/documentation/networkextension/neprovider/startsystemextensionmode()), [XPC peer code-signing requirements](https://developer.apple.com/documentation/foundation/nsxpcconnection/setcodesigningrequirement(_:)).

## Real unsigned FFI gate check

After staging, run `cargo test --offline --manifest-path src-tauri/Cargo.toml unsigned_runtime_gate_crosses_real_swift_ffi -- --ignored`. The runtime test loads the actual release Swift dylib, checks ABI version 3 and invokes activation, disable, removal and startup policy persistence. It verifies the unsigned callback returns `signature_required`, without installing an extension or starting a production service.

## Disable / removal and initial synchronization

Settings exposes **Disable filter** and **Remove extension**. Disable saves and reloads our disabled configuration; removal also removes preferences and submits the OS deactivation request. Neither deletes the saved FlowGuard document. Another product’s configuration is never intentionally modified. Completion describes saved OS configuration / lifecycle replies, not verified traffic recovery.

Activation validates live signed rules and ask fallback before submitting an OS request. The initial live-rule snapshot is copied into secure-coding-compatible Data in vendorConfiguration. The provider loads it before accepting flows. After configuration save, Rust probes readiness and performs one acknowledged synchronization; edits during the operation remain stale. Reboot requests do not bypass user approval or silently resume activation. OS persistence, provider startup ordering, deactivation, traffic recovery and reboot integration remain Pending in a signed environment.

## Signed rule subjects (2026-10-14)

Live policies now require SigningIdentity and a matching canonical signed subject key. Flow identity is verified from sourceAppAuditToken against Apple / Apple-generic-plus-Team-ID signing requirements; executable paths are not used to allow/drop. Identity-less path rules are rejected by InitialPolicy and bridge validation. Rust retains them as disabled intentions for explicit per-rule migration.

Host/provider protocol and digest are v4; initial policy uses flowguard.initialPolicy.v4. Saving removes obsolete v1/v2/v3 slots and verifies removal after reload. Replace desktop/helper/extension together; old replies are rejected. Backup exports are v4 (imports accept v1/v2/v3/v4). Source edits do not deactivate a running old provider.

AskQueue owns bounded paused-flow callbacks, 15-second ContinuousClock deadlines, captured allow/block fallback and one-shot answers. Authenticated pendingAsks/answerAsk XPC is exposed through the bundled helper, polled by Rust and presented in the shared UI. Default fallback is allow to avoid disrupting legitimate traffic without a UI response; users may choose block. Capacity/transport-unavailable intake, policy replacement and graceful shutdown use captured fallback. Actual OS pause/resume, signing-gated listener deployment, crash/sleep and network continuity remain Pending.

SystemProtection protects only verified Apple mDNSResponder outbound TCP/53 and softwareupdated outbound TCP/443. The winning exact rule must carry systemException=true to override default protection. Inbound and UDP/mDNS are excluded. Contract v4 digest includes the exception flag. Real flow and upgrade validation remain Pending.

Emergency disable can preempt an active lifecycle operation. The host serializes preference I/O, waits for an already submitted OS operation's real callback, cancels its continuation and disables before any more normal work. Approval-wait delegate identities are invalidated. C ABI v4 transports operation generations; stale queued submissions are rejected. The Rust host and native runtime must update together (old ABI3 rejected); XPC/policy remains protocol v4. Recovery latches persist until explicit activation within the process. A hung OS operation is never treated as completed at a caller timeout. Actual OS recovery and process-restart continuity remain Pending.
