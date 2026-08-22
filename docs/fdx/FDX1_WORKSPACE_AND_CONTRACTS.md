# FDX1 — Workspace split and domain contracts

Status: planned

Owner: Datasheet-cli

Depends on: FDX0 contract shape agreed; implementation may use pinned Ferrodoc fixtures while FDX0 finishes

## Objective

Separate reusable electronics semantics from the existing CLI/remote-LLM/source-client code and define the stable serialized contracts that every later FDX phase uses.

FDX1 changes architecture, not extraction quality. It must preserve existing user-facing commands unless a command is explicitly documented as migrated/deprecated.

## Deliverable 1: workspace conversion

Create:

```text
crates/datasheet-core
crates/datasheet-compiler
crates/datasheet-model
tools/datasheet-train
apps/datasheet-cli
```

Responsibilities:

### `datasheet-core`

Dependency-light durable types only:

- subject hints/handles;
- predicate term references;
- specification regimes;
- applicability/conditions;
- evidence anchors;
- quantity/value candidate references;
- `ClaimBundle/v1`;
- `DesignConstraintBundle/v1`;
- versioned serialization/schema generation.

It must not depend on HTTP clients, LLM providers, PDF runtimes, PyTorch, Foundry persistence or source implementations.

### `datasheet-compiler`

Owns:

- Ferrodoc DocumentIR adapter;
- `DatasheetSketch/v1`;
- deterministic candidate extraction;
- ontology/predicate retrieval interfaces;
- Quantitas integration;
- source vocabulary mappings;
- weak-label compiler;
- claim assembly/validation.

### `datasheet-model`

Owns:

- model manifest;
- feature schema;
- tokenizer/model artifact loading;
- RTen inference;
- calibration;
- model output validation;
- optional backend trait if later benchmarks justify non-RTen inference.

### `datasheet-train`

Training/export/evaluation tooling. Python/PyTorch is allowed and preferred if it reduces complexity. It emits immutable ONNX/model manifests; production does not embed Python.

### `apps/datasheet-cli`

Moves the existing command application and provider/source clients here. Existing Gemini extraction remains an explicit remote path/fallback.

## Deliverable 2: licensing/API boundary

The current binary is GPL-3.0-only. Decide and document licensing for new reusable library crates before FDX1 merges. Preferred default for ecosystem reuse is `MIT OR Apache-2.0` for new libraries while the application may remain GPL, but implementation must make the final explicit decision rather than accidentally inheriting ambiguous licensing.

Do not change ownership/license of copied existing code without tracking its provenance.

## Deliverable 3: versioned `EvidenceAnchor`

Define a Datasheet-side serialized reference compatible with Foundry FLS5 without importing Foundry internals:

```text
EvidenceAnchor/v1 {
    source_pdf_sha256
    document_ir_logical_sha256
    page_id
    region_id?
    evidence_ids[]
    selectors[]
    source_geometry[]
    geometry_quality[]
}
```

Selectors initially include:

```text
whole_evidence
text_range { evidence_id, start, end }
table_cell { table_region_id, row, column }
```

The adapter validates every anchor against the pinned DocumentIR before emitting a bundle.

## Deliverable 4: `DatasheetSketch/v1`

Define the compact electronics-oriented representation derived from DocumentIR. It is not a second document IR; it is a deterministic semantic candidate index that always points back to Ferrodoc evidence.

Logical shape:

```text
DatasheetSketch/v1 {
    schema_version
    source_pdf_sha256
    document_ir_logical_sha256
    producer
    document_family_id

    subjects[]
    sections[]
    blocks[]
    tables[]
    quantities[]
    symbols[]
    packages[]
    template_signatures[]
}
```

### Subject candidate

```text
SubjectCandidate {
    id
    kind: part | family | package_variant | unknown
    raw_designation
    manufacturer_hint?
    normalized_search_key?
    evidence_anchor
}
```

Normalization is a search/index aid, not identity proof.

### Section candidate

```text
SectionCandidate {
    id
    heading_text?
    heading_anchor?
    parent_section_id?
    block_ids[]
    table_ids[]
    deterministic_type_candidates[]
}
```

### Table candidate

```text
SketchTable {
    id
    source_region_id
    heading_path[]
    rows
    columns
    cells[]
    structural_signature
    reconstruction_quality
}
```

Each cell references the original Ferrodoc table cell/source spans.

### Quantity candidate

```text
QuantityCandidate {
    id
    raw_text
    parsed_shape
    raw_unit?
    quantitas_quantity?
    quantity_kind_hint?
    dimensionality?
    anchor
    parse_warnings[]
}
```

Initial shapes:

```text
scalar
range
inequality
tolerance
set
text_unparsed
```

Do not invent values from context. Parsing transforms exact source text into a candidate representation.

### Symbol candidate

```text
SymbolCandidate {
    id
    raw_text
    normalized_symbol?
    anchor
}
```

### Package candidate

Retain raw package/code/variant terms and anchors; package identity is not resolved merely from a string match.

## Deliverable 5: specification regimes

`datasheet-core` defines explicit regimes compatible with Foundry's assertion model:

```text
absolute_maximum
recommended_operating
guaranteed
tested
characterized_typical
application_guidance
informative
derived
inferred
measured
unknown
```

Do not flatten absolute maximum and recommended operating values into the same property without regime.

## Deliverable 6: `ClaimBundle/v1`

A bundle contains candidate/accepted-by-extractor claims, not Foundry promotion decisions.

```text
ClaimBundle/v1 {
    schema_version
    source_pdf_sha256
    document_ir_logical_sha256
    producer
    subject_hints[]
    claims[]
    warnings[]
}

ClaimCandidate {
    local_id
    subject_candidate_id
    predicate_term_id
    regime
    value_candidate_id
    condition_candidate_ids[]
    package_variant_candidate_ids[]
    evidence_anchors[]
    extraction_confidence
    grounding_status
}
```

`grounding_status` must fail closed if selected candidate IDs/anchors cannot be resolved.

The serialized bundle must preserve enough raw/normalized value detail for Foundry to store the original spelling plus Quantitas-backed normalized quantity without duplicating the DocumentIR.

## Deliverable 7: predicate vocabulary interface

Define an interface over versioned predicate vocabulary-as-data:

```text
PredicateTerm {
    id
    canonical_name
    description
    aliases[]
    symbol_aliases[]
    expected_quantity_kind?
    expected_shapes[]
    regime_constraints?
    category_scope?
}
```

The compiler accepts an ontology snapshot/digest as input. It does not compile one fixed global Rust enum containing every electrical property.

Known application predicates may still have typed Rust adapters where useful, but the model/compiler boundary is vocabulary-driven.

## Deliverable 8: producer identity

Every durable compiler result binds:

```text
code revision/version
configuration digest
DocumentIR schema/version
ontology snapshot digest
Quantitas registry digest/version when used
model digest when used
```

Wall-clock run IDs do not enter deterministic semantic identity.

## Deliverable 9: CLI compatibility

Preserve current remote commands while adding explicit local/introspection commands conceptually equivalent to:

```text
datasheet sketch <document-ir-or-pdf>
datasheet compile characteristics <...>
datasheet explain <...>
datasheet model inspect <bundle>
```

Exact CLI shape may evolve, but local deterministic/model extraction must be distinguishable from remote Gemini extraction.

## Tests

Required:

- workspace builds with dependency boundaries enforced;
- `datasheet-core` has no network/PDF/model runtime dependencies;
- `DatasheetSketch` round-trips deterministically;
- every sketch evidence anchor resolves against fixture DocumentIR;
- quantity candidate preserves original text after normalization;
- absolute-max and recommended-operating regimes remain distinct;
- `ClaimBundle` rejects missing candidate/evidence references;
- ontology snapshot digest participates in derivation identity;
- adding a predicate term does not require changing the serialized `ClaimBundle` schema;
- existing CLI smoke cases continue to work or have an explicit migration test.

## Acceptance criteria

FDX1 is complete when:

1. Datasheet-cli is a buildable workspace with the declared reusable boundaries;
2. a pinned Ferrodoc fixture deterministically produces `DatasheetSketch/v1`;
3. quantity/evidence candidates round-trip with exact source anchors;
4. `ClaimBundle/v1` is serialization-stable enough for Foundry FLS6 integration;
5. predicate vocabulary is data-driven rather than a fixed model-output enum;
6. existing remote extraction remains available as a separate application path;
7. no Foundry persistence/domain types are imported into reusable Datasheet core/compiler crates.

## Landed

Record final crate/license choices, schema versions, Ferrodoc/Quantitas pins and compatibility commands after implementation.
