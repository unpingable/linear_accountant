# CLAUDE.md — Instructions for Claude Code

Start with [`AGENTS.md`](AGENTS.md) — it is the travel guide (quick start, layout,
invariants, conventions, safety). This file adds only Claude-specific notes; it does not
duplicate it.

## What this is (one line)

Linear Accountant: the reference boundary for the Spendability Authority role.
Validity is contractible; capacity is linear. It counts spend; it does not judge.

## The stance, in one sentence

Small, hostile, stupid — a tiny goblin with an integer. The danger is making it smart;
the win is keeping it too dumb to be pleaded with. Every decision should stay mechanical
(timestamp / integer / set-membership / equality). The day a code path reasons about a
justification, it has become a small judge — that is the failure, not a feature.

## Debugging discipline

**Constitutional rule:** belief must be earned by the cheapest available falsification,
not constructed by accretion. When a belief is load-bearing, reach for the test that
disproves it first, not last.

**In this project, "load-bearing" means** any claim that exactly-once held, that a spend
was conserved, that a refusal was recorded, or that the witness can testify. The cheapest
discriminating test is running the single boundary or workload test that exercises that
path (`cargo test --test v0_boundary` / `--test contention_workload`) and reading the
ledger it produces — not reasoning about whether the logic "should" hold.

## Don't

- Don't add a new crate slice without a consumer trigger. The boundary is **frozen**;
  record candidates under `docs/working/`, do not build them.
- Don't widen the seam: no free-text justification into a mint/consume decision, no scope
  containment logic, no refunds/distribution/policy engine in v0.
- Don't make `request_id` a conservation key. The conservation key is token/consumption;
  `request_id` is a testimony/query index only.
- Don't reach for `sed`/`echo` to edit files when Edit/Write fit; don't claim tests pass
  without running them.

## License

Apache-2.0.
