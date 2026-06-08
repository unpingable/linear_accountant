# Provenance

This project is human-directed and AI-assisted. Final design authority,
acceptance criteria, and editorial control rest with the human author.
AI contributions were material and are categorized below by function.

## Human authorship

The author defined the project direction, requirements, and design intent —
including the central distinction (validity is contractible; spendability is
linear), the role boundary, and the discipline of recording candidate surfaces
without building them. AI systems contributed proposals, drafts, implementation,
and critique under author supervision; they did not independently determine
project goals or deployment decisions. The author reviewed, revised, or rejected
AI-generated output throughout development.

## AI-assisted collaboration

### Architectural design and constraint model

Lead collaboration: ChatGPT (OpenAI), with parallel review from Claude
(Anthropic), DeepSeek, and Gemini. The role boundary, the three-axis doctrine
(spend is counted / capability is checked / custody is held), the
sovereignty-relocates-to-arithmetic framing, and the deployment deflation
(a gateway with a `consume()` call, not a constitution) were sharpened across a
multi-model loop and ratified by the author.

### Implementation, tests, and integration

Lead collaboration: Claude (Anthropic) via Claude Code. Heavy contributions to
the v0 reference crate, the test suites (boundary properties and the WL-001 /
WL-002 workloads), the design documents under `docs/`, and build configuration —
including assembling architectural decisions into working Rust.

### Validation and adversarial review

Secondary review from DeepSeek and Gemini on the conservation argument and the
contention falsification test.

## Provenance basis and limits

This document is a functional attribution record based on commit history,
co-author trailers (where present), project notes, and documented working
sessions. It is not a complete forensic account of all contributions.

Some AI contributions (especially design critique, rejected alternatives,
and footguns avoided) may not appear in repository artifacts or commit
metadata.

Model names/tools are recorded at the platform level (e.g., ChatGPT,
Claude Code); exact model versions may vary across sessions and are not
exhaustively reconstructed here.

## What this document does not claim

- No exact proportional attribution. Contributions are categorized by
  function, not quantified by token count or lines of code.
- Design and implementation were not cleanly sequential. Architecture
  informed code, code revealed design gaps, and the feedback loop was
  continuous.
- "Footguns avoided" and "ideas that didn't ship" are real contributions
  that leave no artifact. This document cannot fully account for them.

---

This document reflects the project state as of 2026-06-04 and may be revised.
