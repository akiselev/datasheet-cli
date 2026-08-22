# FDX3 — Weak-supervision and training-data compiler

Status: planned

Owner: Datasheet-cli semantic compiler; Foundry persists immutable observations/votes/datasets

Depends on: FDX2 deterministic candidates; Foundry FLS2 structured-fact transport and FLS4 silver lineage boundary

## Objective

Generate a large, diverse, auditable training corpus without paying an LLM to read every PDF.

FDX3 treats structured source facts, document structure, template regularity and independent engineering representations as noisy labeling functions. Labelers vote or abstain on factorized candidate decisions. The compiler retains every vote and combines them into probabilistic/reviewable training labels.

## Core principle

Do not supervise free-form JSON generation.

Supervise these factors separately:

```text
section/table semantic type
cell/column role
predicate link
specification regime
subject/package/variant scope
applicability/condition evidence
```

The final `ClaimBundle` remains deterministic assembly over selected candidates.

## Deliverable 1: source-fact input contract

Consume Foundry `StructuredFactObservation/v1` or an equivalent serialized fixture contract containing:

```text
source/publisher/transport identity
source artifact + exact record locator
manufacturer / MPN / family / package hints
external namespace/schema version/field key/name
raw value/raw unit/value-shape hint
category path
rights snapshot
```

The Datasheet compiler does not require a live source API to reproduce labels; retained source observations are sufficient.

## Deliverable 2: source vocabulary maps

Define a versioned mapping from source-native fields to predicate candidates:

```text
SourceVocabularyMap/v1 {
    id
    external_namespace
    external_schema_version
    external_field_key
    external_field_name?

    predicate_term_id
    expected_quantity_kind?
    expected_value_shapes[]
    regime_hint?
    category_scope[]

    provenance
    mapping_version
}
```

Mappings may originate from deterministic schema metadata, human review or a compact LLM taxonomy-mapping task. They are not source-plugin assertions.

One reviewed mapping is intentionally reusable across every record carrying that stable source field.

## Deliverable 3: training candidate identity

A `TrainingCandidate` is one factorized decision over immutable sketch/source inputs.

```text
TrainingCandidate/v1 {
    id
    task
    document_ir_logical_sha256
    datasheet_sketch_digest
    local_candidate_ids[]
    source_fact_record_ids[]
    ontology_snapshot_digest
    document_family_id
    template_family_id?
    manufacturer_key?
}
```

Candidate identity is content/producer derived. Physical lake row location is not identity.

## Deliverable 4: `LabelVote/v1`

Every labeling function produces an auditable vote:

```text
LabelVote/v1 {
    id
    training_candidate_id
    task

    labeler_id
    labeler_version

    decision: vote | abstain
    label_json?
    abstain_reason?

    input_record_ids[]
    lineage_groups[]
    reason_codes[]

    producer
}
```

Do not store only an aggregate confidence. Downstream debugging/training must be able to reproduce which labelers agreed or conflicted.

## Deliverable 5: lineage groups

Define source-lineage metadata so copies of one upstream value do not become fake independent evidence:

```text
LineageGroup/v1 {
    id
    publisher_key?
    upstream_dataset_fingerprint?
    source_schema?
    observation_epoch?
    derivation_parent_ids[]
}
```

Two distributor observations with indistinguishable upstream provenance may still be useful for commercial corroboration, but they do not receive the same independence bonus as unrelated publishers/representations.

## Deliverable 6: generic document labelers

Implement these labeler identities first:

### `section.heading.v1`

Votes for section/regime from high-precision heading aliases. Abstains on generic/ambiguous headings.

### `section.table_header_schema.v1`

Votes for table semantic/schema compatibility from column/header pattern.

### `cell.min_typ_max_header.v1`

Votes cell/column roles from explicit Min/Typ/Max/Minimum/Typical/Maximum headers.

### `quantity.dimension.v1`

Filters/votes predicate compatibility from Quantitas dimensionality/quantity kind. It is normally a constraint/filter, not enough by itself to identify a specific predicate.

### `predicate.symbol_alias.v1`

Votes predicate candidates from exact/normalized symbol aliases in the ontology snapshot.

### `predicate.row_name_alias.v1`

Votes from parameter-name aliases/fuzzy normalized forms under strict thresholds.

### `alignment.value_unique.v1`

Given a structured source fact with normalized value/dimension, votes for the unique document row/value candidate matching that value inside a plausible section. Abstains if zero or multiple plausible matches remain.

### `alignment.range.v1`

Matches source min/max ranges to document endpoint candidates with compatible dimension and regime.

### `template.cluster.v1`

Votes table/column semantics only for a previously reviewed high-purity template family.

### `applicability.footnote_link.v1`

Votes condition/applicability evidence from explicit row/header footnote markers and defensible local scope.

### `scope.package.v1`

Votes package applicability when source package and document package candidate agree under a reviewed normalization/mapping.

### `scope.part.v1`

Votes subject scope from exact observed MPN/family evidence around the section/table.

### `value.null_dash_convention.v1`

Votes that blank/dash means unspecified only after table-level convention is established; never emits numeric zero.

### `ranking.regime_conflict.v1`

Creates a hard training negative/ranking preference when an otherwise matching value occurs in the wrong specification regime. This is not a negative Foundry assertion.

### `revision.structure_stability.v1`

Boosts structural/template confidence when the same normalized table structure recurs across related manufacturer document revisions. Related revisions stay in one evaluation family.

## Deliverable 7: TI labelers

Implement stable IDs:

```text
ti.identity_gate.v1
ti.field_map.v1
ti.value_align_unique.v1
ti.range_align.v1
ti.unit_kind.v1
ti.package_align.v1
```

Behavior:

1. establish defensible MPN/family identity between the structured record and datasheet before technical votes count;
2. map stable TI parametric field identity through `SourceVocabularyMap`;
3. normalize the source value/range/unit;
4. find dimension/regime-compatible document candidates;
5. vote only when the alignment is unique or otherwise high precision;
6. retain TI API and TI datasheet as shared-publisher lineage rather than pretending they are independent authorities.

## Deliverable 8: DigiKey labelers

Implement:

```text
digikey.identity_gate.v1
digikey.parameter_id_map.v1
digikey.parameter_text_alias.v1
digikey.value_align_unique.v1
digikey.category_prior.v1
digikey.package_scope.v1
digikey.datasheet_link.v1
```

Prefer stable source parameter IDs over reinterpreting free-form names on every record.

`category_prior` may rerank plausible predicates but must not override incompatible dimensionality/evidence.

A DigiKey datasheet URL is document linkage evidence, not manufacturer authority by itself.

## Deliverable 9: Mouser labelers

Initial scope reflects what the retained source adapter actually supplies:

```text
mouser.mpn_identity.v1
mouser.datasheet_link.v1
mouser.category.v1
mouser.package.v1
mouser.lifecycle.v1
mouser.replacement.v1
```

Do not invent electrical-parametric labelers unless a retained Mouser source schema actually exposes the required fields.

## Deliverable 10: generic manufacturer/PIM labelers

Implement:

```text
manufacturer.field_map.v1
manufacturer.value_align.v1
manufacturer.range_align.v1
manufacturer.package.v1
manufacturer.variant_scope.v1
manufacturer.order_code.v1
manufacturer.family_scope.v1
```

These consume reviewed source vocabulary mappings for REST/GraphQL/PIM/BMEcat/ETIM/catalog field identities.

A field mapping may be reused across many parts/documents but remains versioned so later taxonomy corrections do not rewrite history.

## Deliverable 11: EDA supervision

Consume serialized observations from first-party/other EDA parsers without parsing those formats inside Datasheet-cli.

Initial labelers:

```text
eda.pin_number_set.v1
eda.pin_name_by_number.v1
eda.pin_count.v1
eda.package_code.v1
eda.pad_count.v1
eda.pin_to_pad.v1
eda.body_dimensions.v1
eda.model_package.v1
```

Authority/lineage is part of the vote: manufacturer EDA can receive a different prior than a community library, but even community observations can be valuable alignment signals.

These tasks may be persisted before their corresponding learned heads are implemented.

## Deliverable 12: CMSIS/SVD supervision

Implement only semantics those sources defensibly provide:

```text
cmsis.device_identity.v1
cmsis.cpu_core.v1
cmsis.memory.v1
cmsis.document_link.v1
svd.peripheral.v1
svd.register.v1
```

Do not stretch device metadata into electrical-property claims it cannot support.

## Deliverable 13: cross-source combiners

Implement these explicit vote producers:

```text
agreement.independent.v1
agreement.structured_document.v1
agreement.subject_triangulation.v1
conflict.source.v1
```

### Independent agreement

Boost only after lineage deduplication.

### Structured/document agreement

A structured fact matching exact document evidence is strong alignment supervision even when both ultimately come from the same manufacturer; its strength is evidence localization, not independent authority.

### Subject triangulation

Combine independent identifier/package/document signals to improve which part/family/variant a table applies to.

### Source conflict

Retain the conflicting candidates and emit abstention/review priority. Never silently select one value solely because more transports copied it.

## Deliverable 14: label combination

Start with an auditable calibrated weighted combiner:

```text
hard gates
  |
high-precision votes
  |
lineage deduplication
  |
calibrated weights from reviewed development set
  |
probabilistic target + entropy/disagreement
```

Define `TrainingLabel/v1`:

```text
TrainingLabel/v1 {
    id
    training_candidate_id
    task
    target_json
    posterior_probability?
    supervision_class
    input_vote_ids[]
    combiner_version
    review_id?
}
```

Supervision classes:

```text
deterministic_high
weak_consensus
teacher_reviewed
human_gold
```

Later evaluate a Snorkel-style generative label model that estimates labeler accuracies/correlations from overlap, but do not make an opaque combiner a prerequisite for the first corpus.

## Deliverable 15: active-learning priority

Compute a queue score from:

```text
vote disagreement/entropy
model uncertainty when FDX5 exists
new manufacturer
new template family
new predicate/alias
unusual unit/value syntax
OCR/refinement damage
source conflict
rare package/variant scope
high Foundry enrichment priority
```

This queue becomes the input to FDX5 teacher/human review rather than sampling PDFs uniformly.

## Rights enforcement

Training rights are hard gates.

The compiler may derive ordinary extraction/validation findings from a source when permitted while refusing to export the same observation into a training dataset if `may_train` is false.

A `TrainingLabel`/`TrainingExample` retains the exact rights snapshots contributing to its eligibility decision.

## Tests

Required:

- every labeler has retained positive/negative/abstention fixtures;
- identity gates block technical votes for mismatched subjects;
- value alignment abstains on duplicate matching values;
- dimensional incompatibility blocks a predicate vote;
- wrong-regime matching value can be a ranking negative without becoming a negative engineering assertion;
- blank/dash never becomes zero;
- lineage-deduplicated agreement differs from naive source-count agreement;
- source conflict is retained and does not force a winner;
- a source vocabulary mapping version change creates new derivation identity;
- label combination is deterministic for fixed votes/calibration;
- training-rights forbidden observations cannot produce exportable training examples;
- all votes trace to exact source/sketch/evidence records.

## Acceptance criteria

FDX3 is complete when:

1. at least two materially different structured source families produce retained supervision votes on a real datasheet corpus;
2. one stable source vocabulary mapping supervises many records without per-PDF LLM calls;
3. every aggregate training label can be decomposed back into exact votes and source/evidence inputs;
4. cross-source agreement is lineage-aware;
5. conflicts/ambiguity increase review priority rather than hallucination pressure;
6. a reviewed development sample provides measured precision/calibration for major labelers;
7. the resulting training labels are large enough to begin FDX4/FDX5 while remote LLM labeling remains optional.

## Landed

Record source schemas, vocabulary-map versions, labeler precision/coverage/abstention, rights exclusions, vote counts and active-learning queue composition after implementation.
