# Revoked-fork residue — revocation stops future spend, not produced effects (hazard)

> **Status:** `candidate / non-binding` — out-of-scope hazard record. Names a boundary LA
> must **not** cross; authorizes no build.
> **Filed:** 2026-07-03. Provenance: cross-repo reconciliation with the Lean corpus
> (`~/git/lean`), which carries a specimen written against LA by name.

## The hazard

The sibling Lean corpus flags LA directly. `LeanProofs/Scratch/QuorumResidueCoupling.lean`
carries a "BRIDGE HAZARD (flagged for AG / LA-Claude)" with the shape:

> `RevokedFork ↛ UnwoundForkEffects`

Killing a fork (revoking its token) prevents **future** spend against that token. It does
**not** unwind **already-produced** durable effects. Yet LA's `consume` receipt for the
effects that *already* crossed reads green — the spend was conserved and exactly-once, and
the later `revoke` is honestly recorded. Both receipts are true. The orphaned residue —
files written, messages sent, side effects landed before revocation — is simply **not in
LA's ledger at all**, because LA counts spend, not effects.

Lean names the coverage the honest system would need: `ResidueCovered`,
`winner_can_orphan_residue`, and discharge obligations (`unwind_authority_covers`,
`debt_receipt_covers`). None of those live in LA, and they should not.

## The ruling: this is out of scope, on purpose

> LA counts spend. It does not perform, verify, or unwind durable effects. Revocation is a
> flag that blocks *future* draws (`revokeTok` in `verification/Ledger.lean`; `revoked_at`
> in `src/lib.rs`); it makes no claim about effects that already crossed the membrane.

Making LA responsible for residue would require it to model **effects and their
reversibility** — what a spend *did* in the world and whether it can be undone. That is the
executing/dispatcher membrane's job, the same boundary drawn in
[deployment-shape.md](decisions/deployment-shape.md) (effect taxonomy + atomic consume at
the tool-call dispatcher) and adjacent to the global-effect-identity line in
[event-identity](decisions/event-identity.md). Residue coverage and unwind custody belong
there and in the custody witness ([custody-legibility](decisions/custody-legibility.md)),
not in the goblin with the integer.

## Why record it at all (since we are building nothing)

So future archaeology finds the **named** hazard instead of rediscovering it, and so no one
later reads LA's green consume receipt as "the effect was safe / recoverable." A green
consume means **the count was conserved**, nothing more. The residue question is real; it
is simply someone else's ledger.

## Opening triggers (for the membrane, not LA)

- A dispatcher/executor consumer needs revocation to *also* unwind produced effects — that
  consumer owns the unwind/residue receipts and may *read* LA's spend receipts as inputs,
  but LA does not grow an effect model.
- A custody-witness surface wants to correlate a revoke with the effects that preceded it —
  correlation lives in the witness/testimony index, not in the conservation key.

Until then: LA stays effect-blind by design. This note is the fence, not a to-do.

## Keepers

> A green consume receipt means the count was conserved — not that the effect was safe.
> Revocation blocks the next draw; it does not reach back through the membrane.
> Residue and unwind are the executor's ledger. LA counts spend; it does not un-happen the world.
