# FDX datasheet compiler program

Datasheet-cli owns electronics interpretation over pinned Ferrodoc evidence. Foundry owns acquisition, rights, persistence, lineage, and promotion; Ferrodoc owns generic PDF/OCR/layout/table evidence; Quantitas owns dimensional and unit canonicalization.

Implemented phases:

- FDX1 workspace and evidence/claim contracts;
- FDX2 deterministic analog/power baseline over minimized fixtures;
- FDX3 deterministic supervision contract foundation, including vocabulary
  maps, the stable labeler catalog, exact votes/abstentions, lineage-aware
  combination, rights gates, and reviewed-development/calibration interfaces.

FDX3 production acceptance is not yet claimed: committed FLS5 real states and
two rights-cleared analog/power structured-fact supervision families remain
required. The fail-closed real-source selection audit is checked in at
`../../evidence/fdx3-production-readiness-2026-08-22.json`; it admits one
bounded NASA NPSL electrical family, still below the two-family exit gate, and
persists no restricted content. FDX4 has since frozen that one-family corpus;
FDX5-FDX6 have not started.

The implemented boundaries are documented in
[FDX1](FDX1_WORKSPACE_AND_CONTRACTS.md), the bounded
[FDX2 deterministic baseline](FDX2_DETERMINISTIC_BASELINE.md),
[portable Ferrodoc conformance](FERRODOC_PORTABLE_CONFORMANCE.md), and the
[FDX3 supervision foundation](FDX3_DETERMINISTIC_SUPERVISION.md). Production
FDX4 protected-corpus contracts are implemented, with all current NASA examples
truthfully train-only and FDX5 blocked on distinct protected evaluation groups.
FDX5+ remain gated by `/projects/FOUNDRY-PLAN.md`; this repository
contains no production corpus, trained model, or claim of production DocumentIR
extraction.

The learned-path safety boundary for later phases is already reflected in the contracts: a model may select existing candidate IDs, while deterministic code assembles a claim and rejects any missing candidate, condition, or evidence reference.
