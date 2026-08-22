// SPDX-License-Identifier: MIT OR Apache-2.0
//! Frozen FDX2 deterministic analog/power table baseline.

use std::collections::{BTreeMap, BTreeSet};

use datasheet_core::{
    CandidateId, ClaimBundle, ConditionCandidate, ContentSelector, EVIDENCE_ANCHOR_SCHEMA,
    EvidenceAnchor, GeometryQuality, ParsedConditionCandidate, PredicateVocabularySnapshot,
    ProducerIdentity, QuantityCandidate, Sha256Digest, SketchId, SourceGeometry,
    SpecificationRegime, SubjectCandidate, SubjectKind, ValueRole, tagged_content_id,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::quantity::{ParsedQuantity, QuantityMeaning, QuantityParseError, QuantityParser};
use crate::{
    ClaimSelection, DatasheetSketch, PackageCandidate, SectionCandidate, SketchBlock, SketchTable,
    SketchTableCell, SymbolCandidate, assemble_claim_bundle,
};

pub const FDX2_BASELINE_SCHEMA: &str = "datasheet-deterministic-baseline/1";
pub const FDX2_RULESET_VERSION: &str = "datasheet-analog-power-rules/1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct Fdx2Document {
    pub schema_version: String,
    pub source_pdf_sha256: Sha256Digest,
    pub document_state_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_ir_logical_sha256: Option<Sha256Digest>,
    pub pages: Vec<Fdx2Page>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Fdx2Page {
    pub page_id: String,
    pub title_blocks: Vec<Fdx2TextEvidence>,
    pub headings: Vec<Fdx2TextEvidence>,
    pub paragraphs: Vec<Fdx2TextEvidence>,
    pub tables: Vec<Fdx2Table>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Fdx2TextEvidence {
    pub evidence_id: String,
    pub region_id: String,
    pub text: String,
    pub geometry: Vec<SourceGeometry>,
    pub geometry_quality: GeometryQuality,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Fdx2Table {
    pub region_id: String,
    pub heading_path: Vec<String>,
    pub rows: u32,
    pub columns: u32,
    pub header_rows: u32,
    pub cells: Vec<Fdx2Cell>,
    pub footnotes: Vec<Fdx2Footnote>,
    pub reconstruction_quality: GeometryQuality,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Fdx2Cell {
    pub evidence_id: String,
    pub row: u32,
    pub column: u32,
    #[serde(default = "one")]
    pub row_span: u32,
    #[serde(default = "one")]
    pub column_span: u32,
    pub text: String,
    #[serde(default)]
    pub markers: Vec<String>,
    pub geometry: Vec<SourceGeometry>,
    pub geometry_quality: GeometryQuality,
}

const fn one() -> u32 {
    1
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct Fdx2Footnote {
    pub marker: String,
    pub evidence: Fdx2TextEvidence,
}

impl Fdx2Document {
    pub fn validate(&self) -> Result<(), BaselineError> {
        self.source_pdf_sha256.validate()?;
        if self.schema_version.is_empty() || self.document_state_id.is_empty() {
            return Err(BaselineError::InvalidDocument(
                "missing document identity".into(),
            ));
        }
        let mut pages = BTreeSet::new();
        let mut evidence = BTreeSet::new();
        for page in &self.pages {
            if page.page_id.is_empty() || !pages.insert(page.page_id.clone()) {
                return Err(BaselineError::InvalidDocument(
                    "duplicate/empty page id".into(),
                ));
            }
            for item in
                page.title_blocks
                    .iter()
                    .chain(&page.headings)
                    .chain(&page.paragraphs)
                    .chain(page.tables.iter().flat_map(|table| {
                        table.footnotes.iter().map(|footnote| &footnote.evidence)
                    }))
            {
                validate_text_evidence(item, &mut evidence)?;
            }
            for table in &page.tables {
                if table.region_id.is_empty()
                    || table.rows == 0
                    || table.columns == 0
                    || table.header_rows == 0
                    || table.header_rows >= table.rows
                {
                    return Err(BaselineError::InvalidDocument("invalid table shape".into()));
                }
                for cell in &table.cells {
                    if cell.row >= table.rows
                        || cell.column >= table.columns
                        || cell.row_span == 0
                        || cell.column_span == 0
                        || cell.row + cell.row_span > table.rows
                        || cell.column + cell.column_span > table.columns
                        || !evidence.insert(cell.evidence_id.clone())
                    {
                        return Err(BaselineError::InvalidDocument(
                            "invalid/duplicate table cell".into(),
                        ));
                    }
                    for geometry in &cell.geometry {
                        geometry.validate()?;
                    }
                }
            }
        }
        Ok(())
    }

    pub fn validate_anchor(&self, anchor: &EvidenceAnchor) -> Result<(), BaselineError> {
        anchor.validate()?;
        if anchor.source_pdf_sha256 != self.source_pdf_sha256
            || anchor.document_state_id != self.document_state_id
            || anchor.document_ir_logical_sha256 != self.document_ir_logical_sha256
        {
            return Err(BaselineError::InvalidAnchor(
                "document identity mismatch".into(),
            ));
        }
        let page = self
            .pages
            .iter()
            .find(|page| page.page_id == anchor.page_id)
            .ok_or_else(|| BaselineError::InvalidAnchor("page missing".into()))?;
        for evidence_id in &anchor.evidence_ids {
            let text_found =
                page.title_blocks
                    .iter()
                    .chain(&page.headings)
                    .chain(&page.paragraphs)
                    .chain(page.tables.iter().flat_map(|table| {
                        table.footnotes.iter().map(|footnote| &footnote.evidence)
                    }))
                    .any(|item| {
                        item.evidence_id == *evidence_id
                            && anchor.region_id.as_ref() == Some(&item.region_id)
                    });
            let cell_found = page.tables.iter().any(|table| {
                anchor.region_id.as_ref() == Some(&table.region_id)
                    && table
                        .cells
                        .iter()
                        .any(|cell| cell.evidence_id == *evidence_id)
            });
            if !text_found && !cell_found {
                return Err(BaselineError::InvalidAnchor(evidence_id.clone()));
            }
        }
        Ok(())
    }
}

fn validate_text_evidence(
    item: &Fdx2TextEvidence,
    evidence: &mut BTreeSet<String>,
) -> Result<(), BaselineError> {
    if item.evidence_id.is_empty()
        || item.region_id.is_empty()
        || !evidence.insert(item.evidence_id.clone())
    {
        return Err(BaselineError::InvalidDocument(
            "invalid/duplicate text evidence".into(),
        ));
    }
    for geometry in &item.geometry {
        geometry.validate()?;
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum ColumnRole {
    Parameter,
    Symbol,
    Condition,
    Minimum,
    Typical,
    Maximum,
    Unit,
    Rating,
    Package,
    Notes,
    Unknown,
}

impl ColumnRole {
    fn value_role(&self) -> Option<ValueRole> {
        match self {
            Self::Minimum => Some(ValueRole::Minimum),
            Self::Typical => Some(ValueRole::Typical),
            Self::Maximum => Some(ValueRole::Maximum),
            Self::Rating => Some(ValueRole::Rating),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InferenceStatus {
    Matched,
    Abstained,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct HeaderInference {
    pub status: InferenceStatus,
    pub roles: Vec<ColumnRole>,
    pub signature: String,
    pub explanations: Vec<String>,
}

pub fn infer_header_roles(table: &Fdx2Table) -> HeaderInference {
    let mut headers = vec![String::new(); table.columns as usize];
    for cell in table
        .cells
        .iter()
        .filter(|cell| cell.row < table.header_rows)
    {
        for column in cell.column..cell.column + cell.column_span {
            let slot = &mut headers[column as usize];
            if !slot.is_empty() {
                slot.push(' ');
            }
            slot.push_str(&cell.text);
        }
    }
    let signature = headers
        .iter()
        .map(|header| normalize_label(header))
        .collect::<Vec<_>>()
        .join("|");
    let mut roles = vec![];
    let mut explanations = vec![];
    let mut seen = BTreeSet::new();
    let mut ambiguous = false;
    for header in &headers {
        let normalized = normalize_label(header);
        let candidates = header_role_candidates(&normalized);
        let role = if candidates.len() == 1 {
            candidates[0].clone()
        } else {
            ambiguous = true;
            ColumnRole::Unknown
        };
        if role != ColumnRole::Unknown && !seen.insert(role.clone()) {
            ambiguous = true;
        }
        explanations.push(if role == ColumnRole::Unknown {
            format!("header `{header}` did not have one defensible role")
        } else {
            format!("header `{header}` matched {role:?}@{FDX2_RULESET_VERSION}")
        });
        roles.push(role);
    }
    let has_parameter = roles.contains(&ColumnRole::Parameter);
    let has_value = roles.iter().any(|role| role.value_role().is_some());
    HeaderInference {
        status: if !ambiguous && has_parameter && has_value {
            InferenceStatus::Matched
        } else {
            InferenceStatus::Abstained
        },
        roles,
        signature,
        explanations,
    }
}

fn header_role_candidates(normalized: &str) -> Vec<ColumnRole> {
    match normalized {
        "parameter" | "parameter name" | "description" => vec![ColumnRole::Parameter],
        "symbol" => vec![ColumnRole::Symbol],
        "condition" | "conditions" | "test condition" | "test conditions" => {
            vec![ColumnRole::Condition]
        }
        "min" | "minimum" => vec![ColumnRole::Minimum],
        "typ" | "typical" => vec![ColumnRole::Typical],
        "max" | "maximum" => vec![ColumnRole::Maximum],
        "unit" | "units" => vec![ColumnRole::Unit],
        "rating" => vec![ColumnRole::Rating],
        "package" => vec![ColumnRole::Package],
        "note" | "notes" => vec![ColumnRole::Notes],
        // Generic `value` and merged `rating/package` are intentionally ambiguous.
        "value" | "rating package" | "package rating" | "" => vec![],
        _ => vec![],
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SectionRule {
    pub alias: String,
    pub regime: SpecificationRegime,
}

#[must_use]
pub fn section_rules() -> Vec<SectionRule> {
    [
        (
            [
                "absolute maximum ratings",
                "absolute maximum rating",
                "maximum ratings",
            ]
            .as_slice(),
            SpecificationRegime::AbsoluteMaximum,
        ),
        (
            [
                "recommended operating conditions",
                "recommended operating condition",
                "operating conditions",
            ]
            .as_slice(),
            SpecificationRegime::RecommendedOperating,
        ),
        (
            [
                "electrical characteristics",
                "dc characteristics",
                "ac characteristics",
                "dc ac characteristics",
            ]
            .as_slice(),
            SpecificationRegime::Guaranteed,
        ),
        (
            [
                "thermal information",
                "thermal characteristics",
                "package thermal data",
            ]
            .as_slice(),
            SpecificationRegime::Informative,
        ),
    ]
    .into_iter()
    .flat_map(|(aliases, regime)| {
        aliases.iter().map(move |alias| SectionRule {
            alias: (*alias).into(),
            regime: regime.clone(),
        })
    })
    .collect()
}

#[must_use]
pub fn infer_regime(heading: &str) -> Option<(SpecificationRegime, String)> {
    let normalized = normalize_label(heading);
    section_rules()
        .into_iter()
        .find(|rule| rule.alias == normalized)
        .map(|rule| {
            (
                rule.regime,
                format!(
                    "section heading matched `{}`@{FDX2_RULESET_VERSION}",
                    rule.alias
                ),
            )
        })
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PredicateScore {
    pub predicate_term_id: String,
    pub score: i32,
    pub reasons: Vec<String>,
}

pub fn retrieve_predicates(
    vocabulary: &PredicateVocabularySnapshot,
    parameter: &str,
    symbol: Option<&str>,
    parsed: &ParsedQuantity,
    regime: &SpecificationRegime,
    category_prior: Option<&str>,
    top_k: usize,
) -> Vec<PredicateScore> {
    let parameter = normalize_label(parameter);
    let symbol = symbol.map(normalize_symbol);
    let mut results = vec![];
    for term in &vocabulary.terms {
        let Some(kind) = parsed.quantity_kind.as_deref() else {
            continue;
        };
        if term.expected_quantity_kind.as_deref() != Some(kind) {
            continue;
        }
        if term
            .expected_dimensionality
            .as_deref()
            .is_some_and(|expected| parsed.dimensionality.as_deref() != Some(expected))
        {
            continue;
        }
        if !term.expected_shapes.is_empty() && !term.expected_shapes.contains(&parsed.parsed_shape)
        {
            continue;
        }
        if !term.regime_constraints.is_empty() && !term.regime_constraints.contains(regime) {
            continue;
        }
        let mut score = 20;
        let mut reasons = vec![format!("quantity kind matched {kind}")];
        if let Some(dimension) = &term.expected_dimensionality {
            score += 20;
            reasons.push(format!("dimensionality matched {dimension}"));
        }
        if term
            .aliases
            .iter()
            .any(|alias| normalize_label(alias) == parameter)
            || normalize_label(&term.canonical_name) == parameter
        {
            score += 50;
            reasons.push("parameter alias matched".into());
        }
        if let Some(symbol) = &symbol {
            if term
                .symbol_aliases
                .iter()
                .any(|alias| normalize_symbol(alias) == *symbol)
            {
                score += 40;
                reasons.push("symbol alias matched".into());
            }
        }
        if term.regime_constraints.contains(regime) {
            score += 10;
            reasons.push("regime constraint matched".into());
        }
        if let Some(category) = category_prior {
            if term.category_scope.iter().any(|scope| scope == category) {
                score += 5;
                reasons.push("category prior matched".into());
            }
        }
        results.push(PredicateScore {
            predicate_term_id: term.id.clone(),
            score,
            reasons,
        });
    }
    results.sort_by(|left, right| {
        right
            .score
            .cmp(&left.score)
            .then_with(|| left.predicate_term_id.cmp(&right.predicate_term_id))
    });
    results.truncate(top_k);
    results
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum RowExtractionStatus {
    AcceptedByExtractor,
    Ambiguous,
    Unsupported,
    InvalidGrounding,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct RowValueResult {
    pub table_id: CandidateId,
    pub row: u32,
    pub parameter_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol_text: Option<String>,
    pub regime: SpecificationRegime,
    pub value_role: ValueRole,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_candidate_id: Option<CandidateId>,
    pub condition_candidate_ids: Vec<CandidateId>,
    pub predicate_candidates: Vec<PredicateScore>,
    pub status: RowExtractionStatus,
    pub explanations: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DeterministicBaseline {
    pub schema_version: String,
    pub ruleset_version: String,
    pub sketch: DatasheetSketch,
    pub claim_bundle: ClaimBundle,
    pub header_inferences: Vec<HeaderInference>,
    pub row_results: Vec<RowValueResult>,
    pub abstained_tables: u32,
    pub warnings: Vec<String>,
}

pub fn compile_analog_power_baseline(
    document: &Fdx2Document,
    producer: ProducerIdentity,
    vocabulary: &PredicateVocabularySnapshot,
    category_prior: Option<&str>,
) -> Result<DeterministicBaseline, BaselineError> {
    document.validate()?;
    producer.validate()?;
    if producer.ontology_snapshot_digest != vocabulary.digest {
        return Err(BaselineError::OntologyMismatch);
    }
    if producer.quantitas_registry_identity.as_deref() != Some(crate::quantity::FDX2_REGISTRY_ID) {
        return Err(BaselineError::RegistryMismatch);
    }
    let parser = QuantityParser::fdx2()?;
    let mut sketch = empty_sketch(document, producer);
    let mut header_inferences = vec![];
    let mut pending_rows = vec![];
    let mut abstained_tables = 0;

    index_text_and_sections(document, &mut sketch)?;
    for page in &document.pages {
        for table in &page.tables {
            let table_candidate = build_sketch_table(document, page, table)?;
            let table_id = table_candidate.id.clone();
            sketch
                .template_signatures
                .push(table_candidate.structural_signature.clone());
            let header = infer_header_roles(table);
            let regime = table
                .heading_path
                .iter()
                .rev()
                .find_map(|heading| infer_regime(heading));
            if header.status == InferenceStatus::Abstained || regime.is_none() {
                abstained_tables += 1;
                header_inferences.push(header);
                sketch.tables.push(table_candidate);
                continue;
            }
            let (regime, section_reason) = regime.expect("checked above");
            if let Some(section) = sketch.sections.iter_mut().find(|section| {
                section.heading_text.as_deref().is_some_and(|heading| {
                    infer_regime(heading).is_some_and(|(candidate, _)| candidate == regime)
                })
            }) {
                section.table_ids.push(table_id.clone());
            }
            let compiled = compile_table_rows(
                document,
                page,
                table,
                &table_id,
                &header,
                &regime,
                &section_reason,
                vocabulary,
                category_prior,
                &parser,
                &mut sketch,
            )?;
            pending_rows.extend(compiled);
            header_inferences.push(header);
            sketch.tables.push(table_candidate);
        }
    }

    crate::sort_sketch(&mut sketch);
    sketch.template_signatures.sort();
    sketch.template_signatures.dedup();
    header_inferences.sort_by(|left, right| {
        left.signature
            .cmp(&right.signature)
            .then_with(|| left.roles.cmp(&right.roles))
    });
    pending_rows.sort_by(|left, right| {
        left.result
            .table_id
            .cmp(&right.result.table_id)
            .then_with(|| left.result.row.cmp(&right.result.row))
            .then_with(|| left.result.value_role.cmp(&right.result.value_role))
    });
    sketch.sketch_id = sketch.recompute_id()?;
    sketch.validate()?;
    for anchor in all_sketch_anchors(&sketch) {
        document.validate_anchor(anchor)?;
    }
    let mut selections = vec![];
    for pending in &pending_rows {
        if pending.result.status == RowExtractionStatus::AcceptedByExtractor {
            selections.push(ClaimSelection {
                subject_candidate_id: sketch
                    .subjects
                    .first()
                    .ok_or(BaselineError::MissingSubject)?
                    .id
                    .clone(),
                predicate_term_id: pending.result.predicate_candidates[0]
                    .predicate_term_id
                    .clone(),
                regime: pending.regime.clone(),
                value_role: pending.result.value_role.clone(),
                value_candidate_id: pending
                    .result
                    .value_candidate_id
                    .clone()
                    .ok_or(BaselineError::MissingValue)?,
                condition_candidate_ids: pending.result.condition_candidate_ids.clone(),
                package_variant_candidate_ids: pending.package_ids.clone(),
                extraction_confidence: pending.confidence,
            });
        }
    }
    let claim_bundle = assemble_claim_bundle(&sketch, vocabulary, &selections)?;
    Ok(DeterministicBaseline {
        schema_version: FDX2_BASELINE_SCHEMA.into(),
        ruleset_version: FDX2_RULESET_VERSION.into(),
        sketch,
        claim_bundle,
        header_inferences,
        row_results: pending_rows
            .into_iter()
            .map(|pending| pending.result)
            .collect(),
        abstained_tables,
        warnings: vec![],
    })
}

struct PendingRow {
    result: RowValueResult,
    regime: SpecificationRegime,
    package_ids: Vec<CandidateId>,
    confidence: f64,
}

fn empty_sketch(document: &Fdx2Document, producer: ProducerIdentity) -> DatasheetSketch {
    DatasheetSketch {
        schema_version: crate::DATASHEET_SKETCH_SCHEMA.into(),
        sketch_id: SketchId(String::new()),
        source_pdf_sha256: document.source_pdf_sha256.clone(),
        document_state_id: document.document_state_id.clone(),
        document_ir_logical_sha256: document.document_ir_logical_sha256.clone(),
        producer,
        subjects: vec![],
        sections: vec![],
        blocks: vec![],
        tables: vec![],
        quantities: vec![],
        symbols: vec![],
        packages: vec![],
        conditions: vec![],
        template_signatures: vec![],
    }
}

fn index_text_and_sections(
    document: &Fdx2Document,
    sketch: &mut DatasheetSketch,
) -> Result<(), BaselineError> {
    for page in &document.pages {
        for title in &page.title_blocks {
            let anchor = text_anchor(document, page, title);
            sketch.blocks.push(SketchBlock {
                id: anchored_text_id("block", &title.text, &anchor)?,
                raw_text: title.text.clone(),
                anchor: anchor.clone(),
            });
            if sketch.subjects.is_empty() {
                if let Some(subject) = extract_subject_designation(&title.text) {
                    sketch.subjects.push(SubjectCandidate {
                        id: anchored_text_id("subject", subject, &anchor)?,
                        kind: SubjectKind::Part,
                        raw_designation: subject.into(),
                        manufacturer_hint: extract_manufacturer_hint(&title.text, subject),
                        normalized_search_key: Some(normalize_symbol(subject)),
                        evidence_anchor: anchor,
                    });
                }
            }
        }
        for heading in &page.headings {
            let anchor = text_anchor(document, page, heading);
            sketch.blocks.push(SketchBlock {
                id: anchored_text_id("block", &heading.text, &anchor)?,
                raw_text: heading.text.clone(),
                anchor: anchor.clone(),
            });
            if let Some((regime, _)) = infer_regime(&heading.text) {
                sketch.sections.push(SectionCandidate {
                    id: anchored_text_id("section", &heading.text, &anchor)?,
                    heading_text: Some(heading.text.clone()),
                    heading_anchor: Some(anchor),
                    parent_section_id: None,
                    block_ids: vec![],
                    table_ids: vec![],
                    deterministic_regime_candidates: vec![regime],
                });
            }
        }
        for paragraph in &page.paragraphs {
            let anchor = text_anchor(document, page, paragraph);
            sketch.blocks.push(SketchBlock {
                id: anchored_text_id("block", &paragraph.text, &anchor)?,
                raw_text: paragraph.text.clone(),
                anchor,
            });
        }
    }
    if sketch.subjects.is_empty() {
        return Err(BaselineError::MissingSubject);
    }
    Ok(())
}

fn build_sketch_table(
    document: &Fdx2Document,
    page: &Fdx2Page,
    table: &Fdx2Table,
) -> Result<SketchTable, BaselineError> {
    let mut cells: Vec<_> = table
        .cells
        .iter()
        .map(|cell| SketchTableCell {
            row: cell.row,
            column: cell.column,
            raw_text: cell.text.clone(),
            source_evidence_ids: vec![cell.evidence_id.clone()],
            anchor: cell_anchor(document, page, table, cell),
        })
        .collect();
    cells.sort_by_key(|cell| (cell.row, cell.column));
    let signature = Sha256Digest::of_bytes(&serde_json::to_vec(&(
        table.rows,
        table.columns,
        table.header_rows,
        cells
            .iter()
            .filter(|cell| cell.row < table.header_rows)
            .map(|cell| normalize_label(&cell.raw_text))
            .collect::<Vec<_>>(),
    ))?)
    .0;
    Ok(SketchTable {
        id: tagged_content_id(
            "table",
            &serde_json::to_vec(&(
                &document.source_pdf_sha256,
                &document.document_state_id,
                &page.page_id,
                &table.region_id,
                &signature,
            ))?,
        ),
        source_region_id: table.region_id.clone(),
        heading_path: table.heading_path.clone(),
        rows: table.rows,
        columns: table.columns,
        cells,
        structural_signature: signature,
        reconstruction_quality: table.reconstruction_quality,
    })
}

#[allow(clippy::too_many_arguments)]
fn compile_table_rows(
    document: &Fdx2Document,
    page: &Fdx2Page,
    table: &Fdx2Table,
    table_id: &CandidateId,
    header: &HeaderInference,
    regime: &SpecificationRegime,
    section_reason: &str,
    vocabulary: &PredicateVocabularySnapshot,
    category_prior: Option<&str>,
    parser: &QuantityParser,
    sketch: &mut DatasheetSketch,
) -> Result<Vec<PendingRow>, BaselineError> {
    let mut rows = vec![];
    for row_index in table.header_rows..table.rows {
        let row_cells: BTreeMap<_, _> = table
            .cells
            .iter()
            .filter(|cell| cell.row == row_index)
            .map(|cell| (cell.column, cell))
            .collect();
        let role_cell = |role: &ColumnRole| {
            header
                .roles
                .iter()
                .position(|candidate| candidate == role)
                .and_then(|column| row_cells.get(&(column as u32)).copied())
        };
        let parameter = role_cell(&ColumnRole::Parameter)
            .map(|cell| cell.text.trim())
            .unwrap_or_default();
        if parameter.is_empty() {
            continue;
        }
        let symbol_cell = role_cell(&ColumnRole::Symbol);
        let symbol = symbol_cell
            .map(|cell| cell.text.trim())
            .filter(|text| !text.is_empty());
        if let Some(cell) = symbol_cell.filter(|cell| !cell.text.trim().is_empty()) {
            let anchor = cell_anchor(document, page, table, cell);
            let candidate = SymbolCandidate {
                id: anchored_text_id("symbol", &cell.text, &anchor)?,
                raw_text: cell.text.clone(),
                normalized_symbol: symbol.map(normalize_symbol),
                anchor,
            };
            push_unique_symbol(&mut sketch.symbols, candidate);
        }
        let unit = role_cell(&ColumnRole::Unit)
            .map(|cell| cell.text.trim())
            .filter(|text| !is_missing_value(text));
        let (condition_ids, condition_explanations) = compile_conditions(
            document,
            page,
            table,
            row_index,
            role_cell(&ColumnRole::Condition),
            &row_cells,
            sketch,
        )?;
        let package_ids = compile_packages(
            document,
            page,
            table,
            role_cell(&ColumnRole::Package),
            sketch,
        )?;
        for (column, role) in header.roles.iter().enumerate() {
            let Some(value_role) = role.value_role() else {
                continue;
            };
            let Some(cell) = row_cells.get(&(column as u32)).copied() else {
                continue;
            };
            let mut explanations = vec![section_reason.into()];
            explanations.extend(header.explanations.clone());
            explanations.extend(condition_explanations.clone());
            if is_missing_value(&cell.text) {
                explanations.push(format!(
                    "value r={row_index},c={column} is explicitly unspecified, never zero"
                ));
                rows.push(PendingRow {
                    result: RowValueResult {
                        table_id: table_id.clone(),
                        row: row_index,
                        parameter_text: parameter.into(),
                        symbol_text: symbol.map(str::to_string),
                        regime: regime.clone(),
                        value_role,
                        value_candidate_id: None,
                        condition_candidate_ids: condition_ids.clone(),
                        predicate_candidates: vec![],
                        status: RowExtractionStatus::Unsupported,
                        explanations,
                    },
                    regime: regime.clone(),
                    package_ids: package_ids.clone(),
                    confidence: 0.0,
                });
                continue;
            }
            let parsed = parser.parse(&cell.text, unit)?;
            if let (Some(inherited), Some(raw_unit)) = (unit, parsed.raw_unit.as_deref()) {
                if !units_compatible(parser, inherited, raw_unit)? {
                    explanations.push(format!(
                        "conflicting embedded unit `{raw_unit}` and unit column `{inherited}`"
                    ));
                    rows.push(PendingRow {
                        result: RowValueResult {
                            table_id: table_id.clone(),
                            row: row_index,
                            parameter_text: parameter.into(),
                            symbol_text: symbol.map(str::to_string),
                            regime: regime.clone(),
                            value_role,
                            value_candidate_id: None,
                            condition_candidate_ids: condition_ids.clone(),
                            predicate_candidates: vec![],
                            status: RowExtractionStatus::Ambiguous,
                            explanations,
                        },
                        regime: regime.clone(),
                        package_ids: package_ids.clone(),
                        confidence: 0.0,
                    });
                    continue;
                }
            }
            let anchor = cell_anchor(document, page, table, cell);
            let quantity_id = tagged_content_id(
                "quantity",
                &serde_json::to_vec(&(&parsed, &anchor, &value_role))?,
            );
            let quantity = QuantityCandidate {
                id: quantity_id.clone(),
                raw_text: parsed.raw_text.clone(),
                parsed_shape: parsed.parsed_shape.clone(),
                raw_unit: parsed.raw_unit.clone(),
                normalized_quantity: parsed.normalized_quantity.clone(),
                quantity_kind_hint: parsed.quantity_kind.clone(),
                dimensionality: parsed.dimensionality.clone(),
                anchor,
                parse_warnings: parsed.warnings.clone(),
            };
            push_unique_quantity(&mut sketch.quantities, quantity);
            explanations.push(format!(
                "value selected from table r={row_index},c={column} as {value_role:?}"
            ));
            if unit.is_some() && cell.text.trim().parse::<f64>().is_ok() {
                explanations.push(format!(
                    "unit `{}` inherited from explicit unit column",
                    unit.expect("checked")
                ));
            }
            let predicates = retrieve_predicates(
                vocabulary,
                parameter,
                symbol,
                &parsed,
                regime,
                category_prior,
                5,
            );
            let status = classify_row(&parsed, &predicates);
            if let Some(predicate) = predicates.first() {
                explanations.extend(predicate.reasons.clone());
            }
            rows.push(PendingRow {
                result: RowValueResult {
                    table_id: table_id.clone(),
                    row: row_index,
                    parameter_text: parameter.into(),
                    symbol_text: symbol.map(str::to_string),
                    regime: regime.clone(),
                    value_role,
                    value_candidate_id: Some(quantity_id),
                    condition_candidate_ids: condition_ids.clone(),
                    predicate_candidates: predicates,
                    status: status.clone(),
                    explanations,
                },
                regime: regime.clone(),
                package_ids: package_ids.clone(),
                confidence: if status == RowExtractionStatus::AcceptedByExtractor {
                    1.0
                } else {
                    0.0
                },
            });
        }
    }
    Ok(rows)
}

fn classify_row(parsed: &ParsedQuantity, predicates: &[PredicateScore]) -> RowExtractionStatus {
    if matches!(
        parsed.meaning,
        QuantityMeaning::Unparsed | QuantityMeaning::Symbolic { .. }
    ) {
        return RowExtractionStatus::Unsupported;
    }
    let Some(first) = predicates.first() else {
        return RowExtractionStatus::Ambiguous;
    };
    if first.score < 70
        || predicates
            .get(1)
            .is_some_and(|second| second.score == first.score)
    {
        RowExtractionStatus::Ambiguous
    } else {
        RowExtractionStatus::AcceptedByExtractor
    }
}

fn compile_conditions(
    document: &Fdx2Document,
    page: &Fdx2Page,
    table: &Fdx2Table,
    row: u32,
    condition_cell: Option<&Fdx2Cell>,
    row_cells: &BTreeMap<u32, &Fdx2Cell>,
    sketch: &mut DatasheetSketch,
) -> Result<(Vec<CandidateId>, Vec<String>), BaselineError> {
    let mut ids = vec![];
    let mut explanations = vec![];
    if let Some(cell) = condition_cell.filter(|cell| !is_missing_value(&cell.text)) {
        let anchor = cell_anchor(document, page, table, cell);
        let candidate = ConditionCandidate {
            id: tagged_content_id(
                "condition",
                &serde_json::to_vec(&(&cell.text, &anchor, "condition_cell"))?,
            ),
            raw_text: cell.text.clone(),
            parsed_candidate: Some(ParsedConditionCandidate {
                fields: BTreeMap::from([("source_role".into(), "condition_cell".into())]),
            }),
            evidence_anchors: vec![anchor],
        };
        ids.push(candidate.id.clone());
        push_unique_condition(&mut sketch.conditions, candidate);
        explanations.push(format!("condition selected from r={row}, condition column"));
    }
    let markers: BTreeSet<_> = row_cells
        .values()
        .flat_map(|cell| &cell.markers)
        .filter(|marker| !marker.trim().is_empty())
        .cloned()
        .collect();
    for marker in markers {
        let matches: Vec<_> = table
            .footnotes
            .iter()
            .filter(|footnote| footnote.marker == marker)
            .collect();
        if matches.len() == 1 {
            let footnote = matches[0];
            let anchor = text_anchor(document, page, &footnote.evidence);
            let candidate = ConditionCandidate {
                id: tagged_content_id(
                    "condition",
                    &serde_json::to_vec(&(&footnote.evidence.text, &anchor, &marker))?,
                ),
                raw_text: footnote.evidence.text.clone(),
                parsed_candidate: Some(ParsedConditionCandidate {
                    fields: BTreeMap::from([("footnote_marker".into(), marker.clone())]),
                }),
                evidence_anchors: vec![anchor],
            };
            ids.push(candidate.id.clone());
            push_unique_condition(&mut sketch.conditions, candidate);
            explanations.push(format!(
                "footnote marker `{marker}` resolved within table scope"
            ));
        } else {
            explanations.push(format!(
                "footnote marker `{marker}` abstained because table-local scope was ambiguous"
            ));
        }
    }
    ids.sort();
    ids.dedup();
    Ok((ids, explanations))
}

fn compile_packages(
    document: &Fdx2Document,
    page: &Fdx2Page,
    table: &Fdx2Table,
    package_cell: Option<&Fdx2Cell>,
    sketch: &mut DatasheetSketch,
) -> Result<Vec<CandidateId>, BaselineError> {
    let Some(cell) = package_cell.filter(|cell| !is_missing_value(&cell.text)) else {
        return Ok(vec![]);
    };
    let anchor = cell_anchor(document, page, table, cell);
    let candidate = PackageCandidate {
        id: anchored_text_id("package", &cell.text, &anchor)?,
        raw_text: cell.text.clone(),
        anchor,
    };
    let id = candidate.id.clone();
    if !sketch.packages.iter().any(|existing| existing.id == id) {
        sketch.packages.push(candidate);
    }
    Ok(vec![id])
}

fn units_compatible(
    parser: &QuantityParser,
    inherited: &str,
    embedded: &str,
) -> Result<bool, BaselineError> {
    let inherited = parser.parse("1", Some(inherited))?;
    let embedded = parser.parse("1", Some(embedded))?;
    Ok(inherited.quantity_kind == embedded.quantity_kind)
}

fn is_missing_value(value: &str) -> bool {
    matches!(value.trim(), "" | "-" | "—" | "–" | "N/A" | "NA")
}

fn push_unique_quantity(target: &mut Vec<QuantityCandidate>, candidate: QuantityCandidate) {
    if !target.iter().any(|existing| existing.id == candidate.id) {
        target.push(candidate);
    }
}

fn push_unique_symbol(target: &mut Vec<SymbolCandidate>, candidate: SymbolCandidate) {
    if !target.iter().any(|existing| existing.id == candidate.id) {
        target.push(candidate);
    }
}

fn push_unique_condition(target: &mut Vec<ConditionCandidate>, candidate: ConditionCandidate) {
    if !target.iter().any(|existing| existing.id == candidate.id) {
        target.push(candidate);
    }
}

fn text_anchor(
    document: &Fdx2Document,
    page: &Fdx2Page,
    evidence: &Fdx2TextEvidence,
) -> EvidenceAnchor {
    EvidenceAnchor {
        schema_version: EVIDENCE_ANCHOR_SCHEMA.into(),
        source_pdf_sha256: document.source_pdf_sha256.clone(),
        document_state_id: document.document_state_id.clone(),
        document_ir_logical_sha256: document.document_ir_logical_sha256.clone(),
        evidence_ids: vec![evidence.evidence_id.clone()],
        page_id: page.page_id.clone(),
        region_id: Some(evidence.region_id.clone()),
        source_geometry: evidence.geometry.clone(),
        geometry_quality: evidence.geometry_quality,
        content_selector: ContentSelector::WholeEvidence,
    }
}

fn cell_anchor(
    document: &Fdx2Document,
    page: &Fdx2Page,
    table: &Fdx2Table,
    cell: &Fdx2Cell,
) -> EvidenceAnchor {
    EvidenceAnchor {
        schema_version: EVIDENCE_ANCHOR_SCHEMA.into(),
        source_pdf_sha256: document.source_pdf_sha256.clone(),
        document_state_id: document.document_state_id.clone(),
        document_ir_logical_sha256: document.document_ir_logical_sha256.clone(),
        evidence_ids: vec![cell.evidence_id.clone()],
        page_id: page.page_id.clone(),
        region_id: Some(table.region_id.clone()),
        source_geometry: cell.geometry.clone(),
        geometry_quality: cell.geometry_quality,
        content_selector: ContentSelector::TableCell {
            table_region_id: table.region_id.clone(),
            row: cell.row,
            column: cell.column,
        },
    }
}

fn anchored_text_id(
    tag: &str,
    raw_text: &str,
    anchor: &EvidenceAnchor,
) -> Result<CandidateId, BaselineError> {
    Ok(tagged_content_id(
        tag,
        &serde_json::to_vec(&(raw_text, anchor))?,
    ))
}

fn all_sketch_anchors(sketch: &DatasheetSketch) -> Vec<&EvidenceAnchor> {
    sketch
        .subjects
        .iter()
        .map(|candidate| &candidate.evidence_anchor)
        .chain(sketch.blocks.iter().map(|candidate| &candidate.anchor))
        .chain(sketch.quantities.iter().map(|candidate| &candidate.anchor))
        .chain(sketch.symbols.iter().map(|candidate| &candidate.anchor))
        .chain(sketch.packages.iter().map(|candidate| &candidate.anchor))
        .chain(
            sketch
                .conditions
                .iter()
                .flat_map(|candidate| &candidate.evidence_anchors),
        )
        .chain(
            sketch
                .sections
                .iter()
                .filter_map(|candidate| candidate.heading_anchor.as_ref()),
        )
        .chain(
            sketch
                .tables
                .iter()
                .flat_map(|table| table.cells.iter().map(|cell| &cell.anchor)),
        )
        .collect()
}

fn normalize_label(value: &str) -> String {
    value
        .trim()
        .to_lowercase()
        .chars()
        .map(|character| {
            if character.is_alphanumeric() {
                character
            } else {
                ' '
            }
        })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

fn normalize_symbol(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_uppercase)
        .collect()
}

fn extract_subject_designation(title: &str) -> Option<&str> {
    title.split_whitespace().rev().find(|token| {
        token.len() >= 4
            && token
                .chars()
                .any(|character| character.is_ascii_alphabetic())
            && token.chars().any(|character| character.is_ascii_digit())
            && token
                .chars()
                .all(|character| character.is_ascii_alphanumeric() || character == '-')
    })
}

fn extract_manufacturer_hint(title: &str, subject: &str) -> Option<String> {
    let hint = title.trim_end_matches(subject).trim();
    (!hint.is_empty()).then(|| hint.to_string())
}

#[derive(Debug)]
pub enum BaselineError {
    Compiler(crate::CompilerError),
    Contract(datasheet_core::ContractError),
    InvalidAnchor(String),
    InvalidDocument(String),
    Json(serde_json::Error),
    MissingSubject,
    MissingValue,
    OntologyMismatch,
    Quantity(QuantityParseError),
    RegistryMismatch,
}

impl std::fmt::Display for BaselineError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for BaselineError {}

impl From<crate::CompilerError> for BaselineError {
    fn from(error: crate::CompilerError) -> Self {
        Self::Compiler(error)
    }
}

impl From<datasheet_core::ContractError> for BaselineError {
    fn from(error: datasheet_core::ContractError) -> Self {
        Self::Contract(error)
    }
}

impl From<serde_json::Error> for BaselineError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}

impl From<QuantityParseError> for BaselineError {
    fn from(error: QuantityParseError) -> Self {
        Self::Quantity(error)
    }
}
