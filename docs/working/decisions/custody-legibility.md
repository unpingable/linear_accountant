# Custody Legibility — the second invariant

*Candidate architecture invariant. Not ratified. Names a surface; does not authorize
building it. Companion to [V0_BOUNDARY.md](../../architecture/V0_BOUNDARY.md) and [ROLE.md](../../architecture/ROLE.md).*

## Status / discipline

- Candidate. Marks the *other half* of the stack so it isn't rediscovered later.
- Not an implementation plan. The hard part (externally-anchored evidence) is
  explicitly deferred to a forcing case — see §7.
- If tempted to build the witness/anchor machinery now: **consumer trigger absent.**

## The regress (why "true safety" was a category error)

Every demonstrated "robust semantic governor," inspected, turns out to be a harness:
permission prompts, allowlists, sandboxes, egress proxies, read-only mounts. The
refusal isn't what holds — the out-of-band boundary is. So the impossibility of the
sovereign semantic governor isn't fragile to the next demo; **the demo is the
confirming instance.** The empirical record of attempted counterexamples is a record
of harnesses, not governors.

But the same blade cuts our stack. A linear-accountant-with-witness-and-TTL is a
*better* harness — conserved instead of re-passable, witnessed instead of silent,
expiring instead of standing — three genuine upgrades, all three still operational
containment configured by, and revocable by, a principal. A privileged-enough
operator re-mints, suppresses the witness, resets the clock. At that point you
haven't found the sovereign-free system; you've relocated the sovereign one step
further out — to whoever holds the signing key, whoever can edit the harness.

> The impossibility of the sovereign governor and the impossibility of true safety
> are the same impossibility seen from opposite ends. The governor can't be sovereign
> because something must backstop it. The backstop can't deliver safety because
> something must hold *it*. Both bottom out in custody, and custody is held by someone.

Every escape route confirms it: hardware root of trust relocates custody to an HSM +
key ceremony; m-of-n diffuses the sovereign into a collusion threshold (shadow
governance with a quorum); formal verification proves the fence is well-built and is
silent on who can move it. None reach a keyless system. They reach systems where the
key-holder is more exposed, more distributed, or more expensive to be.

## The two invariants

```
1. Validity is not spendability.   → gives the accountant (conservation)
2. Custody must not be silent.      → gives the witness / override rule (legibility)
```

## Core claim

> A harness cannot remove residual sovereignty. It can only relocate custody.
>
> A defensible harness makes custody **legible**: any privileged act — override,
> witness suppression, re-minting, lease reset, fence movement — must emit evidence
> that is externally anchored, outside the acting principal's unilateral control.

The shippable property is **not safety. It is attributable unsafety.** You do not
make the system safe; you make every unsafe act either impossible to spend (invariant
1) or impossible to hide (invariant 2). The override still exists — it just cannot be
quiet.

## The decision test

> Can the privileged operator act without producing externally-anchored evidence?

- **Yes** → you are back to "trust the operator." You gained nothing structural; it's
  a harness with extra steps.
- **No** → you've shipped the maximum custody permits.

This — not "can the governor be talked out of its limits" — is the seam to run the
interferometer on next: **can the custodian act without testifying against themselves?**

## Re-filing the discipline

Agent safety feels stuck because it is a **runtime-custody problem being worked as a
build-time-correctness problem.** "Align the model / type it correctly / prove the
contract" is the SWE dream of correctness-by-construction aimed at the *speaker* — and
that is the sovereign semantic governor that can't exist. The feasible build-time job
is making the *seam* correct, not the speaker.

```
SWE       — build the unforgeable seam + capability types (the PROPOSE side, correct by construction)
Ops       — run the accountant / witness / revalidation / override-with-receipt (the DISPOSE side, under pressure)
Governance— make residual custody legible to a third party who wasn't in the room
```

The discipline boundary has the same shape as the layer boundary: SWE/ops is
semantic/accountant one level up, and the same failure mode lives at both seams — if
the propose side smuggles authority across into dispose, you've lost it, whether the
smuggler is a jailbroken model or a SWE who shipped a capability type rich enough to
mint. **No minting by the persuadable** applies to the engineer as much as the agent.

So SWE is not doing "whatever vague thing." It holds the one end you can't run
without: making the seam *real* instead of polite. The field's error is pointing SWE
thinking at the wrong layer.

## Adoption wedge

Don't pitch a cosmology. The primitive already exists in ops: the **error budget**.
Product pressure, the launch the VP needs, the agent's "good reason" — all semantic,
all persuasive, all powerless against a budget that's been spent. The linear
accountant is the error budget **generalized from reliability to authority**: retry
budget, mutation budget, blast-radius slots, one-shot grants, tool-call allowance,
egress allowance, override allowance, capability lease — all one property, *conserved
spendability*. Extending a believed primitive's domain, not asking for a new belief.

## The v0 gap — in-process half now closed

The reference crate satisfies invariant 1. The *in-process* half of invariant 2 was
closed on 2026-06-03 when the contact-loop forcing case fired:

- ✅ `deposit` (minting capacity into existence — the most sovereign act in the crate)
  now emits an `Event::Deposited` to an append-only ledger. It is no longer silent.
- ✅ Receipts have **bodies**: every decision's `ReceiptId` indexes a retrievable
  `ReceiptRecord` in `ledger()`; refused replays are recorded too.
- ✅ A read-only `witness::testify_no_double_spend` reads the ledger (it takes only a
  `&[ReceiptRecord]` — structurally it cannot allocate) and confirms the breaker held.

**Still deferred — the genuinely hard half:** an anchor *outside the actor's unilateral
control*. The ledger today is one in-process `Vec` a privileged writer could edit.
In-process legibility ≠ externally-anchored legibility. The decision test below is NOT
yet passed: a sufficiently privileged operator can still edit the ledger silently. That
is the signing-key/anchor regress, and it remains a forcing-case-gated future surface,
not a v0 obligation.

## Opening triggers (when this stops being a note)

- A consumer needs to prove to a third party that no silent override occurred.
- An incident review needs a custodial event log the operator could not have edited.
- The accountant gains a real custodian operation (refund, manual mint, force-revoke)
  whose silent use would be a breach.

Until one fires, this stays a candidate. **No daemon, no crypto, no anchor service,
no Lean, no paper.**

## Keepers

> The override still exists; it just cannot be quiet.
> Not safety — attributable unsafety.
> You never eliminate root. You make root visible, bounded, expensive, and unable to act silently.
> Agent governance is not intrinsic safety; it is runtime custody with conserved spendability, narrow seams, freshness bounds, and witnessed overrides.
> The real job is making every unsafe act either impossible to spend or impossible to hide.
