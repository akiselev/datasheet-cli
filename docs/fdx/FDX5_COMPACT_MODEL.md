# FDX5 — Compact IR-native model and active teacher

Status: planned

Owner: Datasheet-cli

Depends on: FDX4 frozen corpus/baselines

## Objective

Train and qualify a small local model over `DatasheetSketch` candidates, export it to a reproducible ONNX bundle, run it through a Rust inference path, and use remote LLM/VLMs only for active-learning/fallback cases where deterministic/compact-model evidence is insufficient.

The first model must not consume page pixels in the ordinary path and must not generate arbitrary engineering values.

## Initial architecture decision

Start with an ELECTRA-Small-class encoder rather than a document VLM or large generative model.

Reference shape:

```text
~14M parameters
12 transformer layers
hidden size 256
embedding size 128
4 attention heads
```

This is a starting implementation target, not a permanent model brand dependency. Preserve a feature/model interface that permits replacing the encoder after measured comparisons.

Also benchmark one small distilled 6-layer/~384-hidden encoder class as a challenger if export/runtime support is straightforward. Do not proliferate architectures before the baseline experiment exists.

## Why text/IR rather than pixels

Ferrodoc has already paid the generic document-understanding cost:

```text
PDF -> native/OCR evidence -> geometry -> regions -> tables -> reading order
```

The FDX model solves semantic linking over compact candidate windows:

```text
what kind of table/section is this?
which cell is min/typ/max/condition?
which predicate does this row mean?
which regime applies?
which subject/package/variant applies?
which evidence spans are conditions?
```

A page-image encoder would duplicate generic layout work, increase compute and weaken the exact evidence-selection boundary.

## Deliverable 1: canonical model feature schema

Define `datasheet-model-features/v1`.

Each candidate window contains text tokens plus structural features available before the model's target decision.

### Text serialization

Conceptual window:

```text
[SECTION]
Recommended Operating Conditions

[HEADERS]
Parameter | Symbol | Conditions | Min | Typ | Max | Unit

[ROW-1]
...

[ROW]
Supply voltage | VDD | - | 1.8 | - | 5.5 | V

[ROW+1]
...

[FOOTNOTE]
1. ...

[SUBJECT]
raw part/family/package hints
```

Default target sequence length: 128 tokens.

Allow 256 for complex rows/tables. Longer context requires an explicit benchmark showing benefit rather than silently increasing all inference cost.

### Structural embeddings/features

Encode at least:

```text
region kind
table row index bucket
table column index bucket
header/body role candidate
normalized x bucket
normalized y bucket
page-position bucket
native/OCR/reconciled evidence source
geometry-quality class
quantity-kind/dimensionality hint
unit/value-shape hint
```

Use embeddings/projections compatible with the selected encoder. Do not serialize every numeric coordinate as verbose text if a bounded structural feature is more stable.

All feature computation is deterministic and versioned.

## Deliverable 2: shared encoder and factor heads

Use one encoder for the first task family with small heads for:

```text
section/table semantic type
cell/column role
specification regime
applicability/condition selection
subject/package/variant relationship
```

Heads output probabilities over candidate-local labels/relationships, not final database facts.

## Deliverable 3: predicate retrieval instead of fixed classifier

Foundry/Datasheet predicates are vocabulary-as-data. Do not implement a permanently fixed `Linear(hidden, N_predicates)` as the primary linker.

Use a two-stage design:

```text
row/context encoder
      |
      v
candidate embedding
      |
      +---- retrieve top-K predicate terms from ontology snapshot
                   |
                   v
             compact reranker
                   |
                   v
             predicate term ID
```

Predicate term text includes:

```text
canonical name
description
aliases
symbol aliases
expected quantity kind/dimension
expected value shapes
regime/category constraints
```

Apply deterministic compatibility filters before neural ranking, especially dimensionality and impossible regime/category constraints.

Adding a new predicate term should be representable by adding vocabulary data and embeddings. Retraining may improve ranking but is not required to represent the new term.

## Deliverable 4: predicate bi-encoder

Train contrastive representations so matching row/context and predicate definitions are near each other.

Hard negatives should dominate over random negatives:

```text
same dimension, wrong property
same symbol family, wrong context
same English noun, wrong regime
same numeric value, wrong row
same device category, adjacent property
```

Cache predicate embeddings by ontology snapshot digest.

## Deliverable 5: compact reranker

Evaluate two options on the protected development set:

1. cross-encode the row window plus each top-K predicate description with the same/smaller encoder;
2. a cheaper MLP/bilinear reranker over row/predicate embeddings plus deterministic compatibility features.

Prefer the cheaper option unless the cross-encoder produces a material held-out precision gain.

`K` begins small (for example 8-16) and is benchmark-selected.

## Deliverable 6: multi-task training objective

Conceptual loss:

```text
L =
    λ_pred_retrieval * contrastive_predicate_loss
  + λ_pred_rerank   * predicate_ranking_loss
  + λ_cell          * cell_role_loss
  + λ_regime        * regime_loss
  + λ_section       * section_loss
  + λ_applicability * applicability_loss
  + λ_scope         * subject_scope_loss
  + λ_consistency   * structural_consistency_loss
```

Do not choose weights by intuition alone. Record them in the training config and tune only on the development partition.

## Deliverable 7: supervision weighting

Consume FDX3 probabilities/classes rather than flattening all labels to equal truth.

Examples:

```text
human_gold          full weight
teacher_reviewed    high weight, separately measured
 deterministic_high high weight after audited precision gate
weak_consensus      posterior/confidence weighted
low-confidence      omit or low weight
```

Exact weights are configuration, not schema semantics.

## Deliverable 8: optional domain-adaptive pretraining

First train/evaluate using the pretrained encoder unchanged except supervised fine-tuning.

Only if error analysis shows vocabulary/context limitations, run additional domain-adaptive pretraining over large unlabeled/weakly labeled datasheet text/table serialization.

Potential objectives:

```text
ELECTRA-style replaced-token detection
masked language modeling if using a non-ELECTRA challenger
```

Domain pretraining uses the same family split discipline: protected evaluation/final-blind content is excluded.

Do not block the first model on this step.

## Deliverable 9: no-generation output contract

Inference produces a versioned result containing selected local IDs:

```text
ModelDecision/v1 {
    model_bundle_digest
    feature_schema_digest
    ontology_snapshot_digest

    predicate_candidate_id
    value_candidate_id
    regime_id
    condition_candidate_ids[]
    subject_candidate_id

    per-factor probabilities
    calibration_status
}
```

Validation then checks every selected ID against the exact sketch/candidate set.

There is no free-form numeric `value` field.

## Deliverable 10: training/export tool

`tools/datasheet-train` supports reproducible operations conceptually equivalent to:

```text
dataset validate
train
export onnx
evaluate
compare
quantize
manifest
```

The training run records:

```text
git commit
dataset manifest/digest
ontology snapshot
feature schema
tokenizer
base model identity
random seeds
optimizer/scheduler
batch/accumulation
sequence lengths
loss weights
epoch/step counts
hardware/software environment facts needed for reproduction
```

Training may use Python/PyTorch. Generated ONNX/model artifacts are the production boundary.

## Deliverable 11: model bundle

Produce an immutable bundle:

```text
model.onnx
model.int8.onnx? 
tokenizer.json
feature-schema.json
ontology-snapshot.json
calibration.json
manifest.json
evaluation-report.json
```

Manifest binds digests for every component plus:

```text
training dataset manifest
code revision
training config digest
metric/evaluator version
license/provenance
```

Do not activate a model if tokenizer/feature/ontology compatibility does not match the manifest.

## Deliverable 12: Rust inference through RTen

Use RTen as the first production inference runtime because it is Rust-native, accepts ONNX and is already used in the broader Ferrodoc ecosystem.

`datasheet-model` must provide:

```text
bundle validation
model loading
feature encoding
dynamic/batched inference
output decoding
probability calibration
grounding validation
resource measurement hooks
```

Keep the backend behind a narrow internal trait if doing so is cheap, but do not add ONNX Runtime/native dependencies until a benchmark demonstrates a need.

## Deliverable 13: quantization

Evaluate int8/uint8 quantization after the fp32 model is qualified.

Report:

```text
bundle bytes
resident memory
p50/p95 latency
batch throughput
per-factor accuracy/precision delta
complete-claim delta
```

Admit quantized inference only if quality loss stays below a declared threshold and there is a material deployment benefit.

Do not assume quantization is faster on every CPU.

## Deliverable 14: training compute gate

The first model must be trainable on one ordinary single-GPU development machine rather than requiring distributed training.

Initial targets:

```text
~500k-2M factorized training examples if corpus provides them
128 token default windows
256 token complex windows
2-4 supervised epochs as an initial search range
full fine-tuning, not LoRA by default
```

These are experiment starting points, not acceptance claims. Record actual GPU memory, wall time and examples/sec.

LoRA/adapters are unnecessary complexity for a ~14M parameter first model unless experiments show a specific benefit.

## Deliverable 15: model qualification

Compare against FDX2 deterministic baseline on identical FDX4 protected cases.

Required slices:

```text
overall development
document-family-held-out
template-held-out
manufacturer-held-out
native evidence
OCR/refined evidence
```

Metrics remain factorized and end-to-end.

Hard gates:

1. zero tolerated ungrounded/generated values;
2. 100% syntactically valid evidence-anchor/candidate references for accepted outputs;
3. no detected split leakage;
4. high-confidence precision must meet a declared threshold before automatic candidate acceptance;
5. a learned model must materially improve at least one declared quality/coverage frontier over FDX2 without unacceptable regressions elsewhere.

A model may be retained as a negative experiment without being activated.

## Deliverable 16: active teacher

Teacher priority score combines:

```text
weak-label disagreement
student entropy/calibration uncertainty
new manufacturer
new template family
new predicate/alias
unusual unit/value form
source conflict
OCR/refinement damage
rare package/variant scope
Foundry enrichment priority
```

Teacher input is compact by default:

```text
source structured facts
section/table/row sketch
candidate predicate definitions
candidate value/evidence IDs
weak-label votes and disagreement
student decision/probabilities
```

Teacher output chooses candidate IDs/labels or `ABSTAIN`. It does not return arbitrary engineering values.

Batch multiple rows from one table/request when practical.

## Deliverable 17: teacher accounting and distillation loop

Record:

```text
provider/model
prompt/schema version
input/output artifact digests where policy permits
token counts
monetary cost
candidate count
accepted labels
review corrections
trigger reasons
```

Metrics:

```text
tokens per accepted teacher label
cost per accepted teacher label
teacher-label audited precision
fraction of corpus requiring teacher
student improvement after adding teacher labels
```

Teacher-reviewed labels feed the next frozen dataset version; they never mutate an old dataset in place.

## Tests

Required:

- deterministic feature encoding for fixed sketch/schema;
- predicate embedding cache invalidates on ontology/model/feature changes;
- dimensional filters prevent impossible predicate retrieval;
- output IDs must exist in candidate set;
- model cannot serialize a free-form value;
- bundle refuses mismatched tokenizer/feature/ontology artifacts;
- same model/input produces stable inference within backend determinism guarantees;
- protected split evaluation uses identical cases as baseline;
- quantized bundle records distinct digest/evaluation;
- teacher output containing arbitrary values or invalid IDs is rejected;
- teacher abstention remains abstention rather than negative truth.

## Acceptance criteria

FDX5 is complete when:

1. one compact local model is frozen with reproducible training/evaluation artifacts;
2. Rust/RTen inference produces grounded candidate selections on real retained datasheets;
3. model quality is compared against FDX2 on family/template/manufacturer held-out slices;
4. no accepted output can introduce an ungrounded engineering value;
5. CPU model size/latency/throughput are measured;
6. optional quantization is admitted only on measured quality/resource tradeoffs;
7. active teacher can label ambiguous candidate decisions without whole-PDF upload in the normal case;
8. teacher cost/coverage is measurable and the resulting labels enter a new immutable dataset version.

## Landed

Record final encoder/runtime architecture, model/dataset manifests, training compute, protected metrics, CPU/quantization benchmarks, calibration gates and teacher economics after implementation.
