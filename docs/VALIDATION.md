# Validation — 2026-10-11

Environment: Apple Silicon macOS, Node 26.10.0, Cargo 1.99.0, local Xcode/macOS SDK. Browser checks used the connected Brave browser through CUA; native checks used FlowGuard's real macOS WebView windows.

| Check | Result |
| --- | --- |
| TypeScript + Vite production build | Passed |
| UI rule contract tests | 5 passed |
| Rust observation, persistence, scope and GPU geometry tests | 6 passed with `gpu` feature |
| Swift policy tests and provider source compilation | 3 policy tests passed |
| Real established TCP observation and firewall state | Passed in native app |
| Floating inspector → save rule → dock → same rule in main window | Passed |
| Save intent, show not applied, delete and undo | Passed |
| Native import JSON → replace rules → undo → restore previous list | Passed |
| Native export JSON file | Valid FlowGuard v1 backup confirmed |
| Japanese / English / Simplified Chinese switch | Passed |
| Search empty state, clear search, rule-intent filter | Passed |
| Browser application console | No application errors; warnings came from a browser extension |
| 320 / 375 / 414 / 768px layouts | Root width equals viewport; no page horizontal overflow |
| Metal GPU smoke check | Pipeline initialized and route submitted; no frame readback |
| System Extension activation and OS enforcement | Pending; not installed |

Browser file-chooser automation was restricted by the extension's file-access setting. This was resolved for the app workflow using the native file picker; browser permissions were not changed. Test screenshots were saved outside the source tree. The generated ImageGen concept lives under `docs/design/concept.png`; it was used for the layout and palette, while unsaved routes correctly remain neutral rather than implying active protection.

The GPU smoke check requires access to Metal; the sandbox did not expose an adapter. The same check passed with host GPU access. Unit tests and compilation do not substitute for signed Network Extension enforcement tests.

## Roadmap increment — 2026-10-11

- Frontend production build and updated macOS debug `.app` bundle: passed.
- Rust tests with `gpu`: 9 passed, including duplicate-instance rejection / lock release, stale rule edit rejection, executable grouping and unknown process separation.
- TypeScript contract tests: 7 passed, including grouped IDs, distinct processes / destinations and same-name executable separation.
- Browser UI: application expansion and individual connection selection update the inspector; 375px layout has document width exactly 375px. Desktop and mobile screenshots inspected; no application console errors.
- Native updated-window concurrency UI was not re-run against the user's currently open old app. Coordination is tested at the Rust state boundary. Close all older FlowGuard builds before launching the rebuilt app; older versions do not participate in the new file lock.
- Signed extension activation and OS enforcement remain pending, as before.

## Network Extension packaging / host bridge — 2026-10-11

- Swift tests: 9 passed (3 policy, 5 bridge state/acknowledgement and 1 anonymous XPC transport roundtrip). Both executables and the activation/configuration host API compile against the local SDK.
- Rust tests: 10 passed, including staged package completeness and the exact Swift `requestID` / `instanceID` reply fields. Frontend tests: 7 passed.
- Tauri debug `.app` and Swift release executables build. `npm run native:package` generates `native/build/FlowGuard.app`, including the extension and helper; minimum staged macOS version is 13.
- Verified Info.plist executable/class/service metadata, the linked Objective-C provider runtime symbol, unsigned preflight (`packaged=true`, `signed=false`, `signature_required`) and rejection of manual/unsigned `--status` (`trusted_desktop_required`).
- Browser Settings diagnostic text visually inspected; it accurately reports that the browser has no native package and does not imply OS enforcement. No application console errors.
- Anonymous XPC uses a test-only listener endpoint and never registers the production Mach service. Signature requirements remain enforced in the production listener/client. Real signed IPC, sandbox grants, activation, approval, NEFilterManager persistence and actual allow/drop are Pending.
- No System Extension installation, signature exception, network configuration change or traffic blocking was performed. Activation/configuration source is compiled but still needs a Rust-process FFI binding and lifecycle UI.

## Activation FFI / shared application state — 2026-10-11

- Rust: 14 tests passed with `--include-ignored`, including the actual staged Swift dynamic library loaded through the production Rust FFI. ABI version and unsigned activation callback (`signature_required`) were verified without invoking OS activation APIs.
- Frontend: 10 tests passed, including receipt visibility, stale/uncertain/demo messaging and all three language labels. Swift: 9 tests passed. Total: 33 tests.
- Rust state tests cover approval/configuration without false application, edits during apply, provider restart/revision changes, disconnect, unsigned rejection and late callbacks.
- Production frontend, Tauri debug app and unsigned native staging build successfully. The staged bundle now includes the Swift host runtime under `Contents/Frameworks`.
- Signed callback integration, real System Extension approval, NEFilterManager configuration, authenticated production XPC and actual allow/drop remain Pending. No signing exception, extension installation or network-filter change was performed.
- Browser Settings checked in Japanese, English and Simplified Chinese; 375px document width matches the viewport with no horizontal overflow. Native actions are correctly disabled in browser preview. No application console errors. Desktop screenshot saved and inspected.

## Disable / removal / initial policy — 2026-10-11

- Rust: 17 tests pass, including real Swift ABI v2 activation, disable and remove calls. All three unsigned calls return `signature_required` before OS mutation. State tests cover busy initial sync, concurrent edits, receipt clearing, late callbacks, failure and removal reboot state.
- Swift: 12 tests pass, including initial-policy validation and exact rule decision, rejection of invalid/demo/ask/duplicate/oversized bootstrap payloads, and protection of other or unidentified filter configurations. No real NEFilterManager preference operations are tested.
- Frontend: 10 tests pass with new lifecycle labels and receipt rules. Total: 39 tests. Production frontend, Tauri debug app and unsigned staging build successfully.
- Signed OS preference persistence, provider initialization, authenticated synchronization, actual disable/removal, user rejection, restart, existing-flow recovery and rollback behavior remain Pending. No extension was installed, removed or enabled and no firewall setting was changed.
- Browser Settings reviewed in all three languages and at 375px width: no horizontal overflow, native-only lifecycle actions disabled in preview, readable wrapping and no application console errors. Screenshot inspected.

## Classified workspaces / local interface sheet — 2026-10-12

- Rust: 19 tests passed with `--include-ignored`, including parsing IPv4/IPv6 and scope identifiers, UP/DOWN flags and a real local `/sbin/ifconfig -a` read. No probes or configuration changes were performed. Frontend: 10 tests passed.
- Frontend production build, Tauri debug `.app` and unsigned native staging completed. The staged application is `native/build/FlowGuard.app`.
- Browser UI: all three new workspaces navigated successfully; Japanese, English and Simplified Chinese labels reviewed. At 375px the document width is 375px. Collapsing the menu and returning to desktop width retains navigation after a discovered resize bug was fixed. Desktop and narrow screenshots inspected; no application console warnings or errors.
- Browser preview deliberately shows no interface records. Native collection is verified at the Rust boundary; native rendered address rows were not visually tested in the user's currently running app. LAN discovery, port checks, server connections and security diagnostics remain unimplemented, and their screens explicitly state this.
- A concurrent Tauri build invalidated dependency artifacts during the first Rust doc-test invocation; repeating the complete test command after the build passed, including doc-tests.

## LAN discovery / address sheet — 2026-10-12

- Rust: 26 tests passed with `--include-ignored`, including real unsigned ARP/NDP cache collection, one interface-bound ICMP echo to loopback, private-subnet bounds, incomplete neighbor exclusion, scoped IPv6 addresses, duplicate-start rejection, cancellation fencing, late callbacks, stale interface results and the 1,024-row cap during a job. TypeScript: 13 tests passed, including displayed subnet bounds and numeric IPv4 sorting.
- The sandbox initially denied loopback ICMP. The same test passed with host execution permission. Captured bounded stderr now prevents a denied ping from being mistaken for a missing device. No active LAN sweep was performed; no firewall, extension or network setting was changed.
- Browser: production preview keeps native exploration buttons disabled and has no simulated device records. A temporary, clearly labelled synthetic-data fixture tested the actual sheet component: ascending/descending addresses, search, no matches, cached/unverified versus ping response/no response, all three languages, and 375px width with horizontal scrolling inside the table. The fixture was removed before packaging. A fixture-only hot-reload root warning was corrected with teardown; a fresh fixture load and the production app showed no console warnings/errors.
- Frontend production build, macOS Tauri debug application and unsigned native staging passed. Verified base `NSLocalNetworkUsageDescription` metadata and three localized staged strings. The extension preflight remains `packaged=true`, `signed=false`, `signature_required`; LAN discovery does not use the Network Extension.
- Pending: full native WebView start/stop/event integration against real LAN devices, local-network permission prompts/denial, topology changes during a sweep, isolated Wi-Fi, IPv6-only networks and VPN/bridge environments. Device names, latency, gateway identity, CSV and detailed device panels remain planned.

## Optional LAN name lookup — 2026-10-12

- Rust: 32 tests pass with `--include-ignored`; TypeScript: 14 tests pass. New coverage includes IPv4/IPv6 reverse PTR construction, bounded/valid PTR wire data, exact-address system-resolver matching, rejected lookup modes, name search and cancellation that freezes name progress and rejects late names.
- Real unsigned system lookup returns localhost for 127.0.0.1. The macOS DNS-SD FFI executes on the loopback interface within its deadline. No real LAN name queries or active sweeps were performed; positive mDNS wire parsing is tested with controlled records. Full real-LAN name response and privacy permission integration remain Pending.
- DNS-SD is exported by the local SDK through libSystem; the initial standalone dns_sd library link was corrected to the SDK-supported system linkage. No new downloaded runtime dependencies are needed.
- A labelled temporary UI fixture exercised collected names, mDNS versus system sources, collection times and unobserved names. Name search returned the expected single row, all three languages were reviewed, and at 375px the document stayed 375px wide with the table scrolling internally. No console warnings/errors on the final fixture. Temporary files were removed before building.
- Production frontend, Tauri debug app and unsigned native staging build. The local-network usage description now covers optional name lookup in all staged languages. Browser preview still cannot perform native name queries.
- Name querying is disabled by default; selected modes, progress and observations are shared native state. Up to 64 eligible rows, four concurrent queries. System-mode private IPv4 reverse queries may reach configured DNS servers outside the LAN; the UI explains that scope before use. Names are reported evidence, not authenticated identity.

## LAN recollection / details / CSV — 2026-10-12

- Rust: 36 tests passed with `--include-ignored`; TypeScript: 14 passed. New cases cover added/changed/unchanged/not-seen records, timestamp-only updates, scope changes, rejected incomplete baselines, shared selection/panel state, stale or busy CSV requests, quote escaping and formula-cell neutralization. Final CSV field additions also passed focused inventory tests.
- Rust-generated CSV was parsed independently with Python's CSV reader, then downloaded through the actual sheet UI while search showed one row; the downloaded file retained all four comparison rows. UTF-8 BOM/CRLF and fixed headers were verified. The fixture included controlled records only and was removed before the final build.
- Unsigned native app: cache-only collection performed twice with name lookup disabled. The observed two address records compared unchanged despite later observation times. Selecting a row displayed native details; the LAN floating window received the shared selection and docked back correctly. A native CSV download was written to Downloads and independently read back (two records, generation 2). No active LAN ping sweep, name query, rule change or network setting change was performed.
- The native view changed during UI work; an inadvertently opened connection inspector was closed to restore docking, and subsequent visual checks stayed in the isolated fixture. The user's running app was left open. Final refinements preserve previous values for unchanged rows and use LAN-specific panel labels to avoid confusion with the Firewall inspector; these were verified in Rust and browser UI. Restart the running app to load the final staged build.
- UI fixture: all four classifications, changed MAC with previous values, previous-only record details, all three languages and 375px document width checked. Table scroll stays within the sheet, and narrow layouts wrap the detail fields. Final screenshot saved; no fixture console warnings/errors.
- Production frontend, Tauri debug app and unsigned staging built. Real device replacement/disconnection, network switches and permission denial scenarios remain Pending; no such changes were forced on the user's LAN.


## LAN TCP ports (2026-10-12)

Unsigned Rust TCP connect checks use interface and source-address binding, bounded ranges and deadlines. The controlled loopback test checks a listening TCP socket and connection refusal after it closes; it sends no application payload. Real LAN sweeps, local-network permission denial, VPN routing and network changes remain Pending. IPv6 and remote targets are not implemented in this increment. UI shows open records by default and optionally all attempted outcomes; timeouts remain unknown.

Verified: `cargo check --offline`, default Rust suite (32 passed, 7 host-only tests ignored), TCP loopback test (1 passed, interface-bound to lo0), focused latest scope/cancellation and error classification tests, frontend suite (14 passed), production frontend build. Synthetic browser fixture: open-only/all-results toggle, JA/EN/简体中文, 375px width with no page overflow, focus tooltip and clean console. Fixture files/tabs removed. Existing native app was left running; no LAN sweep or extension activation was performed.

Unsigned `.app` build and native packaging completed. Staged bundle: `native/build/FlowGuard.app`; extension preflight: packaged=true, signed=false, signature_required. The running application was not restarted.


## TCP presets and sheet export (2026-10-13)

Rust default suite: 33 passed, 7 host-only tests ignored. Controlled lo0 TCP test: 1 passed; the actual worker checked exactly two requested ports and retained their open/refused outcomes. Frontend: 17 passed; production build passed. CSV parser verified 3 records / 18 columns, requested ports, operation ID, source IP, cancelled stage and stale-collection context. Tests cover preset validation, formula escaping, export rejection during a job and rejection of an old operation within the same LAN generation.

Synthetic browser fixture: preset selection, empty range input, numeric sorting, one-row search with a full 3-record export callback, three locales, 375px page width without horizontal overflow, focus tooltip and clean console. Browser export callback/CSV content verified; a saved Downloads file was not observed. Real LAN sweeps and native OS permission/routing cases remain Pending.

Unsigned Tauri app build completed for this increment. Temporary fixture sources/CSV and verification tabs were removed, and the task-owned Vite server was stopped. The existing native app was not restarted.

Native package refresh completed: packaged=true, signed=false, signature_required. The unsigned staging app includes the latest port presets/search/CSV implementation.


## TCP recollection comparison and details navigation (2026-10-13)

Rust default tests: 34 passed, 7 host-only tests ignored. The ignored TCP loopback test passed with actual listeners swapped between two checks; differences were refused→open and open→refused. Comparison tests cover unknown outcomes, ignored timing changes, interruption without baseline promotion, changed request/generation and invalidation. Shared-state tests verify navigation selects the exact record, increments focus, switches the category and never starts a TCP operation; stale/missing targets are rejected.

CSV parser: 4 records / 24 columns, previous status/time and baseline operation verified. Browser synthetic fixture: details→sheet scroll/focus, changes-only including a newly refused row and a timeout marked uncertain, JA/EN/简体中文, clean console, and 375px document width without overflow. No real LAN scan or native-window focus manipulation was performed. Native floating-window restoration/recreation and full native CSV saving remain Pending.

Unsigned Tauri `.app` build completed. The comparison-only filter was also checked after invalidation: it deactivates and leaves the current rows visible. Temporary browser fixtures/tabs were removed, the task-owned local dev server was stopped, and the existing native app was not restarted.

Native package refreshed successfully: packaged=true, signed=false, signature_required. The staged app contains the latest comparison/navigation implementation.


## LAN IPv6 TCP (2026-10-14)

Rust default suite: 36 passed, 8 host-only tests ignored. Focused scope tests also pass with IPv6-only interface collection, unscoped link-local records, ULA/global compatible sources, wrong zones, forbidden addresses, outside-prefix targets, /63 rejection, down/disappearing sources and changed masks. An unsigned ::1/lo0 listener test exercised the actual IPv6 worker, open→refused comparison and CSV. The existing IPv4 lo0 test also passed. No real LAN probes were sent.

Frontend: 19 tests passed and production build passed. Synthetic browser fixture verified equivalent IPv6 spelling without a false target warning, details→sheet focus, three comparison outcomes, JA/EN/简体中文 and 375px document width without overflow; console warnings/errors were empty. Rust CSV was independently parsed: 1 record, 26 columns, IPv6/source fields and previous-open/current-refused change. Native floating-window restoration and full OS CSV saving remain Pending.

Real LAN link-local/ULA/global routing, privacy address rotation, multiple interfaces, VPN and local-network permission denial remain Pending. The user's running native app was not manipulated or restarted.

Unsigned Tauri debug .app and native packaging completed. Staging preflight: packaged=true, signed=false, signature_required. Temporary fixture sources and browser tab were removed; the task-owned Vite server was stopped and its viewport override reset.


## Open-port service identification / service sheet (2026-10-14)

Rust default suite: 39 passed, 11 host-only tests ignored. New tests cover protocol evidence versus port candidates, generic FTP-style 220 greeting rejection, bounded/untrusted responses, formula-safe CSV, open-only plan selection, invalid mode/old operation/changed source/empty or >64 targets, job exclusion, cancellation with frozen rows, stale callbacks, shared detail selection and network invalidation. Frontend: 21 passed; production build passed.

Three host-only service tests passed without signing, using only IPv4/IPv6 loopback: actual HTTP HEAD plus SSH/SMTP greetings; verified HTTPS on both families with a test trust root; rejection of untrusted certificates and a trusted certificate with the wrong IP identity; actual IPv6 service worker and total timeout. The test key is public fixture data and is never used in production trust configuration. OS root loading and rejection of the test certificate using those roots were also checked; real LAN services were not exercised end-to-end. IPv6 worker CSV was parsed independently: 1 record / 20 columns, captured mode, error field, service, source-port operation and IPv6 family.

CUA in-app browser, temporary fixture outside the repository: synthetic services only. JA/EN/简体中文, product search to one row, shared-selection mock detail, TLS-failure filter, prior-port-operation warning, focus-visible information tooltip, service→TCP sheet→service navigation and 375px layout verified. Page identity/nonblank/no framework overlay and final console health passed (no warning/error). Table scroll remained inside the sheet; narrow details wrapped without page overflow. Screenshot: flowguard-services-qa.png. Initial fixture module/preamble loading was corrected before the final checks. No native window manipulation, actual LAN service probes, network setting changes or extension activation occurred.

Pending: real LAN responses, virtual hosts/SNI, native local-network permission, real-service OS trust-store integration, native CSV saving, certificate expiry/issuer display and service history comparison. Product/version values are self-reported, and no internal service inventory or health assessment is implied.

The refactored shared socket connection also passed the existing IPv4 and IPv6 TCP loopback regression tests. Final temporary browser fixtures were removed from /private/tmp, the task-owned Vite server was stopped, and its viewport override was reset.

Final unsigned Tauri debug .app and native package refresh completed. Preflight: packaged=true, signed=false, signature_required. Local-network usage descriptions include service-response checks in English, Japanese and Simplified Chinese; staged metadata was checked. The running application was left open and was not restarted.


## Software updater and release preparation (2026-10-14)

Rust default suite: 44 passed, 13 host-only tests ignored. Two separately executed updater host tests passed with final reqwest 0.13.5: real plugin manifest/download/signature verification and disposable bundle replacement; tampered payload rejection; backup restoration after failed installation or failed verification. The loopback listener requires execution outside the filesystem/network sandbox. No installed user application was replaced. Frontend: 23 tests passed; production build and Rust formatting checks passed. Release script Python compilation, stable-version/two-architecture manifest checks and CI YAML parsing passed.

Synthetic browser fixture checked download progress/cancellation, ready/installing states, development and active-job installation gates, signature failure, release notes, automatic-update preference and JA/EN/简体中文. Narrow 375px layout had no page overflow; information tooltip worked with keyboard focus. Real preview Settings displayed the update panel with native-only actions disabled. Console warnings/errors were empty. Screenshot: flowguard-updates-qa.png (synthetic). Temporary fixture sources/tabs were removed, viewport override reset and task-owned Vite server stopped.

Final unsigned Tauri debug app and native packaging completed: packaged=true, signed=false, signature_required. The running native app was left open. Development builds disable automatic update traffic and installation. Production defaults to scheduled checks/downloads; installation/restart requires the update button. Signature/version/archive limits and application backup are implemented; signed real-app Gatekeeper, restart, data retention and active Extension update behavior remain Pending.

Developer ID Application identities and the flowguard notarization Keychain profile were verified without printing credentials. Matching FlowGuard Developer ID provisioning profiles remain unavailable. User selected initial publication with both host and Network Extension: no host-only release, GitHub publication, notarization submission or production install was performed. Full distribution awaits profiles for com.flowguard.desktop and com.flowguard.filter. The updater private key stays in the ignored local .release-secrets directory; it is not a release asset.


## Supplied Developer ID profiles (2026-10-14)

Decoded the user-supplied host and extension profiles with security cms. Both have exact App IDs (com.flowguard.desktop / com.flowguard.filter), Team 3WH28SSRZC, ProvisionsAllDevices=true, expiry 2044-10-09 and the installed Developer ID certificate SHA-1 3D0BB7BD18E71AD3DA59704A68B4E9349FC06FB6. Both lack com.apple.developer.networking.networkextension, and the host also lacks com.apple.developer.system-extension.install. The current entitlement requirements are therefore unsatisfied. Full-bundle signing/notarization/publication remain Pending regenerated capability-enabled profiles. No profile was modified, no signed release was produced and no GitHub release was published.


## Full-bundle Developer ID release (2026-10-14)

Regenerated profiles in ignored docs-private match the selected Developer ID certificate and both App IDs. Required Content Filter/System Extension capabilities are present. Release v0.1.0 aarch64 includes the host, Swift bridge/runtime and System Extension. Signing binds App ID and Team ID explicitly. Fixed SecCodeCopySigningInformation to request kSecCSSigningInformation: the signed bridge preflight now reports packaged=true, signed=true, requires_activation and the exact team-qualified Mach service. No activation or installation was performed.

Final notarization: Accepted, submission baa9acc4-7782-483e-b6bf-340fcbc34df4. Staple/validate and Gatekeeper assessment passed (Notarized Developer ID). Private signing keys, profiles and notarization archives stay Git-ignored; GitHub assets are installer ZIP, updater archive/signature, latest.json and SHA256SUMS only. Frontend 23 tests passed. Signed-app update restart and activated Extension behavior remain Pending.

Production updater validation accepted the final archive using the embedded production public key and exact version/archive policy. Rust default suite: 44 passed, 14 ignored; ephemeral-key TLS host test passed. The initial public-push approval review rejected a tracked test TLS private key. Removed all TLS credential fixtures from the staged publication and replaced them with short-lived OpenSSL credentials generated in a temporary test directory; verification passed. No private TLS/updater/Apple key was published. GitHub Release v0.1.0 is public at https://github.com/matsushibadenki/FlowGuard/releases/tag/v0.1.0 with five distribution assets.


## Startup policy persistence on normal apply (2026-10-14)

Normal apply now calls Swift host ABI v3 to persist the validated real policy in our enabled NEFilterManager configuration, reloads preferences and confirms exact policy bytes and ownership before live XPC synchronization. No unrelated vendor fields are discarded. Rust waits on an operation-token-specific callback with a 30s bound; late callbacks cannot satisfy another request. Persistence failures never reach live synchronization or applied acknowledgement. A saved policy can still become the next startup policy after live synchronization fails; automatic reconciliation remains Next.

Rust default suite: 44 passed, 14 ignored. Actual unsigned Swift FFI gate passed for activation, disable/removal and persistence, with no OS mutation. Swift bridge suite: 10 passed including old-policy, disabled, foreign-provider, missing and invalid-data persistence confirmation rejection. Production frontend and unsigned Tauri app built. Signed OS preference saving/provider restart and uncertain-timeout recovery remain Pending. Published v0.1.0 assets were not changed; its GitHub CI completed successfully.


## Extension restart and uncertain-ack recovery (2026-10-14)

Rust tests cover restart detection, explicit-intent requirement, unsaved edits blocking recovery, retry budget/backoff, late acknowledgements after removal and atomic recovery admission during updater installation. Empty/nonempty SHA-256 golden values agree with Swift, and same-count policy edits change the digest. Swift receiver tests seed actual startup state, observe a changed policy after a discarded acknowledgement and omit the digest while unavailable. Swift policy tests (3) and bridge/anonymous-XPC tests (11) passed. Rust default suite: 49 passed, 14 ignored. Unsigned staging retains ABI v3 signature gates.

No real Network Extension restart, activation, OS preference mutation, LAN probes or running-app replacement was performed. Signed actual-provider crash, real XPC acknowledgement loss, OS-save timeout and automatic restart recovery remain Pending. Recovery intent is intentionally session-only; no implicit application of newly edited rules or recovery after host-process restart is claimed. Published release assets are unchanged.

Final current-code unsigned Tauri debug build and native staging completed: packaged=true, signed=false, signature_required. Final Rust 49/14 result and formatting/diff checks passed. The actual unsigned C ABI v3 signature-gate test passed (1). Running applications were not restarted.

## AI Security Phase 1 (2026-10-14)

- `npm run build`: passed.
- `npm test`: 23 passed across 7 files.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib`: 51 passed, 14 explicitly ignored, 0 failed. Includes normalization tests for deduplication, unavailable byte counters/application attribution, unknown direction/capabilities, demo source separation and empty observation.
- This phase adds shared data only; router collection, AI inference and defense execution remain unimplemented. No router/LLM/real blocking validation is claimed.

## Protection status clarity (2026-10-14)

- Build passed; Vitest 24 tests passed. Status tests reject confirmed display during lost/busy/stale/demo states and missing provider identity.
- Browser plugin skill unavailable (`Browser plugin not available`); bundled Playwright with installed Chrome tested http://127.0.0.1:1420 at 1440×960 and 390×960. Page identity/content, no framework overlay, screenshots and no horizontal page overflow verified.
- Japanese demo status → English locale → Save intention (not applied) → Rule saved · not applied verified. No JavaScript runtime errors; existing favicon.ico returns 404.
- Signed-provider lifecycle rendering and real blocking/recovery remain untested. Screenshots/scripts kept in /tmp, outside repository.

## Communication map redesign (2026-10-14)

Scope: frontend map presentation only. No change to src-tauri/, native/, Snapshot, Command, ruleFor or external dependencies. New source files have path comments and remain below 500 lines.

Implementation decisions: one non-sticky map status bar before the page heading; readable wrapping takes precedence over a 96px height cap. The four states are independent confirmations, not continuous percentage progress. Signed provider identity and revision are required before displaying a full receipt. Map-specific wording uses separate contextual keys so other screens retain their existing translations. Inspector changes are enabled only for the map and its floating detail.

### Changed file inventory

| File | Change location / purpose |
| --- | --- |
| src/core/protection.ts + protection.test.ts | New status derivation and lifecycle/observation tests |
| src/core/destinations.ts + destinations.test.ts | New IPv4/IPv6 range parser and boundary/malformed-address tests |
| src/core/mapModel.ts + mapModel.test.ts | New app/socket projection, merged destinations, sorting, receipt/identity/direction tests |
| src/components/IntentBar.tsx | New accessible intent-count bar |
| src/components/AppList.tsx | New full app list, roving keyboard focus and hidden-app recovery |
| src/components/DestinationPane.tsx | New range groups, collapse, 50-row increments and existing-selection highlighting |
| src/components/ProtectionStatus.tsx | New independent four-state track, source switch and counts |
| src/components/FlowMap.tsx | Replaced six-socket diagram with app/destination panes and local display controls |
| src/styles/map.css | New scoped responsive layout, intent styles and accessible-label containment |
| src/main.tsx | Import map.css after existing styles |
| src/App.tsx | Map-only status replacement, omit its old overview, pass filter reset and map inspector flag |
| src/components/Inspector.tsx | Opt-in map styling and map-only conditional save wording; other screens retain behavior |
| src/core/i18n.ts | Add map translations for English/Japanese/Simplified Chinese; existing translations preserved |
| src/styles.css | Remove unused old diagram selectors after reference check; preserve shared rules |
| docs/ROADMAP.md | Mark browser-verified map work Done and native/real-data confirmation Pending |

### Automated and rendered checks

- npm ci: succeeded. npm reports existing allowScripts coverage warnings for esbuild/fsevents; build still succeeds. No dependency changes introduced.
- npm run build: passed (TypeScript strict + Vite).
- npm test: 67 passed across 10 files, including 43 new map-model/status/address cases.
- git diff --check: passed. git diff --name-only -- src-tauri native: empty.
- Browser plugin skill unavailable (Browser plugin not available); bundled Playwright with installed headless Chrome used against http://127.0.0.1:1420/. No new browser dependencies installed.
- Tested 320/375/414/768/1200/1550px in all three languages, 18 combinations. Page identity/content correct, no framework overlay or JavaScript errors, no page horizontal overflow. Existing favicon.ico 404 excluded from app runtime errors.
- Temporary 172-connection/16-app fixture injected only into the browser response at sampleConnections() call site. Includes loopback-only app, IPv6, unknown direction, long app name, multiple PIDs, duplicate destinations and two distinct unidentified processes. Shows all 15 visible apps and recovers the hidden app; unit test also verifies 172 distinct apps without a six-app cap. No fixture/source-store change shipped.
- Interaction loop: app ArrowDown + Enter → destination Enter → block radio Space → save Enter; inspector follows selected app/connection and reports saved/not applied. Loopback toggle, 50-row expansion, left/right docking, no-match search, reset and block-intent filter passed. Existing Applications view still displays its original overview.
- Muted/allow/block/ask text on white surface measured at approximately 6.53/6.38/7.11/6.34:1, exceeding 4.5:1. Intent colors are not protection assertions.
- A 768px overflow from an existing recent-history screen-reader label was fixed only inside the map's scroll container. Narrow headline clauses and parenthetical step labels wrap at meaningful boundaries.

Screenshots are temporary evidence outside the repository: /tmp/flowguard-map-{ja,en,zh-CN}-{320,375,414,768,1200,1550}.png and /tmp/flowguard-map-interaction.png. QA script: /tmp/flowguard-map-qa.cjs.

Pending: Apple Silicon native display; real 172-connection samples; native multi-window selectedId propagation/floating detail; signed-provider state changes; real blocking and recovery. Browser fixtures and receipt tests do not prove OS enforcement.

Screenshot capture note: installed Chrome repeated the top strip in the bottom 87px of uncropped headless captures. Final evidence captures the upper 880px of 1000px viewports to exclude this capture artifact. DOM/page-width checks and interactions cover the full viewport; no repeated app root was introduced.

## Signed rule identity and legacy migration (2026-10-14)

- New `src-tauri/src/identity.rs` uses CoreFoundation/Security.framework to acquire and validate running guests. No codesign text parsing, Info.plist trust, filesystem-path authorization, private credentials or new dependencies.
- Connection identity is optional; old rule JSON without identity remains readable and disabled. Live rule subject keys are canonical and must agree with their identity. Source/destination/port/direction semantics remain intact; demo rules retain their separate path-based intent scope.
- Creation and explicit MigrateRule reacquire the running guest inside the existing state mutation transaction. Migration preserves original ID, action, destination and direction; occupied signed scopes, changed/unknown signatures and absent processes are rejected. Existing expected-rules conflicts and persistence rollback protect migration and undo. No current signature is treated as evidence of the historical signer at an old path.
- Provider independently validates the flow audit token. No path fallback. Old path policies are excluded from payload/digest/recovery/receipt IDs; provider validation rejects them even if injected directly. v2 protocol and identity-bearing digest vectors match in Rust and Swift. Saving v2 OS configuration removes the obsolete v1 policy slot and verifies its absence after reload.
- Frontend adds signing labels, disabled legacy-state warning and explicit per-rule signer review. Unverified live save and browser-only migration are disabled. v2 backups are exported; both v1/v2 are validated on import. Invalid identities/keys/false demo signature claims are rejected.

Verification:

- npm run build: passed; npm test: 71 passed, 11 files.
- cargo test --manifest-path src-tauri/Cargo.toml --lib: 56 passed, 16 explicitly ignored, 0 failed. The 16 include 14 earlier host/environment tests plus 2 new host Security guest tests.
- cargo test --manifest-path src-tauri/Cargo.toml --lib signed_guest -- --ignored --nocapture --test-threads=1: both host tests passed with host guest access. Uses only a newly spawned /bin/sleep child and temporary rule documents, with no firewall/extension configuration changes. Verifies real Apple signing, explicit migration, signature recheck, competing scope rejection, original intent preservation, failed-write rollback/undo and refusal after guest termination.
- swift test (workspace package, module/cache paths under /tmp, --disable-sandbox to avoid unsupported nested sandboxing): 18 passed (5 Policy, 12 Bridge, 1 transport). Builds host runtime/helper/extension targets. Includes legacy policy/protocol rejection, removed v1 vendor slot, different signer/identifier, moved executable and missing audit token tests.
- A null-CF-reference release was found by the terminated-guest test, fixed, and covered by a regression test plus the passing real guest-termination test. Failure never falls back to a path identity.
- Playwright + installed headless Chrome at http://127.0.0.1:1420/; Browser plugin skill unavailable. Three locales at 375/1200px with a temporary browser response fixture. Verified legacy rules do not create matched block intentions, unknown signature disables live save, signer review displays the exact Team ID/identifier, migration stays disabled outside native mode, page identity/content, no overlay, no page overflow or JS errors. Existing favicon.ico 404 excluded. No mock signature data shipped.
- Evidence: /tmp/flowguard-signing-{ja,en,zh-CN}-{375,1200}.png; script /tmp/flowguard-signing-qa.cjs. Captures crop the known Chrome bottom-strip screenshot artifact described above.
- git diff --check: passed. Earlier uncommitted map redesign work retained.

Pending: replacing a signed installed host/provider together, real Provider flow verdicts/audit tokens, Developer ID/App Store update and on-disk tamper scenarios, old installed provider/cached-policy transitions. Do not downgrade the new saved document into an old path-only build. OS configuration persistence/removal behavior still requires signed testing; source edits do not disable a running old provider.

## Ask decisions (2026-10-14)

- `npm run build` and `npm test`: build succeeded, 72 tests passed. Backup validation accepts absent legacy timeout and allow/block, rejects invalid fallbacks.
- `cargo test --manifest-path src-tauri/Cargo.toml --lib`: 61 passed, 16 host-only tests ignored. Ask tests cover bounded queue envelopes, restart IDs, shared-window claim, stale poll rejection, mismatched replies, expired response versus user acknowledgement, default preference persistence, updating saved intentions and conflict control.
- Swift SystemExtension product build succeeded. Custom-cache `swift test --disable-sandbox`: 24 passed (5 policy, 12 bridge, 5 queue, 2 transport). Both timeout actions, exact deadline, duplicate/late answers, unsigned/different scopes, inactive intake, capacity, policy replacement and graceful stop are covered. v3 saves remove both obsolete v1/v2 slots. Anonymous XPC exercises production protocol metadata retrieval and one-flow answer callback exactly once; it does not exercise the production signing-gated Mach listener.
- Temporary browser fixture verified default allow, choosing block, persisted selection after reload, signer/IPv6/scope, captured request fallback and disabled browser answer buttons. JA/EN/简体中文 at 375/1200px showed no horizontal overflow or JavaScript errors. Screenshots of inbox and settings were visually inspected. Fixtures remain outside the repository; no synthetic provider data is shipped.
- ⭕️ [Pending] Signed installation/approval/update and actual NEFilterFlow pause/resume/drop, both timeout network outcomes, multiple native windows, UI termination, capacity saturation, policy update/stop, provider crash, sleep/wake, reboot and VPN/proxy behavior. Provider receipt/history does not prove actual traffic verdicts. No OS filter configuration was changed for this increment.

## Global emergency recovery (2026-10-15)

- Frontend build succeeds. TypeScript 75 tests pass, including recovery admission for signed/native/runtime/busy/install state, and refusal to infer confirmation from failures, disconnection or unsigned/busy acknowledgements.
- Rust library suite: 62 passed, 16 existing host-dependent tests ignored. Added regression checks that a failed disable does not silently restore the automatic recovery target and permits an explicit retry. Existing lifecycle tests cover clearing receipts, late callbacks and retry fencing.
- Browser QA uses bundled Playwright/Chrome at localhost:1420; no dedicated project Browser runtime workflow was configured. JA/EN/简体中文 at 375/1200px: global control and three evidence rows, disabled preview action, navigation through map/rules/settings, no page overflow or JavaScript errors. A temporary synthetic signed-disabled receipt confirms only the OS setting row; Provider/traffic remain unconfirmed. No OS filter state was mutated.
- ⭕️ [Pending] Signed desktop invocation, real NEFilterManager save/reload, Provider shutdown, ask draining and actual network restoration, native multiple windows, failures/retry, VPN, crash and sleep lifecycle. The browser fixture is presentation evidence only.

## System protection presets (2026-10-15)

- Local read-only codesign designated requirements confirm identifiers com.apple.mDNSResponder and com.apple.softwareupdated with `anchor apple`. This is source evidence for the preset allowlist, not a real NEFilterFlow test. The existing provider separately verifies each actual flow audit token.
- TypeScript: 77 tests pass; Rust: 64 passed, 16 existing host-dependent tests ignored. Coverage includes default protection, wrong signing authority/port/direction, demo isolation, exception schema, refusal to label protected blocking intentions as applied, explicit expected-rules review, persistence/rollback, stale edits, import reset and digest changes for the flag.
- Swift: 27 tests pass (8 policy, 19 bridge/queue/transport). Policy tests cover block/ask default bypass, exact exception, destination mismatch, specific-direction rule winning over a broader exception, inbound exclusion, missing identity and different publisher. v4 rejects protocols 1/2/3, removes all old initial slots and matches Rust digest vectors. Extension/helper products compiled during Swift package tests.
- Frontend build succeeds. Bundled Playwright/Chrome against the task-owned localhost:1421 server checked the Inspector warning before saving, per-rule scope/consequence review, disabled browser mutation and Settings allowlist/limitations. JA/EN/简体中文 at 375/1200px had no page overflow or JavaScript errors. Screenshots inspected; synthetic metadata stayed in temporary browser routes/storage. Existing port1420 belongs to another app and was not stopped or changed.
- ⭕️ [Pending] Signed host/helper/Provider v4 deployment, live Apple-daemon audit tokens, real DNS/update TCP allow/block/ask and exception reset/application, update from an installed old provider, startup after reboot, VPN/proxy/custom resolver behavior. No filter activation, OS preference changes or real DNS/update requests were performed. UDP/mDNS and general administration protection remain unimplemented, not tested or claimed.

## Emergency priority during extension operations (2026-10-15)

Final checks: TypeScript 77 passed; Rust 65 passed with 16 existing host-only ignored; Swift 32 passed (8 policy, 22 bridge/queue/barrier/transport, 2 host coordinator). Frontend and Swift native products build successfully.

- Rust library tests: emergency preempts activating/applying/removing generations, clears receipt/recovery intent, rejects duplicate disable and all old configuration/accept/failure callbacks. Existing initial-sync busy test now checks that another apply is blocked while emergency disable is covered separately.
- Swift tests exercise the production ExtensionController against injected preference I/O: recovery waits for an outstanding enable save and then writes false; no overlapping operations; stale SystemExtension callbacks do nothing; approval wait yields without an enable save; persist-load interruption never submits its save; explicit later activation clears the latch. RecoveryBarrier tests cover unreleased I/O at caller timeout, delayed generation submission rejection and latch admission. No signature bypass was added to the public API or real OS path.
- Native ABI4 and Rust signatures compile together; old ABI3 runtime is rejected. Policy/XPC protocol remains v4. No signed native FFI or live NEFilterManager operation was invoked.
- Browser pending-state fixture checks duplicate prevention, wait explanation, separate OS/Provider/traffic evidence and JA/EN/简体中文 at 375/1200px. Desktop admission during busy operations is unit-tested; browser remains unable to invoke native disable.
- ⭕️ [Pending] Real signed approval/save/load/remove overlap, OS callback delays/failures/hangs, process termination/restart with outstanding OS work, actual Provider stop/ask resolution/traffic restoration, VPN and sleep/reboot. A hung callback does not release the serialization barrier or imply recovery success.

## Menu bar residency (2026-10-15)

Offline cargo check and Rust library tests pass with Tauri tray-icon enabled using existing locked dependencies. New state tests reject unsigned/incomplete receipt claims, distinguish busy/disconnected states, allow emergency during apply and deny duplicate disable/update installation. Native menu and main-window hide/reopen integration compiled but were not launched against the user's desktop.

⭕️ [Pending] Actual macOS tray/template icon, native menu localization/interaction, closing/reopening/minimized windows, floating panels, signed Provider queue updates and recovery, explicit quit and post-exit network behavior. No OS filter changes, native menu clicks, notifications or login-item registration were performed.

## Ask notification opt-in (2026-10-15)

Rust: 71 passed, 16 existing host-only ignored; TypeScript: 77 passed; Swift: 32 passed and host native products build with UserNotifications. New Rust tests cover dedupe, batching/cooldown/hourly ceiling, opt-out/no unmute flood, no notifications for unsigned/busy/gone requests, default-off persistence and rollback. Browser preview checks default-off toggle, persistent mute, native permission buttons disabled, details and three languages at 375/1200px with no overflow/errors. Optional native capability absence returns unavailable.

⭕️ [Pending] Actual macOS notification permission requests/delivery, denied permission, Focus, opt-out versus delayed OS completion, native hidden-window return and real Provider ask events. No real notification or permission prompt was sent during verification. Swift tests compile the integration but do not prove notification delivery.

## Unregistered TCP observation alerts (2026-10-15)

Final counts: Rust 77 passed with 16 existing host-only ignored; TypeScript 77 passed; Swift 32 passed. Frontend build succeeded.

Rust tests cover startup baseline, two distinct samples, repeated snapshot, error/mute recovery, unsigned exclusion, existing-rule handling with unknown direction, rule removal, loopback/protected/short-lived flows, 1,024-key cap, interface-change baseline and shared ask/observation budget. TypeScript tests/build and Swift package tests continue to pass. Temporary browser checks cover default-off opt-in, persistent mute, disabled native permission action and JA/EN/简体中文 at 375/1200px without page overflow/errors. No OS permission or notification was requested.

⭕️ [Pending] Real macOS notification delivery and long-running observation noise, short-lived connection coverage, signed host behavior, VPN/proxy/same-address network changes, Focus and mute races. These are limited signed-process TCP observations, not a complete traffic feed or security verdict.

## Single-instance activation (2026-10-15)

- Default Rust library suite: 78 passed, 18 ignored (16 existing host tests plus 2 explicit socket tests). Explicit `instance::local::tests::host_ -- --ignored --test-threads=1`: 2 passed. A real child Python process exercised the fixed wire protocol. Tests cover same-user socket roundtrip, malformed commands, 0600 mode, stale sockets, long-path fallback, normal/symlink file refusal and independent document lock exclusion.
- A separate temporary unsigned .app used identifier com.fg.qa7329 and localhost:1421, leaving normal FlowGuard data untouched. Native CUA inspected its real window, closed it, then a second executable invocation exited 0 with the QA document hash unchanged. CUA then confirmed the existing FlowGuard QA workspace returned. Filter operations remained disabled; no notification permission, filter activation or network scan was requested. Only the existing read-only local observer ran. The QA app/data were removed and the dev server stopped; default build configuration restored.
- ⭕️ [Pending] Signed installed app/old-version coexistence, actual Dock/LaunchServices reopen (the native automation launch helper was unavailable), full-screen/Spaces/minimized behavior and OS focus restrictions. Scheduling acknowledgement is not proof that macOS granted foreground focus. The preexisting document lock remains the safety boundary if focus IPC cannot be reached.


## LAN inventory workbench (2026-10-16)

- Rust library suite: 80 passed, 18 existing host-only ignored. New tests cover exact-row selection/deduplication, stale generation and compare-and-swap rejection, comment limits/control characters, invalid/duplicate/mandatory columns, collection reset, selection-only CSV including escaped/formula-neutralized multiline comments and provenance. The shared-state integration test also exercises selection/comment/copy and rejects old-generation edits/copies after collection restarts.
- TypeScript: 79 passed; production build passed. Column contract tests check mandatory columns, uniqueness, allowlist and labels in all three languages.
- Browser plugin not available (no Browser plugin skill/runtime in this session); used bundled Playwright with local Chrome on task-owned localhost:1421. Temporary synthetic LAN fixtures exercise individual selection, adding filtered rows without clearing hidden selections, separate records with matching names/MACs, comment save, concurrent edit rejection with unsaved draft retained and explicit reload, column hiding/reset, disabled preview clipboard action, and JA/EN/简体中文 at 1200/375px with no root horizontal overflow, runtime errors or Vite overlay. Screenshots remain outside the repository. No real LAN discovery or clipboard write was performed.
- ⭕️ [Pending] Actual macOS clipboard delivery/paste and OS failure, concurrent native windows and real-LAN recollection. Rust tests verify exported bytes and shared-state safety; browser fixtures verify interaction only. Clipboard success means the bounded OS command returned success, not proof of a paste destination. Columns are session-only; comments are collection-scoped user annotations, not device identity evidence.


## HTTPS hostname / SNI and certificate evidence (2026-10-16)

- Rust library suite: 83 passed, 18 existing host-only ignored. New tests check strict DNS/A-label input, URL/IP/port/wildcard/CRLF rejection, UTC/generalized certificate dates, impossible dates and bounded malformed DER, structured validation reasons and plan/snapshot identity capture. Existing shared job cancellation/generation tests remain passing.
- Explicit host test `services::tests::verified_https_and_untrusted_certificate -- --ignored --test-threads=1`: passed. Temporary OpenSSL fixtures and IPv4/IPv6 loopback rustls servers exercise 28 combinations of IP/DNS-only/wrong SANs, trusted/untrusted roots and present/future/past verification times. The server checks actual SNI and HTTP Host for named successful requests. Name mismatch, unknown issuer, expired and not-yet-valid certificates are rejected; successful leaf issuer/validity and CSV metadata are checked. Test-only time providers and trust roots do not change the OS clock or trust store. Test files are automatically removed.
- TypeScript: 81 passed; production build passed. Browser plugin not available; bundled Playwright/Chrome used with task-owned localhost:1421 and temporary synthetic service results. Checked invalid-name feedback, valid input, method switching, successful metadata, rejected certificate/missing evidence, JA/EN/简体中文 at 1200/375px, no root overflow, JS errors or Vite overlay. Screenshots remain outside the repository. Browser checks did not contact LAN or external targets and do not prove native integration.
- ⭕️ [Pending] Real LAN virtual-host devices and certificate chains, native macOS UI-to-command integration, enterprise/OS-specific trust configuration, VPN/proxy and real clock-error behavior. TLS uses OS-loaded roots with rustls/webpki, not every macOS trust policy or online revocation service. Certificate metadata has display-only authority; missing evidence is not success.


## System protection transport boundaries and design (2026-10-16)

- Rust: 84 passed, 18 existing host-only ignored; TypeScript: 83 passed; frontend production build passed. Tests include explicit TCP/UDP/unknown preset classification, non-TCP rule matching/save rejection without state mutation, and unchanged TCP exception behavior. Migration now rejects unsupported transport before attempting fresh signer identification.
- Swift: 34 passed (9 policy, 23 bridge, 2 host), with all package products compiled using a temporary scratch/module cache and `--disable-sandbox`. New regression tests cover UDP/unknown with DNS/mDNS/management-like ports, allow/block/ask and explicit TCP exceptions. Neither policy nor preset may match, and ask rejects without a request, outcome or fallback callback. Existing TCP decisions, policy digest, initial-policy and queue tests pass.
- Browser plugin not available; bundled Playwright/Chrome with task-owned localhost:1421 and temporary synthetic signed-subject TCP/UDP/unknown observations. Confirmed TCP retains its warning, unsupported transport has no preset warning and cannot save a rule, and the unsupported message renders in JA/EN/简体中文 at 1200/375px without root overflow, Vite overlay or JS errors. No real UDP, network scan, OS filter change or installation was performed. Screenshots are outside the repository.
- The linked specification distinguishes shipped guards from future UDP handling and temporary management grants. ⭕️ [Pending] Signed Provider delivery/metadata, multicast/unicast replies, multi-destination UDP, interface/scope and proxy ownership, and real management-path expiry/recovery. Unit tests and successful builds do not prove UDP enforcement. The Provider still registers only TCP and passes unsupported traffic without claiming protection.


## Router adapter foundation (2026-10-16)

- Rust library: 97 passed, 18 existing host-only ignored. New tests exercise strict schema/source/peer/pin checks, payload self-attestation rejection, bounds and atomic batch rejection, null versus zero, identical/conflicting duplicates, replay and collection reset, freshness, IPv6 scope preservation, anonymous/WAN/unknown correlation exclusions and separate candidate records/counters.
- Production coordinator/shared-state tests use synthetic adapters: total deadline, pre-cancelled read, noncooperative adapter return bound, global two-worker saturation, stop while state remains accessible, late-result/revision fencing, live snapshot integration, no document persistence, demo isolation and installation gate. Exact-action/source/peer and expiring control authorization checks pass; no authorization issuer/control dispatcher exists in production.
- TypeScript: 84 passed; production build passed; git diff whitespace check passed. Added update eligibility coverage for active router collection and typed source/error/correlation metadata. No router management UI or new rendered view was added, so browser mock-up is not claimed as device validation. Existing shared UI tests remain passing. Swift code is unchanged in this increment.
- ⭕️ [Pending] Real managed router authentication and transport peer verification, genuine LAN/WAN flow/counter semantics, firmware/reboot/sleep behavior and native UI integration. No real router, network scan, credential lookup, privileged action or settings change was performed. Native adapter code must bound network reads and attest only cryptographically authenticated peers; payload validation itself does not perform a TLS handshake. Up to two faulty in-process workers can remain occupied until exit, but cannot create unlimited workers or publish late data.


## Syslog receiver (2026-10-17)

- Rust library: 103 passed, 20 ignored (18 existing host tests plus 2 new explicit loopback tests). New tests cover acknowledged/configured private IPv4 and current interfaces, no wildcard/public/privileged listener, PRI/UTF-8/control and bidi handling, inert command/HTML text, fragmented CRLF including exact length limit, rate/retention/byte/frame limits, source rejection, shared selection and frozen evidence after stop, clear/restart operation fencing, no document persistence and installation gating.
- Explicit `syslog::tests::actual_ -- --ignored --test-threads=1`: 2 passed on host. Real ephemeral IPv4 loopback UDP/TCP sockets exercise reception, OS peer matching and rejection, oversized UDP, invalid payload, fragmented TCP/CRLF and incomplete EOF, exclusive live bind, duration/stop and port release/rebind. No LAN packets, router connection, OS filter change or root port was used. Production configuration still requires ports >=1024 and duration >=30 seconds; host fixtures use internal ephemeral/short-lived plans only.
- TypeScript: 88 passed; production build passed; git diff whitespace check passed. Browser plugin not available, so bundled Playwright with Chrome and task-owned localhost:1421 used temporary synthetic snapshots. Checked source/port/transport/acknowledgement form and preview start disabled, filtering, 50-row paging, active shared config, literal `<script>` text without executing it, and JA/EN/简体中文 at 1200/375px without root overflow, JS errors or Vite overlay. Screenshots are outside the repository. Narrow-view card padding was corrected and rechecked. No mock data ships or opens a receiving socket in preview.
- ⭕️ [Pending] Native macOS UI-to-command integration and local-network permission/refusal, actual managed-LAN senders, OS inbound firewall behavior, interface changes, sleep/VPN and real-world loss/floods/format variants. Network/OS loss cannot be counted, sender IP is not authentication, and severity is reported data. These checks prove the bounded receiver subset, not complete RFC parsing, security diagnosis, authenticated router identity or enforcement. Swift code is unchanged in this increment.

## 2026-10-17 標準ファイアウォール用ヘルパー

- `cargo test --manifest-path src-tauri/Cargo.toml --lib --offline`: 106 passed、20 ignored。固定引数・確認要求・未知の状態／期待値不一致・busy・遅延応答拒否・文書非変更・更新インストール排他を含む。
- `npm test`: 91 passed、18 files。既知状態・同梱側承認・確認チェック・native実行・busy／errorによる変更可否と更新インストール排他、3言語キーの一致を確認。`npm run build`成功。
- Swift package: 39 passed（既存34＋Firewall 5）。固定コマンド、厳格な読み取り値、認証／接続状態／期待値の拒否、変更後の再読み取りと確認失敗を検証。テストはroot操作やOS認証を行わない。
- 未署名ステージング生成が成功。LaunchDaemonのBundleProgram・固定引数・MachServicesを確認。Team IDのない同梱ヘルパーのstatus／register／enable／daemonがすべて`signature_required`でOS API前に拒否されることを実行確認。
- Browser plugin not availableのためbundled Playwright＋Chromeを使用。`http://127.0.0.1:1421`でJA／EN／简体中文、1200／375px、状態未確認・操作中・承認待ち・エラー、プレビューの全OS操作無効化を確認。ページ識別・コンソール／Vite overlay・横はみ出しの問題なし。スクリーンショット`/tmp/flowguard-firewall-{ja,en,zh-CN}-{1200,375}.png`、日本語375pxを視認。
- `/tmp/flowguard-firewall-interaction.cjs`ではテスト時だけnative表示とコマンドをモック。確認前の変更拒否、確認後の期待状態付きenable、確認の自動解除、共有busyによる全ボタン無効化、未登録時のregisterを確認。実OSコマンドは呼び出さない。
- ⭕️ [Pending] Developer ID署名・公証済み新ヘルパーの導入、SMAppService登録／承認／解除、実管理者認証／キャンセル、異署名拒否とroot XPC、socketfilterfwの実設定変更／再読取、OS・アプリの再起動／更新時。詳細は`macos-firewall-helper.md`。実OSのファイアウォール設定は変更していない。

## 2026-10-17 wgpuネイティブパネル接続

- `cargo test --manifest-path src-tauri/Cargo.toml --lib --offline`: GPU有効の標準構成で111 passed、20 ignored。元の106件に加え、矩形／viewport／クリップ包含、開く前・終了済み・別世代・逆順要求の拒否、有限なメッシュ、最大10,000件の決定的配置と上限外の選択接続の保持を確認。
- `cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --lib --offline`: 108 passed、20 ignored。GPUを含めない構成も確認。
- `npm test`: 95 passed、19 files。部分スクロールでも元の描画矩形を維持し、viewport・スクロールコンテナ・保護状態バーでクリップする計算、不正／ゼロ／領域外の非表示、3言語キーの一致を確認。`npm run build`成功。
- `cargo build --manifest-path src-tauri/Cargo.toml --bin flowguard --offline`: 標準のGPU有効アプリをビルド。既存の未接続ルーターAPIのdead_codeと依存`block`のfuture-incompatibility警告は残るが、この変更でエラーは発生していない。
- `cargo build --manifest-path src-tauri/Cargo.toml --example native_panel_check --features gpu-qa --offline`後に専用確認アプリを実行。一時文書＋about:blankの独立Tauri WebViewで、実Metal Surfaceに10,000／10,001件をpresentし、device.pollでGPU完了を待機。部分クリップ・リサイズ停止／再描画・非表示・終了後の遅延拒否・旧leaseのcloseに影響されない再作成・解放／Destroyed相当の拒否が成功。native hitTestの通過も検査。実ユーザー文書・観測・ネットワーク・OS設定・更新処理は使わない。環境のbacking scaleは1。
- Browser plugin not availableのためbundled Playwright＋Chromeを使用。`http://127.0.0.1:1421`で、プレビューの操作無効化と、IPCだけをモックした開く→矩形同期→画面切替で閉じる→描画失敗時の通常表示への復帰を検証。JA／EN／简体中文・1200／375pxの表示、スクロール／リサイズのクリップ、経路やフレームを含まない要求を確認。ページ識別・console・Vite overlay・横はみ出しの問題なし。`/tmp/flowguard-gpu-ui-qa.cjs`、スクリーンショット`/tmp/flowguard-gpu-{ja,en,zh-CN}-{1200,375}.png`。日本語375pxを視認。これらのブラウザ画像はGPUピクセルの証拠にはしていない。
- ⭕️ [Pending] 署名済み配布アプリでの実UIとの重なり・実ホイール操作・保護状態バーの視認、Retina／HiDPI実表示、複数ディスプレイ、全画面／スリープ復帰、長時間負荷・メモリ／電力、device lossと別macOS／Intel環境。詳細は`native-gpu-panel.md`。

## 2026-10-17 署名済み安定版更新候補 0.1.1

- Rust通常111 passed／21 ignored、フロントエンド95 passed、Pythonの公証ペイロードゲート3 passed。フロントエンド・Tauri releaseビルド成功。
- Developer IDで0.1.1/build 2をローカル署名し、deep/strict・Team／識別子の要件・署名済みpreflightが成功。本体と拡張機能の版を揃え、ブリッジの署名IDを修正。ローカル署名専用段階はApple公証送信の前で終了する。
- `updates::signed_upgrade_tests::production_key_signed_upgrade_on_disposable_app`を明示実行し1 passed。実公開鍵と実プラグイン、旧署名アプリの一時コピーで検証。改ざん拒否、破損時の署名復旧、公証前候補の`Unnotarized Developer ID`拒否→旧版復旧と外部テスト文書保持が成功。利用中のアプリや文書は変更せず、アプリ起動も行わない。初回の一時サーバー試験はaccepted socketのnonblocking継承で失敗したため、blockingへ明示設定して再実行・成功。製品のネットワーク方針は変更していない。
- Browser plugin not availableのためbundled Playwright＋Chrome。localhost:1421でJA／EN／简体中文の動的0.1.1版表示、375pxで横はみ出し／Vite overlay／runtime errorなし。`/tmp/flowguard-011-version-375.png`を視認。
- `cargo fmt --check`、`git diff --check`、Python構文検証が成功。Apple公証ZIPは7,584,489 bytes、SHA-256 `f91dc8dd53380c1132bbdf8a97e9198132dcd8bb4bcad53ed6f0bcc6c2d8fdc2`。
- ⭕️ [Pending] 自動承認レビューがアプリZIPのApple送信への明示承認不足で公証を拒否。ZIPとハッシュを準備して承認待ち。公証後の正常適用・再起動／実文書保持・公開HTTPSフィードと稼働中拡張機能の継続確認は未完了。詳細は`releases/0.1.1-validation.md`。ローカル検証専用非公証成果物を公開用と混同しない。

## 2026-10-25 初回の保護状態ガイド

- `CARGO_TARGET_DIR=/private/tmp/flowguard-guide-target CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=4 cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --lib --offline`: 111 passed／21 ignored。追加3件で旧設定の未確認既定・型検証、確認保存／再ロード・既存ルールと合成の適用受領状態の維持、保存失敗の巻き戻しと更新適用中の拒否を確認。GPUは未変更で、今回はGPUなし構成を検証。
- `npm test`: 97 passed／20 files。`npm run build`と`cargo fmt --check`、`git diff --check`が成功。
- Browser plugin not availableのためbundled Playwright＋Chromeを使用。localhost:1421、JA／EN／简体中文、1550／1200／375pxの9組で4段階・文の区切り・横はみ出しを確認。通常画面を塞ぐダイアログを作らず、フローティング詳細を除外。保存失敗で案内が残ること、再試行・別タブ同期・再読み込み後の保持・設定からの再表示／閉じる・旧設定の初期表示を確認。更新適用中はテスト用Snapshotで保存不可と読み取り専用の再表示を検証した。案内を確認してもルール・他の設定を変更しないことをブラウザでも比較。
- `/tmp/flowguard-guide-qa.cjs`、スクリーンショット`/tmp/flowguard-guide-{ja,en,zh-CN}-{1550,1200,375}.png`。日本語375pxを視認し、意味の途中での改行を減らすため、文単位の表示と列数を調整して再検証。page title／runtime error／console／Vite overlay／横はみ出しに問題なし。
- 検証のためだけに作成した一時Rustビルド領域を削除。既存の署名鍵、0.1.0／0.1.1成果物、従来のアプリバンドルを保持し、公証待ちZIPのSHA-256を再照合した。案内追加を含む署名済み配布物の再生成・Apple送信・GitHub公開は行っていない。
- ⭕️ [Pending] 署名済みTauri画面での初回・旧文書・保存エラー・複数ウインドウ・更新中の通し確認。今回の合成Provider状態の保持テストは、実Providerの適用や遮断を証明しない。

## 2026-10-25 接続先名の明示取得

- 一時領域で`CARGO_TARGET_DIR=/private/tmp/flowguard-destination-target CARGO_PROFILE_TEST_DEBUG=0 CARGO_PROFILE_DEV_DEBUG=0 CARGO_INCREMENTAL=0 CARGO_BUILD_JOBS=4 cargo test --manifest-path src-tauri/Cargo.toml --no-default-features --lib --offline`: 121 passed／21 ignored。追加10件はIP／ASCII名／正確な応答IP照合、同意・取消／再許可・遅延／消去、10秒間隔・64件・5分経過、ネットワーク変更、実測接続照合・デモ／観測失敗・更新排他・文書非変更を検証。固定ローカルprintfで出力上限、sleepで子プロセス取消と回収、取消済み照会のリゾルバー起動前拒否も確認。実DNSは呼び出さない。
- `npm test`: 101 passed／21 files。取得済み情報を正確なIP・実測・同意のある時だけ表示すること、対象外IP、3言語キー、取消済みワーカー完了までの更新インストール拒否を検証。`npm run build`成功。GPU／Swiftのコードとブリッジ契約は変更していない。
- Browser plugin not availableのためbundled Playwright＋Chromeを使用。localhost:1421、JA／EN／简体中文・1500／1200／375px。native IPCとDNSはテスト専用モック。オプトインだけでは照会なし、明示ボタンで一度、処理中の二重受付防止、元IPの維持、未検証・取得元・時刻・古さ、取得不能／不正名／失敗／時間切れ、HTMLを文字列として表示、取消・消去後の非表示を確認。初期プレビューでは取得不可。page title、runtime/console、Vite overlay、root overflowに問題なし。
- `/tmp/flowguard-destination-qa.cjs`、`/tmp/flowguard-destination-{ja,en,zh-CN}-{1500,1200,375}.png`。日本語375pxで実画像を確認。最終QAではViteのHMRクエリ付きURLにもモックを適用し、表示を再確認した。これはテスト配線の修正で、製品の通信方針を変更していない。
- `cargo fmt --check`、`git diff --check`成功。SSD使用量を抑えるため一時Rustビルド領域を削除し、既存0.1.1公証待ちZIPのSHA-256を再照合。署名候補を再生成／送信していない。
- ⭕️ [Pending] 実dscacheutil／OS設定のリゾルバーとDNS等への通信、実IPv4／IPv6・VPN・権限・スリープ・同じIPを保つ経路変更、実Tauri画面での操作と複数ウインドウ。NEFilterSocketFlow.remoteHostnameはAppleの名前指定API由来に限定される仕様を確認したが、Providerの観測実装と実フロー相関は後続。未検証名を実ホストや安全性の証明として扱わない。

## 2026-10-25 実アプリアイコン

- GPUなしRustライブラリテスト126 passed／22 ignored、TypeScript106 passed／22 files、npm run buildが成功。実測対象・署名キー／パス・デモ拒否・文書非変更、PNGサイズ／形式、キャッシュ上限／期限／fingerprint変化、UI同時上限／重複排除・失敗・3言語を検証。GPU有効のcargo check --offlineも成功（既存の未使用routerメソッド・blockの将来互換性警告あり）。cargo fmt --checkとgit diff --checkを実施。
- 明示的なignoredテストactual_workspace_icon_is_a_bounded_pngをこのMacで実行し、1 passed。実TextEdit.appからNSWorkspaceで取得し、/tmp/flowguard-native-app-icon.png（96px）を目視確認。
- Browser plugin not availableのためbundled Playwright＋Chromeを使用。/tmp/flowguard-icons-qa.cjs、localhost:1421、JA／EN／简体中文・1500／375px。実取得PNGをテスト専用IPCに注入し、実画像のnaturalWidth/Height=96、既存枠26〜48px、汎用代替、subject単位の重複排除、表示範囲に近づくまで取得しないこと、デモの画像なし、pageerror／Vite overlay／root overflowなしを確認。/tmp/flowguard-icons-{1500,375}.pngの375pxを視認。実Tauri IPC・実接続ではない。
- ⭕️ [Pending] 署名済み実Tauriでの全表示箇所・実観測／IPC・フローティング、Retina・複数ディスプレイ・別OS・権限・実アプリ変更とOS API遅延。詳細は[app-icons.md](app-icons.md)。既存の署名候補は再生成／送信しない。
- 検証専用Rustビルド領域約1.3GBを削除し、タスク用Viteを停止。署名鍵・既存target・配布物を保持し、公証待ち0.1.1 ZIPのSHA-256が`f91dc8dd53380c1132bbdf8a97e9198132dcd8bb4bcad53ed6f0bcc6c2d8fdc2`のまま一致することを確認。

## 2026-10-25 通信履歴の保持期間

- 一時Rust領域・debug情報なし・incrementalなしでcargo test --manifest-path src-tauri/Cargo.toml --no-default-features --lib --offline: 132 passed／22 ignored。追加6件は設定の閉じた値と旧文書移行、500件・時刻境界・未来時刻、設定保存／再起動・履歴非永続化、保存失敗／更新排他、初回時刻維持・off／再有効化・継続接続と新規接続、観測失敗時の消去、実測／デモ分離と削除後非復活、現在接続／選択／ルール／適用確認の維持を確認。
- npm test: 111 passed／23 files。npm run build成功。GPU有効cargo check --offline成功（既存のrouter未使用・block将来互換性警告）。cargo fmt --checkとgit diff --checkを実施。
- Browser plugin not availableのためbundled Playwright＋Chromeでlocalhost:1421を確認。3言語・1500／375px、履歴→15分→session→off→1h→設定24h→履歴、保存エラーを操作。合成デモ履歴4→2→0件・現在通信4件維持、延長しても復活なし、保存エラーで選択値を巻き戻し。2タブ間の設定と削除の同期・非復活も成功。page title／URL／h1、Vite overlay、pageerror／console、root overflowのチェックが成功。
- /tmp/flowguard-history-qa.cjs、/tmp/flowguard-history-populated-{ja,en,zh-CN}-{1500,375}.pngと保存エラー状態の/tmp/flowguard-history-{ja,en,zh-CN}-{1500,375}.png。日本語375pxを視認。これは合成デモ／ブラウザー検証であり実Tauri IPCではない。
- ⭕️ [Pending] 署名済みTauriでの実観測・複数ウインドウ・トレイ常駐／明示終了、実スリープ／時刻変更を跨ぐ期限処理と長時間運用。詳細は[history-retention.md](history-retention.md)。今回の追加は公証待ち0.1.1 ZIPに含めていない。
- タスク用Viteを停止し、一時Rust領域約1.3GBを削除。既存target・署名鍵・0.1.0／0.1.1配布物を保持し、0.1.1公証待ちZIPのSHA-256が前回と一致することを確認。再署名／Apple送信／公開は行っていない。

## 2026-10-25 GPU概要の表示条件追従

- Rust（GPUなし）137 passed／22 ignored、GPU＋gpu-qa有効140 passed／22 ignored。新しい純粋テスト5件と既存テストを実施。閉じたFilter／UTF-16上限／NUL、IPループバック表記、条件の交差、方向・更新時刻・同率の優先、UDP／旧パスルール除外、Rust／TypeScript共有fixture14ケースを確認。UI112 passed／23 files、npm run build成功。cargo fmt --check・git diff --checkを実施。
- 明示的な`cargo run --manifest-path src-tauri/Cargo.toml --example native_panel_check --features gpu-qa --offline`で、一時文書と専用Tauri WebViewの実Metal Surfaceを検証。backing_scale=1。9段階が成功し、10,001件のうち10,000件、クリップ／リサイズ／非表示／終了／再作成に加え、同じrevisionでの検索変更・0件→復帰、条件一致件数への上限、ループバック非表示、意図1件／ルールなし9,999件、source不一致と逆順sequence拒否、GPU完了待ちを確認。実観測・ネットワーク送信・OS設定変更は行わない。
- 初回の追加ハーネスでは合成データの同じアプリ／接続先が周期的に重複し、意図1件の想定と一致せず失敗。検証対象を一意な接続先に修正して再実行し、全段階成功。失敗終了時に残ったGPU資源がTLS終了時にpanicする検証ハーネスの経路も、RunEvent::Exitで明示解放するよう修正した。最終ログは/tmp/flowguard-gpu-filter-metal-final.log。
- Browser plugin not availableのためbundled Playwright＋Chromeでlocalhost:1421を検証。JA／EN／简体中文・1500／375px。通常マップと同じ合成fixtureで検索・all／allow／block／ask／none・Mac内非表示を操作し、件数・0件を照合。IPCには条件と矩形／lease／sequenceだけがあり、ID一覧・ルール・描画データを送らない。遅延成功を999件に改変しても採用せず、遅延失敗で新しいパネルを閉じず、閉じた後の応答でも再表示しない。source不一致時は件数を保留して再同期する。通常ブラウザーはGPU非対応で通常一覧を維持。ページ名／h1／URL、runtime／console、Vite overlay、root overflowに問題なし。
- /tmp/flowguard-gpu-filter-qa.cjs、/tmp/flowguard-gpu-filter-{ja,en,zh-CN}-{1500,375}.png。日本語375pxを視認。ブラウザーはテスト専用IPCであり、白い描画枠のスクリーンショットをMetalの実ピクセルの証拠としない。実Surfaceの確認は上記の独立ハーネスで行う。
- ⭕️ [Pending] 署名済み実アプリの通常UIから実観測を絞り込んだ際の描画と入力、WebViewとMetalの重なり、Retina／複数ディスプレイ・スリープ・長時間負荷・別OS／Intel。今回の変更を公証待ち0.1.1 ZIPへ同梱していない。詳細は[native-gpu-panel.md](native-gpu-panel.md)。
- タスク用Viteを停止し、検証用Rustビルド領域約1.8GBを削除。既存target・署名鍵・配布物は保持し、公証待ち0.1.1 ZIPのSHA-256が前回と一致することを確認。再署名／Apple送信／公開は行っていない。

## 対象Laterの最終検証 — 2026-10-25

今回の対象はROADMAPの「以降の計画」より前の7項目。後続の計画本文は変更していない。実装対応は [later-completion.md](later-completion.md) を参照。

| 検証 | 結果 |
| --- | --- |
| Rust `--features gpu-qa --lib --offline` | 157成功、23 ignored。ignoredを完了扱いにしない |
| Rust `--no-default-features --lib --offline` | 153成功、23 ignored |
| TypeScript/Vitest | 25ファイル・120成功 |
| TypeScript + Vite production build | 成功。main chunk約498KB |
| Swift package XCTest | Policy 9 + Host 2 + Firewall 5 + Bridge 27 = 43成功 |
| Swift全product／SystemExtensionソースのbuild | 成功。署名・インストール・有効化はしていない |
| ローカルHTTP中断／再開 | 専用ignored試験を明示実行して1成功。部分body切断→cache再open→Range/If-Range/206→全体Minisign検証。外部配信・installerなし |
| 隔離Tauri／実Metal | 10項目成功。10,000件、clip/resize/close、filter・古いsource/sequence拒否、ネイティブラベル・zoom/reset/共有pick、浮動Surfaceと所有窓分離、解放 |
| ブラウザChrome・診断とdrag | JA/EN/简体中文 × 1500/375pxの6条件成功。検索、詳細、対象外理由、CSV/HTML download、既存意図画面への移動、キーボードとdrag配置。ルール0件のまま |
| ブラウザChrome・名前／一時許可／更新 | 同6条件成功。許可だけでは収集しない、複数候補名と別許可の検索、取消、一時許可のpeer確認必須・未受領表示・解除、channelのready中変更拒否、履歴・cache削除 |
| ブラウザChrome・GPU命令 | 同6条件＋非対応fallback成功。zoom/pan/reset/pick、検索／意図／loopback、遅延成功・失敗の棄却、閉じる／再open。描画IPCはモック |
| Python release/merge tests | 4成功。`PYTHONPATH=scripts python3 -m unittest discover -s scripts -p 'test_*.py'` |
| Rust artifact verifier example | GPUなしでcheck成功。実公開フラグは未実行 |
| 差分・既存成果物 | `git diff --check`成功。既存0.1.1 notarization.zipのSHA-256は `f91dc8dd53380c1132bbdf8a97e9198132dcd8bb4bcad53ed6f0bcc6c2d8fdc2` のまま |

追加した境界テストは、旧契約／CPU違い／短すぎるMach-Oの拒否、cache排他・symlink・ETag、ルート署名付き鍵委任と改変拒否、install要求とlaunch履歴の区別、診断の未実施と出力escape、古い世代／曖昧なアプリへのルール移動拒否、停止済み名前／許可store、UUID違いの一時許可拒否、heartbeat失効・古い成功・解除・Provider再起動の非復元、完全一致するpaused askの1回allow、ask pollが一時許可を消さないことを含む。

UI検証はnative IPC／Provider／診断応答をモックした。スクリーンショットとログは `/tmp/flowguard-later-*` に保持。375pxの一時許可フォームと診断詳細も目視確認し、横溢れ・JSエラーなし。Metal試験は実Surfaceを使うが、別ID・一時文書の隔離QAであり配布アプリの起動試験ではない。追加のDNS照会、LAN検査、実通信制御、OS設定保存、Apple送信、GitHub公開、本番鍵変更は実行していない。

⭕️ [Pending] 署名済みv5 Providerの実フロー名・UDP DNS／mDNS・一時許可、VPN/スリープ/経路変更/旧新切替・管理通信継続。元アプリのpeer確認は利用者申告。初回フローの許可や受領を、パケット単位の保護／既存通信の遡及停止／peer本人性の証明へ読み替えない。

⭕️ [Pending] 実Inspector跨ぎdrag、署名済みUIとGPUの通し操作、Retina／複数画面／長時間負荷。Intel実ビルド、両architectureの公証・公開、実HTTPS再開、本番鍵切替、beta配信、実アプリの更新と再起動。既存0.1.1候補に今回のv5/ABI5は含まれず、新しい版の署名済み成果物が必要。

検証後、今回専用の `/private/tmp/flowguard-later-target`・Swift scratch／module cache（計約2GB）を削除し、QA用Viteを停止した。ソース・既存署名候補・鍵・検証ログ／画像は維持した。

## 通常ビルドの未使用警告整理 — 2026-10-25

通常ビルドで発生していた自前コードの8件のdead_code警告を解消。名前なし投影と安定版URL検証のテスト用ラッパー、未公開router境界のテスト用facadeは`cfg(test)`、GPUのmain用open/close・sync・suspendラッパーは`gpu-qa`に限定した。呼出しのない上位suspendラッパーは削除。実運用は既存のowner付きopen/close・sync_action経路を維持する。

`cargo check --manifest-path src-tauri/Cargo.toml --offline`が成功し、自前コードの警告は0件。`cargo test --manifest-path src-tauri/Cargo.toml --features gpu-qa --lib --offline`は157成功・23 ignored。`git diff --check`も成功。

依存`block 0.1.6`の将来互換性警告は残る。`cargo tree -i block@0.1.6`でwgpu27→wgpu-hal27／metal0.32の依存と確認し、Cargo future-incompat reportで`static _NSConcreteStackBlock: Class`の空enum型（uninhabited static）を原因と特定した。現コンパイラーではビルド成功だが、将来のRustへの適合を保証しない。警告の全体抑制・Cargoキャッシュの直接編集・未検証のGPU依存更新は行っていない。

## サービス再確認の前回差分 — 2026-10-25

ROADMAPのサービスシートから、前回の完了結果との比較を実装。仕様・範囲は[service-comparison.md](service-comparison.md)。比較基準と判定はRust共有状態が所有し、WebViewは結果・選択・表示フィルターを扱う。

- Rust `cargo test --manifest-path src-tauri/Cargo.toml --features gpu-qa --lib --offline`: 161成功、23 ignored。追加した4テストは比較条件全体、時刻の除外、不確定の扱い、停止／失敗／不正な結果からの基準保持、無効化後の遅延応答、前回情報のCSVを検証。既存SharedState試験も差分の通し操作まで拡張した。
- Vitest: 25ファイル・124成功。差分フィルター、比較不可時の結果保持、不確定表示、3言語キーを検証。
- `npm run build`: 成功。既存ガイドPNGのimportに必要なVite標準型宣言を`src/vite-env.d.ts`へ追加。画像・ガイド本文は変更していない。
- Browser plugin not availableのためbundled Playwright＋Chromeでlocalhost:1421を検証。JA／EN／简体中文 × 1500／375pxの6条件で、比較件数、検索・状態・差分絞り込み、前回詳細／証明書情報、全結果CSV、条件変更／基準なし／確認中／停止／無効化と再確認が成功。表示とフィルターでは開始命令を送らず、明示開始だけが1回呼ばれる。CSVのブラウザー試験はダウンロード導線と全行取得、Rust試験は実際の出力内容・エスケープを検証する。
- `http://127.0.0.1:1421`のFlowGuard／サーバー確認画面を確認。JS／consoleエラー・Vite overlay・root横溢れなし。1500／375pxの日本語スクリーンショットを視認。表は内部横スクロール、詳細は狭幅で1列になる。合成応答の`<script>`を文字として表示し、ルール0件を維持した。
- ログ: `/tmp/flowguard-service-comparison-{rust-full,ui,build,browser}.log`。ブラウザー手順: `/tmp/flowguard-service-comparison-qa.cjs`。画像: `/tmp/flowguard-service-comparison-{ja,en,zh-CN}-{1500,375}.png`。

⭕️ [Pending] 実LANのサービス／証明書更新、タイムアウトからの回復、VPN／経路変更、署名済みアプリの複数ウインドウ共有・CSV保存の通し操作。ブラウザーは合成スナップショットとテスト用IPC。今回、追加の実LAN検査・OS設定変更・再署名・Apple送信・リリース公開を行っていない。

通常の`cargo check --manifest-path src-tauri/Cargo.toml --offline`も成功し、自前コードの警告は0件。既知の依存`block 0.1.6`の将来互換性警告のみ残る。`git diff --check`成功。既存0.1.1公証候補ZIPのSHA-256は`f91dc8dd53380c1132bbdf8a97e9198132dcd8bb4bcad53ed6f0bcc6c2d8fdc2`のまま。今回の検証には既存Rust targetを使用し、専用の重複ビルド領域は作成していない。

## 通信マップ本体のノードグラフ — 2026-10-25

`FlowMap.tsx`に既定のノードグラフ／既存一覧の切り替えを追加。新しい`TrafficGraph.tsx`／専用CSSでアプリ→接続先を表示し、線の色は保存した意図、実線／破線はルール受領確認の有無を示す。接続先ノードには意図と適用状態を別欄で表示する。1ページ4アプリ・各4接続先を上限に、ページ移動で全件へ到達できる。Rustが所有する選択とInspector、既存のGPU概要、検索／分類／名前情報を維持する。

受領表示の共通ヘルパー`ruleApplied`は署名・Provider instance・revision・エラーなしを追加条件にした。`mapModel`は同一パス・同一接続先でもルール照合結果が異なる観測を混在させない。`ruleFor`の照合規則・Rust／Swift・OS設定は変更していない。

- `npm test`: 27ファイル・132成功。署名等の確認根拠不足、状態変更、不確定、デモ、システム保護、UDP、異なる署名の集約、172件のページ移動とフィルター後の範囲補正を含む。
- `npm run build`と`git diff --check`: 成功。グラフは遅延ロードし、main JSは約499.72KB。新しいライブラリ・画像・外部フォントは追加していない。
- Browser plugin not availableのためbundled Playwright＋Chromeで`http://127.0.0.1:1421`を検証。日本語／英語／简体中文 × 1500／768／414／375／320px。ページ名・h1、初期表示、接続先選択→共有selectedId／Inspector、キーボード選択、アプリ／接続先のページ移動、検索・ルール絞り込み・Mac内通信非表示、一覧往復を確認。ノード内テキストのはみ出し・root横溢れ・Vite overlay・JSエラーなし。既存のfavicon未設定による404は分離した。
- applied→disconnected／busy／未署名／instanceなし／revisionなし／観測エラーで、保存した意図を保持したまま受領表示が消えることを確認。デモではルールが存在しても適用済みとしない。未知方向には矢印を付けない。フロー名の取得元・時刻・未検証表示と検索、空結果も確認した。
- 広幅と375pxを視認。アプリ名・合計／接続数・接続先・意図を大きく、補助メタデータを小さくし、実アプリアイコンの既存取得経路を維持。狭幅の接続線は個別の経路へ分離。ブラウザーでは実アイコン取得は行わない。
- 実状態・署名情報は合成スナップショット。手順・ログは`/tmp/flowguard-traffic-graph-{qa,extra}.cjs`、`/tmp/flowguard-traffic-graph-{tests,build,browser}.log`、画像は`/tmp/flowguard-traffic-graph-*.png`。実フローや実際の遮断を検証したものではない。

⭕️ [Pending] 署名済みTauriアプリでの実アイコン・実フロー・Provider受領、複数ウインドウ選択同期、Retina／複数画面。GPU描画処理・ネイティブ通信制御は今回の変更対象外。既存署名候補の更新・Apple送信・公開は行っていない。


## LAN調査の関連グラフと状態アニメーション — 2026-10-25

ROADMAPの「機器情報とサービス・診断結果の関連付け」を優先して実装。`InvestigationGraph`を機器詳細に遅延ロードし、機器→TCPポート→サービス→診断を選択すると既存記録の内訳が切り替わる。アイコン、40pxのノード件数、リング／横棒と診断の重要度を使い、説明は情報アイコンにまとめる。Rust共有スナップショットを表示側で投影し、ローカルstateは選択表示・読込中・エラーのみ。Rust／Swift、通信制御、wgpuの処理は変更していない。

`investigation-graph.ts`は収集世代・対象IP・インターフェース・現行ネットワークの送信元／ネットマスク・操作IDを照合する。既存の`samePortTarget`を狭い引数型にして再利用し、IPv6の正規化／zone判定を維持。IPv4の収集アンカーとIPv6のTCP送信元を分けて確認する。古い収集・ネットワーク変更・対象違いでは線を結ばない。TCPの不確定／未完了を閉鎖として扱わず、HTTPSの確認件数にはTLS検証を要求する。Rustが未選択レポートのfindingsを省略する契約に合わせ、明示読込まで「—」を表示し、空配列を安全や0件と判定しない。

- `npm test`: 28ファイル・140成功。追加8ケースで関連スコープの一致／不一致、未読込と読込後、IPv6の表記・zone、TLS未検証、3言語キーを確認。
- `npm run build`: 成功。新しいライブラリや画像は追加せず、調査グラフと診断画面のSyslogReceiverを遅延ロード。main JSは約486.48KB、500KB超警告なし。
- Browser plugin not availableのためbundled Playwright＋Chromeを使用。日本語／英語／简体中文 × 1500／768／414／375／320pxの15条件で、4ノード・件数・リング／横棒・キーボード選択・診断の明示読込・TCP／サービス／診断画面への移動、世代／送信元／対象変更・記録欠落・中断を検証。グラフ／ノード内とrootの横溢れ、Vite overlay、JSエラーなし（既存favicon 404を除外）。表示・移動による調査開始コマンドは0回。
- 初回の狭幅試験でツール画面のグリッドが内容幅に拡張し、ボタンが画面外へ出る問題を検出。`tools-workspace`に`minmax(0,1fr)`を指定して修正し、全15条件で再検証。狭幅では縦グラフと全幅のIP見出しに切り替える。
- 内訳の登場／横棒伸長、通信マップの選択時ハイライトを追加。ループするパケット移動や架空の通信量は表示しない。`prefers-reduced-motion: reduce`で調査内訳・横棒・通信マップ選択ノード／線のanimationがnoneになることをブラウザーで確認。
- 合成スナップショット・テスト用IPCを使用。手順: `/tmp/flowguard-investigation-qa.cjs`、`/tmp/flowguard-animation-qa.cjs`。ログ: `/tmp/flowguard-investigation-{tests,build,browser-final}.log`、`/tmp/flowguard-animation-browser.log`。画像: `/tmp/flowguard-investigation-{ja,en,zh-CN}-{1500,768,414,375,320}.png`。広幅と狭幅を視認。

⭕️ [Pending] 署名済みTauriアプリの実LAN調査・ネットワーク切替、フローティング／ドッキング・複数ウインドウ同期、WebKit／Retinaと実機の動きを減らす設定。今回はフロントエンドのみを変更し、実LANの追加調査・OS設定変更・再署名・公証申請・リリース公開は行っていない。


## 2026-10-25 履歴の時間分布

- `npm test`: 29ファイル・143テスト成功。区間境界、空区間、長期間の最大12区間、空／無効時刻を検証。
- `npm run build`: 成功。
- Browser skillが利用可能一覧にないため、既存のbundled PlaywrightとChromeを使用。`http://127.0.0.1:1420`、JA／EN／简体中文 × 1500／375／320px。デモ履歴でサイドバー→履歴→棒選択→1行→解除→4行→検索→1行→保持オフ→0件を確認。ページidentity、非空表示、Vite overlayなし、横はみ出しなし、アプリconsole errorなし。既存favicon 404は分離。
- 画面証跡: `/tmp/flowguard-history-timeline-{ja,en,zh-CN}-{1500,375,320}.png`。日本語のdesktop／375px画像を目視確認。検証スクリプトは`/tmp/flowguard-history-timeline-qa.cjs`。
- 新しい外部依存・Rust／Swift変更なし。ネイティブ実測、長時間・時計補正・複数ウインドウはPending。


## 2026-10-25 通信マップの中継情報・詳細固定

- `npm test`: 143件成功。`npm run build`: 成功。
- Chrome／Playwright、`http://127.0.0.1:1420`、JA／EN／简体中文 × 1500／768／375／320px。各接続先に中継ラベルが存在し、クリックでTerminalと選択状態が共有されることを確認。詳細パネルはスクロール後もviewport上端8／16pxへ追従し、内部に独立スクロールがあることを確認。横はみ出し・Vite overlay・pageerrorなし。
- `/tmp/flowguard-route-detail-qa.cjs`と`/tmp/flowguard-route-detail-{locale}-{width}.png`、`/tmp/flowguard-route-detail-pinned.png`。日本語1500／375px・上端固定のスクリーンショットを目視確認。
- 実Tauri、フローティング／ドッキングのネイティブ操作は未検証。Rust共有状態・通信処理の変更なし。


## 2026-10-25 実Tauriの通信情報・上部固定

- `npm run tauri -- build --debug --bundles app`成功。`src-tauri/target/debug/bundle/macos/FlowGuard.app`をフルパス指定で起動。実WebViewのURLは`tauri://localhost`、表示版0.1.1、1440×960、日本語、実際のTCP観測。GPU既定featureあり、Extension未配置の観測状態。
- CUAで中継ラベルをクリック。codexの172.64.155.209:443とCodex (Service)の104.18.39.21:443が詳細へ反映され、ラベルと接続先の選択色も同期。接続数の定期更新と実アイコンを確認。
- グラフ上を下スクロールしても詳細カード上端がウインドウ上部に維持されることをスクリーンショットで確認。詳細内をスクロールするとグラフ位置を維持したまま下部の保存ボタン等に到達。詳細の先頭へ戻してアプリを起動したまま残した。ルール保存・DNS許可・OS保護設定変更は行っていない。
- アプリ名だけの解決は旧`/Applications/FlowGuard.app` 0.1.0を起動しSIGABRTが発生（ユーザー添付レポート）。旧版の起動panicの原因はこの検証では未確定。最新バンドルでは起動・観測・上記操作に成功。インストール済み旧版の置換はしていない。
- 今回の実機確認は開発debugバンドルのみ。署名／公証済み配布版、他言語・別画面幅、Providerによる許可／遮断・複数画面の検証へ拡大しない。


## 2026-10-25 通信グラフのビジュアル強化

- 143件のVitestとTypeScript／Viteビルド成功。最新debug macOSバンドルの生成成功。
- 既存`/tmp/flowguard-route-detail-qa.cjs`によるJA／EN／简体中文 × 1500／768／375／320pxの回帰確認成功。ノード間の選択・詳細反映・sticky・内部スクロール・横はみ出しなし・overlayなし・pageerrorなしを確認。日本語desktop画像を目視確認。
- CUAで最新debugバンドルをフルパス起動。実測codexのTCP443を中間ラベルから選択。選択経路halo、接続数19／11の強弱、実アイコン、接続先の分類バッジ、詳細上段カードと上端追従を1440×960で目視確認。OS保護設定・ルール保存・名前照会は操作していない。
- 選択haloはUI選択、構成バーはIP分類別の接続件数。転送量・フロー通過・遮断実績には対応しない。常時流れる粒子アニメーションは追加していない。
- 署名／公証済み配布版、Retina／複数画面、他言語／画面幅の実TauriはPending。


## 2026-10-26 LAN機器シートのLater拡張

- `npm test`: 30ファイル・147件成功。分類・数値ソート／未取得・末尾までの仮想範囲・履歴表示の分離を追加。
- `cargo test --manifest-path src-tauri/Cargo.toml --offline --no-default-features --lib`: 165件成功・23件ignored。部分範囲／同時数上限、RTT、列幅の競合、表示名、8回の履歴上限・再読込／原子的保存失敗、NBSTATのtransaction／応答種別／欠損、SRV／DNS名のlabel境界、OUI除外、公開情報の許可／取消／30秒間隔、CSV列整合・0msを検証。実ネットワーク向けignoredテストは実行していない。
- `cargo check --manifest-path src-tauri/Cargo.toml --offline`: GPU既定feature有効で成功。TypeScript／Viteビルドとdebug macOS bundle生成成功。LAN画面を遅延読込にし、初期JSを500kB未満に維持。
- ブラウザーskillが利用可能一覧にないためbundled Playwright／Chromeを使用。`http://127.0.0.1:1420`、合成200行、JA／EN／简体中文 × 1500／768／375／320px。page identity、非空、Vite overlayなし、pageerrorなし、ページ横溢れなし。仮想行は20行未満で201番目のaria行へ到達。縦横スクロール後のheader／address固定、別名検索／保存、幅220px、IP範囲フィルター、IP降順、保存回の読み取り専用表示を確認。外部HTTPS要求0件。
- 一時スクリプト `/tmp/flowguard-lan-sheet-qa.cjs`、画像 `/tmp/flowguard-lan-sheet-{ja,en,zh-CN}-{1500,768,375,320}.png`。desktopと320pxを目視確認し、狭幅のアドレス優先・列幅縮小、headerの不自然な日本語折返しを修正。
- `com.flowguard.lanqa`／`FlowGuard LAN QA.app`を別identifierでビルド。CUAから実Tauri（tauri://localhost、日本語1440×960）で近隣キャッシュ収集→2行→表示名保存→正常完了結果の明示保存→保存回表示を確認。終了／最新bundleで再起動後、現在結果0行・保存回1件を確認し、旧回2行と`LAN QA — local record`が読み取り専用で再表示されることを確認。テストデータは独立QAアプリ領域だけへ保存。既存FlowGuard文書・OS保護・配布候補は変更していない。
- 実機確認はキャッシュだけ。能動Ping、mDNS／NBSTAT実LAN応答、IPinfoの実要求、SSID開示・切替、複数ウインドウ・権限・署名済み配布版はPending。IEEE公式の公開OUIデータのみ取得し、機器情報を外部送信していない。
- 実装詳細と範囲は[lan-sheet-expansion.md](lan-sheet-expansion.md)。IPv6能動探索・仮想／ブリッジIFへの探索拡張は別途検討のLaterを維持。


## 2026-10-26 LAN TCP確認の実行条件・リング表示

- `npm test`: 31ファイル・151件成功。条件境界・120秒見積り・件数上限・プリセット・未確認／不確定の集計・翻訳を追加。
- `cargo test --manifest-path src-tauri/Cargo.toml --offline --no-default-features --lib`: 170件成功・24件ignored。ワーカー注入試験で並列上限・選択待機・停止／予算／ネットワーク変更後の結果拒否・間隔中の停止・条件変更による比較失効・31列CSVを確認。
- `cargo test --manifest-path src-tauri/Cargo.toml --offline --no-default-features --lib custom_timeout_connects_on_ipv4_and_ipv6_loopback -- --ignored`: 1件成功。1800ms設定の実ソケットでIPv4／IPv6のlo0にだけ接続し、送信元・開放・拒否を確認。LAN探索は行っていない。
- TypeScript／Viteビルド成功。GPU既定feature有効のcargo check成功。
- Browser skillが利用可能一覧にないためbundled Playwright／Chromeを使用。通常Viteページからコンポーネント検証ハーネスを載せ、JA／EN／简体中文 × 1500／768／375／320pxで低負荷／標準／長めに待つ、上限・予算による開始拒否、テスト用IPCの設定引数、確認中の編集ロック、停止、入力変更後も実行条件を保持すること、4状態リング、CSV呼び出し、予算切れ表示を確認。page identity・非空・overlayなし・関連console/pageerrorなし・ページ横溢れなし・外部HTTPS要求0件。
- `/tmp/flowguard-port-settings-qa.cjs`と`/tmp/flowguard-port-settings-{ja,en,zh-CN}-{1500,768,375,320}.png`。日本語desktop／320pxを目視し、重複件数をリングへまとめ、送信元／要求ポートの情報アイコン、時間単位、次回入力と実行条件の区別を確認。
- 今回の実LAN、実Tauri WebKit・複数ウインドウ・Retina、署名済み配布版はPending。サービス応答確認・ルール・OS保護設定は変更していない。詳細は[tcp-check-settings.md](tcp-check-settings.md)。

## 2026-10-26 — LAN TCPポートマップ

- 🟢 [Done] TypeScript151件、`npm run build`、`git diff --check`成功。Rust変更なし。
- 🟢 [Done] localhost:1420 + Playwright/Chrome（Browser plugin/skill不在）。通常ページの起動後、PortTopology単体とPortChecks組み込みに合成結果を表示。JA／EN／简体中文 × 1500／768／375／320pxで256タイル、未確認無効、選択・詳細更新、コピー内容／拒否／表示解除、横はみ出し・overlay・consoleを検証。
- 🟢 [Done] `/tmp/flowguard-port-topology-qa.cjs`、画像 `/tmp/flowguard-port-topology-{ja,en,zh-CN}-{1500,768,375,320}.png`。日本語desktop／320pxを目視確認。クリップボードは模擬、LAN送信なし。
- ⭕️ [Pending] 実Tauri／WebKit、OSクリップボード権限、複数ウインドウ選択共有、署名済み配布版。[実装範囲](port-topology.md)。

### 実Tauri追試 — ポートマップ

- 🟢 [Done] 2026-10-26 20:34–20:36 JST、最新debug `FlowGuard LAN QA.app`（別ID `com.flowguard.lanqa`）の実WebKit、日本語1440×960。
- 🟢 [Done] 自身のen0 IPv4のみをGUIから2ポート確認：50943開放1ms／50944接続拒否0ms。タイル選択・詳細状態／IP／interface／時刻の追従、コピー済み表示、実OSクリップボードから検索欄への貼り付け内容一致。2ポートとも確認し、検索欄を空へ復帰。一時TCP待受は終了。
- 🟢 [Done] ネイティブ画面で色・選択枠・送信元→TCP→接続先・コピー表示を目視。通常版・OS保護設定変更なし、他LAN機器への検査なし。詳細は[port-topology.md](port-topology.md)。
- ⭕️ [Pending] 実Tauriの別言語／画面幅／IPv6、クリップボード拒否、複数ウインドウ、署名済み配布版。

## 2026-10-26 — TCPシートの表示列・幅

- 🟢 [Done] Rust173件（24件ignored）・フロントエンド154件、production build、GPU有効cargo check、diffの空白チェック成功。表示設定の必須列／幅境界／競合拒否、文書不変、ジョブ開始・停止・古いワーカー更新での保持、全結果CSV不変を検証。
- 🟢 [Done] Browser plugin/skill不在のためPlaywright/Chromeでlocalhost:1421を使用。通常ページ起動後に実PortChecksへ合成snapshot／模擬IPCを供給し、JA／EN／简体中文 × 1500／768／375／320pxを確認。4／8列、表示切替・必須列、幅変更・初期化、競合から再試行、CSV導線・マップ維持、横はみ出し・overlay・console、256行でのヘッダー／先頭列固定を確認。
- 🟢 [Done] 別IDの最新debug Tauri `FlowGuard LAN QA.app`、実WebKit・日本語1440×960で4列／8列切り替え、幅400pxへの変更・130pxへ初期化、画面往復後の保持、自己en0 IPv4・TCP50944の1ポートの結果更新後の保持を確認。実結果は接続拒否0ms。他のLAN機器への検査やOS保護設定変更なし。
- 🟢 [Done] QA `/tmp/flowguard-port-layout-qa.cjs`、画像 `/tmp/flowguard-port-layout-{ja,en,zh-CN}-{1500,768,375,320}.png`、設定部 `/tmp/flowguard-port-layout-controls-{ja,en,zh-CN}-{1500,768,375,320}.png`。日本語desktop／320pxと実ネイティブ画面を目視確認。
- ⭕️ [Pending] 実複数ウインドウ同時編集、別言語／画面幅・Retina／複数画面、署名済み配布版。列順変更・アプリ再起動後の設定保存は未実装。詳細は[port-sheet-layout.md](port-sheet-layout.md)。

## 2026-10-27 — GitHub Actions整形エラー修正

- 原因をrun `40323466556`／job `121574031400`の実ログで確認。フロントエンド・リリースゲート・Rust177件・更新処理3件は成功し、最後の `cargo fmt --check` が `lib.rs`／`state.rs` の整形差分で失敗していた。
- 🟢 [Done] `cargo fmt`で不足していた整形とモジュール順序を修正。CIの整形チェックをRust toolchainの導入直後へ移し、整形不一致を早期に検出する。
- 🟢 [Done] `actions/checkout`／`actions/setup-node`をv4からNode.js 24ランタイムのv6へ更新。両公式action.ymlでruntimeを確認。アプリのビルド用 `node-version: '22'` と `macos-15`、読み取り権限・検証内容は維持する。macOS arm64のキュー通知は今回の終了コード1とは別。
- 🟢 [Done] ローカル `cargo fmt --check`、`cargo test --locked --offline --lib`（177件・24ignored）、`cargo test --locked --offline --lib updates::tests -- --ignored`（3件、ループバック／使い捨てアプリのみ）、`npm test`（154件）、`npm run build`、`python3 scripts/test_release.py`（3件）、`git diff --check`成功。
- ⭕️ [Pending] 修正のpush後、GitHub上での新しいCI実行。過去の失敗runを再実行するだけでは修正済みのファイルには切り替わらない。今回push・既存runの再実行は行っていない。

## 2026-10-27 — TCP詳細の独立ウインドウとドッキング

- 🟢 [Done] Rust178件（24ignored）・UI155件、production build、cargo fmt --check、更新処理3件・リリースゲート3件、git diff --check成功。閉じる復帰・古いcloseの拒否・enum境界・ジョブ更新時の配置保持・文書不変を検証。
- 🟢 [Done] Browser plugin/skill不在につき既存Playwright/Chromeを使用。localhost:1421、3言語 × 1500／768／375／320pxの合成結果／模擬IPCで左右／下／独立、選択追従、IPv6コピー、失敗時の配置維持、新ジョブでの詳細消去、未確認無効、横はみ出し・overlay・consoleを確認。
- 🟢 [Done] 最新debug Tauri（別ID FlowGuard LAN QA）の実WebKit・日本語で、独立500×680の生成・再表示／再利用、閉じる→下部復帰、左右配置、メイン1440×960との双方向選択同期を確認。自身のIPv6リンクローカルの50063開放／50064拒否を使用し、実コピーを検索欄へ貼り付けて内容を照合。検索欄は空へ復帰、一時待受は終了。IPv4 /19は既存の対応範囲外として開始拒否され、制限は変更していない。他LAN機器の検査・OS保護設定変更なし。
- 🟢 [Done] QA `/tmp/flowguard-port-panel-qa.cjs`。画像 `/tmp/flowguard-port-panel-{docked,floating}-{ja,en,zh-CN}-{1500,768,375,320}.png`、日本語desktop／320pxと実ネイティブ画面を目視確認。
- ⭕️ [Pending] 実Tauriの別言語／画面幅、Retina／複数ディスプレイ、OSウインドウ生成／フォーカス拒否・スリープ／終了との競合、署名済み配布版。ドラッグ式の任意ドッキング／サイズ調整は未実装。[実装と検証範囲](port-detail-panels.md)。

## 2026-10-27 — サービス結果の表示名・コメント

- 🟢 [Done] Rust182件（24ignored）・UI158件、production build、cargo fmt --check、リリースゲート3件。文字数／制御文字・競合／消去後の古い編集拒否、操作番号／ポート／実行中／更新中の拒否、新ジョブでの消去・観測行／文書／比較不変、45列CSVの引用符・数式対策を確認。
- 🟢 [Done] Browser plugin/skill不在のため既存Playwright/Chrome、localhost:1421で実ServiceChecksへ合成Snapshot／模擬IPC。3言語 × 1500／768／375／320pxで保存・検索・競合時の下書き保持・読み直し・入力境界・新ジョブ消去・CSV導線・列幅220px以上・横はみ出し／overlay／consoleを確認。QA `/tmp/flowguard-service-notes-qa.cjs`、画像 `/tmp/flowguard-service-notes-{ja,en,zh-CN}-{1500,768,375,320}.png`。
- 🟢 [Done] 別IDの最新debug Tauri／実WebKit・日本語1440×960。自身のIPv6リンクローカル・TCP50728でHTTP HEAD /の固定応答を取得し、表示名と複数行メモを実保存。コメント検索、画面往復後の保持と観測応答不変を確認。初回の日本語コメント列の細かい折り返しを修正し、再ビルド後の実保存・表示でも意図した改行を目視確認。一時HTTP待受は終了。他LAN機器への検査・外部DNS照会・OS保護設定変更は行わない。
- ⭕️ [Pending] 実Tauri複数ウインドウの競合編集、他言語／画面幅、ネイティブCSV保存・署名済み配布版。長期保存・再確認への自動継承は未実装。[詳細](service-notes.md)。
- 🟢 [Done] 更新処理のCI用3試験（`updates::tests -- --ignored`）も、使い捨てアプリとループバックで成功。

## 2026-10-27 — サービス詳細の独立ウインドウとドッキング

- 🟢 [Done] Rust183件（24ignored）・UI159件、production build、cargo fmt --check、更新処理3件・リリースゲート3件、git diff --check成功。サービス配置の独立性・閉じる復帰・遅いcloseの無効化、文書／結果／選択不変、ジョブ開始・停止・更新時の配置保持を検証。
- 🟢 [Done] Browser plugin/skill不在につき既存Playwright/Chrome、localhost:1421で実ServiceChecks／独立詳細へ合成Snapshot・模擬IPCを供給。3言語 × 1500／768／375／320pxで左右／下／独立、エラー時維持、未保存メモ保持、保存メモ・TLS発行者・前回比較、選択・新ジョブの消去、未確認無効化、横はみ出し・overlay・consoleを確認。QA `/tmp/flowguard-service-panels-qa.cjs`、画像 `/tmp/flowguard-service-panels-{docked,floating}-{ja,en,zh-CN}-{1500,768,375,320}.png`。
- 🟢 [Done] 別IDの最新debug Tauri・実WebKit、日本語メイン1440×960／独立560×800。このMac自身のIPv6リンクローカルのHTTP51857／51858（200 OK／503 Service Unavailable）で、双方向の選択同期・保存メモ表示・独立→右配置での未保存メモ保持、再表示時の同一ウインドウ再利用・閉じる→下部復帰・左右配置を確認。一時HTTP待受は終了。他LAN機器の検査・外部名前照会・OS保護設定変更なし。
- 🟢 [Done] 日本語desktop／375pxと実ネイティブ画面を目視。独立詳細は遅延読込し、初期JS約484.7kBで500kB警告なし。
- ⭕️ [Pending] 署名済み配布版、実Tauriの他言語／画面幅・Retina／複数画面、実TLS更新／前回TLS表示、OSウインドウ生成／フォーカス拒否・終了競合。ドラッグ式任意ドッキング／サイズ調整・複数エンドポイントは未実装。[詳細](service-detail-panels.md)。

## 2026-10-27 — ポート／サービス詳細のドラッグ配置と幅調整

- 🟢 [Done] Rust185件（24ignored）・UI162件、production build、cargo fmt --check、リリースゲート3件成功。幅の範囲・競合・種類別の独立性・更新中拒否・ジョブ更新時保持・文書不変を確認。
- 🟢 [Done] 既存Playwright/Chromeで2パネル × 3言語 × 4画面幅の24ケース成功。実ドラッグ・取消・キーボード・初期化・リリース時の単一IPC・競合復帰・選択／未保存メモ保持・横はみ出し／consoleを確認。desktop／320pxの配置ガイドを目視確認。
- 🟢 [Done] 別IDの最新debug Tauri・実WebKit、日本語1440×960でサービスの右へのドラッグ確定、幅の実ドラッグ・キー操作・初期化、ポート左配置の独立した初期幅とキー変更を確認。空の結果を使用し、今回は能動的なネットワーク確認を開始していない。
- ⭕️ [Pending] 配布版、実Tauriの他言語／狭幅・Retina／複数画面・同時幅変更。任意グリッド・高さ調整・OSウインドウ跨ぎドラッグは未実装。[実装と範囲](evidence-docking.md)。
- 🟢 [Done] 更新処理のCI用3試験（使い捨てアプリ・ループバック）と最終git diff --checkも成功。

## 2026-10-27 — Syslog CSVと申告レベル分布

- 🟢 [Done] Rust186件（24ignored）・UI163件、production build、cargo fmt --check、更新処理3件・リリースゲート3件成功。CSVの旧操作／実行中／空拒否、外部入力の引用符・改行・数式対策、PRI分解と未検証状態を確認。
- 🟢 [Done] frontend-testing-debuggingスキルを使用。Browser skill不在（Browser plugin not available）のため既存Playwright/Chromeでlocalhost:1421、3言語 × 1500／768／375／320pxの12ケース。実SyslogReceiverへ合成記録・模擬IPCを供給し、棒クリックの絞り込みと解除、全記録CSV操作番号・ファイル名、受信中／空の無効化、失敗表示と再試行、非空・ページ識別・overlay／console・ページ横はみ出しを検証。QA /tmp/flowguard-syslog-export-qa.cjs、画像 /tmp/flowguard-syslog-export-{ja,en,zh-CN}-{1500,768,375,320}.png。
- ⭕️ [Pending] 実Tauri保存・実LAN受信・OS保存拒否。今回はソケット待受・能動的ネットワーク確認なし。画面のCSVダウンロードは模擬文字列、実際のCSV内容はRust試験で検証。[仕様](syslog-receiver.md)。

## 2026-10-27 — SyslogグラフとCSVの実Tauri確認

- 🟢 [Done] 最新debug Tauri（別ID FlowGuard Syslog QA）・実WebKit・日本語1440×960。lo0 127.0.0.1:5514にUDP受信を開始し、同一Macから5件（PRI 8、13×2、15、12）を送信。レベル分布0=1、4=1、5=2、7=1を確認し、レベル5をクリックして2／5件、再クリックして5／5件へ復帰。受信中はCSV無効、停止後に出力可能。
- 🟢 [Done] レベル5で絞り込み中に保存されたDownloads/flowguard-syslog-1.csvを実ファイルとして検査。全5行・26列・UTF-8 BOM・CRLF、操作番号・実受信元・未検証状態・受信／破棄件数・network_loss=unknownを確認。数式と引用符を含む本文は先頭アポストロフィとCSV引用で保持。受信停止済み。
- ⭕️ [Pending] 署名済み配布版・実LAN機器、OS保存拒否、実Tauri他言語／画面幅。

## 2026-10-27 — 診断概要と視覚的な絞り込み

- 🟢 [Done] UI 165試験・production build・cargo fmt --check・git diff --check。根拠状態と重要度の独立集計、未知値、重複ポート、空結果を検証。
- 🟢 [Done] Browser skill不在のためPlaywright/Chrome、localhost:1421で実DiagnosticChecksに合成Snapshotを供給。3言語 × 1500／768／375／320pxの12ケースで、カードの組み合わせ・解除、絞り込みに依存しない件数、未知値・キーボード・空状態・横はみ出し・consoleを確認。desktopと320pxを目視し、狭幅のカードとラベルの改行を修正。QA /tmp/flowguard-diagnostic-overview-qa.cjs。
- ⭕️ [Pending] 実Tauriの診断記録・署名済み配布版・他言語／複数ウインドウ。[詳細](diagnostic-overview.md)。

## 2026-10-27 — 診断概要の実Tauri確認

- 🟢 [Done] 最新debug Tauri（別ID FlowGuard Diagnostic QA）・実WebKit・日本語1440×960。近隣キャッシュを読み取り、このMac自身のen0 IPv4の51857／51858だけに一時HTTP待受を設けてTCPとHTTP HEADを確認。両方200 OKのサービス記録から診断を作成。対象2ポート・4事項、観測済み2／要確認0／未実施2、注意0／参考4と実一覧を照合。
- 🟢 [Done] 未実施で2／4、注意を追加して0／4、注意を解除して2／4、参考を追加して2／4、未実施を解除して4／4、参考も解除して全件表示を確認。絞り込み中も概要の全件数は変わらない。表示を目視し、一時HTTP待受は終了。他LAN機器への探索・ポート確認、外部名前照会、OS保護設定変更なし。
- ⭕️ [Pending] 実Tauriで注意が1件以上ある記録、他言語／狭幅・複数ウインドウ、署名済み配布版。

## 2026-10-27 — 診断の根拠と影響

- 🟢 [Done] Rust187件（24 ignored）・UI165件、production build、cargo fmt --check、更新処理3件・リリースゲート3件・git diff --check。取得元と影響の状態別コード、未実施／未検証の区別、CSV追加列・数式対策・HTMLエスケープを確認。
- 🟢 [Done] Browser skill不在のため既存Playwright/Chrome、localhost:1421の実DiagnosticChecksに合成Snapshot・模擬IPCを供給。3言語 × 1500／768／375／320pxの12ケースでTLS／未収集HTTP／旧Snapshotの選択、3ノード表示、外部文字列の非実行、横はみ出し・console・overlayを検証。desktop・320pxを目視。QA /tmp/flowguard-diagnostic-detail-qa.cjs。
- ⭕️ [Pending] 実Tauriの診断詳細・CSV／HTML保存、署名済み配布版、実証明書。[詳細](diagnostic-evidence.md)。

## 2026-10-27 — 診断詳細と出力の実Tauri確認

- 🟢 [Done] 最新debug Tauri（別ID FlowGuard Evidence QA）・実WebKit・日本語1440×960。近隣キャッシュからこのMacのen0 IPv4を選択し、同じMacの51857番に一時HTTP待受を設け、TCP確認とHTTP HEAD 200 OKから診断を作成。HTTPヘッダー未実施の詳細に「HTTPヘッダー未収集」→観測根拠→「設定の有無を判断できません」の3ノードと推奨対応が表示されることを目視・AXで確認。
- 🟢 [Done] 実アプリからCSV／HTMLをDownloadsへ保存して解析。双方2行・16列、全セル値が一致。サービス行のservice_observation／service_scope_review、ヘッダー行のheaders_not_collected／headers_unknown、日本語HTMLの追加列見出しを確認。一時HTTP待受は終了。他LAN機器への能動探索・ポート確認、外部名前照会、OS保護設定変更なし。
- ⭕️ [Pending] 実TauriのTLS検証失敗／証明書メタデータ、他言語・狭幅、署名済み配布版。

## 2026-10-27 — 診断履歴の比較条件

- 🟢 [Done] Rust188件（24 ignored）・UI165件、production build、cargo fmt --check、リリースゲート3件。対象・送信元・マスク・インターフェース・アドレス種別・世代・ポート・方式・HTTPS名の変更、不完全な記録、未確認／未実施の比較不確定、観測済みの一致、比較元付きCSV／HTML見出しを確認。
- 🟢 [Done] Browser skill不在のためPlaywright/Chromeと合成Snapshot／模擬IPCで3言語 × 1500／768／375／320pxの比較元表示・フィルター・空状態・横はみ出し／consoleを確認。320pxを目視。QA /tmp/flowguard-diagnostic-comparison-qa.cjs。
- ⭕️ [Pending] 実Tauriの再診断と条件変更・出力、署名済み配布版・実ネットワーク変更。[詳細](diagnostic-comparison.md)。

## 2026-10-27 — 診断比較の実Tauri確認（一部）

- 🟢 [Done] 最新debug Tauri・実WebKit・日本語。近隣キャッシュからこのMac自身のen0 IPv4を選び、自己宛て51857番の一時HTTPサービスでTCP確認、HTTP HEAD 200 OK、診断 #1 を作成。同条件でHTTPを再確認し診断 #2 を作成。比較元 #1、サービス応答「変化なし」、未実施のHTTPヘッダー「比較不確定」を確認。待受は停止。他のLAN機器への探索・確認は実施せず。
- ⭕️ [Pending] 出力操作の際にComputer UseがnoWindowsAvailableを返し、再取得もtimeoutReached。比較元を含むCSV／HTMLの実ファイルと条件変更後の画面は確認できなかった。原因は未確定。

## 2026-10-27 — 診断比較の実Tauri再確認

- 🟢 [Done] 別ID FlowGuard Compare QA・実WebKit・日本語。前回セッションの診断 #2（同条件HTTP再確認、比較元 #1）をCSV／HTMLで保存。各17列・2行、全セル一致。サービス応答はunchanged、未実施ヘッダーはuncertain、両行baseline_report=1を確認。
- 🟢 [Done] このMac自身のen0 IPv4・51857番に一時HTTP待受を設け、HTTPS方式へ変更してサービス確認。TLS接続失敗後の診断 #3 は4件すべて比較なし、比較元の表示なし。CSV／HTML各17列・4行、全セル一致・baseline_report空欄。待受は終了。他LAN機器への検査・OS保護設定変更は行っていない。
- ⭕️ [Pending] 署名済み配布版、実TLSサーバー・実ネットワーク切替、他言語／狭幅。

## 2026-10-27 — 診断の対応状況カード

- 🟢 [Done] UI166件・production build・cargo fmt --check・git diff --check。対応状況の全件集計と未知値・空結果を検証。
- 🟢 [Done] Browser skill不在につきPlaywright/Chrome、localhost:1421で実DiagnosticChecksに合成Snapshot／模擬IPC。3言語 × 1500／768／375／320pxの12ケースで各件数、絞り込み・解除、根拠状態との組み合わせ、Snapshot更新後の件数と一覧、空状態・横はみ出し・overlay／consoleを確認。desktopと320pxを目視。QA /tmp/flowguard-diagnostic-disposition-qa.cjs。
- ⭕️ [Pending] 複数ウインドウ同時更新・署名済み配布版。[詳細](diagnostic-dispositions.md)。

## 2026-10-27 — 診断対応状況の実Tauri確認

- 🟢 [Done] 別IDの最新debug Tauri・実WebKit・日本語1440×960。このMac自身のen0 IPv4・TCP 51857の一時HTTPサービス（200 OK）から2事項を診断。サービス応答を「再確認待ち」、理由を「QA 確認」として記録し、未対応2→1、再確認待ち0→1、一覧の状態更新を確認。
- 🟢 [Done] 再確認待ちと未対応の各カードでそれぞれ該当1／2件、再クリックで2／2件、再確認待ちと未実施の併用で0／2件を確認。フィルター中もカード件数は全体のまま。サービス画面へ移動して戻った後も記録と件数を確認。一時HTTP待受は終了。
- ⭕️ [Pending] 実Tauriの複数ウインドウ同時更新・他言語／狭幅、署名済み配布版。対応状況はセッション内の記録であり、再起動後の永続化は対象外。

## 2026-10-27 — 診断差分カード

- 🟢 [Done] UI167件、production build。変化あり・新規・変化なし・比較不確定・比較なし・未知を別々に集計し、空結果も確認。
- 🟢 [Done] Browser plugin不在のためPlaywright／Chromeと合成Snapshotで、日本語・英語・简体中文 × 1500／768／375／320pxのカード表示、差分と対応状況の複合絞り込み・解除、横はみ出しとconsoleを確認。画像は `/tmp/flowguard-diagnostic-change-{ja,en,zh-CN}-{1500,768,375,320}.png`。
- ⭕️ [Pending] 複数ウインドウ・署名済み配布版。

## 2026-10-27 — 診断差分カードの実Tauri確認

- 🟢 [Done] 別IDの最新debug Tauri・実WebKit・日本語1440×960。このMacのen0 IPv4・TCP 51857の一時HTTPサービスから診断を2回作成。初回の比較なし2件、同条件再診断の変化なし1件・比較不確定1件を一覧と照合。
- 🟢 [Done] サービス応答を再確認待ちとして記録し、変化なし＋再確認待ち＋観測済みで該当1／2件、根拠を未実施へ変更すると0／2件。比較不確定＋未対応＋未実施でHTTPヘッダー1／2件。解除で2／2件に戻り、絞り込み中もカード件数は全体を維持。一時HTTP待受は終了。
- ⭕️ [Pending] 変化あり・新規が1件以上の実記録、他言語／狭幅、複数ウインドウ、署名済み配布版。

## 2026-10-27 — WHOIS検索

- 🟢 [Done] IANAのIPv4／IPv6／DNSブートストラップで登録機関を選び、HTTPS RDAPの登録データをRust共有状態に保持。入力検証・最長プレフィックスのRustテスト2件、UI167件、production build。取得は手動開始、応答サイズ・時間に上限、リダイレクトは追跡しない。
- 🟢 [Done] Browser plugin不在のためPlaywright／Chromeで合成結果を使い、日本語・英語・简体中文 × 1500／375／320pxの画面・入力・横はみ出しとconsoleを確認。実debug Tauriで142.250.21.139のARIN応答（登録範囲・Google LLC）とexample.comのVerisign応答を確認。
- ⭕️ [Pending] IPv6・他登録機関・他言語実Tauri・署名済み配布版。登録情報は位置情報や本人性の証明ではない。地図・従来WHOIS・リダイレクト追跡は未実装。

## 2026-10-27 — HTTP応答ヘッダーの診断

- 🟢 [Done] HTTP／HTTPSの完全な最終HEAD応答から4つのセキュリティヘッダーの非空値の有無を保存。値とCookieは保持せず、未取得・不正・暫定応答は未実施。Rust192件（24 ignored）、UI167件、production build。診断出力と不正・不完全な応答、秘匿値を保存しないことを試験。
- 🟢 [Done] Browser plugin不在のためPlaywright／Chromeと合成Snapshotで3言語 × 1500／768／375／320pxの根拠ノードと検出／未検出表示、選択操作、横はみ出し・consoleを確認。
- ⭕️ [Pending] 実TauriのHTTP／HTTPS応答と実証明書・他言語／狭幅、署名済み配布版。値の妥当性、HTTPS誘導と他ページは未確認。

## 2026-10-27 — HTTPヘッダー診断の実Tauri確認

- 🟢 [Done] 別IDの最新debug Tauri・実WebKit・日本語1440×960。このMac自身のen0 IPv4・TCP 51857で、CSPとX-Content-Type-Optionsを含み、Referrer-PolicyとHSTSを含まない一時HTTP応答を用意。TCPとHTTPの確認から診断を作り、HTTPヘッダーの詳細で前者2項目が検出、後者2項目が未検出と表示されることを目視とアクセシビリティ情報で確認。取得元「HTTP応答ヘッダー」、値・設定の有効性は未確認との説明を確認。
- 🟢 [Done] テスト用Cookieが画面上に表示されないことを確認。一時HTTP待受は終了。保持しないこと自体はRust試験で検証済み。
- ⭕️ [Pending] 実HTTPS応答・実証明書、他言語／狭幅の実Tauri、署名済み配布版。値の有効性・HTTPS誘導・他ページは検査対象外。

## 2026-10-27 — HTTPからの誘導診断

- 🟢 [Done] 既存のHTTP HEAD応答の応答コードとLocationの方式を分類し、HTTP→誘導先のノードと限界を表示。リダイレクトは追跡せず、URLやクエリは保存しない。複数Location、不正な参照、その他の3xxを判定不能とし、不完全な応答は未実施。Rust193件（24 ignored）・UI167件・production build。
- 🟢 [Done] Browser plugin不在のためPlaywright／Chromeと合成Snapshotで3言語 × 1500／768／375／320pxのHTTPS・判定不能・未実施の表示、はみ出し・consoleを確認。
- ⭕️ [Pending] 実Tauriの誘導応答・他言語／狭幅、署名済み配布版。HTTPS接続と証明書・他ページの動作は本機能の対象外。

## 2026-10-27 — HTTPS誘導診断の実Tauri確認

- 🟢 [Done] 別IDの最新debug Tauri・実WebKit・日本語1440×960。このMac自身のen0 IPv4・TCP 51857に、HTTPS Locationとテスト用クエリを持つHTTP 301応答を用意。TCP確認→HTTP HEAD確認→診断の詳細で「HTTP 301 → HTTPSへ誘導」、観測済み、接続先とTLSの安全性は未確認との説明を確認。
- 🟢 [Done] テスト用URLとクエリが診断画面に表示されないことを確認。URLを共有状態に保存しないことはRust試験で検証済み。一時HTTP待受は終了。
- ⭕️ [Pending] 実Tauriでの他のHTTP応答・他言語／狭幅・複数ウインドウ、署名済み配布版。実際のHTTPS接続と証明書・他ページの動作は対象外。

## 2026-10-27 — サービスHTTP観測の差分

- 🟢 [Done] 前回と今回のセキュリティヘッダー有無・HTTP誘導方式を比較対象に追加。未取得や判定不能を「不確定」とし、詳細で前後を整列表示。CSVは末尾に現在／前回のHTTP観測JSONを追加して47列とし、生のヘッダー値とURLは含めない。
- 🟢 [Done] Rust194件（24 ignored）・UI167件、production build。差分・未取得・判定不能・復帰時の比較とCSVを確認。Browser plugin不在のためPlaywright／Chromeと合成Snapshotで3言語 × 1500／768／375／320pxの比較表と未取得を確認。
- ⭕️ [Pending] 実Tauriでの再確認・CSV保存・HTTPS、複数ウインドウ、署名済み配布版。

## 2026-10-27 — サービスHTTP差分の実Tauri確認

- 🟢 [Done] 別IDの最新debug Tauri・日本語1440×960。このMacのen0 IPv4・TCP 51857の一時HTTPサービスで、同じ301応答のCSP未検出→検出・HTTP→HTTPS誘導に切り替え、同条件で再確認。サービス操作#3は基準#2との差分「HTTPヘッダー・HTTP誘導」を表示し、詳細の前回／今回の各値が応答と一致。
- 🟢 [Done] その後、不正なヘッダーを返して再確認。操作#4は基準#3に対し比較不確定1件、前回の値と今回の未確認を表示。出力した各CSVは1行47列で、#3はchangedと両方のHTTP観測、#4はuncertain・今回空欄・前回の観測を確認。テスト用URLとクエリは両ファイルに含まれない。出力先は `~/Downloads/flowguard-services-3.csv` と `~/Downloads/flowguard-services-4.csv`。一時HTTP待受は終了。
- ⭕️ [Pending] 実HTTPS、他言語／狭幅の実Tauri、複数ウインドウ、署名済み配布版。

## 2026-10-27 — 診断履歴の時系列表示

- 🟢 [Done] セッション内の診断を古い順のノードで表示し、選択中・最新・比較元を区別。ノードの操作は既存のRust共有状態の診断選択を利用し、比較の条件や履歴の保存期間を変更しない。
- 🟢 [Done] UI167件・production build。Browser plugin不在のためPlaywright／Chromeと合成Snapshotで3言語 × 1500／768／375／320pxのノード順・比較元・クリック／キーボード選択・選択ノードの可視化・長いIPv6・横はみ出しとconsoleを確認。
- ⭕️ [Pending] 実Tauriでの複数履歴切替、複数ウインドウ、署名済み配布版。

## 2026-10-27 — 診断履歴ノードの実Tauri確認

- 🟢 [Done] 別IDの最新debug Tauri・実WebKit・日本語1440×960。このMac自身のen0 IPv4・TCP 51857の一時HTTPサービスから記録を取得し、同じ記録を3回診断。ノードが#1→#2→#3の順に並び、#3が最新として表示されることを目視・アクセシビリティ情報で確認。
- 🟢 [Done] ノードで#3→#2→#1→#3と切り替え、#3の比較元#2、#2の比較元#1、#1に比較元なし、選択欄と概要が同期することを確認。一時HTTP待受は終了。
- ⭕️ [Pending] 複数ウインドウ・署名済み配布版。実Tauriでのキーボード切替は操作ツールの状態変化で未確認（ブラウザの合成Snapshotでは確認済み）。

## 2026-10-27 — nosniff値の診断

- 🟢 [Done] 単一の `nosniff`・ヘッダーなし・空／別の値・複数値を分類。複数値は判定不能、不完全応答は未実施。生の値とCookieを保存せず、診断と前回／今回の差分へ分類コードを反映。Rust195件（24 ignored）・UI167件・production build。
- 🟢 [Done] Browser plugin不在のためPlaywright／Chromeと合成Snapshotで3言語 × 1500／768／375／320pxの診断詳細・サービス差分・未確認の表示、横はみ出しとconsoleを確認。
- ⭕️ [Pending] 実TauriでのHTTP／HTTPS応答・CSV・複数ウインドウ・署名済み配布版。Content-Type、ブラウザ動作と他ページの評価は未実装。
