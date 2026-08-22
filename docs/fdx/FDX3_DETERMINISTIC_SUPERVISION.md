# FDX3 deterministic supervision foundation

Status: contract foundation implemented; production acceptance pending.

This phase adds the deterministic, auditable supervision layer without starting
a production corpus or model. Datasheet owns electronics vocabulary mapping,
the stable labeler catalog, vote construction, reviewed-development/calibration
interfaces, and deterministic combination. Foundry owns physical persistence,
rights policy, source lineage, and frozen datasets.

## Contracts

- `SourceVocabularyMap/v1` binds a source schema/namespace/field key to a term
  in an exact predicate-vocabulary snapshot and retains mapping observations,
  producer/configuration, and version.
- Every label result is `silver.label_votes/v1`, including abstentions. A vote
  retains exact source observations, source producer, pinned Ferrodoc state and
  evidence anchors when document-derived, lineage groups, rights snapshots,
  reason codes, labeler identity/version, and Datasheet semantic identity.
- `silver.training_labels/v1` names every input vote, fixed combiner/configuration,
  target, supervision class, integer posterior/disagreement, and independent
  lineage count.
- `silver.structured_facts/v1` and `training.examples/v1` are durable portable
  boundary contracts. Corpus family/split assignments remain outside
  `DatasheetSketch` identity.
- Reviewed development sets and calibration snapshots are immutable, ontology
  bound, fixed-point interfaces. There is no floating-point weight identity.

Checked schemas are under `schemas/fdx3/`. Datasheet semantic IDs are not
physical Foundry lake record IDs; Foundry assigns its own provenance-bound row
identity when persisting them.

## Deterministic combination

The frozen catalog contains every initial generic and source-specific labeler
identity named by the cross-repository plan. Combination applies:

1. hard identity gates;
2. fail-closed retention/training-rights gates;
3. calibrated deterministic-high and weak votes;
4. connected-component lineage deduplication, so copied distributor records do
   not become independent evidence;
5. deterministic fixed-point scoring and explicit uncertainty.

An unmet gate, no positive votes, missing calibration, insufficient independent
lineage, low posterior, or source conflict produces a retained abstention. None
is converted into a negative real-world label. Conflicting votes sharing one
lineage refuse rather than fabricating a winner.

## Evidence and scope

`tests/fixtures/fdx3_synthetic_fls5_supervision.json` is a minimized synthetic
FLS5-shaped state/source fixture. It proves pinned-state/anchor validation,
structured-source observations, copied versus independent lineage, and separate
retention/training/remote-disclosure decisions. It is not a real datasheet or a
rights-cleared production supervision family.

Full FDX3 production acceptance remains open until all of the following exist:

- committed FLS5 real document states and evidence-bearing analog/power cells;
- at least two rights-cleared structured-fact source families, or a documented
  access/rights blocker without prohibited substitute data;
- a reviewed real development set with measured per-labeler precision;
- retained production votes/abstentions and combined labels through Foundry;
- production counts, abstention rates, calibration, and failure buckets.

No FDX4 split corpus, FDX5 trained model, or FDX6 production cascade is claimed.
