// SPDX-License-Identifier: Apache-2.0
//! `la_cli` — thin transport over the Linear Accountant v0 boundary.
//!
//! Consumer-trigger build (the Agent Governor bootstrap-lab effect gate needs
//! `consume` at a real tool-effect boundary, 2026-06-16). Until now the crate
//! was library-only ("there is no binary"); this binary is the minimal,
//! contract-preserving transport that lets an out-of-process consumer reach the
//! already-implemented decisions.
//!
//! ## What it is NOT
//!
//! It adds **no policy**. Every command maps 1:1 onto an `InMemoryAccountant`
//! method; the accountant stays authoritative for spendability and the CLI only
//! serializes its decisions. The seam stays narrow (no free-text justification
//! enters a decision — `eligibility_reference` is an opaque sealed pointer, as
//! in the lib). It is still small, hostile, and stupid.
//!
//! ## Lifetime
//!
//! Session-lived: one in-memory accountant per process. State lives only for the
//! process — there is no persistence (the lib's non-production stance is
//! preserved). A consumer spawns one `la_cli` per supervised session and kills
//! it at session end.
//!
//! ## Protocol (stable, `protocol = "v0"`)
//!
//! - **stdin:** one command per line; TAB-separated `key=value` pairs. The first
//!   pair must be `cmd=<deposit|request_capacity|consume>`. Values may not
//!   contain a TAB or newline; everything else (including `=`, `/`, `:`) is fine
//!   (only the first `=` of a pair is the separator).
//! - **stdout:** one JSON object per line — the decision, an ack, or an error.
//! - **first stdout line:** a banner so the consumer can record LA identity:
//!   `{"la":"linear-accountant","version":"…","commit":"…","protocol":"v0"}`.
//! - **fail-closed:** any malformed line, unknown `cmd`, missing/non-uint field,
//!   or unknown token handle → `{"error":"…","fail_closed":true}`. Never a
//!   silent allow; a consumer must treat any non-decision line as a refusal.
//!
//! Token handles: `request_capacity` returns an opaque handle (`t0`, `t1`, …),
//! NOT the accountant's `TokenId` (which has no public constructor — invariant
//! 1). `consume` cites the handle; the CLI maps it back to the held `TokenId`.

use std::collections::HashMap;
use std::io::{self, BufRead, Write};

use linear_accountant::{
    CapacityDecision, CapacityRequest, ConsumeRequest, ConsumptionDecision, EventId,
    InMemoryAccountant, RequestId, Scope, Tick, TokenId,
};

fn main() {
    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut out = stdout.lock();

    let version = env!("CARGO_PKG_VERSION");
    let commit = option_env!("LA_GIT_COMMIT").unwrap_or("unknown");
    // Banner first — the consumer records LA identity at the boundary.
    let _ = writeln!(
        out,
        "{{\"la\":\"linear-accountant\",\"version\":\"{}\",\"commit\":\"{}\",\"protocol\":\"v0\"}}",
        esc(version),
        esc(commit)
    );
    let _ = out.flush();

    let mut acct = InMemoryAccountant::new();
    // Wire-handle namespace: the CLI's own opaque references to issued tokens.
    let mut handles: Vec<TokenId> = Vec::new();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => {
                emit_error(&mut out, "stdin read error");
                continue;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        let resp = handle_line(&line, &mut acct, &mut handles);
        let _ = writeln!(out, "{}", resp);
        let _ = out.flush();
    }
}

/// Parse one TAB-separated key=value command line. Fail-closed on any defect.
fn handle_line(line: &str, acct: &mut InMemoryAccountant, handles: &mut Vec<TokenId>) -> String {
    if line.contains('\t') && line.contains('\n') {
        return err("line contains a newline");
    }
    let mut fields: HashMap<&str, &str> = HashMap::new();
    for pair in line.split('\t') {
        match pair.split_once('=') {
            Some((k, v)) => {
                fields.insert(k, v);
            }
            None => return err("malformed field (expected key=value)"),
        }
    }
    let cmd = match fields.get("cmd") {
        Some(c) => *c,
        None => return err("missing cmd"),
    };
    match cmd {
        "deposit" => cmd_deposit(&fields, acct),
        "request_capacity" => cmd_request(&fields, acct, handles),
        "consume" => cmd_consume(&fields, acct, handles),
        other => err(&format!("unknown cmd: {other}")),
    }
}

fn cmd_deposit(f: &HashMap<&str, &str>, acct: &mut InMemoryAccountant) -> String {
    let scope = match req_str(f, "scope") {
        Ok(s) => s,
        Err(e) => return e,
    };
    let amount = match req_u64(f, "amount") {
        Ok(n) => n,
        Err(e) => return e,
    };
    let receipt = acct.deposit(&Scope(scope.to_string()), amount);
    format!(
        "{{\"ok\":true,\"event\":\"deposited\",\"scope\":\"{}\",\"amount\":{},\"receipt\":\"{}\"}}",
        esc(scope),
        amount,
        esc(&format!("{receipt:?}"))
    )
}

fn cmd_request(
    f: &HashMap<&str, &str>,
    acct: &mut InMemoryAccountant,
    handles: &mut Vec<TokenId>,
) -> String {
    let request_id = match req_str(f, "request_id") {
        Ok(s) => s,
        Err(e) => return e,
    };
    let actor = match req_str(f, "actor") {
        Ok(s) => s,
        Err(e) => return e,
    };
    let action = match req_str(f, "action") {
        Ok(s) => s,
        Err(e) => return e,
    };
    let target = match req_str(f, "target") {
        Ok(s) => s,
        Err(e) => return e,
    };
    let scope = match req_str(f, "scope") {
        Ok(s) => s,
        Err(e) => return e,
    };
    let requested_capacity = match req_u64(f, "requested_capacity") {
        Ok(n) => n,
        Err(e) => return e,
    };
    let eligibility_reference = match req_str(f, "eligibility_reference") {
        Ok(s) => s,
        Err(e) => return e,
    };
    let eligibility_valid_until = match req_u64(f, "eligibility_valid_until") {
        Ok(n) => n,
        Err(e) => return e,
    };
    let expires_after = match req_u64(f, "expires_after") {
        Ok(n) => n,
        Err(e) => return e,
    };
    let tick = match req_u64(f, "tick") {
        Ok(n) => n,
        Err(e) => return e,
    };
    let idempotency_key = f.get("idempotency_key").map(|s| s.to_string());

    let req = CapacityRequest {
        request_id: RequestId(request_id.to_string()),
        actor: actor.to_string(),
        action: action.to_string(),
        target: target.to_string(),
        scope: Scope(scope.to_string()),
        requested_capacity,
        eligibility_reference: eligibility_reference.to_string(),
        eligibility_valid_until: eligibility_valid_until as Tick,
        expires_after: expires_after as Tick,
        idempotency_key,
    };
    match acct.request_capacity(req, tick as Tick) {
        CapacityDecision::Granted {
            token_id,
            granted_capacity,
            scope,
            expires_at,
            receipt,
        } => {
            // Hand out our own opaque handle; never expose the TokenId value.
            let handle = format!("t{}", handles.len());
            handles.push(token_id);
            format!(
                "{{\"decision\":\"Granted\",\"token_id\":\"{}\",\"granted_capacity\":{},\"scope\":\"{}\",\"expires_at\":{},\"receipt\":\"{}\"}}",
                esc(&handle),
                granted_capacity,
                esc(&scope.0),
                expires_at,
                esc(&format!("{receipt:?}"))
            )
        }
        CapacityDecision::Denied {
            denial_reason,
            receipt,
        } => format!(
            "{{\"decision\":\"Denied\",\"denial_reason\":\"{}\",\"receipt\":\"{}\"}}",
            esc(&denial_reason),
            esc(&format!("{receipt:?}"))
        ),
    }
}

fn cmd_consume(
    f: &HashMap<&str, &str>,
    acct: &mut InMemoryAccountant,
    handles: &mut [TokenId],
) -> String {
    let event_id = match req_str(f, "consumption_event_id") {
        Ok(s) => s,
        Err(e) => return e,
    };
    let handle = match req_str(f, "token_id") {
        Ok(s) => s,
        Err(e) => return e,
    };
    let actor = match req_str(f, "actor") {
        Ok(s) => s,
        Err(e) => return e,
    };
    let action = match req_str(f, "action") {
        Ok(s) => s,
        Err(e) => return e,
    };
    let target = match req_str(f, "target") {
        Ok(s) => s,
        Err(e) => return e,
    };
    let amount = match req_u64(f, "amount") {
        Ok(n) => n,
        Err(e) => return e,
    };
    let scope = match req_str(f, "scope") {
        Ok(s) => s,
        Err(e) => return e,
    };
    let tick = match req_u64(f, "tick") {
        Ok(n) => n,
        Err(e) => return e,
    };

    // Map the opaque wire handle back to the held TokenId. Unknown handle is
    // fail-closed — a consumer cannot conjure a token the CLI never issued.
    let token_id = match parse_handle(handle, handles) {
        Some(t) => t,
        None => return err(&format!("unknown token handle: {handle}")),
    };

    let req = ConsumeRequest {
        consumption_event_id: EventId(event_id.to_string()),
        token_id,
        actor: actor.to_string(),
        action: action.to_string(),
        target: target.to_string(),
        amount,
        scope: Scope(scope.to_string()),
    };
    match acct.consume(req, tick as Tick) {
        ConsumptionDecision::Consumed {
            consumed_amount,
            remaining_capacity,
            receipt,
            ..
        } => format!(
            "{{\"decision\":\"Consumed\",\"token_id\":\"{}\",\"consumed_amount\":{},\"remaining_capacity\":{},\"receipt\":\"{}\"}}",
            esc(handle),
            consumed_amount,
            remaining_capacity,
            esc(&format!("{receipt:?}"))
        ),
        ConsumptionDecision::AlreadyConsumed { receipt, .. } => format!(
            "{{\"decision\":\"AlreadyConsumed\",\"token_id\":\"{}\",\"receipt\":\"{}\"}}",
            esc(handle),
            esc(&format!("{receipt:?}"))
        ),
        ConsumptionDecision::InsufficientCapacity {
            remaining_capacity,
            requested_amount,
            receipt,
            ..
        } => format!(
            "{{\"decision\":\"InsufficientCapacity\",\"token_id\":\"{}\",\"remaining_capacity\":{},\"requested_amount\":{},\"receipt\":\"{}\"}}",
            esc(handle),
            remaining_capacity,
            requested_amount,
            esc(&format!("{receipt:?}"))
        ),
        ConsumptionDecision::Expired {
            expired_at, receipt, ..
        } => format!(
            "{{\"decision\":\"Expired\",\"token_id\":\"{}\",\"expired_at\":{},\"receipt\":\"{}\"}}",
            esc(handle),
            expired_at,
            esc(&format!("{receipt:?}"))
        ),
        ConsumptionDecision::Revoked {
            revoked_at, receipt, ..
        } => format!(
            "{{\"decision\":\"Revoked\",\"token_id\":\"{}\",\"revoked_at\":{},\"receipt\":\"{}\"}}",
            esc(handle),
            revoked_at,
            esc(&format!("{receipt:?}"))
        ),
        ConsumptionDecision::UnknownToken { receipt, .. } => format!(
            "{{\"decision\":\"UnknownToken\",\"token_id\":\"{}\",\"receipt\":\"{}\"}}",
            esc(handle),
            esc(&format!("{receipt:?}"))
        ),
        ConsumptionDecision::ScopeMismatch {
            expected_scope,
            requested_scope,
            receipt,
            ..
        } => format!(
            "{{\"decision\":\"ScopeMismatch\",\"token_id\":\"{}\",\"expected_scope\":\"{}\",\"requested_scope\":\"{}\",\"receipt\":\"{}\"}}",
            esc(handle),
            esc(&expected_scope.0),
            esc(&requested_scope.0),
            esc(&format!("{receipt:?}"))
        ),
    }
}

fn parse_handle(handle: &str, handles: &[TokenId]) -> Option<TokenId> {
    let idx: usize = handle.strip_prefix('t')?.parse().ok()?;
    handles.get(idx).copied()
}

fn req_str<'a>(f: &HashMap<&'a str, &'a str>, key: &str) -> Result<&'a str, String> {
    match f.get(key) {
        Some(v) => Ok(*v),
        None => Err(err(&format!("missing field: {key}"))),
    }
}

fn req_u64(f: &HashMap<&str, &str>, key: &str) -> Result<u64, String> {
    match f.get(key) {
        Some(v) => v
            .parse::<u64>()
            .map_err(|_| err(&format!("field {key} is not a u64: {v}"))),
        None => Err(err(&format!("missing field: {key}"))),
    }
}

fn emit_error(out: &mut impl Write, msg: &str) {
    let _ = writeln!(out, "{}", err(msg));
    let _ = out.flush();
}

/// A fail-closed error line. A consumer treats any `fail_closed` response as a
/// refusal — never a silent allow.
fn err(msg: &str) -> String {
    format!("{{\"error\":\"{}\",\"fail_closed\":true}}", esc(msg))
}

/// Minimal JSON string escaping (std-only, no serde): quotes, backslash, control.
fn esc(s: &str) -> String {
    let mut o = String::with_capacity(s.len());
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            '\r' => o.push_str("\\r"),
            '\t' => o.push_str("\\t"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o
}
