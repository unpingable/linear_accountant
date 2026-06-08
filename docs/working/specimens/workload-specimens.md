# Workload specimens

*Registry of real workloads bound to the Linear Accountant, one entry per slice. A
specimen records what a workload proved on contact and what it surfaced — it is a
catalog, not a backlog. Each new workload is its own slice; recording a specimen does
not authorize the next. Detailed prose for specimen 1 lives in
[workload-contact-notes.md](workload-contact-notes.md); this file is the terse index.*

## Schema

`id` · `date` · `workload` · `result` · `api delta` · `gaps surfaced (classified)` · `artifacts`

---

## WL-001 — file-write

- **date:** 2026-06-03
- **workload:** controlled file write — `std::fs::write` to a temp path, one capacity unit, real effect (not a mock).
- **path exercised:** full loop — propose → eligibility_reference → `request_capacity` → token → `consume` → write → receipt → replay denied → witness testifies.
- **result: PASS.** Eligibility replay blocked by spent token: the same valid
  `eligibility_reference` was cited a second time (legitimately — eligibility is
  contractible) and the write was still denied because the budget was spent. File never
  became `"second"`. Empirically: *eligibility alone cannot execute; only consumed
  spendability crosses the effect boundary.*
- **v0 API delta: none.** The accountant required zero changes. The entire execution
  boundary is consumer code; the proposer's `Action` type cannot express minting.
- **gaps surfaced:**
  - **request_id testimony index / request-to-token correlation.**
    **Classification: candidate testimony/query-shape gap — NOT an accountant
    correctness bug.** The accountant counted spend correctly and blocked the
    double-spend; nothing is miscounted. The gap is in what the *witness layer* can be
    asked: ledger events key on `token_id`, so the query *"did request R
    double-spend?"* requires scanning for the `Granted` event to recover the token.
    Request-keyed testimony would need a correlation id (`request_id` / workload id) on
    events. This is the first concrete shape for NQ's testimony schema (role-note
    trigger #3). Belongs to the witness/query layer; the accountant is not to be
    "fixed" for it.
- **artifacts:** `tests/file_write_workload.rs`, [workload-contact-notes.md](workload-contact-notes.md).
- **regime:** the **AG-less minimal loop** — `Wicket→Linear→execution→NQ` with a
  *pre-budgeted* effect (budget hard-coded via `deposit(1)`, eligibility_reference a
  fixed string). No semantic request-governance. Fine for a toy; a workload needing real
  agent behavior (actor identity, variable budget, scope decisions) would need the AG
  role — see [ROLE.md](../../architecture/ROLE.md) §4.
- **what it did NOT prove:** atomicity under contention. The workload was
  single-threaded, so `consume`'s exactly-once property came for free from the
  exclusive `&mut self` borrow. The race was never run.

---

## Priority ordering for the next slice

Command-exec and contention are **not equivalent** next workloads:

- **Command-exec** (run `true`/`echo`/`date`) would re-prove the *seam shape* WL-001
  already proved, with a noisier effect. A **lower-value smoke test.** Only choose it
  if a smoke test is the deliberate goal.
- **Contention** is the **falsification test for the consume mechanism** — the thing
  WL-001 could not stress. This is the priority next slice.

## WL-002 — contention

- **date:** 2026-06-04
- **workload:** `RACERS = 8` threads race one token holding exactly one unit, through
  an `Arc<Mutex<InMemoryAccountant>>` serialization point in *consumer code* — the toy
  stand-in for the production CAS / row lock the deployment note names. The shared
  effect is an `AtomicUsize` sink incremented only behind a `Consumed` verdict, so
  exactly-once is proven from observable side effect, not just the decision enum.
- **path exercised:** the **falsification test for the consume mechanism** WL-001 could
  not run (it was single-threaded; exactly-once came free from the `&mut self` borrow).
  Two races: (a) distinct event ids — the genuine double-spend race, defended only by
  the linear remaining-capacity check; (b) one reused event id — the replay breaker as
  the active defense under concurrency.
- **result: PASS** (stress-run 50×, zero failures). Distinct-event race: exactly one
  `Consumed`, seven `InsufficientCapacity`, sink == 1, token `Exhausted` with
  `remaining == 0`, witness `NoDoubleSpend{events:1, total:1, replays:0}`. Replay race:
  exactly one `Consumed`, seven `AlreadyConsumed`, sink == 1, witness
  `NoDoubleSpend{… replays_refused: 7}`. **The deployment note's load-bearing claim —
  "atomic consume is just a conditional write" — holds: given one serialization point,
  the consume logic is correct under real contention.** No torn ledger state observed.
- **v0 API delta: none.** The accountant was not modified. The `Arc<Mutex<…>>` lives
  entirely in test code; `request_id` never entered the conservation key.
- **gaps surfaced:**
  - **none new.** The WL-001 request/token correlation gap was *exercised and kept in
    its layer*: the test recovers "which request won the race?" by joining a
    consumer-side index (`event_id → request_id`) against the ledger's
    token/event-keyed `Consumed` record. Confirmed the correlation is answerable
    without `request_id` becoming a conservation key — closing the WL-002 acceptance
    criterion without pushing the gap down into the accountant.
  - **honest boundary of the proof:** this tests correctness *through a single
    serialization point*, not lock-free or distributed atomicity. Production must
    supply that point (CAS / Postgres row lock / DynamoDB conditional put); the
    Mutex models it. That generalization stays deferred — it is deployment hard-part
    #2, now de-risked at the logic level but not at the distributed-systems level.
- **artifacts:** `tests/contention_workload.rs`.
- **regime:** same AG-less minimal loop as WL-001; contention is an *enforcement*-axis
  test and is AG-irrelevant (request governance does not bear on the consume race).

---

## Status: FROZEN as reference boundary (2026-06-04)

The two questions a reference boundary must answer are answered:

- **WL-001** proved eligibility alone cannot execute (the contractible/linear split is
  real on contact, zero API change needed).
- **WL-002** proved the consume mechanism is correct under real contention through a
  serialization point (the falsification test passed).

The Linear Accountant is **frozen as a reference boundary.** No further crate slices
without a *consumer trigger* — a real agent stack wanting `consume()` at its tool-call
dispatcher (see [deployment-shape](../decisions/deployment-shape.md)). Recording a candidate does not authorize a build.

Deferred, forcing-case-gated (NOT open):

- **command-exec workload** — would only re-prove WL-001's seam shape; a lower-value
  smoke test, do only if a smoke test is the deliberate goal.
- **distributed / lock-free atomic consume** — deployment hard-part #2 beyond the
  single serialization point; needs a real store.
- **NQ request-keyed testimony schema** — correlation id on events; the WL-001 gap,
  now twice-exercised and twice kept in the witness/query layer.
- **external custody anchor** — write-authority partition for the ledger
  ([custody-legibility](../decisions/custody-legibility.md)); in-process half closed, external half deferred.
- **budget-setting witness fields on `deposit`** — basis/approval, not just amount.
- **shadow-mode dispatcher shim** — the smallest first deployment step, sprint-sized,
  gated on a consumer.
