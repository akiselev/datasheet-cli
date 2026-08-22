# FDX datasheet compiler program

Datasheet-cli owns electronics interpretation over pinned Ferrodoc evidence. Foundry owns acquisition, rights, persistence, lineage, and promotion; Ferrodoc owns generic PDF/OCR/layout/table evidence; Quantitas owns dimensional and unit canonicalization.

The implemented boundaries are documented in [FDX1](FDX1_WORKSPACE_AND_CONTRACTS.md) and the bounded [FDX2 deterministic baseline](FDX2_DETERMINISTIC_BASELINE.md). Later FDX phases remain gated by `/projects/FOUNDRY-PLAN.md`; this repository contains no production corpus, trained model, or claim of production DocumentIR extraction.

The learned-path safety boundary for later phases is already reflected in the contracts: a model may select existing candidate IDs, while deterministic code assembles a claim and rejects any missing candidate, condition, or evidence reference.
