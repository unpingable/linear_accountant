# Capability redemption — issuance is not spend

*Candidate discipline note. Not ratified. Names the boundary between minting a
`SpendCapability` and spending; authorizes no new build. Companion to
[V0_BOUNDARY.md](../../architecture/V0_BOUNDARY.md) and
[capability-composition.md](capability-composition.md). This is the **spend** axis: it is
about the crossing effect, not about judging what a capability may do.*

## Status / discipline

- Candidate. Filed alongside the shipped `issue_capability` slice
  (`src/lib.rs`, `tests/spend_capability.rs`, `src/bin/la_cli.rs`) so the discipline is
  written down, not just implied by the code.
- **No reservation engine, no redemption ledger, no execute-from-capability path.** If
  tempted to make an issued capability *hold* capacity: **consumer trigger absent** (and
  see the reservation trap below).

## The ruling

> A `SpendCapability` is a bounded execution **envelope**, not a spend and not a
> reservation. Issuing one does **not** draw down `remaining_capacity` and does **not**
> reserve it. The only effect that crosses the spend boundary is `consume`. A capability
> alone cannot execute.

`issue_capability` in `src/lib.rs` is additive: it fails closed against any token that
cannot back it (unknown / revoked / expired / exhausted), binds the token's opaque
`eligibility_reference` verbatim, records `Event::CapabilityIssued`, and returns the
envelope — **without touching stock or `remaining_capacity`**. The burn still happens, and
only happens, at `consume`, where conservation and per-token drawdown are enforced (see
`conservation` and `token_balance_preserved` in `verification/Ledger.lean`).

## The reservation trap (why "not reserved" is load-bearing)

It is tempting to read "issued capability" as "capacity already set aside." Do **not**.
LA has no reservation rule: a token with `remaining = 3` can back three issued
capabilities *and* still be drawn down independently by `consume`. If issuance silently
reserved, you would get **double-accounting** — capacity counted once as reserved and
again as remaining — with no conservation theorem covering the reserved column. So the
discipline is deliberately the weaker, honest one:

> Issuance neither spends nor reserves. Conservation is enforced at `consume`, over
> `remaining`/`consumed` only. There is no reserved column.

If a genuine reservation semantics is ever wanted (issue = hold capacity until redeemed or
released), that is a **separate future ruling** with its own conservation obligation — it
would add a reserved quantity to the token and must extend the balance identity to
`original = remaining + consumed + reserved`. Not in v0; not implied by this note.

## What this note does NOT do

- It does not judge whether a capability's `effect_class`/`target` *should* be permitted —
  that is the capability axis ([capability-composition.md](capability-composition.md)), a
  hardness gradient, not a spend count. Keep them separate.
- It does not enforce a 1:1 between issued capabilities and consumes. Nothing in v0 binds
  an issued capability to a specific later `consume`; the envelope carries no spend of its
  own. Binding issuance to redemption (single-use *enforced at consume*) is a candidate,
  not a v0 guarantee.

## Opening triggers (when this stops being a note)

- A consumer needs capacity actually **held** between issuance and use (reservation).
- A consumer needs issuance and consume **bound** so an issued-but-unspent capability is
  itself accountable (redemption tracking).

Until one fires: candidate only. The invariant under test today is the weak, true one —
`tests/spend_capability.rs` asserts issuing N capabilities against one token leaves
`available(scope)` unchanged.

## Keepers

> Issuance is not spend. Issuance is not reservation.
> Only `consume` crosses the spend boundary. A capability alone cannot execute.
> No reserved column — so don't let "issued" quietly mean "reserved."
