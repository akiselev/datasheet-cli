# FDX4 — Template propagation, synthetic corpus, and protected evaluation

Status: planned

Owner: Datasheet-cli

Depends on: FDX3 label compiler

## Objective

Turn weak supervision into a leakage-resistant, diverse training/evaluation corpus and multiply reviewed work through table-template propagation and synthetic DocumentIR/Sketch generation.

FDX4 must make it difficult for a model to look good merely by memorizing one manufacturer's recurring document layout.

## Deliverable 1: template signatures

Compute deterministic table/section signatures from generic structure plus limited source identity where appropriate:

```text
normalized section-heading path
column count
normalized column-header tokens
row/column span pattern
header-depth pattern
symbol-column presence
unit-column presence
condition-column presence
x/width ratio buckets
row density/geometry ratios
manufacturer/template hints when available
```

The signature is a clustering feature, not proof that two tables have identical semantics.

## Deliverable 2: template-family clustering

Cluster recurring structures within and across manufacturers.

Required outputs:

```text
template_family_id
member table IDs
representative/medoid IDs
structural purity metrics
semantic label distribution where reviewed
outlier score
```

A review/teacher can label the representative of a high-purity cluster and propagate the mapping only under an explicit versioned `template.cluster` labeling function.

Outliers are not forced into the propagated label.

## Deliverable 3: template review workflow

Review one representative with context:

```text
section heading/path
normalized headers
several representative rows
candidate column roles
sample source anchors
cluster size/purity/outliers
```

Review result can define:

```text
table semantic type
column roles
regime constraints
known manufacturer/template scope
```

One reviewed template should be able to supervise thousands of structurally identical tables without one LLM call per document.

## Deliverable 4: synthetic sketch/IR generator

Generate exact labeled examples at the domain-structure level before paying to render PDFs.

Synthetic families should vary:

```text
column order
missing Min/Typ/Max fields
merged headers
multi-line parameter names
multi-line conditions
row/column spans
footnotes
package-specific columns
variant-specific columns
unit column vs embedded units
µ/u normalization
Ω/Ohm variants
scientific notation
inequalities
± tolerances
ranges
dash/blank/NA conventions
symbolic relative values such as VDD+0.3V
repeated identical numeric values in different regimes
similar predicate names with identical dimensions
```

Generated truth includes exact:

```text
section/regime
cell roles
predicate term
value candidate
condition/applicability spans
subject/package scope
```

Synthetic data augments real data; it does not replace held-out real evaluation.

## Deliverable 5: rendered/degraded synthetic subset

Render a minority of synthetic families into PDFs/documents and feed them back through Ferrodoc to expose downstream training to reconstruction noise.

Controlled variations may include:

```text
font changes
kerning/spacing changes
vector vs raster text
low DPI
rotation
compression
blur/noise
broken/absent ToUnicode mapping
multi-column pages
ruling-line and whitespace-only tables
```

Every rendered/degraded derivative shares the parent `document_family_id` so related variants cannot cross train/test partitions.

## Deliverable 6: real corpus family identity

Define document families using conservative signals such as:

```text
exact PDF/text hash ancestry
manufacturer + product family
document revision lineage
near-duplicate normalized page/table hashes
known template lineage
source document-version relations
```

If uncertain whether two revisions are related, prefer keeping them in the same family to reduce leakage.

## Deliverable 7: `TrainingExample/v1`

Export examples as immutable references rather than giant flattened prompt blobs:

```text
TrainingExample/v1 {
    example_id
    task

    document_ir_logical_sha256
    datasheet_sketch_digest
    candidate_ids[]
    target_json
    training_label_id

    document_family_id
    template_family_id?
    manufacturer_key?

    ontology_snapshot_digest
    rights_snapshot_ids[]
    producer
}
```

A dataset manifest records exact shard artifacts, example count, split policy, ontology/schema revisions and input parent batches.

## Deliverable 8: split policy

Required logical slices:

```text
train
development
document-family-held-out
template-held-out
manufacturer-held-out
final-blind-test
```

The first four may overlap in the sense of evaluation tags only if the manifest makes their policy explicit; the final blind test must be protected from model/labeler iteration.

Random row-level split is not a valid primary evaluation.

No document family may cross training and any family-held-out/final partition. Related synthetic degradations remain together.

## Deliverable 9: trusted reviewed corpus

Build a claim-level trusted set rather than manually annotating entire PDFs unnecessarily.

Review should verify:

```text
subject scope
section/regime
predicate
min/typ/max/value role
exact raw value/unit
condition/applicability evidence
evidence anchor
```

Prefer a few thousand diverse, carefully reviewed claims spanning manufacturers/templates over tens of thousands of shallow whole-PDF labels.

Teacher-generated labels are not automatically `human_gold`. Preserve their supervision class separately.

## Deliverable 10: hard-negative generation

Generate difficult ranking negatives from real candidate sets:

```text
same quantity dimension, wrong predicate
same numeric value, wrong section/regime
same symbol, different device context
same predicate, wrong package/variant
nearby row, wrong value candidate
same source field, wrong part identity
```

These are training negatives for candidate selection. They do not become negative real-world assertions.

## Deliverable 11: benchmark metrics

Report each factor separately:

```text
section/table-type precision/recall
cell-role accuracy
predicate retrieval recall@K
predicate rerank top-1/MRR
regime precision/recall
condition/applicability span F1/exact match
subject/package scope accuracy
complete claim exact match
evidence-anchor validity
grounding validity
abstention/coverage
calibration (ECE/Brier where appropriate)
```

Also report end-to-end quality by:

```text
manufacturer
template family
document type/revision family
native vs OCR/refined evidence
supervision class
```

## Deliverable 12: safety gates

Qualification treats these as hard failures:

```text
model/compiler value not present in deterministic candidate set
invalid evidence anchor
wrong source document identity
silent absolute-max/recommended-operating regime collapse
training/test family leakage
use of training-rights-forbidden example
```

Coverage is not improved by suppressing these failures from the denominator.

## Deliverable 13: FDX2 baseline freeze

Run the FDX2 deterministic compiler on every protected slice before training the FDX5 model.

The evaluation artifact records:

```text
compiler commit/configuration
ontology/Quantitas snapshots
Ferrodoc version/IR schema
corpus manifest
per-case outputs
metric code digest
results
```

FDX5 must compare on identical cases.

## Tests

Required:

- related revisions cannot cross family split;
- synthetic derivatives inherit parent family;
- template propagation refuses low-purity/outlier clusters;
- dataset builder rejects rights-forbidden examples;
- manifest reproduces exact shard/example identities;
- final blind cases are inaccessible to ordinary training export path;
- hard negatives reference valid alternative candidates but are never emitted as database negations;
- metrics count failures/missing predictions honestly;
- deterministic baseline report is immutable/content-identifiable.

## Acceptance criteria

FDX4 is complete when:

1. a real weakly supervised corpus is frozen with reproducible family/template/manufacturer splits;
2. at least one reviewed table template propagates supervision to many real tables with measured audited precision;
3. a synthetic structure corpus covers the known first-task edge cases and a rendered subset has passed through Ferrodoc;
4. a trusted claim-level evaluation set exists independently of weak-label training targets;
5. FDX2 baseline reports are frozen on all protected slices;
6. no detected family/template leakage or rights violation remains in the qualified dataset.

## Landed

Record dataset manifests, split policy/version, template families, reviewed corpus size/composition, synthetic families and frozen baseline reports after implementation.
