# Linear Accountant — Handoff Packets

*Companion to [ROLE.md](ROLE.md). Defines how existing constellation tools would interact with a Linear Accountant **if/when** a consumer trigger fires.*

## Status / discipline

- **Handoff-shape audit, not a defect audit.** We map what each tool would send, receive, refuse, and testify about — not whether it currently has bugs.
- Candidate. Not ratified. No implementation, no daemon, no repo split, no Rust planning here.
- Posture correction (see [ROLE.md](ROLE.md) §11 log): we are no longer waiting passively for a consumer. A role other systems must not accidentally re-implement needs a **reference boundary** before consumers can target it. These packets are the contract that boundary must satisfy. The socket is not yet a power plant.
- If a section tempts a defect audit: **handoff shape only; defect audit requires a consumer trigger.**

## Canonical wire shapes (the field dictionary)

Defined once; each tool below references which it touches. Every identifier minted by the accountant is **opaque** — no other tool may construct one.

| Field | Meaning | Lives in |
|---|---|---|
| `actor` | who is requesting (identity) | request |
| `action` | the effectful verb being gated (e.g. `restart_service`) | request, consume |
| `target` | object the action affects | request |
| `scope` | bounding axes the capacity is valid within | request, grant |
| `requested_capacity` | units asked for | request |
| `basis` / `eligibility_reference` | **pointer** to the validity fact justifying the request — contractible, NOT payment | request |
| `nonce` / `request_id` | idempotency for the *request* (dedupes retries) | request |
| `expiry` | caller-supplied validity window of the grant (no ambient `now`) | grant |
| `token_id` / `lease_id` | the minted spendability handle — accountant-issued only | grant, consume |
| `consumption_event_id` | idempotency key for a *consume* — the exactly-once guard | consume |
| `receipt_reference` | pointer to a consumption receipt — **evidence, never capacity** | receipt, testimony |
| `witness_reference` | pointer to NQ testimony about a consumption | testimony |
| `denial_reason` | why a request/consume was refused | denial |

**Packets** (fields only):

- **CapacityRequest** → `{actor, action, target, scope, requested_capacity, basis, request_id}`
- **Grant::Granted** → `{request_id, token_id, scope, granted_capacity, expiry}`
- **Grant::Denied** → `{request_id, denial_reason}`
- **ConsumeRequest** → `{token_id, action, amount, consumption_event_id}`
- **Consumption** → `Consumed{receipt_reference, remaining_after, consumption_event_id}` | `AlreadyConsumed` | `InsufficientCapacity` | `Expired` | `UnknownToken`
- **Receipt** → `{receipt_reference, token_id, consumption_event_id, amount, remaining_after}` — evidence object; type-distinct from any token.
- **TestimonyQuery** → `{token_id}` → **Testimony** `{witness_reference, subject(token_id), assertion}` where `assertion ∈ {no_double_spend, double_spend, lease_reuse, quota_overrun, missing_consumption}`

## Shared failure/refusal taxonomy

Every tool that touches the boundary inherits the relevant subset:

1. **eligible but no capacity** → `Denied(insufficient_stock)`. Validity passed; stock said no.
2. **capacity granted but token expired** → `Expired`. The grant's `expiry` lapsed before consume.
3. **token already consumed** → `AlreadyConsumed` (event replay) or `InsufficientCapacity` (stock exhausted).
4. **receipt presented as token** → `UnknownToken`. `receipt_reference` is type-distinct from `token_id`; it is not in the token namespace.
5. **witness/testimony presented as allocation** → rejected. The accountant does not accept a `witness_reference` as a token.
6. **replayed receiver acceptance** → no-op via `request_id`/`consumption_event_id` idempotency. A second acceptance mints/consumes nothing.
7. **stale eligibility reference** → `Denied(stale_basis)`. Eligibility is re-checkable, but its age is not a stock of capacity.
8. **accountant unavailable** → caller **fails closed**: no effect. It must NOT proceed on prior eligibility.

---

## Per-tool handoff packets

### Agent Governor

1. **Role:** semantic governor / enforcement coordinator. *(May coordinate validity and spendability — must not collapse them onto one mutable substrate.)*
2. **May send:** `CapacityRequest` (override-uses, scope-grant budgets, dispatcher leases, per-tool caps, quorum/Neff slots); lease-renewal as a *fresh* `CapacityRequest`; `TestimonyQuery`.
3. **May receive:** `Granted`, `Denied`, `Expired`, `AlreadyConsumed`, `Testimony`.
4. **Must never:** mint capacity from a policy ALLOW (a verdict is not a budget); regenerate a token from re-validation or summarized agent context; mutate accountant state directly. The validity decision and the spend decision must not be **convertible** through one mutable surface.
5. **Packet:** sends `CapacityRequest{... basis = its own audit/standing verdict id ...}`; receives `Grant`. The `basis` is a *reference* to AG's eligibility finding, never the capacity itself.
6. **Failures:** (1) eligible-but-no-capacity is AG's most common case — it validated, the accountant declines; (7) stale basis → re-audit; (8) accountant down → AG must DENY the downstream action.
7. **Integration trigger:** a concrete AG spend surface where a validity decision is genuinely **convertible** into a finite count with a real double-spend/over-grant risk (e.g. exactly-once override-uses or dispatcher leases). The 2026-06-03 audit found AG surfaces *co-located* but **not** convertible — so the trigger is "a convertible spend path appears," not "co-location exists."

### Wicket

1. **Role:** admission gate / receiver policy kernel. *(Admission ≠ consumption.)*
2. **May send:** typically **no direct calls** — Wicket admits, it does not spend. May *require* that an action-bearing claim carry a live `token_id` reference and validate it read-only; it does not mint.
3. **May receive:** read-only token-liveness confirmation, or **no direct response**. It receives token *references* inside claims, not granted capacity for itself.
4. **Must never:** treat admission as spendable authority; treat receiver acceptance as replayable mutation authority; mint capacity; let an admitted envelope be replayed into two effects (the one-shot guard is the accountant's `consumption_event_id`, not admission).
5. **Packet:** reads `token_id` *references* embedded in claims; enforces `nonce`/`request_id` at admission for replay detection. Populates no `CapacityRequest` for itself.
6. **Failures:** (6) replayed receiver acceptance → no-op; (4) `receipt_reference` in a claim must not be treated as spend authority; (8) accountant down → fail-closed admission for token-requiring effects.
7. **Integration trigger:** one-shot effect-token semantics — a concrete effect where admitting the same envelope twice causes two effects and content-addressing alone won't stop it. *(Role note §11 trigger #2.)*

### WLP

1. **Role:** wire/claim transport — envelope/claim shape. *(The envelope is not money.)*
2. **May send:** nothing executable — it is the medium. It **carries** requests, grants, consume requests, receipts, and references between parties.
3. **May receive:** nothing for itself; carries `Denied`/`Granted`/`Receipt`/`Testimony` as opaque payloads.
4. **Must never:** embed spendable value (carry token *references*, never the stock); let envelope-validity be read as capacity; let `acted: true` be read as authorization (the deferred `open-issues.md` split — validation_result ≠ action_attestation).
5. **Packet:** defines **type-distinct slots** for `token_reference`, `receipt_reference`, `witness_reference` — opaque, non-interchangeable. Carries `request_id`/`nonce` for downstream dedupe. `causal_parents`/`artifact_hash` remain lineage/content-address, never capacity.
6. **Failures:** (6) transport replay → carriage of `request_id` enables downstream/accountant dedupe; (4) the type-distinct slots are what prevent a `receipt_reference` from being read where a `token_reference` is expected.
7. **Integration trigger:** a real consumer needs to *carry* token/receipt/witness references with type-distinct slots — i.e. once Wicket or AG actually emits/consumes tokens. Until then, carriage is hypothetical.

### NQ

1. **Role:** witness/testimony layer. *(Testifies; does not allocate.)*
2. **May send:** `Testimony` (answers to `TestimonyQuery`). No `CapacityRequest`.
3. **May receive:** consumption receipts and token/lease metadata **as evidence**; a `TestimonyQuery`. Receives no capacity.
4. **Must never:** allocate or enforce capacity; have its testimony treated as allocation authority; produce a "receipt" that becomes a reusable lease.
5. **Packet:** produces `Testimony{witness_reference, subject(token_id), assertion ∈ {no_double_spend, double_spend, lease_reuse, quota_overrun, missing_consumption}}`; consumes the `receipt_reference` + `consumption_event_id` stream as input evidence.
6. **Failures:** (5) testimony-as-allocation is the cardinal forbidden flow — a `no_double_spend` assertion must never be turned into a fresh token; missing-consumption → NQ testifies *absence*, never fabricates; (8) accountant down → NQ can still testify about the historical event stream it observed, but cannot assert current capacity.
7. **Integration trigger:** NQ needs a testimony schema for double-spend / lease-reuse / quota-overrun. The audit confirmed this capability is **absent today**. Fires when a real accountant emits a consumption-event stream NQ can observe. *(Role note §11 trigger #3.)*

### Nightshift

1. **Role:** temporal scheduler / operator (proceed / defer / revalidate). *(Does not own capacity.)*
2. **May send:** `TestimonyQuery`; read-only state observation. It may *trigger* the rightful actor (AG/executor) to request capacity, but issues no mint/consume calls itself.
3. **May receive:** read-only accountant snapshots `{token_id, remaining, expiry}`; `Testimony`. No granted tokens for itself.
4. **Must never:** own or renew a lease (renewal is a fresh `CapacityRequest` by the rightful actor); let passage of time regenerate capacity; treat a "proceed" decision as a grant; decrement a token. *(Audit confirmed Nightshift clean here — it observes, it doesn't allocate.)*
5. **Packet:** consumes read-only views and `Testimony`; emits proceed/defer/revalidate decisions that *reference* (never consume) a `token_id`. Carries no `consumption_event_id`.
6. **Failures:** (2) token expired by wake time → defer/revalidate, never extend; (7) stale basis at wake → revalidate, don't proceed; (8) accountant down → defer.
7. **Integration trigger:** scheduled-effect work where an action must check live capacity before proceeding — blast-radius budgets, rollout slots, maintenance windows. *(Role note §11 trigger #4.)*

### Execution layer / generic executor

1. **Role:** execution / token consumer. *(Consumes atomically, emits receipts.)*
2. **May send:** `ConsumeRequest{token_id, action, amount, consumption_event_id}` — exactly-once, around the effect.
3. **May receive:** `Consumed(receipt)`, `AlreadyConsumed`, `InsufficientCapacity`, `Expired`, `UnknownToken`.
4. **Must never:** self-issue capacity; proceed with the effect absent a `Consumed`; reuse stale validation as new spendability on retry; re-feed its own `receipt_reference` as a token.
5. **Packet:** sends `ConsumeRequest` with a unique `consumption_event_id` per effect attempt; on `Consumed`, performs the effect and retains `receipt_reference` for NQ.
6. **Failures:** **all of them land here** — `InsufficientCapacity`/`Expired`/`UnknownToken` → abort; `AlreadyConsumed` → the effect already happened or is in flight, **do not double-execute** (this is the exactly-once guard); (8) accountant down → fail-closed.
7. **Integration trigger:** the first real one-shot effect that must not double-fire. The reference harness's `restart_service` scenario is the toy version of exactly this.

---

## Cross-cutting note (prior-art, anticipatory)

The consume↔effect boundary carries a known failure class (well-charted in distributed-systems prior art, so it counts as evidence before it bites locally): a crash between "consume succeeded" and "effect happened" cannot be made truly exactly-once by the accountant alone. The achievable shape is **exactly-once *consumption* at the accountant + an effect that is idempotent on `consumption_event_id`** — i.e. at-least-once delivery made safe by an idempotency key. Flagged here so that when the executor trigger fires, this is designed in, not rediscovered. No build now — naming the rake, not stepping on it.

## Sequence

```
role note  →  handoff packets (this)  →  reference boundary / toy harness  →  real integrations
```

Each arrow waits for the prior to exist. No arrow authorizes the next beyond its own scope.
