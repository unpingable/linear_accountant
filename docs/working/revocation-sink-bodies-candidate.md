# Revocation / refusal sink bodies — typed reason & basis (candidate)

> **Status:** `candidate / non-binding`. Names a surface; authorizes no build and admits
> no architecture. A handle for review, not a spec.
> **Filed:** 2026-07-03. Provenance: static source/docs audit (ChatGPT pass), reconciled
> against the code.

## The observation

Today revocation is a bare flag-flip with a bodyless receipt:

- `revoke(token_id, _reason, now)` in `src/lib.rs` takes a `reason` and **discards it**
  (the parameter is underscore-bound); it records `Event::Revoked { token_id }` — no
  reason, no actor/custodian, no basis pointer.
- Refusals are richer but still thin: `consume` writes `Event::ConsumeRefused { token_id,
  kind: String, event_id }` — a `kind` label, not a typed reason with standing.

For pure spend accounting this is fine: the ledger records **that** a revoke/refusal
happened (custody is never silent — the act is on the ledger), which is all the goblin
owes. But for a **custody witness** that must answer *why, by whom, under what standing*, a
bodyless `Revoked` is the "attributable act without attributable basis" gap named in
[custody-legibility](decisions/custody-legibility.md).

## The candidate shape (NOT to build now)

If a custody-witness consumer ever forces it, the sink bodies would carry:

```
Event::Revoked {
  token_id,
  reason: RevokeReason,        // typed, not free-text — no plea enters the seam
  custodian: ActorRef,         // who revoked (standing, not a name string)
  basis: BasisPointer,         // opaque sealed pointer to the warrant, never parsed
}
```

Same texture for refusal sinks: a typed reason and a basis pointer, not a `String` label.

## Why it is deferred (and what would be wrong to do)

- **A reason edges toward "why/standing,"** which is custody territory, not spend. The
  litmus (`CLAUDE.md`): the day a code path *reasons about* a justification it has become a
  small judge. So the reason must stay **typed and opaque** — a sealed pointer the
  accountant records but never reads — exactly like `eligibility_reference`. Free-text
  justification into the seam is the forbidden move.
- **This is not an accountant-correctness bug.** Conservation and per-token drawdown
  (`conservation`, `token_balance_preserved`) are untouched by revocation-body richness;
  `revokeTok` never touches `original`. The gap is *witness legibility*, not *counting*.

## Opening triggers (when this stops being a note)

- A custody-witness consumer needs revocation/refusal receipts that carry attributable
  basis (who/why/under-what-standing), not just the act.
- The external custody anchor in [custody-legibility](decisions/custody-legibility.md)
  opens — anchored bodies want a typed schema, and this is that schema's first draft.

Until one fires: candidate only. No `Event` variant changes, no reason plumbing.

## Keepers

> The ledger already records the *act*; the basis is what's missing.
> A revoke reason must be typed and sealed — recorded, never read. A read reason is a judge.
> Body richness is a witness concern, not a conservation one.
