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
