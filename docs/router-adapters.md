# Router adapter contract

Updated: 2026-10-16. This is the implemented Rust foundation for later device-specific adapters. No router transport, discovery, management UI, credentials store or control dispatcher is enabled by this increment.

## Ownership and authority

`routers::RouterAdapter` exposes only collection. It receives native source configuration and a budget and returns bounded wire bytes plus transport peer evidence. `RouterControlAdapter` is a distinct trait; read collection never receives a control action or control authorization. A control authorization is opaque, non-serialized, bound to the exact source configuration and action, and checked against a monotonic expiry and authenticated peer. No production API issues an authorization yet. The future user-review service must issue it explicitly; read availability never grants write authority.

`SharedState` owns the source registrations, read operations, cancellation flags, accepted results and snapshot projection. `collect_router` runs adapter I/O outside the state mutex so stop, removal, replacement and unrelated UI operations remain possible. Returned batches commit under the lock only if the source and operation are still current. Cancellation, removal or replacement fences late results. Stale completion does not advance the shared revision. Registration and read start are refused during update installation; collection also blocks starting an installation. Configuration/results are session-only and do not modify the saved document or OS settings.

The module is a native Rust API; no Tauri command accepts peer authentication claims or raw router batches from a WebView. Later transport implementations and Router Management UI must use this boundary. Constructors that attest a peer are for trusted transport code only and are **not cryptographic verification by themselves**. Adapter implementations must obtain the actual socket peer and validate the transport's authenticated identity before attesting it; they must not deserialize or copy such claims from the payload.

## Read budget and errors

At most eight sources, 128 retained rows per source, and 1 MiB of wire bytes per result. With the existing Mac cap, a live security snapshot contains at most 2,048 flow records. There are at most two in-flight native read workers across all sources, including workers that ignore cancellation. A read deadline must be positive and at most 30 seconds. The coordinator returns on deadline/cancellation without waiting indefinitely for a noncooperative adapter. Its worker slot remains occupied until that adapter exits, so repeated timeouts cannot create unlimited detached workers. In-process code cannot forcibly kill an arbitrary adapter thread; a stuck adapter can leave up to two slots unavailable until process exit.

Adapters must bound their network reads before allocating payload buffers, apply the remaining budget to every connect/read/write, and cooperate with `Budget::check`. The coordinator enforces the returned payload cap and post-return deadline; it cannot undo network I/O performed by a faulty native implementation. This is an API separation, not a sandbox for arbitrary third-party native code.

Errors are fixed `code` and `retryable` values: invalid configuration, busy, timeout, cancelled, authentication, source mismatch, invalid payload, limit, replay, stale, unavailable and internal. No upstream error text, credentials, certificate pins or raw payload is serialized into error/snapshot messages. Failed or cancelled sources do not project old flows as current. A successful result ages out after 300 seconds; old source metadata remains marked stale. No data is manufactured for missing counters or failed reads.

## Source and transport verification

A source has a bounded ASCII ID, exact IP endpoint including port/scope, collection position (`lan_before_nat`, `wan_after_nat`, `unknown`) and either anonymous or pinned-SHA256 authentication policy. Pins are 64 lowercase hex characters and are configuration, never payload data. Native peer endpoint must equal the configured endpoint. Pinned policy requires a matching identity attested by the transport adapter. Anonymous collection is always labeled `unverified`, even if the payload claims authentication or the native output happens to carry a pin.

The wire batch source ID must match the registration as well. A matching IP alone does not authenticate a device; anonymous observations cannot participate in cross-source correlation. A validated adapter response can establish current flow-export availability; it cannot establish firewall-write, quarantine or backup capability. Those remain unknown.

Credential acquisition, TLS/API/SNMP authentication and OS credential storage remain separate device-adapter work. Secrets must stay inside native credential handles and out of the document, WebView, snapshots and logs.

## Wire schema 1

The strict JSON schema requires `schema: 1`, `sourceId`, `sequence`, `capturedAt` and `records`. A record contains exactly:

- `id`: bounded ASCII source-local identifier.
- `localEndpoint` / `remoteEndpoint`: literal IP socket addresses with nonzero ports, same address family. DNS names, unspecified and multicast addresses are not accepted by this initial unicast-flow format. IPv6 scope identifiers are preserved.
- `protocol`: `TCP` or `UDP`; `firstObservedAt`: positive Unix seconds no later than the batch capture.
- `bytesSent` / `bytesReceived`: optional nonnegative integer endpoint counters; missing is null, observed zero stays zero. These are reported counters, not independently validated traffic or rates.

Unknown fields, including claimed process IDs, app keys, authentication or capabilities, are rejected. This initial format uses one orientation: `localEndpoint` is the adapter's reported endpoint for the sent/received counters, **not necessarily this Mac**. Direction remains unknown; no PID or application identity is inferred. Values exposed to JavaScript are bounded to safe integers. Capture time must be within the last 300 seconds and at most five seconds into the future. Sequence must strictly increase and capture time must not move backward within a source registration. Reboot/sequence reset requires explicit re-registration; it is not silently accepted as fresh data.

Batch validation is atomic. Identical IDs with identical records deduplicate within a batch; conflicting duplicates reject the entire batch. A new valid batch replaces that source's previous batch. Source-prefixed IDs keep separate sources distinct; source updates do not merge with local Mac history. This format does not represent multicast packets, encrypted tunnel inner flows or arbitrary syslog lines.

## Cross-source correlation

Only authenticated `lan_before_nat` records can produce a candidate link to a real Mac socket record. Canonical IP/port pairs, endpoint orientation and protocol must match exactly, samples must be within two seconds, and observed intervals must overlap. Unknown/unparseable endpoints, loopback addresses and unhandled scope representations do not produce a link. WAN/after-NAT, unknown position, anonymous sources and demo records are excluded. At most 512 candidate links are retained.

Links are labeled `same_tuple_near_sample_candidate_only`. They do not prove process ownership, device identity or a single underlying flow. Records, timestamps and reported counters remain separate. There is no cross-source sum, deduplication that deletes either observation, application attribution or security decision from a candidate link. Future NAT-aware correlation requires independent mapping evidence and a separate specification.

## Validation and remaining work

Synthetic adapters exercise the actual coordinator and shared-state path; no shipped fixture is connected to the app. Tests cover bad source/pin/authentication claims, unknown schema/fields, bounds, malformed addresses/counters, duplicate conflict, replay, expiry, cancellation and late commits, worker saturation, exact expiring control authorization, conservative correlation and demo isolation. The default source list is empty; startup makes no router request.

⭕️ [Pending] Real managed-router TLS/API/SNMP authentication, actual sender metadata, LAN/WAN collection positions and counters, firmware/reboot/sleep behavior and native UI integration. Real router mutation is neither implemented nor tested. Device-specific collection, Router Management UI and reviewed control authorization issuance remain planned; this increment completes the common adapter/data-validation foundation only.
