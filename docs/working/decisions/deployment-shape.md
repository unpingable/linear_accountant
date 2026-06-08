# Deployment shape — the gateway, not the constitution

*Candidate. The operational deflation of the doctrine: what actually ships is small
and boring. Not a build order; no slice opened. The conserved core already exists —
the v0 crate is the four-verb stateful object. Companion to
[V0_BOUNDARY.md](../../architecture/V0_BOUNDARY.md), [custody-legibility.md](custody-legibility.md).*

## The runtime heart

```
before execute(tool, args):
    consume(token)
    if ok:
        execute(tool, args)
        emit receipt
    else:
        refuse
        emit receipt
```

That is the whole runtime. Everything in the doctrine exists to justify these six lines.

> **Nobody deploys a constitution. They deploy a gateway with a `consume()` call.**

## It's a shape ops has run for decades

A small, strongly-consistent stateful service with a four-verb API
(`request` / `consume` / `inspect` / `+ deposit`/`revoke`), backed by a transactional
store where **atomic consume is just a conditional write** — CAS, a Postgres row lock,
a DynamoDB conditional put. HA is the lock-service story (Zookeeper-shaped, 20 years
old). The witness is an append-only log shipped to whatever SIEM exists, plus DSSE
attestation.

Vulgar handles for humans:

- **OPA with memory** (stateful where OPA is stateless).
- **Vault for actions instead of secrets** (capacity under custody, not secrets).

Nobody asks how to operationalize Vault. It's a sidecar and a habit.

## Integration point: the tool-call chokepoint

Agent stacks already funnel every effect through one place — the tool-call dispatcher.
You don't touch the model, training, or app. You wrap the dispatcher: before
`execute(tool, args)`, require `consume(token)`. With MCP there's a standard slot — a
policy-enforcement gateway in front of the MCP server, a product category that already
exists. Today those PEPs are stateless allow/deny. **We are the conserved core they're
missing. The slot is built; this is the thing that goes in it.**

## Rollout (the ops playbook)

```
observe / shadow:
  mint + "consume" virtually; block nothing; witness records what WOULD have refused.
  Zero blast radius. After a month the witness data says "agents attempted N
  double-spends" — that is both the evidence base and the entire pitch.

limited enforcement:
  enforce on ONE boring, low-stakes effect class. Explicit fail-closed/fail-open policy.

ratchet:
  more effect classes; stronger custody; witness everything.
```

## The four honest hard parts (none mystical)

1. **Effect taxonomy.** What counts as one spendable unit per domain? Real
   per-deployment work, doesn't generalize — but it's the SLI-definition kind of hard.
2. **Atomic consume.** Conditional write / row lock / CAS / single-writer service.
   *This is exactly what specimen WL-002 (contention) is the falsification test for.*
3. **Fail-open politics.** The first time the accountant being down blocks a
   revenue-touching agent, someone demands fail-open. That is the custody fight arriving
   on schedule. **The fail-open toggle emits a receipt from day one — non-negotiable,
   designed in before anyone asks.** This is the runtime form of
   [custody-legibility](custody-legibility.md): the override exists; it cannot be quiet.
4. **Ownership.** Platform/ops owns agent-effect custody because platform/ops already
   owns prod-effect custody. This quietly *answers the custody-root question*: agent
   authority lands with the team that already holds prod credentials. Custody is
   assigned, not eliminated — and assignment is politics, not design.

> **It's your job, generalized.** Not "solve agent safety" — put a small stateful
> custody service in front of effect execution, start in shadow mode, and let the
> witness data tell you where the real pressure is.

## How this lands on what exists

- **v0 crate** = the conserved core (the four verbs, ledger, witness). Deployment wraps
  it in a consistent store + the dispatcher hook.
- **WL-002 (contention)** = the falsification test for hard-part #2. The gateway is only
  as sound as its atomic consume; contention is how you find out it holds.
- **Latency is not on the list.** One conditional write against tool calls measured in
  hundreds of ms is noise.

## What this is NOT

- NOT opening a slice. NOT a production service to build now. The consumer trigger still
  gates: a real agent stack wanting `consume()` on its dispatcher.
- The smallest real first step, when a consumer says go, is the **shadow-mode dispatcher
  shim** — sprint-sized, blocks nothing, produces witness data. Not the cathedral.
