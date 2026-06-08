//! WL-002 — contention. The falsification test WL-001 could not run.
//!
//! WL-001 was single-threaded, so `consume`'s exactly-once property came for free
//! from the exclusive `&mut self` borrow — the race was never run. This slice runs it:
//! N threads race one token holding exactly one unit of capacity. The claim under test
//! is the load-bearing one from the deployment note —
//!
//! > atomic consume is just a conditional write (CAS / row lock).
//!
//! The accountant is single-writer by design and not thread-safe. The serialization
//! point therefore lives in **consumer code**, here an `Arc<Mutex<…>>` — the toy
//! stand-in for the production CAS / Postgres row lock / DynamoDB conditional put.
//! This tests whether the consume mechanism is *correct when actually contended*. It
//! does NOT test lock-free distributed atomicity, which stays deferred. The crate is
//! not modified: zero API delta, consistent with freezing it as a reference boundary.
//!
//! Acceptance (from `docs/working/specimens/workload-specimens.md` WL-002):
//!   - exactly ONE effect crosses the boundary;
//!   - every loser receives a typed refusal (`InsufficientCapacity` / `AlreadyConsumed`);
//!   - the ledger stays coherent (one `Consumed`, refusals recorded, no torn state);
//!   - the witness correlates request/token WITHOUT `request_id` becoming the
//!     conservation key (the conservation key stays token/consumption).

use linear_accountant::*;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;

const RACERS: usize = 8;

/// Deposit one unit and mint a single token holding it. Returns the opaque token id.
/// The token has no public constructor — it can only be obtained this way.
fn mint_one_unit_token(acc: &mut InMemoryAccountant, scope: &Scope) -> TokenId {
    acc.deposit(scope, 1); // exactly one effect is affordable — ever.
    match acc.request_capacity(
        CapacityRequest {
            request_id: RequestId("req-grant".into()),
            actor: "agent-a".into(),
            action: "effect".into(),
            target: "shared".into(),
            scope: scope.clone(),
            requested_capacity: 1,
            eligibility_reference: "wicket-admission#1".into(),
            eligibility_valid_until: 1_000,
            expires_after: 1_000,
            idempotency_key: Some("req-grant".into()),
        },
        0,
    ) {
        CapacityDecision::Granted { token_id, .. } => token_id,
        other => panic!("expected Granted, got {other:?}"),
    }
}

/// The shared side effect double-spend would corrupt. Incremented ONLY behind a
/// `Consumed` verdict; asserting it lands on exactly 1 proves exactly-once empirically,
/// not just from the decision enum.
struct WriteOnceSink {
    crossings: AtomicUsize,
}

impl WriteOnceSink {
    fn new() -> Self {
        WriteOnceSink {
            crossings: AtomicUsize::new(0),
        }
    }
    fn cross(&self) {
        self.crossings.fetch_add(1, Ordering::SeqCst);
    }
    fn total(&self) -> usize {
        self.crossings.load(Ordering::SeqCst)
    }
}

/// One racer's record — a consumer-side testimony index. Note it pairs `request_id`
/// with `event_id` and the outcome OUTSIDE the accountant. This is how request/token
/// correlation is recovered without the accountant ever keying conservation on
/// `request_id`.
#[derive(Clone, Debug)]
struct RacerReport {
    request_id: String,
    event_id: String,
    won: bool,
}

#[test]
fn contention_distinct_events_exactly_one_effect_crosses() {
    let scope = Scope("contended".into());
    let mut acc = InMemoryAccountant::new();
    let token = mint_one_unit_token(&mut acc, &scope);

    let acc = Arc::new(Mutex::new(acc));
    let sink = Arc::new(WriteOnceSink::new());

    // N racers, each with a DISTINCT consumption event id, all drawing amount=1 from
    // the same one-unit token. This is the genuine double-spend race (not a replay):
    // distinct events mean the idempotency breaker does NOT short-circuit them — only
    // the linear remaining-capacity check can.
    let reports: Vec<RacerReport> = {
        let handles: Vec<_> = (0..RACERS)
            .map(|i| {
                let acc = Arc::clone(&acc);
                let sink = Arc::clone(&sink);
                let scope = scope.clone();
                thread::spawn(move || {
                    let request_id = format!("req-{i}");
                    let event_id = format!("evt-{i}");
                    let decision = {
                        let mut acc = acc.lock().unwrap();
                        acc.consume(
                            ConsumeRequest {
                                consumption_event_id: EventId(event_id.clone()),
                                token_id: token,
                                actor: "agent-a".into(),
                                action: "effect".into(),
                                target: "shared".into(),
                                amount: 1,
                                scope: scope.clone(),
                            },
                            1,
                        )
                    };
                    let won = match decision {
                        ConsumptionDecision::Consumed { .. } => {
                            // ONLY here does the effect cross the boundary.
                            sink.cross();
                            true
                        }
                        ConsumptionDecision::InsufficientCapacity { .. } => false,
                        other => panic!("unexpected decision under contention: {other:?}"),
                    };
                    RacerReport {
                        request_id,
                        event_id,
                        won,
                    }
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    };

    // 1. Exactly one effect crossed — proven from the shared sink, independent of the
    //    decision enum.
    assert_eq!(
        sink.total(),
        1,
        "exactly one effect must cross the boundary"
    );

    // 2. Exactly one racer won; the rest got a typed refusal.
    let winners: Vec<&RacerReport> = reports.iter().filter(|r| r.won).collect();
    assert_eq!(winners.len(), 1, "exactly one racer wins");
    assert_eq!(reports.iter().filter(|r| !r.won).count(), RACERS - 1);

    let acc = acc.lock().unwrap();

    // 3. Ledger coherent: exactly one Consumed for the token, RACERS-1 refusals, no
    //    torn state (remaining capacity is 0, status Exhausted).
    let consumed = acc
        .ledger()
        .iter()
        .filter(|r| matches!(&r.event, Event::Consumed { token_id, .. } if *token_id == token))
        .count();
    let refused = acc
        .ledger()
        .iter()
        .filter(|r| {
            matches!(&r.event,
            Event::ConsumeRefused { token_id, kind, .. }
                if *token_id == token && kind == "InsufficientCapacity")
        })
        .count();
    assert_eq!(consumed, 1, "ledger shows exactly one consumption");
    assert_eq!(refused, RACERS - 1, "every loser's refusal is recorded");
    let view = acc.inspect_token(token).unwrap();
    assert_eq!(view.remaining_capacity, 0);
    assert_eq!(view.status, TokenStatus::Exhausted);
    assert_eq!(view.consumption_count, 1);

    // 4. The witness independently confirms no double-spend from the immutable ledger.
    assert!(matches!(
        witness::testify_no_double_spend(acc.ledger(), token),
        witness::Testimony::NoDoubleSpend {
            consumption_events: 1,
            total_consumed: 1,
            replays_refused: 0,
            ..
        }
    ));

    // 5. Request/token correlation WITHOUT request_id as a conservation key. The
    //    accountant's ledger keys only on token_id + event_id (its conservation keys).
    //    To answer "which request won the race?", the witness joins the consumer-side
    //    index (RacerReport) against the ledger — request_id lives purely in the query
    //    layer. This is the WL-001 gap kept in its own layer, never pushed down.
    let index: HashMap<&str, &RacerReport> =
        reports.iter().map(|r| (r.event_id.as_str(), r)).collect();
    let winning_event = acc.ledger().iter().find_map(|r| match &r.event {
        Event::Consumed {
            token_id, event_id, ..
        } if *token_id == token => Some(event_id.as_str()),
        _ => None,
    });
    let winning_request = winning_event
        .and_then(|e| index.get(e))
        .map(|r| r.request_id.as_str());
    assert_eq!(winning_request, Some(winners[0].request_id.as_str()));
}

#[test]
fn contention_same_event_replay_breaker_holds_under_race() {
    // Same race, but every racer reuses ONE event id. Now the idempotency breaker is
    // the active defense, not the capacity check: the first to land consumes, the rest
    // are AlreadyConsumed replays — even though they arrive concurrently.
    let scope = Scope("contended-replay".into());
    let mut acc = InMemoryAccountant::new();
    let token = mint_one_unit_token(&mut acc, &scope);

    let acc = Arc::new(Mutex::new(acc));
    let sink = Arc::new(WriteOnceSink::new());

    let outcomes: Vec<ConsumptionDecision> = {
        let handles: Vec<_> = (0..RACERS)
            .map(|_| {
                let acc = Arc::clone(&acc);
                let sink = Arc::clone(&sink);
                let scope = scope.clone();
                thread::spawn(move || {
                    let decision = {
                        let mut acc = acc.lock().unwrap();
                        acc.consume(
                            ConsumeRequest {
                                consumption_event_id: EventId("evt-shared".into()),
                                token_id: token,
                                actor: "agent-a".into(),
                                action: "effect".into(),
                                target: "shared".into(),
                                amount: 1,
                                scope: scope.clone(),
                            },
                            1,
                        )
                    };
                    if matches!(decision, ConsumptionDecision::Consumed { .. }) {
                        sink.cross();
                    }
                    decision
                })
            })
            .collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    };

    assert_eq!(
        sink.total(),
        1,
        "exactly one effect crosses even with a reused event id"
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|d| matches!(d, ConsumptionDecision::Consumed { .. }))
            .count(),
        1
    );
    assert_eq!(
        outcomes
            .iter()
            .filter(|d| matches!(d, ConsumptionDecision::AlreadyConsumed { .. }))
            .count(),
        RACERS - 1,
        "losers are refused as replays, not as out-of-budget"
    );

    let acc = acc.lock().unwrap();
    // The witness counts the refused replays as evidence the breaker held.
    assert!(matches!(
        witness::testify_no_double_spend(acc.ledger(), token),
        witness::Testimony::NoDoubleSpend {
            consumption_events: 1,
            total_consumed: 1,
            replays_refused,
            ..
        } if replays_refused == RACERS - 1
    ));
}
