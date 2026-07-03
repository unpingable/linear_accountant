# Linear Accountant v0 — Spendability Authority Boundary

**Status:** Reference boundary. Not production architecture. Not a daemon.
**Executable form:** this crate (`src/lib.rs` + `tests/v0_boundary.rs`, 15 passing tests). The
code is authoritative for behaviour. The boundary suite is complemented by companion
targets — CLI transport, preflight door, contention & file-write workloads, spend
capability, and the Lean differential oracle (`tests/*.rs`); run `cargo test` for the
whole suite. (Exact per-target counts drift; `cargo test -- --list` is authoritative.)

**Three kinds of refusal, never conflated:** *spend is counted* (this crate),
*capability is checked or judged* ([capability-composition](../working/decisions/capability-composition.md) — a hardness gradient;
composition is minting), *custody is held* ([custody-legibility](../working/decisions/custody-legibility.md) — legibility, not
elimination). This crate is only the spend axis.
**Companions:** [ROLE.md](ROLE.md) (role), [HANDOFF_PACKETS.md](HANDOFF_PACKETS.md) (per-tool contracts).

## 0. The spine

The earlier framing was "no sovereign, only separated powers." That concedes too
much and misnames the structure. Sovereignty did not disappear — **it relocated,
from semantics to arithmetic.**

> The sovereign cannot be semantic.

The Linear Accountant *is* sovereign over spendability: its verdict is final and
unappealable. That is safe **because it is too stupid to hear an appeal.** A
semantic governor is dangerous as sovereign because it can be pleaded with,
recontextualized, summarized, retried against, prompt-shaped, and "yes-but"-ed into
mush. An accountant that only knows `token exists / scope matches / unexpired /
remaining > 0 / consume atomically` cannot be negotiated with. Absolutism is
tolerable exactly where there is no surface to plead against.

The defensible claim — strong without being discourse bait:

> Safety is not a governor you build; it is a property you arrange: one conserved
> quantity that can't be argued with, narrow seams that can't carry an argument, and
> a freshness bound so authority can't outlive its warrant. The semantic layer
> advises; it never decides alone, never mints, and never gets a channel wide enough
> to plead.

**Burden split (say this out loud — the asymmetry is a feature):**

- **Pillar 1 — conservation — *provable*.** You cannot double-spend a linear
  capability: `[A] ⊬ A ⊗ A`. This is the `ContractionHinge`/LinCalc core and does
  **not** depend on semantics being weak. This is the hard kernel.
- **Pillar 2 — eligibility separation — *posture, not proof*.** The semantic layer
  *might* be talked out of its limits, so nothing load-bearing may depend on it not
  being. We do **not** claim "a single semantic governor is impossible" — that's a
  contestable conjecture that collapses the day someone demos a robust one. We claim
  only: *don't make safety depend on semantics being unbreakable.* That is modest and
  nearly unattackable, and it starves the discourse goblin.

## 1. The three pillars

### Pillar 1 — Conservation (provable)

Validity is contractible; spendability is linear: `valid(x) ∧ valid(x) ≡ valid(x)`
but `[A] ⊬ A ⊗ A`. Enforced by **finite stock**, not prose. Two linear boundaries:

1. **stock → token** at grant — a depletable pool per scope (`deposit`). Citing the
   same eligibility forever cannot refill it; the pool runs dry and requests are denied.
2. **token → effect** at consume — `remaining_capacity` drawn down atomically,
   exactly-once per `(token_id, consumption_event_id)` (replay is scoped to the token,
   not a global effect id — see
   [event-identity](../working/decisions/event-identity.md)).

### Pillar 2 — Narrow seams (engineering security property)

A circuit breaker only works if the channel into it cannot carry a plea. A dumb
breaker on a narrow seam beats a smart breaker on a fat one, every time. **Security
of the stack ≈ impoverishment of the inter-layer messages.**

```
The accountant may accept descriptors.
The accountant may not interpret justifications.
```

Safe descriptor fields: `actor`, `action`, `target`, `scope`, `amount`,
`eligibility_reference` (opaque sealed pointer), `eligibility_valid_until`,
`token_id`, `consumption_event_id`, `expiry`. **Unsafe** (must never reach a
mint/consume decision): free-text reason, agent-authored summary, policy
interpretation, exception narrative, retry justification, "equivalent authority"
claim. Free text may exist as *evidence for the semantic layer*, never as input to
the accountant.

In this crate the seam is structurally narrow: `CapacityRequest`/`ConsumeRequest`
are typed; `eligibility_reference` is checked for *presence*, never parsed; there is
no free-text field in any decision path.

### Pillar 3 — Freshness (temporal circuit breaker)

`eligible_at t₀ ≠ eligible_at t₁`. A verdict fresh at request time can be stale
later; **proceeding on a stale warrant is a distinct breach from proceeding on an
ineligible one** (temporal capture). Every grant carries a time constant:

- `eligibility_valid_until` — how long the *warrant to mint* stays fresh (checked at request).
- `token.expires_at` — how long the *minted capacity* lives (checked at consume).

**Nightshift's role sharpens accordingly: it is the temporal circuit breaker /
revalidation scheduler** — it decides whether a stale warrant must be rechecked
before a workflow continues. Not accountant, not witness, not semantic governor.

### The joint, interrogated honestly

Does "narrow seam" secretly smuggle the impossibility claim back into Pillar 1? Only
if the descriptor is expressive enough to plead. *As implemented*, it is not: the
accountant's decision is a pure function of typed comparisons (existence, scope
equality, expiry, remaining, warrant freshness) plus the *presence* of an opaque
reference — zero interpretation. So narrow-seam does reference-checking, not semantic
work, and the conjecture is not smuggled in. The standing obligation: **keep the
descriptor typed.** The day a free-text field starts influencing a decision, Pillar 2
dependence is back. That is the joint to watch on every future widening.

But typed is not the same as checkable. The seam blocks prose; it does not block a
*typed* field that lies — a `Tick` or a `request_id` is a caller-supplied claim the
accountant cannot verify. So narrowness has two axes, not one: prose width *and*
claim-checkability. The full obligation on every widening is in §2b vigilance point 3.

## 2. The inviolable rule

> No minting by the persuadable.

The semantic layer may *propose* spend and *request* consumption. It may never
*mint*. Minting authority lives with the accountant, out of semantic reach.

## 2b. Cash register, not judge — the size discipline

> **Wicket is a judge. Linear Accountant is a cash register.**
> Wicket decides whether a request is admissible. The accountant decides whether there
> is still a coin to spend.

That is a *smaller* authority surface than Wicket — potentially the smallest in the
constellation. Wicket carries intent / policy / basis / standing / reason codes /
receiver semantics. The accountant carries `token_id / scope / remaining / expires_at /
status` and one verb that matters: `consume`. The doctrine around it is huge; the
**executable authority must stay tiny**, because the whole point is that it does not
understand stories.

**Self-audit (2026-06-03): still a cash register.** Every decision path is mechanical —
timestamp compare (`eligibility_valid_until`, `expires_at`), integer compare
(`remaining`), set membership (replay `event_id`), string equality (`scope`), or
presence (`eligibility_reference` non-empty). The seven-variant refusal enum is refusal
*taxonomy*, not intelligence; nothing parses a justification. The richer surface
(`revoke`, `deposit`, the refusal variants) is all custodian/mechanical, not semantic.

**Two vigilance points — the camel's nose of semantics:**

1. **Scope is `==`, never containment.** The accountant matches scope by exact
   equality; it does *not* reason about scope hierarchy/containment (does `deploy:*`
   cover `deploy:svc-x`?). Any scope *semantics* belongs in the eligibility layer
   (Wicket/AG), never here. The day the accountant reasons about scope, it has become a
   small judge — back to a surface that can be pleaded with.
2. **The spendable unit's granularity is the effect-taxonomy question, not baked in.**
   v0 binds a token to *scope*, records `action` only as a ledger label, and does **not**
   match `action` on consume. Whether the unit should be per-action or per-scope is
   deployment hard-part #1 ([deployment-shape](../working/decisions/deployment-shape.md)), decided per domain — a deliberate v0
   simplification, not a bug.
3. **The seam is narrow against prose, not against typed claims.** The descriptor
   carries no free text, and §"the joint" rightly guards that boundary. But two typed
   fields already influence decisions as caller-supplied ground truth the accountant
   cannot verify: `Tick` (is it still *now*?) and `request_id` (is this the *same*
   operation?). Neither is a free-text plea, so neither trips the narrow-seam guard —
   yet each carries a claim whose truth lives entirely in the caller. A falsifiable
   timestamp is a plea in numeric clothing; a fresh `request_id` asserting "new
   operation" is a plea in identifier clothing. So the precise property is **narrow
   against pleas-as-prose, not narrow against pleas.** Where the trust actually sits:
   `Tick` is Nightshift's to own (freshness breaker); `request_id`-as-operation-identity
   is caller discipline, pinned as the fallback dedupe key (`unwrap_or_else`,
   regression-guarded at `v0_boundary.rs`). The obligation on every future widening is
   therefore not only "is this field free text?" but **"does this typed field carry a
   claim the accountant can't check?"** If yes, it needs a named owner outside the
   accountant — it must not be treated as self-certifying because it happens to be typed.

> The danger is making it smart. The win is keeping it stupid enough that nobody can
> plead with it.

## 2c. Budget setting vs budget enforcement (the irreducible priesthood)

The accountant **enforces** budgets; it cannot and must not **set** them.

```
Budget setting       : judgment / policy / custody / politics
  how much budget should this actor get?
  who may allocate budget?  what risks are worth spending against?  when to raise it?

Budget enforcement   : arithmetic / ledger / consume / receipt
  is there budget left?  can this token be consumed?  has this been spent?
```

> **Budgets are set by custody. Budgets are spent by accounting. Agents may request
> both, but author neither.**

The win is *not* eliminating budget-setting (the hard social part is irreducible). The
win is preventing it from being **silently re-decided at runtime by the agent**. The
accountant's stupidity forces the budget question into the open instead of letting it
hide inside "the agent decided it was fine":

> I was told actor A gets 3 deploy attempts for target X until 17:00. One is spent. Two
> remain. No, I do not care about your inspirational paragraph.

**Where setting lives:** budget-setting is a custody act and is the place Wicket / AG /
policy return — as the **budget admission surface** (not the spender). Because it is the
hard/social part, it needs witness: *who set it? for whom? what scope? how long? under
what basis? who approved the increase? what changed since last budget?*

**Honest v0 state.** `deposit` is the enforcement-side entry point where a
custody-decided budget lands. It records the *act* (`Event::Deposited{scope, amount}` —
not silent, per invariant 2 in-process) but **not the basis**: no who-set-it / for-whom
/ until-when / under-what-basis / who-approved. So a budget is currently recorded as a
bare mint, not a witnessed grant. Capturing those basis fields is the **budget-setting
witness gap** — a quiet custody act in [custody-legibility](../working/decisions/custody-legibility.md) terms, and it ties to
deployment hard-parts #1 (effect taxonomy) and #4 (ownership). Candidate, **not built**;
opens when a real budget-admission consumer needs the witness fields.

## 2d. The preflight door (refusal exposed, machinery withheld)

> **Doors are allowed. Hidden rooms are not.**
> **A frozen boundary may expose refusal preflight; it must not mutate state until the
> named consumer trigger fires.**

A frozen permit office still needs a door to knock on. If the only answer to "may I
`consume()`?" is silence, the freeze isn't principled restraint — it's the permit office
behind an unmarked wall, and no consumer can ever *become* the trigger that thaws it.

So v0 exposes exactly one refusal-only surface: [`preflight::preflight_consume`]. A
consumer (Maude at a write-tool dispatcher, say) presents a `PreflightInquiry` — opaque
mechanical fields, no free-text plea — and gets back a structured `not thawed`:

```
refusal:                   ConsumePathNotThawed   (typed, not prose)
consumer_trigger_required: true
expected_trigger:          "real agent stack requesting consume() at its dispatcher"
observed_boundary:         "maude.write_tool_approval"   (echoed back)
mutation_performed:        false                  (the type's standing promise)
```

This is the **non-stupid middle path**: enough interface for a consumer to knock, not
enough machinery for the accountant to self-thaw. The function takes no accountant and no
`&mut` — it *structurally cannot* touch stock, tokens, or the ledger. What stays withheld
behind the freeze is unchanged: durable ledger, deposits, refunds, conservation-witness
extension, expiry lifecycle, and any actual capacity mutation.

The point is the forcing case: once a real consumer *integrates* this check, the trigger
is no longer hypothetical — you have an observed consumer boundary with a denied consume
attempt on record. That denial is the evidence that thaw is now justified ([authority and
self-thaw discipline]: a future trigger is not a fired trigger; a *door* is how a future
trigger becomes a fired one).

## 3. Object vocabulary

- **Eligibility** — non-spendable admissibility statement. A *reference* + a
  `valid_until` are required on every request; the accountant does not re-decide it.
- **Capacity / stock** — the finite consumable pool the accountant owns.
- **Spend token** — minted, accountant-owned. Spendable iff `active ∧ remaining>0 ∧ not expired ∧ not revoked ∧ scope matches`.
- **Consumption event** — atomic transition reducing capacity.
- **Receipt** — copyable *evidence* an event occurred. Never spendable.
- **Testimony** — witness-layer statement (NQ). Never allocation authority.

## 4. Interface (implemented)

```
request_capacity(request_id, actor, action, target, scope, requested_capacity,
                 eligibility_reference, eligibility_valid_until, expires_after,
                 idempotency_key?) -> CapacityDecision
    → Granted { token_id, granted_capacity, scope, expires_at, receipt }
    → Denied  { denial_reason, receipt }        // incl. stale-eligibility and insufficient-stock

consume(consumption_event_id, token_id, actor, action, target, amount, scope) -> ConsumptionDecision
    → Consumed | AlreadyConsumed | InsufficientCapacity | Expired | Revoked
      | UnknownToken | ScopeMismatch     (each carries token_id + receipt)

inspect_token(token_id) -> Option<TokenView>     // read-only; observation is not a lease
revoke(token_id, reason, now) -> RevocationDecision { Revoked | AlreadyFinal }

deposit(scope, amount) -> ReceiptId              // custodial mint — recorded, not silent
ledger() -> &[ReceiptRecord]                     // append-only; receipts have bodies
witness::testify_no_double_spend(ledger, token_id) -> Testimony   // read-only; cannot allocate
```

The contact loop runs end to end in code: eligible request → token → consume →
receipt → replay denied → **witness testifies**.

Caller-supplied logical time (`Tick`) — no ambient clock. `TokenId`/`ReceiptId` are
opaque with no public constructor: the only source of a token is a grant, the
compile-time form of "eligibility cannot mint capacity."

## 5. Forbidden flows (what this blocks)

`policy allow → budget exists` · `receiver acceptance → replayable mutation authority`
· `receipt/testimony → reusable lease` · `prior allowance summarized into context →
fresh capacity` · `retry reuses stale validation as new spendability` · `stale warrant
→ live action` · `scope grant combines eligibility and usage count without a hard
consumption boundary` · `NQ testimony treated as allocation authority` · `WLP envelope
treated as money` · **`free text in the seam influences a mint/consume decision`**.

## 6. Deliberately absent in v0 (where demons live)

- **No refunds.** Consumption is terminal; a failed execution does not return capacity. Refund rules are where double-spend creeps back. Any refund must later be an explicit, audited accountant rule — and the *shape* of that rule is already determined: **a refund is a new custodial `deposit`, explicit, receipted, and gated by standing — never an automatic reversal of `consume`.** An auto-refund on failed effect is a retry storm re-entering through the accounting department: free retries with extra steps. Restocking capacity is minting, so it lives where all minting lives (§2c, budget-setting under custody), not on the spend path. The demo beat — *the confabulated citation consumed real budget and bought a refusal* — depends on this paragraph holding.
- **No distribution.** Single in-memory writer. A real multi-writer accountant needs single writer / CAS / fencing tokens / monotonic sequence / durable log / idempotency — named here as known prior-art, not built.
- **No semantic policy engine.** If the accountant gets clever, it fails back into laundering with a nicer hat.

## 7. Honest scope

- Resource limits / one-shot authority / blast-radius reduction: **strong** (Pillar 1, provable). The model cannot argue with `AlreadyConsumed`.
- Preventing semantic laundering of capacity: **strong, if the goblin stays dumb and external and the seam stays narrow.**
- Agent safety in general: **no.** This fences exactly one failure class — turning valid context into reusable capacity. A circuit breaker, not a conscience. "Is this wise / correct / acceptable?" stays with AG / Wicket / NQ.

## 8. The constitutional stack

```
Semantic Governor / AG : eligibility, policy, standing, scope; may request; cannot mint
Wicket                 : admission / receiver eligibility; may require a token; admission ≠ execution
Linear Accountant      : SOVEREIGN over spendability; counts, mints, consumes, expires; parses no stories
Execution              : consumes token; performs effect; emits receipt
NQ                     : testifies; cannot allocate
Nightshift             : freshness / revalidation circuit breaker; stale eligibility ↛ live action
```

Not one brain. Five suspicious offices. Ugly, but not fantasy architecture.

## 9. Required tests (all passing)

1. eligibility alone cannot execute
2. token can be consumed once
3. consumed token cannot be consumed again
4. receipt cannot be used as token
5. repeated request with same idempotency key does not mint duplicate capacity
6. keyless replay of the same `request_id` does not mint duplicate capacity
   (refactor-guard: pins `request_id` as the fallback dedupe key when no
   `idempotency_key` is supplied — see `unwrap_or_else` in `request_capacity`)

Plus: contractible-eligibility-vs-linear-capacity, **stale-eligibility-is-a-distinct-breach**,
idempotent event replay, token expiry, scope mismatch, revoke finality,
**custodial-deposit-is-recorded** (invariant 2, in-process), **witness-can-testify-no-double-spend**,
and the `restart_service` toy-consumer scenario closing the full contact loop through the witness.

A separate **differential oracle** (`tests/differential_oracle.rs` against a Lean
model in `verification/`) checks the conservation identity and replay-refusal over
randomized event sequences — see §9b.

## 9b. The differential oracle (hardening, not thawing)

This is the one component where machine-checked proof earns its keep instead of
cosplaying rigor: a tiny state machine, affine/multiset semantics, a core claim that is
a linear-logic fragment, and the highest blast radius if wrong — it is the thing that
makes failure *finite*, so a bug here unmakes finiteness everywhere.

So the ledger is modelled in Lean (`verification/Ledger.lean`) as a fold over an event
list, with two machine-checked theorems (zero `sorry`, mathlib-free):

- **`conservation`** — `minted = available + Σ original`, for every event sequence. The
  books always balance. (Conservation is stated over `original`, which neither consume
  nor expiry mutates — which is *why* it holds without a restock rule.)
- **`replay_is_noop`** — consuming an already-seen event id changes nothing:
  replay-refusal and no-double-consume are one theorem.

The Rust implementation is then **differential-tested** against a faithful reference
model of that same fold (`tests/differential_oracle.rs`): randomized event sequences run
against both, asserting decision-category agreement plus the proven invariants after
every operation.

This **adds no surface and opens no spend path** — it hardens the frozen boundary rather
than thawing it (per "doors are allowed; hidden rooms are not"). It also gives the Lean
repo its first real consumer, which is the only thing the promotion discipline has ever
let matter. See `verification/README.md`.

## 10. Repo-Claude handoff prompt

To each constellation repo (one repo only, against this boundary):

> Read the Linear Accountant v0 boundary (this doc + the running crate). Do a
> **handoff-shape pass for this repo only.** Do NOT audit defects, refactor,
> implement, or create a component. Answer only: (1) role relative to Linear
> Accountant; (2) packets it would send; (3) packets it would receive; (4) what it
> must never treat as spendable authority; (5) the concrete future trigger that
> would justify integration; (6) existing docs/gaps that should cross-reference this.
> **Co-location is not a violation; convertibility is.** Also: the seam is narrow by
> design — flag any place this repo would want to send free-text justification into a
> mint/consume decision, because that is the joint where the security property leaks.
> Output: a short handoff note.

## 11. Keepers

> The sovereign cannot be semantic.
> The semantic layer advises; the accountant conserves; the scheduler expires; the witness testifies; execution consumes.
> A dumb breaker on a narrow seam beats a smart breaker on a fat one.
> Typed is not the same as checkable. The seam blocks prose; it does not block a timestamp that lies.
> No minting by the persuadable.
> Conservation is the proof; eligibility separation is the posture — and the asymmetry is the point.
> Eligibility is a request. It is not payment.
> Convertibility, not co-location, is the violation criterion.
