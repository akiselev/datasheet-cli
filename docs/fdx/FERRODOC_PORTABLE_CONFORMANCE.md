# Ferrodoc portable conformance

Status: implemented against landed FP0/FDX0 contracts; no runtime dependency

Datasheet consumes Ferrodoc evidence and logical state through a checked JSON wire bridge. The bridge mirrors only the public fields needed for state identity, selected text/table evidence, evidence-bearing cells, geometry quality, physical checkpoint identity, and page-qualified region targets. It does not link the Ferrodoc runtime or duplicate PDF/layout behavior.

## Contract lock

- Audited Ferrodoc revision: `2a4d886703753a8893bfa17ade8a554df61f59ab` (includes FP0/FDX0 commit `6906d1b`).
- `evidence-delta-v1.json` SHA-256: `ab98db6810db12bdf22b04e6b113bbbe4cbe43aa187702967ad1db30c1e0ff16`.
- `document-state-manifest-v1.json` SHA-256: `a9a81cd6b65acf1a256529337c985ad512d34a7b79274f48a926490c35f1aa2b`.
- Delta schema: `ferrodoc-evidence-delta/1`.
- State schema: `ferrodoc-document-state/1`.

The checked upstream Ferrodoc golden delta/state fixtures reproduce the exact landed logical IDs. A second minimized Datasheet-oriented fixture uses the same wire shape and includes two pages, one evidence-bearing table, exact text source spans, a footnote, an optional materialized checkpoint, and a page-qualified table target.

## Enforced boundary

The bridge independently replays Ferrodoc's domain-separated, length-prefixed delta and state identity projections. It refuses contract-lock drift, wrong source/schema state pairs, a manifest whose delta set differs from the supplied delta, stale target state, empty capabilities, and a region that does not exist on the qualified page.

Conversion preserves:

- Ferrodoc `DocumentStateId` as every Datasheet anchor's `document_state_id`;
- optional checkpoint digest as `document_ir_logical_sha256` without treating it as logical state identity;
- exact evidence IDs and page/region ownership;
- cell source spans as table-cell anchor evidence;
- selected raw titles, headings, conditions, values, and footnotes;
- honest geometry quality, with PDF point rectangles normalized against the owning page.

The deterministic baseline can only select raw quantity and condition text present in the supplied Ferrodoc evidence. Tests prove all emitted claim anchors resolve to supplied evidence, and stale anchors fail against the adapted state.

## Scope

This is portable contract conformance, not a production Ferrodoc adapter quality claim. It does not materialize arbitrary delta chains or page additions, execute refinement capabilities, perform PDF extraction, or add any corpus/model path. Those behaviors remain owned by Ferrodoc or later gated Datasheet work.

FDX3 remains dependency-blocked on the Foundry production corpus/supervision gates. This bridge closes the pre-FDX3 cross-repository contract check only; it does not authorize weak-label or training work.
