//! Differential oracle — the executable twin of `verification/Ledger.lean`.
//!
//! The Lean model proves the abstract ledger satisfies: conservation
//! (`minted = available + Σ original`), per-token `original = remaining + consumed`,
//! and replay-refusal / no-double-consume. This test confirms the *real* Rust
//! `InMemoryAccountant` matches a faithful reference model of that same fold over
//! large randomized event sequences, and that the proven invariants hold on the
//! implementation after every operation.
//!
//! It adds no surface to the crate — it hardens the frozen boundary. Zero new deps:
//! randomness is a self-contained SplitMix-style PRNG, seeded deterministically so a
//! failure reproduces exactly (`seed`/`step` are printed on every assertion).
//!
//! Note: `UnknownToken` is not exercised — a `TokenId` has no public constructor, so
//! a forged id is unrepresentable from outside the crate. That inability is itself a
//! boundary property, not a coverage gap.

use linear_accountant::witness::{self, Testimony};
use linear_accountant::*;
use std::collections::{HashMap, HashSet};

const SCOPES: [&str; 2] = ["alpha", "beta"];

/// Deterministic PRNG (SplitMix64). No external dependency; reproducible per seed.
struct Rng(u64);
impl Rng {
    fn new(seed: u64) -> Self {
        Rng(seed)
    }
    fn next_u64(&mut self) -> u64 {
        self.0 = self.0.wrapping_add(0x9E3779B97F4A7C15);
        let mut z = self.0;
        z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
        z ^ (z >> 31)
    }
    /// Uniform in `[0, n)`. Caller guarantees `n > 0`.
    fn below(&mut self, n: u64) -> u64 {
        self.next_u64() % n
    }
}

#[derive(Clone)]
struct RefTok {
    scope: usize,
    original: u64,
    remaining: u64,
    consumed_events: HashSet<u64>,
    consumed_total: u64,
    revoked: bool,
    expires_at: u64,
}

/// Reference model mirroring `src/lib.rs` branch-for-branch.
struct RefModel {
    stock: [u64; 2],
    deposited: [u64; 2],
    toks: Vec<RefTok>,
    dedupe: HashMap<String, usize>,
}

#[derive(Clone, Copy, PartialEq, Debug)]
enum ReqOutcome {
    NewGrant(usize, u64),
    DedupGrant(usize, u64),
    Denied,
}
impl ReqOutcome {
    fn cat(self) -> ReqCat {
        match self {
            ReqOutcome::NewGrant(_, c) | ReqOutcome::DedupGrant(_, c) => ReqCat::Granted(c),
            ReqOutcome::Denied => ReqCat::Denied,
        }
    }
}

#[derive(PartialEq, Debug)]
enum ReqCat {
    Granted(u64),
    Denied,
}
#[derive(PartialEq, Debug)]
enum ConCat {
    Consumed(u64),
    AlreadyConsumed,
    Insufficient,
    Expired,
    Revoked,
    ScopeMismatch,
}
#[derive(PartialEq, Debug)]
enum RevCat {
    Revoked,
    AlreadyFinal,
}

impl RefModel {
    fn new() -> Self {
        RefModel {
            stock: [0; 2],
            deposited: [0; 2],
            toks: Vec::new(),
            dedupe: HashMap::new(),
        }
    }
    fn deposit(&mut self, scope: usize, amt: u64) {
        self.stock[scope] += amt;
        self.deposited[scope] += amt;
    }
    #[allow(clippy::too_many_arguments)]
    fn request(
        &mut self,
        key: String,
        scope: usize,
        amount: u64,
        elig_empty: bool,
        now: u64,
        valid_until: u64,
        expires_after: u64,
    ) -> ReqOutcome {
        // Branch order mirrors request_capacity: empty-elig, then dedup, then stale,
        // then zero, then insufficient stock, then grant.
        if elig_empty {
            return ReqOutcome::Denied;
        }
        if let Some(&i) = self.dedupe.get(&key) {
            return ReqOutcome::DedupGrant(i, self.toks[i].original);
        }
        if now >= valid_until {
            return ReqOutcome::Denied;
        }
        if amount == 0 {
            return ReqOutcome::Denied;
        }
        if self.stock[scope] < amount {
            return ReqOutcome::Denied;
        }
        self.stock[scope] -= amount;
        let i = self.toks.len();
        self.toks.push(RefTok {
            scope,
            original: amount,
            remaining: amount,
            consumed_events: HashSet::new(),
            consumed_total: 0,
            revoked: false,
            expires_at: now.saturating_add(expires_after),
        });
        self.dedupe.insert(key, i);
        ReqOutcome::NewGrant(i, amount)
    }
    fn consume(&mut self, i: usize, event: u64, amount: u64, req_scope: usize, now: u64) -> ConCat {
        // Branch order mirrors consume: replay, revoked, expired, scope, insufficient.
        let t = &mut self.toks[i];
        if t.consumed_events.contains(&event) {
            return ConCat::AlreadyConsumed;
        }
        if t.revoked {
            return ConCat::Revoked;
        }
        if now >= t.expires_at {
            return ConCat::Expired;
        }
        if req_scope != t.scope {
            return ConCat::ScopeMismatch;
        }
        if amount > t.remaining {
            return ConCat::Insufficient;
        }
        t.remaining -= amount;
        t.consumed_total += amount;
        t.consumed_events.insert(event);
        ConCat::Consumed(t.remaining)
    }
    fn revoke(&mut self, i: usize) -> RevCat {
        let t = &mut self.toks[i];
        if t.revoked {
            RevCat::AlreadyFinal
        } else {
            t.revoked = true;
            RevCat::Revoked
        }
    }
}

fn req_cat(d: &CapacityDecision) -> ReqCat {
    match d {
        CapacityDecision::Granted {
            granted_capacity, ..
        } => ReqCat::Granted(*granted_capacity),
        CapacityDecision::Denied { .. } => ReqCat::Denied,
    }
}
fn con_cat(d: &ConsumptionDecision) -> ConCat {
    use ConsumptionDecision::*;
    match d {
        Consumed {
            remaining_capacity, ..
        } => ConCat::Consumed(*remaining_capacity),
        AlreadyConsumed { .. } => ConCat::AlreadyConsumed,
        InsufficientCapacity { .. } => ConCat::Insufficient,
        Expired { .. } => ConCat::Expired,
        Revoked { .. } => ConCat::Revoked,
        ScopeMismatch { .. } => ConCat::ScopeMismatch,
        UnknownToken { .. } => panic!("UnknownToken unreachable: TokenId is unforgeable"),
    }
}
fn rev_cat(d: &RevocationDecision) -> RevCat {
    match d {
        RevocationDecision::Revoked { .. } => RevCat::Revoked,
        RevocationDecision::AlreadyFinal { .. } => RevCat::AlreadyFinal,
    }
}

fn scope_of(i: usize) -> Scope {
    Scope(SCOPES[i].into())
}

/// Conservation + per-token identity, checked on the real accountant after every op.
fn check_invariants(
    acc: &InMemoryAccountant,
    model: &RefModel,
    tokens: &[(TokenId, usize)],
    seed: u64,
    step: usize,
) {
    let mut orig_sum = [0u64; 2];
    for &(tid, i) in tokens {
        let v = acc
            .inspect_token(tid)
            .expect("granted token must be inspectable");
        let rt = &model.toks[i];
        orig_sum[rt.scope] += v.original_capacity;
        assert_eq!(
            v.remaining_capacity, rt.remaining,
            "seed {seed} step {step}: remaining diverged"
        );
        // original = remaining + consumed (the affine spend identity, no restock)
        assert_eq!(
            v.original_capacity,
            v.remaining_capacity + rt.consumed_total,
            "seed {seed} step {step}: original != remaining + consumed"
        );
    }
    // `s` indexes four parallel arrays (stock, deposited, orig_sum, scope_of) — the
    // canonical case where `needless_range_loop`'s single-array enumerate rewrite is a
    // false positive. (Pre-existing; surfaced by a rustfmt/clippy toolchain bump.)
    #[allow(clippy::needless_range_loop)]
    for s in 0..2 {
        let avail = acc.available(&scope_of(s));
        assert_eq!(
            model.stock[s], avail,
            "seed {seed} step {step}: stock diverged on scope {s}"
        );
        // conservation: minted == available + Σ original
        assert_eq!(
            model.deposited[s],
            avail + orig_sum[s],
            "seed {seed} step {step}: conservation broken on scope {s}"
        );
    }
}

/// Witness independently confirms no double-spend, and its tally matches the model.
fn check_witness(
    acc: &InMemoryAccountant,
    model: &RefModel,
    tokens: &[(TokenId, usize)],
    seed: u64,
    step: usize,
) {
    for &(tid, i) in tokens {
        match witness::testify_no_double_spend(acc.ledger(), tid) {
            Testimony::NoDoubleSpend { total_consumed, .. } => assert_eq!(
                total_consumed, model.toks[i].consumed_total,
                "seed {seed} step {step}: witness tally diverged"
            ),
            Testimony::DoubleSpendObserved {
                reused_event_id, ..
            } => panic!(
                "seed {seed} step {step}: witness observed double-spend (event {reused_event_id})"
            ),
        }
    }
}

fn run_seed(seed: u64) {
    let mut acc = InMemoryAccountant::new();
    let mut model = RefModel::new();
    let mut assoc: HashMap<TokenId, usize> = HashMap::new();
    let mut tokens: Vec<(TokenId, usize)> = Vec::new();
    let mut rng = Rng::new(seed);
    let mut now: u64 = 0;
    let mut req_counter: u64 = 0;
    let mut evt_counter: u64 = 0;

    for step in 0..1500 {
        if rng.below(4) == 0 {
            now += rng.below(5);
        }
        match rng.below(10) {
            0..=2 => {
                let scope = rng.below(2) as usize;
                let amt = rng.below(20);
                acc.deposit(&scope_of(scope), amt);
                model.deposit(scope, amt);
            }
            3..=6 => {
                let scope = rng.below(2) as usize;
                let amount = rng.below(8);
                let elig_empty = rng.below(8) == 0;
                let reuse = rng.below(4) == 0 && req_counter > 0;
                let key_n = if reuse {
                    rng.below(req_counter)
                } else {
                    let n = req_counter;
                    req_counter += 1;
                    n
                };
                let use_idem = rng.below(2) == 0;
                let request_id = format!("r{key_n}");
                let idem = if use_idem {
                    Some(format!("k{key_n}"))
                } else {
                    None
                };
                let dedupe_key = idem.clone().unwrap_or_else(|| request_id.clone());
                let valid_until = now + rng.below(4); // sometimes == now -> stale
                let expires_after = 1 + rng.below(6);
                let elig = if elig_empty {
                    String::new()
                } else {
                    format!("e{key_n}")
                };
                let req = CapacityRequest {
                    request_id: RequestId(request_id),
                    actor: "agent".into(),
                    action: "act".into(),
                    target: "tgt".into(),
                    scope: scope_of(scope),
                    requested_capacity: amount,
                    eligibility_reference: elig,
                    eligibility_valid_until: valid_until,
                    expires_after,
                    idempotency_key: idem,
                };
                let d = acc.request_capacity(req, now);
                let outcome = model.request(
                    dedupe_key,
                    scope,
                    amount,
                    elig_empty,
                    now,
                    valid_until,
                    expires_after,
                );
                assert_eq!(
                    req_cat(&d),
                    outcome.cat(),
                    "seed {seed} step {step}: request category diverged"
                );
                if let CapacityDecision::Granted { token_id, .. } = d {
                    match outcome {
                        ReqOutcome::NewGrant(i, _) => {
                            assoc.insert(token_id, i);
                            tokens.push((token_id, i));
                        }
                        ReqOutcome::DedupGrant(i, _) => assert_eq!(
                            assoc.get(&token_id),
                            Some(&i),
                            "seed {seed} step {step}: dedup returned wrong token"
                        ),
                        ReqOutcome::Denied => unreachable!(),
                    }
                }
            }
            7..=8 => {
                if tokens.is_empty() {
                    continue;
                }
                let (tid, i) = tokens[rng.below(tokens.len() as u64) as usize];
                let scope = if rng.below(5) == 0 {
                    rng.below(2) as usize // sometimes wrong scope
                } else {
                    model.toks[i].scope
                };
                let amount = rng.below(6);
                let reuse_evt = rng.below(3) == 0 && evt_counter > 0;
                let evt = if reuse_evt {
                    rng.below(evt_counter)
                } else {
                    let e = evt_counter;
                    evt_counter += 1;
                    e
                };
                let req = ConsumeRequest {
                    consumption_event_id: EventId(format!("ev{evt}")),
                    token_id: tid,
                    actor: "agent".into(),
                    action: "act".into(),
                    target: "tgt".into(),
                    amount,
                    scope: scope_of(scope),
                };
                let d = acc.consume(req, now);
                let cat = model.consume(i, evt, amount, scope, now);
                assert_eq!(
                    con_cat(&d),
                    cat,
                    "seed {seed} step {step}: consume category diverged"
                );
            }
            _ => {
                if tokens.is_empty() {
                    continue;
                }
                let (tid, i) = tokens[rng.below(tokens.len() as u64) as usize];
                let d = acc.revoke(tid, "test", now);
                let cat = model.revoke(i);
                assert_eq!(
                    rev_cat(&d),
                    cat,
                    "seed {seed} step {step}: revoke category diverged"
                );
            }
        }

        check_invariants(&acc, &model, &tokens, seed, step);
        if step % 25 == 0 {
            check_witness(&acc, &model, &tokens, seed, step);
        }
    }
    check_witness(&acc, &model, &tokens, seed, usize::MAX);
}

#[test]
fn rust_matches_lean_model_over_random_sequences() {
    for seed in [1u64, 2, 3, 7, 11, 42, 1337, 0xDEAD_BEEF] {
        run_seed(seed);
    }
}
