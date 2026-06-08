# Changelog

All notable changes to this project are documented here. Format loosely follows
[Keep a Changelog](https://keepachangelog.com/); this crate is a pre-1.0 reference
boundary and versions are not yet semver-stable.

## [0.0.0] — 2026-06-04

The v0 reference boundary. **Frozen** — no further crate slices without a consumer trigger.

### Added
- Conserved core (`src/lib.rs`): `deposit`, `request_capacity`, `consume`,
  `inspect_token`, `revoke`, backed by finite per-scope stock and opaque,
  no-public-constructor token/receipt handles.
- Append-only ledger of bodied receipts, including the custodial `Deposited` event.
- Read-only `witness::testify_no_double_spend` — testifies from the ledger, cannot spend.
- `tests/v0_boundary.rs` — boundary properties (eligibility ≠ capacity, exactly-once,
  receipt-is-not-token, freshness, scope, revocation, idempotent replay).
- `tests/file_write_workload.rs` — **WL-001**: a real `std::fs::write` gated by the
  accountant. Result: zero API change needed; the execution boundary is pure consumer
  code.
- `tests/contention_workload.rs` — **WL-002**: concurrent consumers race one token
  through a serialization point. Result: exactly one effect crosses; the consume
  mechanism is correct under real contention.

### Notes
- In-memory, single-writer, non-production. No persistence, no distribution.
- Repository brought up to the standard layout (docs lifecycle under `docs/`,
  license/notice/provenance, CI) on 2026-06-04.
