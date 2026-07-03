# Event identity — replay is `(token_id, event_id)`, not a global effect id

*Candidate ruling. Not ratified. Records a boundary already present in the code so it is
not misread; authorizes no build. Companion to
[V0_BOUNDARY.md](../../architecture/V0_BOUNDARY.md). This is the **spend** axis.*

## Status / discipline

- Candidate. Filed because prose in `V0_BOUNDARY.md` and `LA_CLI_PROTOCOL.md` calls
  `consumption_event_id` "*the* exactly-once key," which reads as a *global effect*
  guarantee the code does not make. This note pins the actual domain and defers the
  global one.
- **No global-idempotency engine.** If tempted to dedup effects across tokens: **consumer
  trigger absent** — that belongs to the dispatcher/executor membrane, not the accountant.

## The ruling

> Replay/idempotency is scoped to **`(token_id, consumption_event_id)`**. LA does not
> provide global effect idempotency. The same `event_id` used against two distinct tokens
> is two distinct token spends, not one globally deduplicated external effect.

This is exactly what `src/lib.rs` implements: `consumed_events` is a `HashSet<String>`
stored **per `TokenState`**, and the replay check tests membership in *that token's* set
(`consume`, replay branch). The Lean twin matches it: `replay_is_noop` is stated over a
token's own `events` list (`verification/Ledger.lean`).

## Why this is correct for the spend plane, not a gap

Each token is its own finite linear budget. A `consumption_event_id` is the caller's claim
of *operation identity for a spend against a given token* — it lets the accountant refuse a
replayed draw on **that** budget. It is deliberately **not** a claim about a downstream
external effect. Two tokens are two budgets; spending one unit from each under a reused id
is two conserved spends, and conservation is untouched (`minted = available + Σ original`
still holds — see `conservation`, and per-token drawdown holds too — see
`token_balance_preserved`).

Making `event_id` a *global* effect key would require LA to reason about **effect
identity** across tokens — what two operations "are the same real-world effect." That is a
membrane question about the world outside the ledger, not an arithmetic property of the
books. The goblin does not judge; it counts. So global effect dedup is out of scope by
construction, not by omission.

## Where the global concern lives (deferred)

Global effect idempotency — "this side-effect must happen at most once regardless of which
token backed it" — is **dispatcher/executor-membrane territory**. It is named in
[deployment-shape.md](deployment-shape.md) (atomic consume + effect taxonomy at the
tool-call dispatcher) and is the same family as the residue/unwind concern in
[revoked-fork-residue-hazard.md](../revoked-fork-residue-hazard.md): the membrane owns what
happens to *effects*; LA owns what happens to *counts*.

## Opening triggers (when this stops being a note)

- A real dispatcher consumer needs at-most-once **effect** delivery and asks LA to be the
  dedup point. (Answer will still likely be "no, the membrane owns that" — but the trigger
  forces the design conversation.)
- Someone proposes making `consumption_event_id` unique across tokens inside LA. That is a
  conservation-key change and would need explicit ratification (it touches the exactly-once
  guarantee's domain).

Until one fires: candidate only. The wording in the protocol/boundary docs is softened to
say *per-token* so no one reads a global promise into it.

## Keepers

> Replay is `(token_id, event_id)`. Same id, two tokens, two spends.
> Effect identity is a membrane question, not an arithmetic one.
> LA counts token spend; it does not deduplicate the world.
