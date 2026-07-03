# `la_cli` transport protocol (v0)

`la_cli` is a **thin transport** over the Linear Accountant v0 boundary. It was
added on the first consumer trigger (the Agent Governor bootstrap-lab effect
gate, 2026-06-16) so an out-of-process consumer can reach the already-implemented
decisions. It adds **no policy**: every command maps 1:1 onto an
`InMemoryAccountant` method, and the accountant stays authoritative for
spendability. The seam stays narrow — no free-text justification enters a
decision.

## Lifetime

Session-lived. One in-memory accountant per process; no persistence (the lib's
non-production stance is preserved). A consumer spawns one `la_cli` per
supervised session and kills it at session end. State dies with the process.

## Framing

- **stdin:** one command per line, TAB-separated `key=value` pairs. The first
  pair must be `cmd=<deposit|request_capacity|consume|issue_capability>`. The first `=` in a pair
  is the separator (values may contain `=`, `/`, `:`); values may **not** contain
  a TAB or newline.
- **stdout:** one JSON object per line — a decision, an ack, or an error.
- **first stdout line:** a banner, so the consumer records LA identity at the
  boundary:
  `{"la":"linear-accountant","version":"<semver>","commit":"<sha|unknown>","protocol":"v0"}`
  (`commit` is `option_env!("LA_GIT_COMMIT")` at build time; consumers should
  also record `git -C <repo> rev-parse HEAD` for authority.)

## Fail-closed

Any malformed line, unknown `cmd`, missing/non-`u64` field, or unknown token
handle returns `{"error":"<msg>","fail_closed":true}` and **never** a decision.
A consumer must treat any line lacking a `decision` (or any `fail_closed`) as a
**refusal** — there is no silent allow.

## Commands

### `deposit` — seed finite stock for a scope (custodial)

```
cmd=deposit   scope=<str>   amount=<u64>
→ {"ok":true,"event":"deposited","scope":"<scope>","amount":<n>,"receipt":"<ReceiptId(..)>"}
```

Capacity enters the system only via `deposit`, and it is recorded in the ledger
(custody is never silent). Depositing is the consumer's bounded-allocation act;
the accountant performs and records it.

### `request_capacity` — mint a token from deposited stock

```
cmd=request_capacity  request_id=<str>  actor=<str>  action=<str>  target=<str>
  scope=<str>  requested_capacity=<u64>  eligibility_reference=<str>
  eligibility_valid_until=<u64>  expires_after=<u64>  tick=<u64>  [idempotency_key=<str>]
→ {"decision":"Granted","token_id":"t<N>","granted_capacity":<n>,"scope":"<scope>","expires_at":<tick>,"receipt":"<..>"}
→ {"decision":"Denied","denial_reason":"<str>","receipt":"<..>"}
```

`token_id` is an **opaque wire handle** (`t0`, `t1`, …) — NOT the accountant's
`TokenId` (which has no public constructor; invariant 1). The CLI holds the real
token and maps the handle back on `consume`. `eligibility_reference` is an opaque
sealed pointer — never parsed. Empty eligibility, stale warrant (`tick >=
eligibility_valid_until`), zero request, or insufficient stock → `Denied`.

### `consume` — spend one unit against a granted token

```
cmd=consume  consumption_event_id=<str>  token_id=t<N>  actor=<str>  action=<str>
  target=<str>  amount=<u64>  scope=<str>  tick=<u64>
→ {"decision":"Consumed","token_id":"t<N>","consumed_amount":<n>,"remaining_capacity":<n>,"receipt":"<..>"}
→ {"decision":"AlreadyConsumed","token_id":"t<N>","receipt":"<..>"}          # replay-kill
→ {"decision":"InsufficientCapacity","token_id":"t<N>","remaining_capacity":<n>,"requested_amount":<n>,"receipt":"<..>"}
→ {"decision":"Expired"|"Revoked"|"UnknownToken","token_id":"t<N>",...,"receipt":"<..>"}
→ {"decision":"ScopeMismatch","token_id":"t<N>","expected_scope":"<s>","requested_scope":"<s>","receipt":"<..>"}
```

`consumption_event_id` is the exactly-once key **within the token**: a replayed id on the
same token → `AlreadyConsumed`. The replay/idempotency domain is `(token_id,
consumption_event_id)`, not a global effect id — see
[event-identity](working/decisions/event-identity.md). Eligibility is contractible;
capacity is linear.

### `issue_capability` — mint a single-use SpendCapability from a granted token

```
cmd=issue_capability  token_id=t<N>  target=<str>  effect_class=<str>
  capability_id=<str>  tick=<u64>
→ {"capability_id":"<str>","token_id":"t<N>","scope":"<scope>","target":"<str>","effect_class":"<str>","eligibility_reference":"<str>","issued_at":<tick>,"expires_at":<tick>,"single_use":true}
→ {"error":"capability refused: <UnknownToken|TokenRevoked|TokenExpired|Exhausted>","fail_closed":true}
```

Issuance is **additive, not a spend**: minting a SpendCapability does **not** draw down
the token's `remaining_capacity` and does **not** reserve it — stock is unchanged by
issuance. The SpendCapability is a bounded execution *envelope* that binds the token's
opaque `eligibility_reference` verbatim; the only effect that crosses the spend boundary
is still `consume`. See
[capability-redemption](working/decisions/capability-redemption.md). Fails closed against
any token that cannot back it (unknown handle, revoked, expired, or exhausted).

## Tests

`tests/la_cli_transport.rs` drives the real binary and asserts the happy path,
replay-kill, exhaustion, scope mismatch, and the three fail-closed paths
(malformed line, unknown command, unknown token handle).

`tests/spend_capability.rs` exercises the `issue_capability` decision at the library
boundary: verbatim eligibility binding, single-use, fail-closed issuance, and that
issuance leaves stock unchanged (issuance is not spend).
