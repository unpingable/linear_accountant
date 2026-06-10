/-
  Linear Accountant — Lean differential oracle (model + proofs)

  A machine-checked model of the spend ledger as a fold over an event list. It is the
  formal twin of `src/lib.rs`: it proves the invariants the Rust implementation is
  differential-tested against (see `tests/differential_oracle.rs`).

  This adds NO surface to the crate. It hardens the *frozen* boundary — it does not
  thaw it. Mathlib-free: depends only on Lean 4 core, so `lean Ledger.lean` typechecks
  with no package fetch.

  Scope of the model:
    * Single scope. Multi-scope is the disjoint union of independent single-scope
      ledgers; the conservation identity holds per scope, so one scope is faithful.
    * No clock / expiry. Expiry only *blocks* consume; it never restocks in v0
      (the restock-on-expiry ruling is parked under the freeze). Conservation is
      stated over `original`, which neither consume nor expiry ever mutates, so the
      omission is sound: expiry cannot break the identity the way a restock could.

  Proven below (zero `sorry`):
    1. `conservation`   — minted = available + Σ original, for every event sequence.
    2. `replay_is_noop` — consuming an already-seen event id changes nothing
                          (replay-refusal == no-double-consume).
-/

namespace LinearAccountant

/-- A minted token. `original` is fixed at grant; `remaining` is drawn down by consume;
    `consumed` is the running total spent; `events` are the consumption event ids seen. -/
structure Tok where
  id        : Nat
  original  : Nat
  remaining : Nat
  consumed  : Nat
  events    : List Nat
  revoked   : Bool
  deriving Repr

/-- Single-scope ledger state. -/
structure State where
  minted    : Nat
  available : Nat
  tokens    : List Tok
  nextId    : Nat
  deriving Repr

def State.init : State := ⟨0, 0, [], 0⟩

inductive Event where
  | deposit (amount : Nat)
  | request (amount : Nat)
  | consume (tokId evId amount : Nat)
  | revoke  (tokId : Nat)

/-- Sum of `original` over a token list. Defined by hand — no library dependency. -/
def sumOriginal : List Tok → Nat
  | []      => 0
  | t :: ts => t.original + sumOriginal ts

/-- Apply `f` to the first token with `id = tokId`, leaving the rest untouched. -/
def updTok (tokId : Nat) (f : Tok → Tok) : List Tok → List Tok
  | []      => []
  | t :: ts => if t.id = tokId then f t :: ts else t :: updTok tokId f ts

def findTok (tokId : Nat) : List Tok → Option Tok
  | []      => none
  | t :: ts => if t.id = tokId then some t else findTok tokId ts

/-- The drawdown a successful consume applies. Note `original` is untouched. -/
def consumeTok (evId amount : Nat) (t : Tok) : Tok :=
  { t with remaining := t.remaining - amount,
           consumed  := t.consumed + amount,
           events    := evId :: t.events }

def revokeTok (t : Tok) : Tok := { t with revoked := true }

/-- One state transition — the conservation-relevant skeleton of `src/lib.rs`: a refusal
    is a no-op on state. `request` here models only mint-vs-no-op (the full refusal
    taxonomy — eligibility, stale, dedup, zero — is no-op-on-stock and lives in the
    differential test); `consume` keeps lib.rs branch order (replay first) but omits the
    no-clock/single-scope refusals (expiry, scope). See `verification/README.md`. -/
def step (s : State) : Event → State
  | .deposit a =>
      { s with minted := s.minted + a, available := s.available + a }
  | .request a =>
      if a = 0 then s
      else if a ≤ s.available then
        { s with available := s.available - a,
                 tokens := ⟨s.nextId, a, a, 0, [], false⟩ :: s.tokens,
                 nextId := s.nextId + 1 }
      else s
  | .consume tokId evId amount =>
      match findTok tokId s.tokens with
      | none   => s
      | some t =>
          if t.events.contains evId then s             -- replay: no-op (checked first, as in lib.rs)
          else if t.revoked then s
          else if amount > t.remaining then s
          else { s with tokens := updTok tokId (consumeTok evId amount) s.tokens }
  | .revoke tokId =>
      { s with tokens := updTok tokId revokeTok s.tokens }

/-- Fold the transition over an event sequence. -/
def run (s : State) : List Event → State
  | []      => s
  | e :: es => run (step s e) es

/-- `updTok` preserves Σ original when `f` leaves the `original` field alone. -/
theorem sumOriginal_updTok (tokId : Nat) (f : Tok → Tok)
    (hf : ∀ t, (f t).original = t.original) :
    ∀ ts, sumOriginal (updTok tokId f ts) = sumOriginal ts := by
  intro ts
  induction ts with
  | nil => rfl
  | cons t ts ih =>
      simp only [updTok]
      by_cases h : t.id = tokId
      · simp [h, sumOriginal, hf]
      · simp [h, sumOriginal, ih]

/-- The conservation invariant. -/
def Conserved (s : State) : Prop := s.minted = s.available + sumOriginal s.tokens

/-- Every transition preserves conservation. -/
theorem step_preserves (s : State) (e : Event) (h : Conserved s) :
    Conserved (step s e) := by
  unfold Conserved at *
  cases e with
  | deposit a =>
      simp only [step]; omega
  | request a =>
      simp only [step]
      split
      · exact h
      · split
        · dsimp only [sumOriginal]; omega
        · exact h
  | consume tokId evId amount =>
      simp only [step]
      split
      · exact h
      · split
        · exact h
        · split
          · exact h
          · split
            · exact h
            · rw [sumOriginal_updTok tokId (consumeTok evId amount) (fun _ => rfl)]
              exact h
  | revoke tokId =>
      show s.minted = s.available + sumOriginal (updTok tokId revokeTok s.tokens)
      rw [sumOriginal_updTok tokId revokeTok (fun _ => rfl)]
      exact h

/-- Conservation holds after any event sequence from any conserved start. -/
theorem run_conserves (es : List Event) (s : State) (h : Conserved s) :
    Conserved (run s es) := by
  induction es generalizing s with
  | nil => exact h
  | cons e es ih => exact ih (step s e) (step_preserves s e h)

/-- **Conservation identity.** From the empty ledger, for every event sequence:
    minted = available + Σ original. The books always balance. -/
theorem conservation (es : List Event) : Conserved (run State.init es) :=
  run_conserves es State.init rfl

/-- **Replay-refusal / no-double-consume.** Consuming an already-seen event id on a
    token leaves the whole state unchanged — so the same event can never spend twice. -/
theorem replay_is_noop (s : State) (tokId evId amount : Nat) (t : Tok)
    (hfind : findTok tokId s.tokens = some t)
    (hseen : evId ∈ t.events) :
    step s (.consume tokId evId amount) = s := by
  -- replay is the first check in `step`, so this closes directly
  simp [step, hfind, hseen]

end LinearAccountant
