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
| System Extension activation and OS enforcement | Pending; not installed or connected |

Browser file-chooser automation was restricted by the extension's file-access setting. This was resolved for the app workflow using the native file picker; browser permissions were not changed. Test screenshots were saved outside the source tree. The generated ImageGen concept lives under `docs/design/concept.png`; it was used for the layout and palette, while unsaved routes correctly remain neutral rather than implying active protection.

The GPU smoke check requires access to Metal; the sandbox did not expose an adapter. The same check passed with host GPU access. Unit tests and compilation do not substitute for signed Network Extension enforcement tests.
