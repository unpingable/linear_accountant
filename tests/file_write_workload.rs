//! First real workload bound to the Linear Accountant: a controlled file write.
//!
//! This is *contact* — an actual effect (`std::fs::write`) gated by the accountant,
//! not a toy. It proves the sentence empirically:
//!
//! > Eligibility alone cannot execute; only consumed spendability crosses the effect boundary.
//!
//! Only the spend axis is exercised here. Capability judgement and custody anchoring
//! are out of scope for this slice.

use linear_accountant::*;
use std::path::PathBuf;

// --- Proposer vocabulary ---------------------------------------------------
// The proposer's language CANNOT express minting: `Action` has no token variant,
// `Proposal` carries no token field, and `TokenId` has no public constructor. The
// proposer can describe an effect and cite an eligibility reference; it can never
// hand itself spendability.
enum Action {
    WriteFile { path: PathBuf, contents: String },
}

struct Proposal {
    actor: String,
    eligibility_reference: String, // opaque pointer from a (possibly untrusted) oracle
    scope: Scope,
    request_id: String,
    event_id: String,
    action: Action,
}

// --- Execution boundary ----------------------------------------------------
// The only component that ever holds a token. It does not trust any memory of a
// prior allowance: every submission re-requests capacity and re-consumes against
// live accountant state. Only a `Consumed` verdict crosses the effect boundary.
#[derive(Debug)]
enum Outcome {
    Executed { receipt: ReceiptId },
    RefusedNoCapacity { reason: String },
    RefusedAlreadySpent,
    RefusedOther,
}

struct ExecutionBoundary<'a> {
    acc: &'a mut InMemoryAccountant,
    ttl: Tick,
}

impl ExecutionBoundary<'_> {
    fn submit(&mut self, p: &Proposal, now: Tick) -> Outcome {
        let action_kind: &str = match &p.action {
            Action::WriteFile { .. } => "write_file",
        };
        let target = match &p.action {
            Action::WriteFile { path, .. } => path.display().to_string(),
        };

        // Eligibility alone does NOT authorize the effect. The boundary must obtain
        // capacity from the accountant.
        let grant = self.acc.request_capacity(
            CapacityRequest {
                request_id: RequestId(p.request_id.clone()),
                actor: p.actor.clone(),
                action: action_kind.into(),
                target: target.clone(),
                scope: p.scope.clone(),
                requested_capacity: 1,
                eligibility_reference: p.eligibility_reference.clone(),
                eligibility_valid_until: now + self.ttl,
                expires_after: self.ttl,
                idempotency_key: Some(p.request_id.clone()),
            },
            now,
        );
        let token = match grant {
            CapacityDecision::Granted { token_id, .. } => token_id,
            CapacityDecision::Denied { denial_reason, .. } => {
                return Outcome::RefusedNoCapacity {
                    reason: denial_reason,
                };
            }
        };

        // A read is a snapshot, not standing: the boundary may peek at token state,
        // but the peek authorizes nothing. The effect is gated on `consume`, below —
        // never on this observation.
        let _snapshot = self.acc.inspect_token(token);

        // `consume` revalidates live token state. Exactly-once per event id.
        match self.acc.consume(
            ConsumeRequest {
                consumption_event_id: EventId(p.event_id.clone()),
                token_id: token,
                actor: p.actor.clone(),
                action: action_kind.into(),
                target,
                amount: 1,
                scope: p.scope.clone(),
            },
            now,
        ) {
            ConsumptionDecision::Consumed { receipt, .. } => {
                // ONLY here does the real effect occur.
                match &p.action {
                    Action::WriteFile { path, contents } => {
                        std::fs::write(path, contents).expect("workload file write");
                    }
                }
                // `receipt` is copyable evidence, not spendable authority: there is no
                // path that turns a ReceiptId back into a write.
                Outcome::Executed { receipt }
            }
            ConsumptionDecision::AlreadyConsumed { .. } => Outcome::RefusedAlreadySpent,
            _ => Outcome::RefusedOther,
        }
    }
}

#[test]
fn file_write_workload_eligibility_alone_cannot_execute() {
    let scope = Scope("fs_write".into());
    let mut acc = InMemoryAccountant::new();
    acc.deposit(&scope, 1); // exactly one write is affordable — ever.

    let path = std::env::temp_dir().join(format!("la_workload_{}.txt", std::process::id()));
    let _ = std::fs::remove_file(&path);

    let mut boundary = ExecutionBoundary {
        acc: &mut acc,
        ttl: 1000,
    };

    // 1. Full path: eligible proposal → token → consume → REAL write → receipt.
    let p1 = Proposal {
        actor: "agent-a".into(),
        eligibility_reference: "wicket-admission#1".into(),
        scope: scope.clone(),
        request_id: "req-1".into(),
        event_id: "evt-1".into(),
        action: Action::WriteFile {
            path: path.clone(),
            contents: "first".into(),
        },
    };
    let receipt_1 = match boundary.submit(&p1, 0) {
        Outcome::Executed { receipt } => receipt,
        other => panic!("expected Executed, got {other:?}"),
    };
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "first",
        "the effect occurred"
    );

    // 2. Idempotent resend (same request, same event id) → AlreadyConsumed → no second write.
    assert!(matches!(
        boundary.submit(&p1, 1),
        Outcome::RefusedAlreadySpent
    ));
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "first");

    // 3. A fresh attempt with PERFECTLY VALID eligibility but a spent budget is denied.
    //    Eligibility is contractible (the same warrant is cited again, legitimately);
    //    capacity is linear (already spent). The adversary holds a good warrant and
    //    still cannot write a second time.
    let p2 = Proposal {
        actor: "agent-a".into(),
        eligibility_reference: "wicket-admission#1".into(),
        scope: scope.clone(),
        request_id: "req-2".into(),
        event_id: "evt-2".into(),
        action: Action::WriteFile {
            path: path.clone(),
            contents: "second".into(),
        },
    };
    match boundary.submit(&p2, 2) {
        Outcome::RefusedNoCapacity { reason } => assert!(reason.contains("insufficient")),
        other => panic!("expected RefusedNoCapacity, got {other:?}"),
    }
    assert_eq!(
        std::fs::read_to_string(&path).unwrap(),
        "first",
        "the file never became 'second' — spend is linear"
    );

    // The receipt from step 1 is retrievable evidence — and it is *only* evidence:
    // there is no path that turns it back into a write. (Borrow of `acc` via the
    // boundary ends here, at the boundary's last use above.)
    assert!(acc.receipt(receipt_1).is_some());

    // 4. The witness can report the outcome — read-only, from the ledger. It is not
    //    allocation authority; it returns a Testimony with no path back into spend.
    let token = acc
        .ledger()
        .iter()
        .find_map(|r| match &r.event {
            Event::Granted { token_id, .. } => Some(*token_id),
            _ => None,
        })
        .expect("a token was minted during the workload");
    assert!(matches!(
        witness::testify_no_double_spend(acc.ledger(), token),
        witness::Testimony::NoDoubleSpend {
            consumption_events: 1,
            total_consumed: 1,
            ..
        }
    ));

    let _ = std::fs::remove_file(&path);
}
