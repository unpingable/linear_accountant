# Candidate: zero-amount consume asymmetry

**Status:** `UNRATIFIED-CANDIDATE` — surfaced 2026-06-10 by an independent (codex) review
of the differential oracle. **Behavior: frozen, observed, not changed.** This note records
a real semantic asymmetry so it does not get "fixed" casually (erasing possibly-intentional
behavior) or ignored (becoming folklore). Does **not** authorize a behavior change; a
change here is a behavior bump requiring explicit ratification + migration + test updates.

## The asymmetry (observed)

- **`request_capacity` denies a zero request.** `requested_capacity == 0` → `Denied`
  ("zero capacity requested").
- **`consume` accepts a zero amount.** The only quantity guard is `req.amount > remaining`;
  `0` is never `> remaining`, so a zero-amount consume falls through to `Consumed`, inserts
  the `event_id` into `consumed_events`, and records an `Event::Consumed{ amount: 0 }`.

So:

```text
consume(event_id = X, amount = 0) → Consumed         # no resource movement…
consume(event_id = X, amount = 1) → AlreadyConsumed  # …but the event id is now burned
```

## What this is NOT

**Not a conservation bug.** `minted = available + Σ original` holds either way (a zero
consume moves nothing). The Lean proof and the differential oracle both pass; neither can
adjudicate this, because it is not an arithmetic question. See `verification/README.md`
(zero-amount consume is named there as a shared model/impl blind spot).

## The actual question — authority-effect, not math

> Can a **zero-spend** consume close over **future spend** for the same event id?

Today it can: a zero-amount consume durably burns the `event_id`, so a later real spend
under that same id is refused as `AlreadyConsumed`. That is event-finalization semantics
riding inside the consume verb. The question is whether that is a feature or a footgun.

## Two possible futures

1. **Ratify as a valid no-op / idempotency marker.**
   - Zero consume intentionally burns the event id — marks an event "handled" without spend.
   - Useful only if there is a concrete event-closure use case.
   - Would have to be documented as explicit *closure semantics*, not an accident of the
     `amount > remaining` guard.

2. **Reject as malformed consume** (symmetric with `request_capacity`).
   - `amount == 0` refuses, the way a zero request does.
   - Requires a behavior bump: a new refusal variant (or reuse), migration, test updates.
   - Removes the zero-burn "denial of future spend" weirdness.

## Bias (non-binding)

Lean toward **(2), reject eventually** — *unless* a concrete event-closure use case appears.
In a linear accountant, `consume` should mean *spend*. A call that spends nothing but burns
uniqueness is not consumption; it is event-finalization wearing a consumption hat — a small
laundering smell (a non-spend acquiring spend-like authority over an event id). That is
exactly the class of meaning-promotion the boundary exists to refuse.

But **do not act now.** Freeze the observed behavior, keep this note, and make the future
change require explicit ratification. Opens when: (a) a real consumer needs zero-amount
event closure (→ future 1), or (b) the zero-burn-denies-future-spend path is shown to bite
a consumer (→ future 2).

## Composes with

- `docs/architecture/V0_BOUNDARY.md` §2b (cash-register discipline: mechanical guards only)
  and the anti-laundering stance — this is a candidate laundering seam, not a confirmed one.
- `verification/README.md` — where the behavior is disclosed as a model/impl blind spot.
