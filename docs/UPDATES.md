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

All versions in package.json, Cargo.toml and tauri.conf.json must agree. The script builds the release app, embeds the Swift bridge/extension before signing, signs nested binaries and bundles with hardened runtime, submits a temporary archive to notarytool, requires Accepted, staples and assesses the app, then creates the downloadable zip, updater tar.gz/signature, latest.json and checksums. The signature is created after notarization/stapling and explicitly binds the version. It prepares assets locally under ignored `release/<version>/<architecture>/` without publication.

The existing Developer ID certificate and `flowguard` Keychain profile have been checked. Regenerated Developer ID profiles supplied on 2026-10-14 match both bundle IDs, team and signing certificate and include the required content-filter-provider-systemextension capability; the host also includes system-extension.install. A release containing the Network Extension requires those host/extension profiles. `--observe-only` explicitly prepares a host-only release without the unprovisioned extension; this must be an intentional distribution choice, not a silent fallback. The selected initial distribution scope is the full host plus Network Extension bundle; both required profiles are now available in the ignored docs-private directory. Neither preparing nor installing an update activates the Network Extension.

## Publishing after preparation

Review the notes and verified artifacts, commit/tag the matching version, and push the tag. Run the same release command with `--publish` only when publication is intended. It requires the remote tag, creates a draft release, uploads only public distribution files and checksums, then publishes it. Existing versions/assets are never overwritten. The signature private key stays local; GitHub receives no signing/notarization secrets. `.github/workflows/verify.yml` runs frontend, Rust and controlled updater tests on pushes/PRs, not production publication.

The initial installer must be downloaded/installed normally; older builds without an updater cannot bootstrap themselves. A subsequent newer stable release exercises the real installed-app update route. One architecture is prepared per invocation; merge both architecture entries into one latest.json before publishing a multi-architecture release. The current script never silently replaces another architecture's feed entry. Intel/universal and activated-extension update recovery remain Pending.

Implementation follows the [Tauri 2 updater contract](https://v2.tauri.app/plugin/updater/); app-specific signature-version and archive checks are stricter than the bundled plugin's defaults.
