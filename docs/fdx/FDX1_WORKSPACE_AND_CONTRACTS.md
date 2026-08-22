# FDX1 workspace and contracts

Status: implemented locally; not pushed

## Bounded outcome

FDX1 converts the project into a workspace and freezes the first reusable contracts. It does not improve extraction quality, build a production corpus, train a model, or claim a production Ferrodoc adapter.

| Member | License | Responsibility |
|---|---|---|
| `crates/datasheet-core` | `MIT OR Apache-2.0` | dependency-light durable evidence, subject, quantity, condition, vocabulary, producer, claim, and constraint contracts |
| `crates/datasheet-compiler` | `MIT OR Apache-2.0` | state-pinned fixture adapter, deterministic candidate/sketch identity, claim assembly and validation |
| `crates/datasheet-model` | `MIT OR Apache-2.0` | model manifest and candidate-selection output contracts only; no runtime/model is claimed |
| `tools/datasheet-train` | `MIT OR Apache-2.0` | manifest-inspection shell only; training is deferred |
| `apps/datasheet-cli` | `GPL-3.0-only` | the moved existing application/provider/PDF code plus explicit fixture sketch command |

Existing GPL application source was moved without relicensing. New reusable crate source carries its own SPDX expression. The root `LICENSE` continues to cover the application.

## Frozen contract decisions

- `EvidenceAnchor` serializes as `evidence-anchor/2` and binds source PDF digest, logical `document_state_id`, optional materialized-view digest, page, optional region, evidence IDs, normalized geometry quality, and an explicit whole-evidence/text-range/table-cell selector.
- `DatasheetSketch` serializes as `datasheet-sketch/1`. It owns `conditions[]`; every `ConditionCandidate` retains raw condition text, optional parsed candidate fields, and exact anchors.
- Sketch identity is SHA-256 over a fixed serialization projection after all candidate collections are sorted. Wall-clock/run/checkpoint lineage and corpus-derived document/template/manufacturer family assignments are absent.
- Candidate IDs are tagged SHA-256 values over candidate kind, exact raw text, and state-aware anchor. Reordering fixture pages, regions, or evidence leaves IDs and sketch bytes unchanged.
- Producer identity binds code revision/version, configuration digest, DocumentIR schema, ontology snapshot digest, optional Quantitas registry identity, and optional model digest.
- The model-manifest inspection shell validates its schema version, every required artifact/configuration/evaluation digest, source revision, and license provenance; it remains a contract-only shell with no model runtime.
- Predicate terms are vocabulary data. Adding a term does not change the `ClaimBundle` schema.
- `ClaimBundle` serializes as `claim-bundle/1`, pins the sketch and state, preserves exact selected `QuantityCandidate` records, and fails closed for missing subject/value/condition/package/evidence references, ontology mismatch, or ungrounded claims.

## Conformance fixture and pins

`tests/fixtures/fdx1_synthetic_document_ir.json` is a minimized synthetic bridge fixture with these pins:

- fixture schema: `ferrodoc-document-ir-fixture/1`;
- PDF digest: `1759310ab11053297113c688c781bd6829682f06d4e0b6d32e4be1c83d8ff5db`;
- state: `document-state:synthetic-power-table-v1`;
- optional materialized-view digest: `1f7b128b313beab6f8bd7999b4432107876bbe0449a917d11558cce88ad1061b`;
- synthetic Quantitas identity: `quantitas-synthetic-registry/1`.

Fixture annotations exist only to exercise contracts deterministically. They are not a hidden rule-based production parser. The golden sketch ID is asserted by tests.

## Commands and schemas

The remote commands remain under the `datasheet` application. A deliberately honest local command compiles only the minimized bridge fixture:

```text
datasheet sketch FIXTURE \
  --code-revision REVISION \
  --configuration-digest SHA256 \
  --ontology-snapshot-digest SHA256 \
  --quantitas-registry-identity ID
```

Generate stable JSON Schemas with:

```text
cargo run -p datasheet-compiler --example export_fdx1_schemas -- schemas/fdx1
cargo run -p datasheet-model --example export_schema -- \
  schemas/fdx1/model-manifest-v1.schema.json
```

The core boundary test rejects accidental HTTP, PDF, model-runtime, or Foundry dependencies. CI checks formatting and warning-free FDX crates on Linux, then builds/tests the whole workspace on Linux, macOS, and Windows.

## Deferred gates

- A production Ferrodoc adapter and real FDX0 state are deferred; FDX1 uses only the pinned synthetic fixture permitted by the coordination plan.
- Quantitas was represented by an identity contract at the FDX1 boundary; the pinned registry integration is documented by FDX2.
- Deterministic analog/power extraction quality beyond the bounded synthetic FDX2 conformance baseline remains unqualified on a production corpus.
- Production supervision/corpus/model work remains blocked on the Foundry FLS2/FLS4/FLS5 gates and FDX3-FDX5.
