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
