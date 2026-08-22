# FDX6 — Production cascade and Foundry integration

Status: planned

Owner: Datasheet-cli runtime/integration with Foundry FLS6

Depends on: FDX5 qualified local model; Foundry FLS5 document plane and FLS4 silver/promotion boundary

## Objective

Move the FDX compiler/model into the real Foundry enrichment path as a progressive, budget-aware cascade. Cheap deterministic/local extraction runs broadly; expensive table/OCR refinement and remote teacher/VLM work run only on ambiguous/high-value cases.

## Deliverable 1: serialized Foundry boundary

Stabilize the production serialization for:

```text
DatasheetSketch/v1
ClaimBundle/v1
DesignConstraintBundle/v1
ModelDecision/v1
ExtractionTrace/v1
```

Foundry consumes serialized contracts and exact Artifactum/IR references. It does not import Datasheet internal model/training implementation types.

## Deliverable 2: progressive pass contract

Expose explicit stages equivalent to:

```text
P0 inspect
P1 cheap document reconstruction
P2 local datasheet compile/model
P3 targeted document refinement
P4 remote teacher/fallback
P5 human review handoff
```

Datasheet-cli directly owns P2 and P4 semantic behavior; Ferrodoc owns P1/P3 document evidence; Foundry owns scheduling/prioritization/persistence.

Each stage consumes immutable identities and produces a new deterministic/provenance-bearing artifact or explicit observational remote result.

## Deliverable 3: P2 local extraction API

Provide a library/CLI operation accepting:

```text
pinned DocumentIR
ontology snapshot
Quantitas registry snapshot
qualified model bundle (optional; deterministic-only mode supported)
compiler configuration
```

and returning:

```text
DatasheetSketch
ClaimBundle candidates
ExtractionTrace
escalation recommendations
```

The operation is offline/network-free when no remote teacher is requested.

## Deliverable 4: escalation reasons

Use stable reason codes, initially:

```text
insufficient_geometry
ambiguous_table_structure
quantity_parse_failure
predicate_low_margin
regime_conflict
subject_scope_ambiguous
source_conflict
weak_label_disagreement
model_low_confidence
unseen_template
unseen_manufacturer
unsupported_formula_or_graph
```

A recommendation includes target page/region IDs and required capability where possible.

## Deliverable 5: targeted P3 refinement

Translate eligible escalation reasons into Ferrodoc refinement requests rather than rerunning the whole PDF.

Example:

```text
ambiguous_table_structure
  -> table region R17
  -> request stronger table recognition only for R17/page 6
  -> new pinned IR generation
  -> re-run affected DatasheetSketch/claim derivations
```

Recompute only descendants whose semantic inputs changed. Unrelated accepted claims remain cache hits.

## Deliverable 6: remote P4 fallback

Remote teacher/fallback receives compact structured context by default.

Allowed request modes:

```text
candidate_selection
source_vocabulary_mapping
template_review
ambiguous_row_review
compact_region_interpretation
whole_pdf_vlm_last_resort
```

`whole_pdf_vlm_last_resort` must require explicit policy and be separately metered.

Teacher results retain provider/model/prompt/schema/token/cost provenance and must pass the same grounding validator before normal claim candidates are emitted.

## Deliverable 7: policy profiles

Expose useful runtime profiles conceptually:

```text
bulk-cheap
interactive
high-confidence
training-harvest
```

Example intent:

### `bulk-cheap`

P0-P2 broadly, no remote calls, P3 only under cheap explicit bounds.

### `interactive`

Allow targeted P3 and small P4 request when a user is waiting for one requested property.

### `high-confidence`

Escalate important disagreements/low-margin candidates before recommending promotion.

### `training-harvest`

Prioritize novel/disagreeing examples for teacher/review labels rather than maximizing immediate claim coverage.

Foundry remains the final budget/policy authority.

## Deliverable 8: local model hot path

Load the qualified model bundle once per worker/process where possible.

Support row/table batching while preserving per-candidate trace identity.

Measure cold-load and warm throughput separately. Do not repeatedly load the model for each row/PDF.

## Deliverable 9: cache/derivation boundaries

Separate cache identities for:

```text
DocumentIR generation
DatasheetSketch
quantity/candidate index
rule extraction
model decisions
ClaimBundle assembly
refined region IR
teacher result
```

A new promotion rule does not invalidate extraction. A new ontology snapshot invalidates predicate-link descendants but should not require re-OCR. A new model invalidates model decisions/claim descendants but reuses deterministic sketch/candidates.

## Deliverable 10: incremental document revisions

Where Foundry/Ferrodoc can identify unchanged page/table evidence across document revisions, reuse lower-level extraction artifacts according to their stable content identity.

Do not assume two visually similar revisions are byte/semantic identical; reuse only content-addressed sub-results whose inputs actually match.

## Deliverable 11: confidence/calibration routing

Apply model calibration only to the model factors it was evaluated for.

A final extraction confidence remains a vector/structured explanation including:

```text
identity evidence
source authority/lineage
structural extraction confidence
predicate model probability/margin
regime confidence
grounding validity
source agreement/conflict
```

Do not collapse everything into an unexplained scalar that Foundry treats as promotion truth.

## Deliverable 12: feedback into training

Production generates new immutable observations:

```text
novel template
new manufacturer/source field
model disagreement
teacher correction
human correction
source conflict
promotion rejection reason
cross-validation finding
```

FDX6 exports eligible records to the next training-candidate derivation. Retraining is explicit and produces a new dataset/model version; production corrections never mutate old model provenance.

## Deliverable 13: quality/drift monitoring

Track by model/compiler version:

```text
candidate volume
accepted-by-extractor coverage
abstention/escalation reasons
P3 refinement rate
P4 remote rate/cost
invalid grounding count
predicate/regime confidence distributions
new template/manufacturer rates
source-conflict rates
sampled audited precision
```

Trigger reevaluation/retraining proposals on meaningful drift, not automatically on every new document.

## Deliverable 14: first end-to-end proving run

Run the full path on the initial analog/power corpus:

```text
source structured facts
manufacturer PDFs
Ferrodoc DocumentIR
DatasheetSketch
FDX2 rules
FDX5 model
selective P3 cases
limited P4 active teacher cases
Foundry Silver ClaimBundle ingestion
sample promotion/cross-validation
```

Attach exact version/digest identities and cost/resource metrics.

## Deliverable 15: remote-dependency removal from default commands

Make the ordinary extraction command choose the local path by default once it is qualified for a supported task family.

Preserve explicit remote commands/options for:

```text
unsupported task family
user-requested remote extraction
last-resort fallback
teacher/review workflow
```

Do not silently send source documents to a remote model.

## Tests

Required:

- P2 local operation runs with network disabled;
- escalation identifies exact target region where possible;
- P3 changes only affected downstream derivations;
- unmodified claims remain cache/reuse hits after targeted refinement;
- P4 response with invented value fails grounding validation;
- whole-PDF remote mode requires explicit policy;
- remote cost/token trace is retained;
- model bundle is reused across batched documents rather than loaded per row;
- ontology/model changes invalidate only appropriate descendants;
- production feedback creates new immutable training candidates rather than mutating historical labels;
- deterministic-only mode remains available if the learned model is unqualified/unavailable.

## Acceptance criteria

FDX6 is complete when:

1. Foundry can run supported datasheet enrichment through the local Datasheet compiler/model path without a remote model;
2. ambiguous tables/regions can selectively escalate through Ferrodoc refinement;
3. remote teacher/fallback is an explicit measured minority path on the proving corpus;
4. every emitted claim value remains grounded to immutable source/evidence candidates;
5. model/compiler/dataset/ontology identities make every candidate extraction reproducible;
6. production feedback can generate the next immutable training dataset version;
7. cost/latency/coverage/quality metrics demonstrate the cascade's actual economics;
8. the existing full-PDF Gemini path is no longer required for the supported first task family.

## Landed

Record Foundry/Ferrodoc pins, proving-run manifests, P0-P5 rates, CPU performance, remote cost, extraction quality and feedback/retraining outputs after implementation.
