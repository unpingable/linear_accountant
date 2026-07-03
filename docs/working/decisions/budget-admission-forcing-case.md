# Budget admission — `deposit` cites an admission or fails closed (decided, built)

*Decision record. The mint boundary must cite a budget admission; LA records and carries
the reference, never evaluates it. Built 2026-07-03. Companion to
[V0_BOUNDARY.md §2c](../../architecture/V0_BOUNDARY.md), the sibling
[custody-legibility](custody-legibility.md), and the cross-constellation
[predicate-legitimacy candidate](../predicate-legitimacy-not-from-proof-candidate.md).*

> **Status:** decided and built. **Ruling date:** 2026-07-03. **Provenance:** operator
> ruling on a cross-model thread, promoting budget-admission from named hazard to shipped
> behavior. Not held on an Agent Governor producer — **the producer is whoever calls
> `deposit`.**

## The ruling

> **Deposit must cite a budget admission reference. LA records and carries the reference.
> LA does not evaluate authorization.**

Build the receiver now; callers must supply the opaque reference.

- **C — end-to-end authorization: rejected, permanently.** LA never reads an admission
  and decides it was legitimate. Same refusal as the
  [predicate-legitimacy candidate](../predicate-legitimacy-not-from-proof-candidate.md),
  one substrate out: the spend plane does not look at a basis and rule the category real.
- **B — admitted-budget accounting: ratified and built.** `deposit` carries an opaque,
  typed `BudgetAdmissionRef`; LA stores and cites it verbatim, never evaluates it.
- **A — bare mechanical deposit: refused for ordinary deposit,** because the mint boundary
  must cite admission. A `deposit` with an empty admission reference fails closed.

**Why now, not later.** The repo's own evidence already fired it. `request_capacity`
refuses an empty `eligibility_reference`, while `deposit` minted stock from `scope +
amount` with no basis at all — the row-1 laundering seam, and a standing inconsistency
with **invariant 4** (mint decisions accept only sealed references and typed
descriptors). A token had to cite its eligibility; the stock it was drawn from cited
nothing. Closing that does not wait on a named consumer: whoever calls `deposit` is the
producer. LA's job is only to require, carry, and cite the sealed reference — never to
evaluate it.

## What shipped

- **`BudgetAdmissionRef { admission_ref, basis_kind }`** — an opaque sealed pointer plus a
  typed descriptor. `admission_ref` is load-bearing and must be non-empty; `basis_kind` is
  a carried label, never branched on. No free-text justification field.
- **`deposit(scope, amount, &admission) -> DepositDecision`** — `Deposited { .. }` on a
  non-empty reference; `Refused { reason, receipt }` on an empty one (fail-closed and
  recorded — the mint boundary is never silently skipped).
- **`Event::Deposited { scope, amount, admission }`** and **`Event::DepositRefused {
  reason }`** — the ledger carries the linkage verbatim; a refused mint still leaves a
  record.
- **`la_cli` `deposit`** requires `admission_ref` (optional `basis_kind`): a missing field
  fails closed, an empty one is refused, the success line echoes the reference. Protocol
  doc reconciled.
- **Tests** — bare deposit refused; non-empty deposit succeeds and the `Deposited` event
  carries the reference verbatim; an unvetted/hostile `basis_kind` still deposits (the
  non-evaluation proof); plus the transport fail-closed and echo tests. Conservation,
  `consume`, and capability-is-not-spend are unchanged — the existing suites and the Lean
  cross-check (`verification/Ledger.lean`) stay green (the opaque reference is orthogonal
  to the conserved quantity, so the Lean twin needed no change).

## What LA may and may not say

- LA **may** say: "this stock was minted against admission R."
- LA **may not** say: "R was legitimate authorization."

The reference is evidence linkage, not evidence validation. The only check is non-empty,
never valid — legitimacy is the budget-setting priesthood's job (§2c), not the goblin's.

## Scope fences (this slice crosses none of them)

- **Consume remains unchanged.** No thaw; the spend path is untouched.
- **No authorization checker.** LA carries the reference; it does not evaluate it.
- **Global effect idempotency remains out of scope.** Replay is still `(token_id,
  event_id)` — see [event-identity](event-identity.md).
- **Ledger custody anchor remains deferred** — see the sibling section below.
- **No free-text justification.** Invariant 4 holds: `admission_ref` / `basis_kind` are
  sealed / typed, never prose.
- **No AG dependency, no policy semantics, no `~/git/lean` authorization apparatus.**

## Still open (named, not built)

- The **richer basis fields** beyond the sealed reference — who-set-it / for-whom /
  until-when / who-approved — remain candidate; the reference + `basis_kind` are the
  minimal linkage, not the full witness record.
- The **budget-admission witness proper** — a read-only ledger testimony
  (`testify_stock_fully_admitted`, NQ-shaped) that "every unit of stock in this scope
  traces to an admission basis." Now *expressible*, because ingress carries the reference;
  built when a witness consumer needs it. This is the payoff half of the slice.

## Sibling: the ledger custody anchor stays deferred (not a deadlock)

Ingress provenance and state durability are **asymmetric seams**, so neither waits on the
other:

```
budget-admission  :  why is this stock allowed to exist?     (ingress) — BUILT
ledger custody anchor :  is this ledger the one that existed? (durability / export) — deferred
```

The custody anchor is [custody-legibility](custody-legibility.md)'s "genuinely hard
half": *an anchor outside the actor's unilateral control.* The ledger is still one
in-process `Vec` a privileged writer could edit; in-process legibility ≠
externally-anchored legibility. Its opening triggers — third-party proof of no silent
override, an incident review needing an un-editable log, a new silent-custodian operation
— have not fired, and it needs external machinery (signing / export / replay) that
budget-admission did not. Budget-admission closed by **carrying a typed reference without
evaluating it**; the custody anchor cannot. Shipping one does not ship the other. **Not a
bilateral deadlock.**

## Keepers

> A token must cite its eligibility; the stock it draws from must cite its admission. Same seam, one level up.
> `deposit` records the amount and the basis. It does not judge the basis — that is the line between B and C.
> LA carries the admission reference; it never reads it. The only check is non-empty, never valid.
> Ingress provenance and state durability are different seams. Shipping one does not open the other.
