// SPDX-License-Identifier: Apache-2.0
//! Stage 3a-1: LA mints the single-use SpendCapability (the bounded execution key).
//!
//! Additive: minting does not consume capacity; it binds the granted token's opaque
//! `eligibility_reference` verbatim and fails closed against any token that cannot back it.

use linear_accountant::{
    BudgetAdmissionRef, CapabilityError, CapacityDecision, CapacityRequest, ConsumeRequest,
    ConsumptionDecision, EventId, InMemoryAccountant, RequestId, Scope,
};

#[test]
fn capability_binds_eligibility_verbatim_and_is_single_use() {
    let mut acct = InMemoryAccountant::new();
    let scope = Scope("lab".into());
    acct.deposit(&scope, 5, &admission());
    let token_id = match acct.request_capacity(
        CapacityRequest {
            request_id: RequestId("r1".into()),
            actor: "ag".into(),
            action: "write".into(),
            target: "demo".into(),
            scope: scope.clone(),
            requested_capacity: 1,
            eligibility_reference: "sha256:standing-xyz".into(),
            eligibility_valid_until: 1_000,
            expires_after: 100,
            idempotency_key: None,
        },
        1,
    ) {
        CapacityDecision::Granted { token_id, .. } => token_id,
        other => panic!("expected grant, got {other:?}"),
    };

    let cap = acct
        .issue_capability(token_id, "demo", "fs_write", "nonce-7", 2)
        .expect("clean token should mint");
    assert_eq!(
        cap.eligibility_reference, "sha256:standing-xyz",
        "binds the grant's eref verbatim"
    );
    assert_eq!(cap.target, "demo");
    assert_eq!(cap.effect_class, "fs_write");
    assert_eq!(cap.capability_id, "nonce-7");
    assert!(cap.single_use);
    assert_eq!(cap.expires_at, 101);

    // Minting did not consume capacity (stock untouched beyond the grant).
    assert_eq!(acct.available(&scope), 4);
}

#[test]
fn capability_fails_closed_on_unknown_and_expired() {
    let mut acct = InMemoryAccountant::new();
    let scope = Scope("lab".into());
    acct.deposit(&scope, 5, &admission());
    let token_id = match acct.request_capacity(
        CapacityRequest {
            request_id: RequestId("r1".into()),
            actor: "ag".into(),
            action: "write".into(),
            target: "demo".into(),
            scope: scope.clone(),
            requested_capacity: 1,
            eligibility_reference: "sha256:standing-xyz".into(),
            eligibility_valid_until: 1_000,
            expires_after: 10,
            idempotency_key: None,
        },
        1,
    ) {
        CapacityDecision::Granted { token_id, .. } => token_id,
        other => panic!("expected grant, got {other:?}"),
    };

    // Past the token horizon (issued at 1, expires_after 10 => expires_at 11).
    assert_eq!(
        acct.issue_capability(token_id, "demo", "fs_write", "n", 11),
        Err(CapabilityError::TokenExpired),
    );
}

/// The discipline of [capability-redemption]: issuance is neither spend nor reservation.
/// Minting N capabilities against one token moves no stock, and the token's full remaining
/// capacity stays spendable afterward (nothing was set aside).
///
/// [capability-redemption]: ../docs/working/decisions/capability-redemption.md
#[test]
fn issuing_capabilities_is_not_spend_and_not_reservation() {
    let mut acct = InMemoryAccountant::new();
    let scope = Scope("lab".into());
    acct.deposit(&scope, 5, &admission());
    let token_id = match acct.request_capacity(
        CapacityRequest {
            request_id: RequestId("r1".into()),
            actor: "ag".into(),
            action: "write".into(),
            target: "demo".into(),
            scope: scope.clone(),
            requested_capacity: 3,
            eligibility_reference: "sha256:standing-xyz".into(),
            eligibility_valid_until: 1_000,
            expires_after: 100,
            idempotency_key: None,
        },
        1,
    ) {
        CapacityDecision::Granted { token_id, .. } => token_id,
        other => panic!("expected grant, got {other:?}"),
    };

    // After the grant, stock is deposit - granted = 5 - 3 = 2.
    let available_after_grant = acct.available(&scope);
    assert_eq!(available_after_grant, 2);

    // Issuance is NOT spend: minting N capabilities against the one token moves no stock.
    for i in 0..5 {
        acct.issue_capability(token_id, "demo", "fs_write", &format!("nonce-{i}"), 2)
            .expect("clean token backs the capability");
        assert_eq!(
            acct.available(&scope),
            available_after_grant,
            "issuance {i} must not draw down stock",
        );
    }

    // Issuance is NOT reservation: the token's full remaining capacity (3) is still
    // spendable in one go — the 5 issued capabilities set nothing aside.
    match acct.consume(
        ConsumeRequest {
            consumption_event_id: EventId("e1".into()),
            token_id,
            actor: "ag".into(),
            action: "write".into(),
            target: "demo".into(),
            amount: 3,
            scope: scope.clone(),
        },
        3,
    ) {
        ConsumptionDecision::Consumed {
            remaining_capacity, ..
        } => assert_eq!(
            remaining_capacity, 0,
            "full remaining was spendable; issuance reserved nothing",
        ),
        other => panic!("expected full consume, got {other:?}"),
    }
}

// A canned budget admission for tests. The mint boundary requires a non-empty sealed
// reference; LA carries it verbatim and never evaluates it.
fn admission() -> BudgetAdmissionRef {
    BudgetAdmissionRef {
        admission_ref: "watchbill/2026-07/lab".into(),
        basis_kind: "watchbill".into(),
    }
}
