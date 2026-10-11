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
