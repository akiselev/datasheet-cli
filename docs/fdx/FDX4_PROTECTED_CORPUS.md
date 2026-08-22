# FDX4 protected corpus

Status: protected-corpus contracts and physical persistence complete; model qualification blocked.

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
