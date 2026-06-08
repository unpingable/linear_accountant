# Capability Composition — the hardness gradient

*Candidate architecture note. Not ratified. Names a surface; authorizes no build.
Companion to [V0_BOUNDARY.md](../../architecture/V0_BOUNDARY.md). This is the **capability** axis, not
the spend axis — keep them separate (see the three-axis doctrine below).*

## Status / discipline

- Candidate. Filed so the composition trap is not rediscovered the hard way.
- **No flow checker, no capability algebra, no implementation.** If tempted to build
  Tier 2/3 machinery: **consumer trigger absent.**

## Where this sits

> Spend is counted. Capability is **checked or judged**. Custody is held.

The Linear Accountant handles *spend* (arithmetic, conserved, sovereign-because-dumb).
It cannot handle *capability*: deciding whether an action/tool is admissible at all.
Open-ended tool authoring **cannot be made arithmetic** — novel capability admission
is irreducibly semantic. So capability has a *hardness gradient*, not a clean kernel.

## The core trap

> Composition is itself a minting operation. Capability is not closed under composition.

Approved primitives are not enough. The canonical example:

```
read_secret      — approved
network_send     — approved
read_secret ; network_send   — exfiltration
```

A **membership allowlist is forgeable by composition.** Checking that each primitive
is approved proves nothing about the composed effect. This is the same shape as the
spend-side forbidden flows: a contractible "each part is fine" laundered into a
non-existent "the whole is fine."

## The hardness gradient

| Tier | What it admits | Hardness | Cost |
|---|---|---|---|
| **0 — Fixed registry** | only preapproved tools | hardest, checkable | least expressive |
| **1 — Membership-bounded composition** | compositions of approved primitives | **weak** | composition-laundering risk (the trap above) |
| **2 — Flow-typed composition** | approved primitives + typed effects; checker reasons about the *composed* flow | harder, conservative | buys hardness with **false rejections** of some safe things |
| **3 — Open authoring** | arbitrary tools via semantic review | not hard-safe | witnessed, attributable, never *assumed* safe |

Two things to hold:

1. Tier 1 is a trap dressed as safety — membership ≠ flow.
2. Tier 2 is only as hard as its algebra: it must be **flow-typed** and reason about
   the composed effect, not primitive membership. Flow hardness is paid for in false
   rejections — there is no free conservative checker.
3. Tier 3 cannot be made arithmetic. It bottoms out in custody legibility
   (see [custody-legibility](custody-legibility.md)): admission is *witnessed and attributable*, never
   assumed hard-safe.

## The three-axis doctrine (why this is a separate note)

```
Spend       → counted        → Linear Accountant (this repo, running)   — arithmetic, sovereign-because-dumb
Capability  → checked/judged → semantic layer + this hardness gradient  — gradient, not kernel
Custody     → held           → witness / override-with-receipt          — legibility, not elimination
```

> Never pretend those are the same kind of refusal.

Conflating them is the root error: treating capability admission as if it were a
spend count (Tier 1's mistake), or treating spend conservation as if it delivered
capability safety (it doesn't), or treating either as custody (it can't).

## Opening triggers (when this stops being a note)

- A consumer admits agent-authored or composed tools and needs more than a membership allowlist.
- A real exfiltration-by-composition path is found in a live tool registry.
- Someone proposes a capability type rich enough to mint — the engineer-side of "no minting by the persuadable."

Until one fires: candidate only.

## Keepers

> Composition is minting.
> A membership allowlist is forgeable by composition.
> Capability is not closed under composition.
> Flow hardness is bought with false rejections.
