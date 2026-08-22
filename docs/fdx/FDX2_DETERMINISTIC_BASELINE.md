# FDX2 deterministic analog/power baseline

Status: frozen bounded conformance baseline; production-corpus qualification deferred

## Scope and ownership

FDX2 compiles a generic, state-pinned DocumentIR shape into a `DatasheetSketch` and grounded `ClaimBundle` for four table regimes: Absolute Maximum Ratings, Recommended Operating Conditions, Electrical Characteristics, and Thermal Information. It does not parse PDFs, infer document families, call a model, create weak labels, or claim production coverage.

Ferrodoc continues to own document state, evidence, layout, and table reconstruction. Quantitas owns unit/dimension conversion. This compiler owns electronics section/header/row interpretation, predicate retrieval, abstention, and claim assembly.

## Frozen behavior

- Ruleset: `datasheet-analog-power-rules/1`.
- Quantitas revision: `734d78cd6ff516afee54201bc70cd59fd34e67e3`, pinned in `Cargo.lock`.
- Registry identity: `datasheet-fdx2-analog-power-units/1`, a minimized frozen registry for V, A/nA, ohm/kohm, Hz/MHz, percent, Celsius, and Celsius/Watt.
- Vocabulary digest: `3333333333333333333333333333333333333333333333333333333333333333`.
- Producer configuration digest: `4444444444444444444444444444444444444444444444444444444444444444` for the checked fixture.
- Symbolic relative limits such as `VDD + 0.3 V` are retained as symbolic candidates and never scalarized.
- Blank, dash, and N/A values are explicit unsupported results and never become zero.
- Ambiguous headers, generic `Characteristics`, unit conflicts, unsupported quantities, tied predicates, and missing grounding abstain or refuse deterministically.
- Min/typ/max/rating roles remain distinct in row results and claims. Conditions and footnotes are sketch-owned, exactly anchored, and resolved only within table scope.
- Predicate top-K results expose scores and contributing signals. Impossible quantity kinds, shapes, and regimes are filtered before ranking.

The source fixture, vocabulary, and truth live under `tests/fixtures/`. Frozen compiler and evaluation outputs live under `tests/goldens/`; tests compare exact pretty-JSON bytes and regenerated JSON Schemas.

## Evaluation contract

The checked minimized fixture reports 1.0 section precision and recall (4/4), header-role accuracy (5/5), quantity and condition-span accuracy (10/10), predicate top-1/top-5 accuracy (7/7 expected predicates), exact-claim accuracy (7/7 expected claims), and claim-anchor validity (7/7). It emits seven grounded claims, zero ungrounded outputs, four abstentions (three missing/unsupported row values plus one ambiguous table), and 0.7 row coverage.

Missing predictions count as wrong. Wall time and peak resident memory are intentionally host observations rather than frozen golden values; the evaluation record carries explicit nulls and a reason.

These perfect fixture scores are conformance evidence, not a production quality estimate.

## Commands

```text
datasheet baseline DOCUMENT.json VOCABULARY.json \
  --code-revision REVISION \
  --configuration-digest SHA256 \
  --output baseline.json

cargo run -p datasheet-compiler --example compile_fdx2_fixture -- \
  tests/goldens/fdx2_synthetic_baseline.json \
  tests/goldens/fdx2_synthetic_metrics.json

cargo run -p datasheet-compiler --example export_fdx2_schemas -- schemas/fdx2
```

## Deferred gate

The upstream FDX2 phase description also asks for evaluation on a retained real corpus. The Foundry coordination gate for this implementation explicitly permits only synthetic/minimized FDX0-shaped fixtures and forbids starting FDX3 production corpus/model work. Therefore production-corpus accuracy, failure buckets, wall-time distributions, and peak-memory distributions remain deliberately unmet and must not be inferred from this baseline.
