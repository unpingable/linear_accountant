// SPDX-License-Identifier: Apache-2.0
//! Transport contract for the `la_cli` binary (consumer-trigger build).
//!
//! The binary is a thin transport that adds no policy: each command maps 1:1 to
//! an `InMemoryAccountant` method. These tests drive the real binary over its
//! stdin/stdout line protocol and assert the decisions + fail-closed behavior
//! (malformed input, unknown command, unknown token handle). They are the
//! owning repo's proof that the transport preserves the boundary's contract:
//! replay-kill, exhaustion, and scope mismatch all refuse, and nothing
//! malformed produces a silent allow.

use std::io::Write;
use std::process::{Command, Stdio};

/// Drive the binary with a script (one command per line) and return the
/// response lines (banner stripped).
fn run(script: &str) -> Vec<String> {
    let mut child = Command::new(env!("CARGO_BIN_EXE_la_cli"))
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("spawn la_cli");
    child
        .stdin
        .take()
        .unwrap()
        .write_all(script.as_bytes())
        .expect("write stdin");
    let out = child.wait_with_output().expect("wait");
    assert!(out.status.success(), "la_cli exited non-zero");
    let text = String::from_utf8(out.stdout).expect("utf8");
    let mut lines: Vec<String> = text.lines().map(|s| s.to_string()).collect();
    // First line is the banner.
    assert!(
        lines[0].contains("\"protocol\":\"v0\""),
        "missing banner: {:?}",
        lines.first()
    );
    assert!(lines[0].contains("linear-accountant"));
    lines.remove(0);
    lines
}

fn deposit(scope: &str, amount: u64) -> String {
    // The mint boundary must cite a budget admission; the CLI requires `admission_ref`.
    format!("cmd=deposit\tscope={scope}\tamount={amount}\tadmission_ref=adm_budget\tbasis_kind=watchbill")
}

fn request(scope: &str, cap: u64) -> String {
    format!(
        "cmd=request_capacity\trequest_id=r1\tactor=w\taction=tool_effect\ttarget={scope}\t\
         scope={scope}\trequested_capacity={cap}\teligibility_reference=adm_lab\t\
         eligibility_valid_until=100\texpires_after=100\ttick=1"
    )
}

fn consume(event: &str, scope: &str) -> String {
    format!(
        "cmd=consume\tconsumption_event_id={event}\ttoken_id=t0\tactor=w\taction=tool_effect\t\
         target={scope}\tamount=1\tscope={scope}\ttick=2"
    )
}

#[test]
fn happy_path_deposit_grant_consume() {
    let script = format!(
        "{}\n{}\n{}\n",
        deposit("lab/s", 1),
        request("lab/s", 1),
        consume("e1", "lab/s"),
    );
    let r = run(&script);
    assert!(r[0].contains("\"event\":\"deposited\""), "{}", r[0]);
    assert!(r[1].contains("\"decision\":\"Granted\""), "{}", r[1]);
    assert!(r[1].contains("\"token_id\":\"t0\""), "{}", r[1]);
    assert!(r[2].contains("\"decision\":\"Consumed\""), "{}", r[2]);
    assert!(r[2].contains("\"remaining_capacity\":0"), "{}", r[2]);
}

#[test]
fn replay_of_event_id_is_already_consumed() {
    let script = format!(
        "{}\n{}\n{}\n{}\n",
        deposit("lab/s", 5),
        request("lab/s", 5),
        consume("dup", "lab/s"),
        consume("dup", "lab/s"),
    );
    let r = run(&script);
    assert!(r[2].contains("\"decision\":\"Consumed\""), "{}", r[2]);
    assert!(
        r[3].contains("\"decision\":\"AlreadyConsumed\""),
        "{}",
        r[3]
    );
}

#[test]
fn exhaustion_is_insufficient_capacity() {
    let script = format!(
        "{}\n{}\n{}\n{}\n",
        deposit("lab/s", 1),
        request("lab/s", 1),
        consume("e1", "lab/s"),
        consume("e2", "lab/s"),
    );
    let r = run(&script);
    assert!(r[2].contains("\"decision\":\"Consumed\""), "{}", r[2]);
    assert!(
        r[3].contains("\"decision\":\"InsufficientCapacity\""),
        "{}",
        r[3]
    );
}

#[test]
fn scope_mismatch_refuses() {
    let script = format!(
        "{}\n{}\n{}\n",
        deposit("lab/s", 1),
        request("lab/s", 1),
        consume("e1", "lab/OTHER"), // consume cites a different scope than the grant
    );
    let r = run(&script);
    assert!(r[2].contains("\"decision\":\"ScopeMismatch\""), "{}", r[2]);
}

#[test]
fn malformed_line_fails_closed() {
    let r = run("this is not a command\n");
    assert!(r[0].contains("\"fail_closed\":true"), "{}", r[0]);
}

#[test]
fn unknown_cmd_fails_closed() {
    let r = run("cmd=mint\tscope=lab/s\tamount=1\n");
    assert!(r[0].contains("\"fail_closed\":true"), "{}", r[0]);
    assert!(r[0].contains("unknown cmd"), "{}", r[0]);
}

#[test]
fn unknown_token_handle_fails_closed() {
    // No grant issued, so t0 was never minted by the CLI.
    let r = run(&format!("{}\n", consume("e1", "lab/s")));
    assert!(r[0].contains("\"fail_closed\":true"), "{}", r[0]);
    assert!(r[0].contains("unknown token handle"), "{}", r[0]);
}

#[test]
fn missing_field_fails_closed() {
    let r = run("cmd=deposit\tscope=lab/s\n"); // no amount
    assert!(r[0].contains("\"fail_closed\":true"), "{}", r[0]);
}

#[test]
fn deposit_without_admission_ref_fails_closed() {
    // A mint that cannot cite an admission is refused over the wire, fail-closed —
    // no silent stock. The mint boundary must cite an admission (invariant 4).
    let r = run("cmd=deposit\tscope=lab/s\tamount=1\n"); // no admission_ref
    assert!(r[0].contains("\"fail_closed\":true"), "{}", r[0]);
    assert!(r[0].contains("admission_ref"), "{}", r[0]);
    assert!(!r[0].contains("\"event\":\"deposited\""), "{}", r[0]);
}

#[test]
fn deposit_echoes_admission_ref() {
    // The success line carries the cited admission reference back to the consumer.
    let r = run(&format!("{}\n", deposit("lab/s", 1)));
    assert!(r[0].contains("\"event\":\"deposited\""), "{}", r[0]);
    assert!(
        r[0].contains("\"admission_ref\":\"adm_budget\""),
        "{}",
        r[0]
    );
}
