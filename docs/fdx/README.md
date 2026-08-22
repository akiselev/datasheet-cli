# FDX datasheet extraction and model-training program

Status: planned

Owner: Datasheet-cli for FDX1-FDX6

Upstream dependency: Ferrodoc FDX0

Production integration: Foundry FLS2/FLS4/FLS5/FLS6

## Goal

Turn Datasheet-cli from a PDF-to-remote-LLM command into a reusable electronics-specific document compiler with a cheap deterministic path, weak-supervision/data compiler, compact local model and explicit remote-teacher fallback.

The existing remote Gemini extraction commands remain useful as a fallback/teacher and compatibility surface. They stop being the architectural center of the project.

## Program map

| Phase | Purpose |
|---|---|
| FDX0 (Ferrodoc) | evidence-grade native geometry, table cells and selective refinement |
| [FDX1](FDX1_WORKSPACE_AND_CONTRACTS.md) | workspace split, domain contracts, `DatasheetSketch/v1` |
| [FDX2](FDX2_DETERMINISTIC_COMPILER.md) | deterministic electrical-table compiler and qualification baseline |
| [FDX3](FDX3_WEAK_SUPERVISION.md) | source-taxonomy mappings and weak-label compiler |
| [FDX4](FDX4_CORPUS_AND_EVALUATION.md) | template propagation, synthetic corpus, leakage-safe benchmark |
| [FDX5](FDX5_COMPACT_MODEL.md) | compact IR-native model, RTen inference and active teacher |
| [FDX6](FDX6_PRODUCTION_CASCADE.md) | production cascade, Foundry integration, retraining/evaluation loop |

## Repository boundary

Datasheet-cli owns electronics interpretation:

```text
Ferrodoc DocumentIR
        |
        v
DatasheetSketch
        |
        +--> deterministic candidate compiler
        +--> weak-supervision compiler
        +--> compact model
        +--> teacher/review tooling
        |
        v
ClaimBundle / DesignConstraintBundle
```

It does not own:

- manufacturer/source acquisition policy;
- Foundry canonical identities or promotion state;
- Artifactum storage internals;
- generic PDF/OCR/table engines;
- a competing unit/dimensional registry.

Foundry supplies immutable observations/rights/lake manifests and consumes serialized claim bundles. Ferrodoc supplies immutable DocumentIR/evidence. Quantitas supplies dimensional/unit semantics.

## Workspace target

The current project is a single binary crate containing remote LLM extraction, PDF handling and multiple data-source clients. FDX1 turns it into a small workspace with boundaries that correspond to real reuse/compile/runtime needs:

```text
.
├── crates/
│   ├── datasheet-core/
│   ├── datasheet-compiler/
│   └── datasheet-model/
├── tools/
│   └── datasheet-train/
├── apps/
│   └── datasheet-cli/
├── docs/fdx/
└── prompts/
```

Do not create more crates merely to mirror every module. Training may use Python/PyTorch where that is the shortest path; production inference and domain contracts remain Rust.

## Core safety invariant

A learned model does not emit arbitrary engineering values. The compiler first enumerates deterministic candidates from exact DocumentIR/source evidence, then the model chooses candidate IDs/relationships.

Expected learned output resembles:

```text
predicate_candidate_id
value_candidate_id
regime_id
condition_candidate_ids[]
subject_candidate_id
```

Deterministic code constructs the final claim. A numeric/string value without an exact selected evidence/source candidate is invalid.

## First task family

Do not port every existing prompt at once. The first qualification corpus is analog/power semiconductor characteristics:

```text
Absolute Maximum Ratings
Recommended Operating Conditions
Electrical Characteristics
Thermal Information
```

This task family has strong table structure, abundant structured-source supervision and immediate Foundry value.

After the loop is proven, add pinouts, package/footprint semantics, power sequencing, high-speed constraints, boot configuration, reference designs and application circuits as additional task families.

## Qualification philosophy

Every learned component competes against a deterministic baseline on a protected case set. Family/template/manufacturer leakage is prohibited. Precision, grounding validity, evidence-anchor validity, resource cost and coverage remain separately visible.

Remote LLM/VLM usage is measured as an escalation tier and should concentrate on new source vocabularies, new templates and high-disagreement examples rather than one request per PDF.
