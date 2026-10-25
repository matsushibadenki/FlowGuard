# Software updates and GitHub distribution

FlowGuard checks the stable GitHub Releases feed 30 seconds after startup and every six hours. Automatic checking/downloading is enabled by default and can be disabled in Settings. Downloaded updates are prepared automatically; installation and restart require the in-app button. Development builds make no automatic requests and cannot install an update. A manual check/download is available for verification.

Rust owns the job, selected plugin Update and downloaded bytes. Only status/progress/version/release notes cross IPC. Installation waits for LAN/TCP/service jobs and extension operations to finish, and freezes document-changing commands during replacement. App data and rules remain outside the app bundle. A sibling `.FlowGuard-update-backup-<pid>` is retained until the new bundle passes codesign, matching Team ID and Gatekeeper checks; failures restore it. If automatic recovery fails, retain that backup and restore the app from it before retrying. Writability of the app's parent directory is required for preparing the backup.

## Verification policy

- HTTPS metadata endpoint: `https://github.com/matsushibadenki/FlowGuard/releases/latest/download/latest.json`.
- Stable versions only, strictly newer than the installed version. Downloads must start at this repository's GitHub release path and remain on allowlisted HTTPS GitHub asset hosts.
- Maximum compressed download 200MiB; network timeout five minutes. Progress is throttled and late/cancelled results cannot become installable.
- Minisign verification uses the app's embedded public key. The trusted signature comment must bind exactly the manifest version (`version:<semver>`).
- The tar.gz must contain `FlowGuard.app`, normal files/directories only, bounded file count/uncompressed size, the expected bundle ID, executable and version. Symlinks/hardlinks are rejected by this first policy.
- After replacement, macOS code signing and Gatekeeper are checked before restart. Development builds never replace the running app.

The local `.release-secrets/updater.key` is the long-lived updater signing key; its directory/file permissions are 0700/0600 and Git ignores the entire directory. Keep a secure backup. Only the public key in `tauri.conf.json` is published. Do not regenerate it after distribution without a key migration plan. Apple signing/notarization credentials remain in Keychain and are separate from this updater key. No credential is passed to the WebView, logs, release JSON or GitHub.

## Preparing a release

`python3 scripts/release.py --identity <Developer-ID-certificate-SHA1> --team-id <TEAMID> --notary-profile flowguard --notes <release-notes-file> --host-profile <Developer-ID-host-profile> --extension-profile <Developer-ID-extension-profile>`

The preparation script requires Python 3.11 or newer. --resume-prepared-app resumes signing an unfinished staged bundle without rebuilding; it refuses completed assets. App ID and Team ID are included explicitly in generated signing entitlements.

All versions in package.json, Cargo.toml and tauri.conf.json must agree. The default command builds the release app, embeds the Swift bridge/extension/firewall helper, signs nested binaries with explicit IPC identifiers and hardened runtime, verifies host/extension versions and signing requirements, and prepares notarization.zip plus preparation.json locally. It stops before sending the app to Apple and does not generate a public latest.json. A positive integer macOS bundleVersion is required and copied to the extension. The sidebar uses the running application version.

After explicit approval of that specific app archive and destination, repeat the command with `--submit-prepared --expected-notarization-sha256 <approved-SHA256>`. This phase neither rebuilds nor re-signs: it checks the archive hash and the staged bundle digest against preparation.json before using notarytool. Accepted, staple/validate and Gatekeeper are required before creating the downloadable zip, updater tar.gz/signature, latest.json and checksums. Changed payloads require new review. The public updater key remains unchanged. The signature is created after notarization/stapling and explicitly binds the version. It prepares assets locally under ignored `release/<version>/<architecture>/` without publication.

The existing Developer ID certificate and `flowguard` Keychain profile have been checked. Regenerated Developer ID profiles supplied on 2026-10-14 match both bundle IDs, team and signing certificate and include the required content-filter-provider-systemextension capability; the host also includes system-extension.install. A release containing the Network Extension requires those host/extension profiles. `--observe-only` explicitly prepares a host-only release without the unprovisioned extension; this must be an intentional distribution choice, not a silent fallback. The selected initial distribution scope is the full host plus Network Extension bundle; both required profiles are now available in the ignored docs-private directory. Neither preparing nor installing an update activates the Network Extension.

## Publishing after preparation

Review the notes and verified artifacts, commit/tag the matching version, and push the tag. If publication has also been explicitly approved and the matching source tag is already remote, add `--publish` to the explicit `--submit-prepared` invocation. To publish files after submission has already completed, review and upload the five final public assets with GitHub Releases separately; do not rebuild or overwrite the prepared version. It requires the remote tag, creates a draft release, uploads only public distribution files and checksums, then publishes it. Existing versions/assets are never overwritten. The signature private key stays local; GitHub receives no signing/notarization secrets. `.github/workflows/verify.yml` runs frontend, Rust and controlled updater tests on pushes/PRs, not production publication.

The initial installer must be downloaded/installed normally; older builds without an updater cannot bootstrap themselves. A subsequent newer stable release exercises the real installed-app update route. One architecture is prepared per invocation; merge both architecture entries into one latest.json before publishing a multi-architecture release. The current script never silently replaces another architecture's feed entry. Intel/universal and activated-extension update recovery remain Pending.

Implementation follows the [Tauri 2 updater contract](https://v2.tauri.app/plugin/updater/); app-specific signature-version and archive checks are stricter than the bundled plugin's defaults.

## 0.1.1 signed candidate verification (2026-10-17)

The local 0.1.1/build 2 candidate is Developer ID signed, including the extension, bridge/runtime and firewall helper. Apple notarization submission is Pending explicit payload approval; there is no public 0.1.1 feed yet. Details and the exact archive hash are in [0.1.1-validation.md](releases/0.1.1-validation.md).

`python3 scripts/prepare-update-verification.py release/0.1.1/aarch64` creates locally signed updater test assets under the separate `verification/` directory. These are not notarized, are not public release assets, and must never be uploaded. The integration test uses a loopback server solely in test configuration; production HTTPS/host policies remain unchanged.

```sh
FLOWGUARD_RELEASE_DIRECTORY="$PWD/release/0.1.1/aarch64/verification" \
FLOWGUARD_PREVIOUS_APP="$PWD/release/0.1.0/aarch64/FlowGuard.app" \
FLOWGUARD_EXPECT_GATEKEEPER_REJECTION=1 \
cargo test --manifest-path src-tauri/Cargo.toml --lib --offline \
  updates::signed_upgrade_tests::production_key_signed_upgrade_on_disposable_app \
  -- --ignored --nocapture --test-threads=1
```

This checks the production public key, downloaded signed archive, tamper rejection, failed-replacement rollback of the real signed old app, rejection of the unnotarized new candidate and restoration. All mutations are in a newly created temporary directory; the test never launches either application. An external test document is preserved byte-for-byte, which does not establish running-app data migration or restart success.

After approved notarization produces final root assets, set `FLOWGUARD_RELEASE_DIRECTORY` to `release/0.1.1/aarch64` and omit `FLOWGUARD_EXPECT_GATEKEEPER_REJECTION` to require a successful verified installation. Running-app restart, live extension continuity and public HTTPS delivery still require separate confirmation. `python3 scripts/test_release.py` validates the exact-payload submission gate without network, keys or OS control.

## Persistent downloads, channels, key delegation and merged feeds (2026-10-25)

The current source keeps one private, locked download cache under the app cache directory (`updater-v1`, up to 200MiB). Resume requires the exact version, architecture, URL and signature plus a strong ETag. `Range`/`If-Range` and the 206 range/validator/size must match; a 200 response restarts. Weak validators never resume. A candidate is ready only after complete signature, signed-version, archive and Mach-O CPU checks. Corrupt complete data is discarded. An active worker holds the cache lock; clear cannot race it. Symlinks are rejected. No cache data is executable before verification.

Stable is the default; beta is an explicit preference with a fixed project `releases/download/beta/latest.json` source. Channel changes are refused while checking/downloading/ready/installing. The private history sidecar retains 20 bounded events. Installation requested is distinct from observing the compiled version on a subsequent launch; neither receipt nor a launch proves extension continuity. UI and errors are localized in Japanese, English and Simplified Chinese.

`src-tauri/update-trust.json` is build-pinned. Its root is the updater public key in tauri.conf.json; the default has no delegated keys. A delegation specifies ID, public key, minimum version, stable/beta channels and a root Minisign authorization over the canonical `FlowGuardUpdaterKey/v1` payload. Unknown feed keys are never trusted. Optional root expiry allows a later prepared build to retire the root. Authorization must be prepared and reviewed before a release; this task did not rotate production keys. Tests use separate public fixtures; test private keys were deleted.

`python3 scripts/merge_update_feeds.py --help` documents local merging of notarized arm64 and Intel stage directories. Versions/notes, expected repository asset URLs, signed-version markers and both architectures must agree. Default output is local only. Explicit `--publish-draft` additionally verifies full signatures, archive CPU, codesign, stapler and Gatekeeper before a new immutable version draft; `--promote-beta` separately updates the beta feed after checking the published version. Neither publishing option was executed. The Rust `verify_update_artifact` example supports independent local verification.

⭕️ [Pending] Intel signed builds, merged notarization/publication, actual HTTPS server resume behavior, production delegation/retirement and beta delivery, installed signed-app restart/data migration and extension continuity. The existing 0.1.1 candidate remains byte-for-byte unchanged and lacks this source and ABI5. A separately versioned signed build is required; do not overwrite the candidate or reuse its earlier payload approval.
