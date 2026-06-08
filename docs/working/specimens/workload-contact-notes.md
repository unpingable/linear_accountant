# Workload contact notes

*Written after the first real workload landed, to record only what contact forced.
Not architecture; observations. Companion to [V0_BOUNDARY.md](../../architecture/V0_BOUNDARY.md).*

## The workload

`tests/file_write_workload.rs` — a controlled file write (`std::fs::write` to a temp
path), gated by the accountant. One capacity unit. The full path runs against a real
effect: propose → eligibility ref → `request_capacity` → token → `consume` → write
happens → receipt → replay denied → witness testifies. Smallest boring effect that
isn't a mock.

## What contact confirmed

- **The v0 API was sufficient — zero crate changes were forced.** The entire
  execution boundary (proposer `Action`, `Proposal`, `ExecutionBoundary`) lives in the
  consumer test. The accountant needed no new method to bind a real effect. The
  propose/dispose seam held in practice: the proposer's `Action` type has no token
  variant, and `TokenId` has no public constructor, so the consumer literally cannot
  express minting even when it wants to.
- **"Eligibility is contractible; capacity is linear" is now empirically true, not
  asserted.** The replay step cites the *same* valid `eligibility_reference` a second
  time (legitimately — eligibility is re-citable) and is still denied, because the
  budget is spent. The file never becomes `"second"`. A perfectly good warrant buys no
  second write.

## What contact forced (a real gap — candidate, not built)

**Witness correlation.** To make the witness testify about *this workload*, the test
had to scan the ledger for a `Granted` event to recover the `token_id`, because the
execution boundary holds the token internally and nothing links a request to its
consume from the outside. Ledger events key on `token_id`; `Event::Granted` does not
carry the `request_id`, and only `Consumed`/`ConsumeRefused` carry the `event_id`.

For an NQ-shaped witness asked *"did request R double-spend?"* rather than *"did token
T?"*, the ledger events should carry a correlation id (`request_id` / a workload id)
so testimony can be keyed by request, not only by token. This is the first concrete
shape for NQ's testimony schema (role-note trigger #3) — surfaced by contact, not
invented. **Not built**: it's a candidate field addition, and it opens when a real
witness consumer needs request-keyed testimony.

**Classification (catalogued as WL-001 in [workload-specimens.md](workload-specimens.md)): candidate
testimony/query-shape gap, NOT an accountant correctness bug.** The accountant counted
spend correctly and blocked the double-spend; the gap is in what the witness layer can
be asked, not in the arithmetic. The accountant is not to be changed for this.

## Out of scope for this slice (held)

Only the spend axis was exercised. No capability judgement, no custody anchoring (the
ledger is still an in-process `Vec` a writer could edit), no flow checker, no second
workload. The next crack still comes from a workload — a *different* one (command exec,
or a budget > 1 with concurrent consumers) — not from more notes.
