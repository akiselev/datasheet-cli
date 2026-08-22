# FDX2 — Deterministic datasheet compiler baseline

Status: planned

Owner: Datasheet-cli

Depends on: FDX1; FDX0 evidence contract available through a pinned Ferrodoc revision/fixture

## Objective

Build a useful, evidence-grounded electronics extractor without machine learning. This establishes the production fallback and the qualification baseline that every later learned component must beat.

Initial scope is deliberately narrow:

```text
Absolute Maximum Ratings
Recommended Operating Conditions
Electrical Characteristics
Thermal Information
```

## Deliverable 1: DocumentIR -> `DatasheetSketch`

Implement the deterministic adapter from pinned Ferrodoc DocumentIR into the FDX1 sketch.

Required steps:

```text
validate IR + digest
   |
build heading/section ancestry
   |
index tables/paragraphs/source spans
   |
extract subject/package/symbol candidates
   |
parse exact quantity candidates through Quantitas-backed registry
   |
compute template/structural signatures
   |
produce canonical DatasheetSketch
```

If table/cell geometry is coarse, preserve the quality and lower extraction confidence; do not invent precise anchors.

## Deliverable 2: quantity lexer/parser

Implement deterministic parsing over exact cell/span text.

Support at least:

```text
3.3 V
60 nA
-0.3 V
1.8 to 5.5 V
1.8–5.5 V
<= 5.5 V
5.5 V max
±2 %
25 °C
-40 °C to 125 °C
180 °C/W
10 kΩ
1 MHz
```

Keep raw source spelling and parse structure separately.

Represent symbolic/relative expressions that cannot safely canonicalize, for example:

```text
VDD + 0.3 V
VOUT - 0.5 V
0.7 * VDD
```

as structured/raw candidates rather than pretending they are fixed scalar quantities.

A failed parse remains explicit and does not erase source text.

## Deliverable 3: section/regime rules

Implement conservative heading/section aliases for the first task family.

Examples:

```text
absolute maximum ratings
absolute maximum rating
maximum ratings

recommended operating conditions
recommended operating condition
operating conditions

 electrical characteristics
dc characteristics
ac characteristics
dc/ac characteristics

thermal information
thermal characteristics
package thermal data
```

Aliases are versioned data/configuration, not scattered string comparisons.

Map only sufficiently strong headings to a regime. Ambiguous generic headings such as `Characteristics` should remain candidates until table/header context resolves them.

## Deliverable 4: table-header role inference

Recognize common generic schemas:

```text
Parameter | Symbol | Conditions | Min | Typ | Max | Unit
Parameter | Test Conditions | Min | Typ | Max | Units
Symbol | Parameter | Conditions | Minimum | Typical | Maximum
Parameter | Conditions | Rating | Unit
Parameter | Min | Max | Unit
```

Infer roles:

```text
parameter
symbol
condition
min
typ
max
unit
rating/package
notes
unknown
```

Use header ancestry/merged cells from Ferrodoc where available.

Rules must abstain when multiple assignments are equally plausible.

## Deliverable 5: row compiler

Compile one structured row into deterministic candidates:

```text
row text/cells
   |
   +--> parameter label
   +--> symbol candidate
   +--> condition/application candidates
   +--> min/typ/max/rating quantity candidates
   +--> unit inheritance
   +--> footnote references
   +--> package/variant scope
```

Do not collapse min/typ/max into one scalar. Preserve the semantic role of each value.

A row may yield multiple `CandidateClaim` objects when multiple values genuinely express separate constraints.

## Deliverable 6: unit inheritance and cell conventions

Datasheets frequently place units in a dedicated column or heading and use blank/dash cells.

Implement explicit table-level conventions derived from headers/body consistency:

```text
unit column inheritance
unit embedded in each value
blank means unspecified
em dash/hyphen means unspecified when table convention establishes it
`N/A` / `NA` explicit unavailable token
```

Never turn a blank/dash into numeric zero.

Retain which convention produced each interpretation.

## Deliverable 7: footnote/applicability linking

Use Ferrodoc evidence/reading order and textual markers to connect:

```text
row marker -> table footnote
header marker -> table/column footnote
section note -> scoped condition candidate
```

Emit condition/applicability candidates with exact evidence anchors.

Do not apply a footnote globally if its marker scope is ambiguous.

## Deliverable 8: predicate candidate retrieval baseline

Build an auditable non-neural predicate retriever over the versioned vocabulary.

Signals:

```text
normalized parameter text alias
exact/normalized symbol alias
quantity kind/dimensionality
section/regime compatibility
component/category prior when supplied
```

Return top-K candidate term IDs with per-signal reasons.

Do not require one exact winner. Ambiguous rows remain candidate sets for FDX3/FDX5.

## Deliverable 9: deterministic claim assembly

For candidates passing configurable confidence gates, assemble `ClaimBundle/v1` by selecting only existing evidence/value IDs.

The deterministic compiler may emit:

```text
accepted_by_extractor
ambiguous
unsupported
invalid_grounding
```

These are extraction statuses, not Foundry promotion decisions.

## Deliverable 10: explanations

Every row/claim exposes deterministic reasons, for example:

```text
section heading matched recommended_operating@v1
header schema matched parameter|symbol|condition|min|typ|max|unit
quantity parsed as electric potential
symbol alias matched VDD
predicate alias matched supply voltage
value selected from table r=12,c=5
condition selected from r=12,c=2 + footnote 3
```

This explanation becomes useful both for debugging and as FDX3 labeling-function evidence.

## Deliverable 11: protected deterministic baseline

Create an evaluation harness before FDX3/FDX5 changes the system.

Metrics:

```text
section/regime precision/recall
header/cell-role accuracy
quantity parse accuracy
predicate top-1/top-k accuracy
condition-span accuracy
claim exact match
claim evidence-anchor validity
ungrounded output count
abstention/coverage
wall time / peak memory where practical
```

Treat a missing prediction as missing work, not success.

## Tests

Required:

- supported quantity forms preserve exact raw text and canonical meaning;
- symbolic relative values remain symbolic rather than guessed scalars;
- dash/blank never becomes zero;
- absolute max and recommended operating sections cannot silently merge regimes;
- min/typ/max roles survive serialization;
- unit inheritance is deterministic and fails closed on conflicting units;
- ambiguous header schema abstains;
- footnote marker resolves only within defensible scope;
- predicate dimension filters reject impossible kinds;
- claim bundle cannot contain a value not present in sketch candidates;
- every claim anchor validates against pinned IR;
- repeated compilation of the same inputs produces byte-identical canonical outputs.

## Acceptance criteria

FDX2 is complete when:

1. a retained real analog/power datasheet corpus produces useful grounded claims with no learned model;
2. all emitted values come from exact deterministic candidates;
3. rule explanations make every accepted/abstained row auditable;
4. protected baseline metrics are frozen for later comparison;
5. ambiguous/unsupported rows are retained for FDX3/FDX5 rather than guessed;
6. the compiler consumes only generic Ferrodoc evidence and does not bypass it with its own PDF parser.

## Landed

Record corpus identity, Quantitas/ontology snapshots, supported forms, baseline metrics and known failure clusters after implementation.
