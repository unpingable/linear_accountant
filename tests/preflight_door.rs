//! The preflight door — refusal-only, mutation-free. "Doors are allowed; hidden rooms
//! are not." These tests pin the two promises that keep the door from becoming a thaw:
//! it refuses with a typed reason, and it cannot mutate state (the signature takes no
//! accountant, so there is nothing to mutate).

use linear_accountant::preflight::*;

fn knock() -> PreflightInquiry {
    PreflightInquiry {
        consumer: "maude".into(),
        boundary: "write_tool_approval".into(),
        principal: "nightshift-bot".into(),
        action_class: "write_tool".into(),
        repo: "linearaccountant".into(),
        estimated_units: 1,
        basis_receipt: "watchbill:abc123".into(),
    }
}

#[test]
fn door_refuses_with_typed_reason_and_never_mutates() {
    let d = preflight_consume(&knock());
    assert_eq!(d.refusal, PreflightRefusal::ConsumePathNotThawed);
    assert!(d.consumer_trigger_required);
    assert!(!d.mutation_performed, "the door must not mutate state");
}

#[test]
fn door_echoes_observed_boundary_and_names_the_trigger() {
    let d = preflight_consume(&knock());
    assert_eq!(d.observed_boundary, "maude.write_tool_approval");
    assert!(
        d.expected_trigger.contains("consume()"),
        "the refusal must name the trigger that would change the answer"
    );
}

#[test]
fn door_is_pure() {
    // The door takes no accountant and no `&mut` anything: it structurally cannot touch
    // stock, tokens, or the ledger. Two knocks yield identical answers.
    assert_eq!(preflight_consume(&knock()), preflight_consume(&knock()));
}
