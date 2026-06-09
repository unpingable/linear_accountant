//! v0 reference boundary tests.
//!
//! These run from a *separate crate*, so they cannot reach private fields. In
//! particular they cannot construct a `TokenId` — the only way to obtain one is a
//! successful grant. That inability is itself the proof of test #1.

use linear_accountant::*;

fn deploy() -> Scope {
    Scope("deploy".into())
}

fn request(req_id: &str, cap: u64, elig: &str) -> CapacityRequest {
    CapacityRequest {
        request_id: RequestId(req_id.into()),
        actor: "agent-a".into(),
        action: "restart_service".into(),
        target: "svc-1".into(),
        scope: deploy(),
        requested_capacity: cap,
        eligibility_reference: elig.into(),
        eligibility_valid_until: 1_000_000,
        expires_after: 100,
        idempotency_key: None,
    }
}

fn consume(token_id: TokenId, event: &str, amount: u64) -> ConsumeRequest {
    ConsumeRequest {
        consumption_event_id: EventId(event.into()),
        token_id,
        actor: "agent-a".into(),
        action: "restart_service".into(),
        target: "svc-1".into(),
        amount,
        scope: deploy(),
    }
}

fn token_of(d: CapacityDecision) -> TokenId {
    match d {
        CapacityDecision::Granted { token_id, .. } => token_id,
        other => panic!("expected Granted, got {other:?}"),
    }
}

// --- Required test 1 ------------------------------------------------------
#[test]
fn eligibility_alone_cannot_execute() {
    let mut acc = InMemoryAccountant::new();
    // Scope has NO stock. The request is perfectly eligible...
    let d = acc.request_capacity(request("r1", 1, "elig-1"), 0);
    // ...and is still denied. Eligibility is a request, not payment.
    assert!(matches!(d, CapacityDecision::Denied { .. }));

    // A missing eligibility reference is also refused outright.
    let d2 = acc.request_capacity(request("r2", 1, ""), 0);
    assert!(matches!(d2, CapacityDecision::Denied { .. }));

    // There is no `consume(eligibility)` path: consume requires a TokenId, and a
    // TokenId can only come from a grant. This test crate cannot fabricate one.
}

// --- Required test 2 ------------------------------------------------------
#[test]
fn token_can_be_consumed_once() {
    let mut acc = InMemoryAccountant::new();
    acc.deposit(&deploy(), 5);
    let t = token_of(acc.request_capacity(request("r1", 1, "elig-1"), 0));
    let d = acc.consume(consume(t, "e1", 1), 1);
    assert!(matches!(
        d,
        ConsumptionDecision::Consumed {
            remaining_capacity: 0,
            ..
        }
    ));
}

// --- Required test 3 ------------------------------------------------------
#[test]
fn consumed_token_cannot_be_consumed_again() {
    let mut acc = InMemoryAccountant::new();
    acc.deposit(&deploy(), 5);
    let t = token_of(acc.request_capacity(request("r1", 1, "elig-1"), 0));
    let _ = acc.consume(consume(t, "e1", 1), 1);
    // Fresh event id, exhausted token.
    let d = acc.consume(consume(t, "e2", 1), 2);
    assert!(matches!(
        d,
        ConsumptionDecision::InsufficientCapacity {
            remaining_capacity: 0,
            ..
        }
    ));
}

// --- Required test 4 ------------------------------------------------------
#[test]
fn receipt_cannot_be_used_as_token() {
    let mut acc = InMemoryAccountant::new();
    acc.deposit(&deploy(), 5);
    let t = token_of(acc.request_capacity(request("r1", 1, "elig-1"), 0));
    let consumed = acc.consume(consume(t, "e1", 1), 1);
    let _receipt = match consumed {
        ConsumptionDecision::Consumed { receipt, .. } => receipt,
        other => panic!("expected Consumed, got {other:?}"),
    };

    // `_receipt` is a ReceiptId. It is type-distinct from TokenId: there is NO
    // accountant method that accepts a ReceiptId to mint or consume. The receipt
    // grants nothing — re-consuming the token it documents still fails, because
    // the receipt did not regenerate capacity.
    let d = acc.consume(consume(t, "e2", 1), 1);
    assert!(matches!(
        d,
        ConsumptionDecision::InsufficientCapacity { .. }
    ));
}

// --- Required test 5 ------------------------------------------------------
#[test]
fn same_idempotency_key_does_not_mint_duplicate_capacity() {
    let mut acc = InMemoryAccountant::new();
    acc.deposit(&deploy(), 5);

    let mut r = request("r1", 2, "elig-1");
    r.idempotency_key = Some("key-A".into());
    let first = token_of(acc.request_capacity(r.clone(), 0));
    assert_eq!(acc.available(&deploy()), 3); // 5 - 2

    // Same idempotency key, even with a different request_id, mints nothing new.
    let mut r2 = r.clone();
    r2.request_id = RequestId("r2-different".into());
    let second = token_of(acc.request_capacity(r2, 0));

    assert_eq!(first, second, "same key must return the same token");
    assert_eq!(acc.available(&deploy()), 3, "stock must not be drawn twice");
}

// Refactor-guard, not an exposure fix. With no idempotency_key, request_id IS the
// fallback dedupe key (lib.rs: `idempotency_key.unwrap_or_else(|| request_id)`).
// Every keyless caller relies on this branch, but the suite only exercises it
// incidentally (the `request()` helper always passes `idempotency_key: None`, yet
// no other test replays the *same* request_id). This pins that fallback as
// intentional: a future cleanup that drops the `unwrap_or_else` would silently turn
// every keyless caller into a duplicate-minter, and this test would catch it.
#[test]
fn keyless_replay_of_same_request_id_does_not_mint_duplicate_capacity() {
    let mut acc = InMemoryAccountant::new();
    acc.deposit(&deploy(), 5);

    let r = request("r1", 2, "elig-1"); // idempotency_key: None
    let first = token_of(acc.request_capacity(r.clone(), 0));
    assert_eq!(acc.available(&deploy()), 3); // 5 - 2

    // Same request_id, still no key: the second call must replay the original grant.
    let second = token_of(acc.request_capacity(r, 0));

    assert_eq!(first, second, "same request_id must return the same token");
    assert_eq!(acc.available(&deploy()), 3, "stock must not be drawn twice");
}

// --- Headline invariant: contractible eligibility, linear capacity --------
#[test]
fn eligibility_is_contractible_but_capacity_is_linear() {
    let mut acc = InMemoryAccountant::new();
    acc.deposit(&deploy(), 2);
    // The SAME eligibility fact, cited three times across distinct requests.
    let a = acc.request_capacity(request("r1", 1, "elig-shared"), 0);
    let b = acc.request_capacity(request("r2", 1, "elig-shared"), 0);
    let c = acc.request_capacity(request("r3", 1, "elig-shared"), 0);

    assert!(matches!(a, CapacityDecision::Granted { .. }));
    assert!(matches!(b, CapacityDecision::Granted { .. }));
    // Eligibility is still valid; the stock is gone. valid(x) ∧ valid(x) ≡ valid(x),
    // but [A] ⊬ A ⊗ A.
    assert!(matches!(c, CapacityDecision::Denied { .. }));
    assert_eq!(acc.available(&deploy()), 0);
}

// --- Freshness: the temporal circuit breaker ------------------------------
#[test]
fn stale_eligibility_is_a_distinct_breach() {
    let mut acc = InMemoryAccountant::new();
    acc.deposit(&deploy(), 5);

    let mut r = request("r1", 1, "elig-1");
    r.eligibility_valid_until = 10; // warrant fresh only until t=10

    // Fresh request at t=5 mints normally.
    assert!(matches!(
        acc.request_capacity(r.clone(), 5),
        CapacityDecision::Granted { .. }
    ));

    // Same warrant, new request id, asked at t=20: stale. Refused — and the
    // refusal is distinct from "missing eligibility" and from "insufficient stock".
    let mut late = r.clone();
    late.request_id = RequestId("r-late".into());
    match acc.request_capacity(late, 20) {
        CapacityDecision::Denied { denial_reason, .. } => {
            assert!(denial_reason.contains("stale"), "got: {denial_reason}");
        }
        other => panic!("expected stale Denied, got {other:?}"),
    }
    // Stock was not touched by the stale request.
    assert_eq!(acc.available(&deploy()), 4);
}

// --- The goblin's other hostile answers -----------------------------------
#[test]
fn replayed_event_is_idempotent() {
    let mut acc = InMemoryAccountant::new();
    acc.deposit(&deploy(), 5);
    let t = token_of(acc.request_capacity(request("r1", 2, "elig-1"), 0));
    let first = acc.consume(consume(t, "e1", 1), 1);
    assert!(matches!(first, ConsumptionDecision::Consumed { .. }));
    // Re-sending the same event id consumes nothing more. The real idempotency
    // property is "no additional capacity consumed" — remaining is unchanged.
    let replay = acc.consume(consume(t, "e1", 1), 1);
    assert!(matches!(
        replay,
        ConsumptionDecision::AlreadyConsumed { .. }
    ));
    assert_eq!(acc.inspect_token(t).unwrap().remaining_capacity, 1);
}

// --- Custody legibility (in-process): the sovereign act is not silent ------
#[test]
fn custodial_deposit_is_recorded() {
    let mut acc = InMemoryAccountant::new();
    // Minting stock into existence — the single most sovereign act — leaves a record.
    acc.deposit(&deploy(), 5);
    let deposited = acc
        .ledger()
        .iter()
        .any(|r| matches!(&r.event, Event::Deposited { amount: 5, .. }));
    assert!(deposited, "deposit must not be silent");
}

// --- The contact loop's final step: witness can testify --------------------
#[test]
fn witness_can_testify_no_double_spend() {
    let mut acc = InMemoryAccountant::new();
    acc.deposit(&deploy(), 2);
    let token = token_of(acc.request_capacity(request("r1", 2, "elig-1"), 0));
    let _ = acc.consume(consume(token, "e1", 1), 1); // Consumed
    let _ = acc.consume(consume(token, "e1", 1), 1); // replay → refused, recorded

    // The witness reads the immutable ledger (read-only — it cannot allocate) and
    // independently confirms the breaker held.
    let summary = match witness::testify_no_double_spend(acc.ledger(), token) {
        witness::Testimony::NoDoubleSpend {
            consumption_events,
            total_consumed,
            replays_refused,
            ..
        } => (consumption_events, total_consumed, replays_refused),
        other => panic!("expected NoDoubleSpend, got {other:?}"),
    };
    assert_eq!(summary, (1, 1, 1)); // one consume, one unit spent, one replay refused.

    // Testimony is NOT allocation authority: it is a `Testimony`, type-distinct from
    // any token, with no path back into request_capacity/consume.
}

#[test]
fn expired_token_cannot_be_consumed() {
    let mut acc = InMemoryAccountant::new();
    acc.deposit(&deploy(), 5);
    // expires_after = 100, issued at now = 0 → expires_at = 100.
    let t = token_of(acc.request_capacity(request("r1", 1, "elig-1"), 0));
    let d = acc.consume(consume(t, "e1", 1), 100);
    assert!(matches!(
        d,
        ConsumptionDecision::Expired {
            expired_at: 100,
            ..
        }
    ));
}

#[test]
fn scope_mismatch_is_refused() {
    let mut acc = InMemoryAccountant::new();
    acc.deposit(&deploy(), 5);
    let t = token_of(acc.request_capacity(request("r1", 1, "elig-1"), 0));
    let mut c = consume(t, "e1", 1);
    c.scope = Scope("read-only".into());
    let d = acc.consume(c, 1);
    assert!(matches!(d, ConsumptionDecision::ScopeMismatch { .. }));
}

#[test]
fn revoked_token_cannot_be_consumed() {
    let mut acc = InMemoryAccountant::new();
    acc.deposit(&deploy(), 5);
    let t = token_of(acc.request_capacity(request("r1", 2, "elig-1"), 0));
    let r = acc.revoke(t, "operator pulled it", 1);
    assert!(matches!(r, RevocationDecision::Revoked { .. }));
    let d = acc.consume(consume(t, "e1", 1), 2);
    assert!(matches!(d, ConsumptionDecision::Revoked { .. }));
    // Second revoke is final, not a fresh effect.
    assert!(matches!(
        acc.revoke(t, "again", 3),
        RevocationDecision::AlreadyFinal { .. }
    ));
}

// --- The toy consumer scenario from the role note -------------------------
#[test]
fn restart_service_scenario() {
    // Wicket (elsewhere) says: agent-a is eligible to restart svc-1.
    // It hands the accountant a reference to that admission, not authority.
    let mut acc = InMemoryAccountant::new();
    acc.deposit(&deploy(), 1); // exactly one restart is affordable

    let token =
        token_of(acc.request_capacity(request("req-restart-1", 1, "wicket-admission#abc"), 0));

    // Execution consumes the token to perform the restart.
    let first = acc.consume(consume(token, "restart-attempt-1", 1), 1);
    assert!(matches!(first, ConsumptionDecision::Consumed { .. }));

    // A replayed restart with a NEW attempt id finds no capacity left.
    let replay = acc.consume(consume(token, "restart-attempt-2", 1), 1);
    assert!(matches!(
        replay,
        ConsumptionDecision::InsufficientCapacity { .. }
    ));

    // Full contact loop closes: the witness can testify, after the fact, that the
    // one restart was spent exactly once and never double-spent.
    assert!(matches!(
        witness::testify_no_double_spend(acc.ledger(), token),
        witness::Testimony::NoDoubleSpend {
            consumption_events: 1,
            total_consumed: 1,
            ..
        }
    ));
}
