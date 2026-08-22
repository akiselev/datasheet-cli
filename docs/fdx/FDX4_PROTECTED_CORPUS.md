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

Checked schemas live under `schemas/fdx4/`; adversarial tests cover every group
kind, order-independent replay, missing splits, hidden-state misuse,
trainability/restriction refusal, and exclusion/admission conflicts.
