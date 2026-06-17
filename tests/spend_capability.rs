// SPDX-License-Identifier: Apache-2.0
//! Stage 3a-1: LA mints the single-use SpendCapability (the bounded execution key).
//!
//! Additive: minting does not consume capacity; it binds the granted token's opaque
//! `eligibility_reference` verbatim and fails closed against any token that cannot back it.

use linear_accountant::{
    CapabilityError, CapacityDecision, CapacityRequest, InMemoryAccountant, RequestId, Scope,
};

#[test]
fn capability_binds_eligibility_verbatim_and_is_single_use() {
    let mut acct = InMemoryAccountant::new();
    let scope = Scope("lab".into());
    acct.deposit(&scope, 5);
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
    assert_eq!(cap.eligibility_reference, "sha256:standing-xyz", "binds the grant's eref verbatim");
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
    acct.deposit(&scope, 5);
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
