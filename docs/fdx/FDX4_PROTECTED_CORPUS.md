# FDX4 protected corpus

Status: overlap-aware protected-corpus v2 and reviewed open-document expansion complete;
model qualification remains blocked.

FDX4 freezes the audited FDX3 examples into an immutable corpus manifest with
six explicit partitions: train, development, template-held-out,
document-family-held-out, manufacturer-held-out, and final-blind-test. Split
membership is dataset metadata and does not alter `DatasheetSketch` identity.

The builder rejects cross-split reuse of document families, template families,
manufacturer groups, publisher lineage, revision groups, exact/near-duplicate
groups, or synthetic-derivative roots. An example must be retained and
training-permitted with no restriction codes. Excluded examples cannot also be
members. Final-blind-test is the only split allowed to be marked hidden.

## Production corpus

`evidence/fdx4-protected-corpus-manifest.json` binds the exact audited FDX3 NASA
bundle at Datasheet `430d487` and Foundry `bb7f197`. All 210 real examples come
from one March 1998 NPSL table, one document family, one publisher lineage, one
revision group, and one near-duplicate group. They therefore remain together
in train.

No synthetic derivative is admitted to an evaluation split: any derivative of
these examples would inherit the NASA synthetic root/document/lineage grouping
and remain train-only. No artificial manufacturer key is assigned. The five
evaluation splits are present but empty, each with an exact insufficiency
limitation. In particular, there is no hidden rights-cleared test artifact.

The corpus is consequently leakage-safe and replayable, but
`model_qualification_ready` is false. FDX5 must not begin until genuinely
distinct rights-cleared development, held-out, and hidden groups are acquired
and independently audited.

This is a corpus insufficiency result, not a terminal source-exhaustion proof.
The authoritative grouping contract is split-specific: each held-out slice
must be disjoint from training at the grouping level it declares. The current
v1 manifest is conservatively stricter—it assigns one split per member and
rejects every group kind across all splits—so six nonempty v1 splits do imply
six values per populated grouping dimension. That implementation constraint is
not a universal data prerequisite; declared overlap among evaluation tags needs
an additive manifest version. A `manufacturer_key` is grouping metadata rather
than proof of manufacturer authority. FDX4 also permits independently synthetic
protected corpora, while requiring every derivative of an existing example to
remain with its root.
Synthetic-only results must be reported as such and cannot establish real-world
manufacturer/template generalization. No learned FDX5 model may be trained
until the frozen corpus supplies truthful nonempty partitions and every model
qualification gate is satisfied.

Checked schemas live under `schemas/fdx4/`; adversarial tests cover every group
kind, order-independent replay, missing splits, hidden-state misuse,
trainability/restriction refusal, and exclusion/admission conflicts.

## Additive v2 expansion

`fdx4.protected-corpus-manifest/v2` remains additive to the unchanged v1
contract. V2 projects members through slice tags, so evaluation tags may overlap
with each other. Every non-train slice declares `held_out_from_train_by`, and
validation compares that slice against train for exactly those dimensions.
Exact-content, near-duplicate, revision, and synthetic-root separation are
mandatory baselines; template-, document-family-, and manufacturer-held-out
slices additionally declare their named grouping level. Publisher lineage and
the other meaningful dimensions can be declared per slice. A member can never
be tagged both train and evaluation.

`evidence/fdx4-protected-corpus-manifest-v2.json` contains 266 members:

- 210 audited NASA examples remain train-only;
- 55 independently reviewed TPX00242 source-native electrical/operating cells
  carry overlapping development, template-held-out, document-family-held-out,
  and manufacturer-held-out tags, all train-disjoint at every declared level;
- one independently rooted minimized synthetic case is a visible development
  contract case only. It is not admitted to final-blind-test.

The NASA rows carry the truthful template grouping
`nasa-nepp-npsl.html-table.lp-npt` and the explicit grouping cohort
`nasa-npsl:unresolved-manufacturer-cohort`. The latter is not a manufacturer
identity or authority assertion; it prevents unknown source rows from silently
bypassing manufacturer-level disjointness. V2 refuses any slice that declares
template, manufacturer, or revision separation while either its members or the
train members lack that grouping metadata.

The TPX source is Arduino `docs-content` commit `ad72210c...`; exact Markdown,
repository license, rendered-PDF, Foundry row-export, policy, authority, and
Artifactum identities are pinned. The 55 admitted cells are enumerated in
`evidence/fdx4-tpx00242-independent-review.json`. The deterministic builder
emits 31 vocabulary maps, 55 reviewed-development examples, 165 votes including
55 explicit shape abstentions, 55 combined labels, and 55 training examples.
No regime is inferred and no authority for underlying component manufacturers
is claimed.

Foundry source/profile semantics are pinned at `d4e35dd`; the resulting corpus,
TPX supervision bundle, six slice artifacts, and visible synthetic contract
bytes are physically retained through Foundry's audited Artifactum graph.

`contract_plumbing_ready` is false and every evaluation slice deliberately sets
`supports_real_world_generalization` false. The same single real Arduino
document backs four declared evaluation views, while final-blind-test is empty:
the repository-visible synthetic payload cannot truthfully be treated as blind.
Therefore `model_qualification_ready` remains false and FDX5 training must not
begin.
