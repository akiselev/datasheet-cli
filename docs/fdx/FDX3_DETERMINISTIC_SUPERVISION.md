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

## Production source eligibility audit

`evidence/fdx3-production-readiness-2026-08-22.json` is the immutable,
identity-bound selection manifest for the currently retained real candidates.
It contains hashes and decisions only, never source PDF, table text, or other
potentially restricted content. Its contract is checked by
`schemas/fdx3/production-readiness-v1.schema.json` and fails closed unless a
family has both an explicit `training: allow` policy and suitable analog/power
structured facts or evidence-bearing electrical cells.

The audit now admits one bounded production family, still short of the required
two independent families:

- NASA NEPP NPSL is committed in Foundry revision `7f90a7e` with a policy that
  permits local training and derived facts while denying remote disclosure and
  leaving raw republication under review. The exact pinned March 1998
  low-power NPN HTML table yielded 35 part mentions and 210 source-native
  current/voltage/gain facts. Six checked-in `SourceVocabularyMap` records bind
  only the exact `I_C`, `V_CBO`, `V_CEO`, `V_EBO`, `h_FE` range and `h_FE`
  condition fields. They deliberately make no absolute-maximum, current
  availability, manufacturer, or manufacturer-spec authority claim.

- Raspberry Pi RP2040 document policy is `training: review_required`. The
  policy itself is also still part of the uncommitted FLS5 working tree. The
  retained Ferrodoc qualification state is additionally a page-only two-column
  pin-description table, not an analog/power electrical-specification table,
  and the corresponding Foundry FLS5 state has not been committed.
- The Microchip SAMD21 public-datasheet policy is `training: deny` and no
  committed FLS5 state exists; that policy is likewise not yet committed. Its
  retained PDF hash is recorded only as audit evidence and is not copied into
  supervision.
- The distinct Microchip SAMD21 CMSIS Apache profile is `training: allow`, but
  the retained Bronze/Silver batches contain 34 part-identity mentions rather
  than analog/power structured facts. CMSIS identity labelers are selected for
  future alignment, but this source does not count as electrical supervision.
- DLA PUB LOG remains rights-approved, but its retained 25,499
  `P_CHARACTERISTICS_PICK` observations flatten each item's characteristics
  into an unordered token bag. MRC/value/unit association is absent, the
  current lossless CSV is HTTP-403-blocked in this environment, and the
  retained DVD tables are encrypted. It cannot receive an exact vocabulary map
  or count as the second family.
- DLA QPD/QPL/QML and SMD are not an independent second publisher family. The
  official qualification pages expose approved-source/qualification identity
  and linked PDFs, not a retained source-native analog/power field table. Any
  electrical cells in a linked drawing still require artifact-specific control
  review and a committed FLS5/Ferrodoc state.
- NASA NTRS record `20205003326` is public-use-permitted, contains no declared
  third-party material, and is not export-controlled, but its derating table is
  image-only general guidance rather than part-specific structured evidence.
  It remains blocked on committed FLS5 cells and would still be the same NASA
  publisher family as NPSL.
- LibrePCB/other open EDA and Microchip CMSIS retained sources contain
  identity, package, symbol, pin, or device metadata rather than electrical
  parametric evidence. TI/DigiKey-style API material remains excluded wherever
  the committed source policy denies durable training. Manufacturer PDFs with
  review-required or denied training rights are not substituted.

The NASA source-vocabulary-map IDs are present, but reviewed-development
examples, calibration, production votes, combined labels, and training examples
remain absent. A second genuinely independent eligible family is still
required. This is one production-valid source, not FDX3 acceptance and not
permission to begin FDX4.

No FDX4 split corpus, FDX5 trained model, or FDX6 production cascade is claimed.
