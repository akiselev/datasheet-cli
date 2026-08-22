// SPDX-License-Identifier: MIT OR Apache-2.0
//! Deterministic candidate indexing and fail-closed claim assembly.

pub mod baseline;
pub mod corpus;
pub mod evaluation;
pub mod ferrodoc;
pub mod quantity;
pub mod supervision;

use std::collections::{BTreeMap, BTreeSet};

use datasheet_core::{
    CLAIM_BUNDLE_SCHEMA, CandidateId, ClaimBundle, ClaimCandidate, ConditionCandidate,
    ContentSelector, ContractError, EVIDENCE_ANCHOR_SCHEMA, EvidenceAnchor, GeometryQuality,
    GroundingStatus, NormalizedQuantity, ParsedConditionCandidate, ParsedQuantityShape,
    PredicateVocabularySnapshot, ProducerIdentity, QuantityCandidate, Sha256Digest, SketchId,
    SourceGeometry, SpecificationRegime, SubjectCandidate, SubjectHint, SubjectKind, ValueRole,
    tagged_content_id,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const DATASHEET_SKETCH_SCHEMA: &str = "datasheet-sketch/1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SectionCandidate {
    pub id: CandidateId,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading_text: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub heading_anchor: Option<EvidenceAnchor>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub parent_section_id: Option<CandidateId>,
    pub block_ids: Vec<CandidateId>,
    pub table_ids: Vec<CandidateId>,
    pub deterministic_regime_candidates: Vec<SpecificationRegime>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SketchBlock {
    pub id: CandidateId,
    pub raw_text: String,
    pub anchor: EvidenceAnchor,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SketchTableCell {
    pub row: u32,
    pub column: u32,
    pub raw_text: String,
    pub source_evidence_ids: Vec<String>,
    pub anchor: EvidenceAnchor,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SketchTable {
    pub id: CandidateId,
    pub source_region_id: String,
    pub heading_path: Vec<String>,
    pub rows: u32,
    pub columns: u32,
    pub cells: Vec<SketchTableCell>,
    pub structural_signature: String,
    pub reconstruction_quality: GeometryQuality,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct SymbolCandidate {
    pub id: CandidateId,
    pub raw_text: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub normalized_symbol: Option<String>,
    pub anchor: EvidenceAnchor,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct PackageCandidate {
    pub id: CandidateId,
    pub raw_text: String,
    pub anchor: EvidenceAnchor,
}

/// Electronics candidate index. Corpus-derived family and split assignments are deliberately
/// absent, so later clustering cannot mutate this artifact's identity.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct DatasheetSketch {
    pub schema_version: String,
    pub sketch_id: SketchId,
    pub source_pdf_sha256: Sha256Digest,
    pub document_state_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_ir_logical_sha256: Option<Sha256Digest>,
    pub producer: ProducerIdentity,
    pub subjects: Vec<SubjectCandidate>,
    pub sections: Vec<SectionCandidate>,
    pub blocks: Vec<SketchBlock>,
    pub tables: Vec<SketchTable>,
    pub quantities: Vec<QuantityCandidate>,
    pub symbols: Vec<SymbolCandidate>,
    pub packages: Vec<PackageCandidate>,
    /// Explicit owner for every condition ID that claims may select.
    pub conditions: Vec<ConditionCandidate>,
    pub template_signatures: Vec<String>,
}

#[derive(Serialize)]
struct SketchIdentityView<'a> {
    schema_version: &'a str,
    source_pdf_sha256: &'a Sha256Digest,
    document_state_id: &'a str,
    document_ir_logical_sha256: &'a Option<Sha256Digest>,
    producer: &'a ProducerIdentity,
    subjects: &'a [SubjectCandidate],
    sections: &'a [SectionCandidate],
    blocks: &'a [SketchBlock],
    tables: &'a [SketchTable],
    quantities: &'a [QuantityCandidate],
    symbols: &'a [SymbolCandidate],
    packages: &'a [PackageCandidate],
    conditions: &'a [ConditionCandidate],
    template_signatures: &'a [String],
}

impl DatasheetSketch {
    pub fn recompute_id(&self) -> Result<SketchId, CompilerError> {
        let projection = SketchIdentityView {
            schema_version: &self.schema_version,
            source_pdf_sha256: &self.source_pdf_sha256,
            document_state_id: &self.document_state_id,
            document_ir_logical_sha256: &self.document_ir_logical_sha256,
            producer: &self.producer,
            subjects: &self.subjects,
            sections: &self.sections,
            blocks: &self.blocks,
            tables: &self.tables,
            quantities: &self.quantities,
            symbols: &self.symbols,
            packages: &self.packages,
            conditions: &self.conditions,
            template_signatures: &self.template_signatures,
        };
        let canonical = serde_json::to_vec(&projection)?;
        Ok(SketchId(format!(
            "datasheet-sketch:{}",
            Sha256Digest::of_bytes(&canonical).0
        )))
    }

    pub fn validate(&self) -> Result<(), CompilerError> {
        if self.schema_version != DATASHEET_SKETCH_SCHEMA {
            return Err(CompilerError::Contract(ContractError::SchemaVersion(
                self.schema_version.clone(),
            )));
        }
        self.source_pdf_sha256.validate()?;
        self.producer.validate()?;
        if self.document_state_id.is_empty() || self.recompute_id()? != self.sketch_id {
            return Err(CompilerError::IdentityMismatch);
        }
        let mut ids = BTreeSet::new();
        for (id, anchor) in self.all_candidates_with_anchor() {
            if !ids.insert(id.0.clone()) {
                return Err(CompilerError::DuplicateCandidate(id.0.clone()));
            }
            anchor.validate()?;
            self.validate_anchor_identity(anchor)?;
        }
        for quantity in &self.quantities {
            if let Some(normalized) = &quantity.normalized_quantity {
                if self.producer.quantitas_registry_identity.as_deref()
                    != Some(normalized.registry_identity.as_str())
                {
                    return Err(CompilerError::QuantitasRegistryMismatch);
                }
            }
        }
        for condition in &self.conditions {
            if !ids.insert(condition.id.0.clone()) {
                return Err(CompilerError::DuplicateCandidate(condition.id.0.clone()));
            }
            if condition.raw_text.is_empty() || condition.evidence_anchors.is_empty() {
                return Err(CompilerError::MissingConditionEvidence(
                    condition.id.0.clone(),
                ));
            }
            for anchor in &condition.evidence_anchors {
                anchor.validate()?;
                self.validate_anchor_identity(anchor)?;
            }
        }
        for section in &self.sections {
            if !ids.insert(section.id.0.clone()) {
                return Err(CompilerError::DuplicateCandidate(section.id.0.clone()));
            }
            if let Some(anchor) = &section.heading_anchor {
                anchor.validate()?;
                self.validate_anchor_identity(anchor)?;
            }
        }
        for table in &self.tables {
            if !ids.insert(table.id.0.clone()) {
                return Err(CompilerError::DuplicateCandidate(table.id.0.clone()));
            }
            for cell in &table.cells {
                cell.anchor.validate()?;
                self.validate_anchor_identity(&cell.anchor)?;
            }
        }
        Ok(())
    }

    fn validate_anchor_identity(&self, anchor: &EvidenceAnchor) -> Result<(), CompilerError> {
        if anchor.source_pdf_sha256 != self.source_pdf_sha256
            || anchor.document_state_id != self.document_state_id
            || anchor.document_ir_logical_sha256 != self.document_ir_logical_sha256
        {
            Err(CompilerError::AnchorIdentityMismatch)
        } else {
            Ok(())
        }
    }

    fn all_candidates_with_anchor(&self) -> Vec<(&CandidateId, &EvidenceAnchor)> {
        self.subjects
            .iter()
            .map(|candidate| (&candidate.id, &candidate.evidence_anchor))
            .chain(
                self.blocks
                    .iter()
                    .map(|candidate| (&candidate.id, &candidate.anchor)),
            )
            .chain(
                self.quantities
                    .iter()
                    .map(|candidate| (&candidate.id, &candidate.anchor)),
            )
            .chain(
                self.symbols
                    .iter()
                    .map(|candidate| (&candidate.id, &candidate.anchor)),
            )
            .chain(
                self.packages
                    .iter()
                    .map(|candidate| (&candidate.id, &candidate.anchor)),
            )
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FixtureDocumentIr {
    pub schema_version: String,
    pub source_pdf_sha256: Sha256Digest,
    pub document_state_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub document_ir_logical_sha256: Option<Sha256Digest>,
    pub pages: Vec<FixturePage>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FixturePage {
    pub page_id: String,
    pub regions: Vec<FixtureRegion>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FixtureRegion {
    pub region_id: String,
    pub geometry: SourceGeometry,
    pub geometry_quality: GeometryQuality,
    pub evidence: Vec<FixtureEvidence>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub table_shape: Option<FixtureTableShape>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FixtureTableShape {
    pub rows: u32,
    pub columns: u32,
    pub heading_path: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FixtureEvidence {
    pub evidence_id: String,
    pub text: String,
    #[serde(flatten)]
    pub candidate: FixtureCandidate,
}

/// Explicit annotations are permitted only in minimized conformance fixtures. They make FDX1
/// contract tests deterministic without claiming a production electronics parser.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "candidate_kind", rename_all = "snake_case")]
pub enum FixtureCandidate {
    Block,
    Subject {
        subject_kind: SubjectKind,
        #[serde(default)]
        manufacturer_hint: Option<String>,
    },
    Section {
        regime: SpecificationRegime,
    },
    Condition {
        #[serde(default)]
        parsed_fields: BTreeMap<String, String>,
    },
    Quantity {
        shape: ParsedQuantityShape,
        #[serde(default)]
        raw_unit: Option<String>,
        #[serde(default)]
        canonical_value: Option<String>,
        #[serde(default)]
        canonical_unit: Option<String>,
        #[serde(default)]
        quantity_kind_hint: Option<String>,
    },
    Symbol {
        #[serde(default)]
        normalized_symbol: Option<String>,
    },
    Package,
    TableCell {
        row: u32,
        column: u32,
    },
}

impl FixtureDocumentIr {
    pub fn validate_anchor(&self, anchor: &EvidenceAnchor) -> Result<(), CompilerError> {
        anchor.validate()?;
        if anchor.source_pdf_sha256 != self.source_pdf_sha256
            || anchor.document_state_id != self.document_state_id
            || anchor.document_ir_logical_sha256 != self.document_ir_logical_sha256
        {
            return Err(CompilerError::AnchorIdentityMismatch);
        }
        let page = self
            .pages
            .iter()
            .find(|page| page.page_id == anchor.page_id)
            .ok_or_else(|| CompilerError::MissingPage(anchor.page_id.clone()))?;
        let region = anchor
            .region_id
            .as_ref()
            .map(|region_id| {
                page.regions
                    .iter()
                    .find(|region| &region.region_id == region_id)
                    .ok_or_else(|| CompilerError::MissingRegion(region_id.clone()))
            })
            .transpose()?;
        if let ContentSelector::TableCell {
            table_region_id,
            row,
            column,
        } = &anchor.content_selector
        {
            let region =
                region.ok_or_else(|| CompilerError::MissingRegion(table_region_id.clone()))?;
            let cell = region.evidence.iter().find(|evidence| {
                matches!(
                    evidence.candidate,
                    FixtureCandidate::TableCell {
                        row: candidate_row,
                        column: candidate_column,
                    } if candidate_row == *row && candidate_column == *column
                )
            });
            if cell.is_none_or(|cell| !anchor.evidence_ids.contains(&cell.evidence_id)) {
                return Err(CompilerError::MissingEvidence(format!(
                    "{table_region_id}[{row},{column}]"
                )));
            }
        }
        for evidence_id in &anchor.evidence_ids {
            let found = region
                .map_or_else(
                    || {
                        page.regions
                            .iter()
                            .flat_map(|region| &region.evidence)
                            .collect::<Vec<_>>()
                    },
                    |region| region.evidence.iter().collect::<Vec<_>>(),
                )
                .iter()
                .any(|evidence| evidence.evidence_id == *evidence_id);
            if !found {
                return Err(CompilerError::MissingEvidence(evidence_id.clone()));
            }
        }
        Ok(())
    }
}

pub fn sketch_from_fixture(
    fixture: &FixtureDocumentIr,
    producer: ProducerIdentity,
) -> Result<DatasheetSketch, CompilerError> {
    fixture.source_pdf_sha256.validate()?;
    producer.validate()?;
    if producer.document_ir_schema != fixture.schema_version {
        return Err(CompilerError::DocumentSchemaMismatch);
    }
    let mut sketch = DatasheetSketch {
        schema_version: DATASHEET_SKETCH_SCHEMA.into(),
        sketch_id: SketchId(String::new()),
        source_pdf_sha256: fixture.source_pdf_sha256.clone(),
        document_state_id: fixture.document_state_id.clone(),
        document_ir_logical_sha256: fixture.document_ir_logical_sha256.clone(),
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
    };

    for page in &fixture.pages {
        for region in &page.regions {
            region.geometry.validate()?;
            let mut table_cells = vec![];
            for evidence in &region.evidence {
                let anchor = anchor_for(fixture, page, region, evidence);
                fixture.validate_anchor(&anchor)?;
                let id = candidate_id(&evidence.candidate, &anchor, &evidence.text)?;
                match &evidence.candidate {
                    FixtureCandidate::Block => sketch.blocks.push(SketchBlock {
                        id,
                        raw_text: evidence.text.clone(),
                        anchor,
                    }),
                    FixtureCandidate::Subject {
                        subject_kind,
                        manufacturer_hint,
                    } => sketch.subjects.push(SubjectCandidate {
                        id,
                        kind: subject_kind.clone(),
                        raw_designation: evidence.text.clone(),
                        manufacturer_hint: manufacturer_hint.clone(),
                        normalized_search_key: Some(normalize_search_key(&evidence.text)),
                        evidence_anchor: anchor,
                    }),
                    FixtureCandidate::Section { regime } => {
                        sketch.sections.push(SectionCandidate {
                            id,
                            heading_text: Some(evidence.text.clone()),
                            heading_anchor: Some(anchor),
                            parent_section_id: None,
                            block_ids: vec![],
                            table_ids: vec![],
                            deterministic_regime_candidates: vec![regime.clone()],
                        });
                    }
                    FixtureCandidate::Condition { parsed_fields } => {
                        sketch.conditions.push(ConditionCandidate {
                            id,
                            raw_text: evidence.text.clone(),
                            parsed_candidate: (!parsed_fields.is_empty()).then(|| {
                                ParsedConditionCandidate {
                                    fields: parsed_fields.clone(),
                                }
                            }),
                            evidence_anchors: vec![anchor],
                        });
                    }
                    FixtureCandidate::Quantity {
                        shape,
                        raw_unit,
                        canonical_value,
                        canonical_unit,
                        quantity_kind_hint,
                    } => {
                        let normalized_quantity = match (canonical_value, canonical_unit) {
                            (Some(value), Some(unit)) => Some(NormalizedQuantity {
                                canonical_value: value.clone(),
                                canonical_unit: unit.clone(),
                                registry_identity: sketch
                                    .producer
                                    .quantitas_registry_identity
                                    .clone()
                                    .ok_or(CompilerError::MissingQuantitasIdentity)?,
                            }),
                            (None, None) => None,
                            _ => return Err(CompilerError::IncompleteNormalization),
                        };
                        sketch.quantities.push(QuantityCandidate {
                            id,
                            raw_text: evidence.text.clone(),
                            parsed_shape: shape.clone(),
                            raw_unit: raw_unit.clone(),
                            normalized_quantity,
                            quantity_kind_hint: quantity_kind_hint.clone(),
                            dimensionality: None,
                            anchor,
                            parse_warnings: vec![],
                        });
                    }
                    FixtureCandidate::Symbol { normalized_symbol } => {
                        sketch.symbols.push(SymbolCandidate {
                            id,
                            raw_text: evidence.text.clone(),
                            normalized_symbol: normalized_symbol.clone(),
                            anchor,
                        });
                    }
                    FixtureCandidate::Package => sketch.packages.push(PackageCandidate {
                        id,
                        raw_text: evidence.text.clone(),
                        anchor,
                    }),
                    FixtureCandidate::TableCell { row, column } => {
                        table_cells.push(SketchTableCell {
                            row: *row,
                            column: *column,
                            raw_text: evidence.text.clone(),
                            source_evidence_ids: vec![evidence.evidence_id.clone()],
                            anchor,
                        });
                    }
                }
            }
            if let Some(shape) = &region.table_shape {
                table_cells.sort_by_key(|cell| (cell.row, cell.column));
                let cell_coordinates: Vec<_> = table_cells
                    .iter()
                    .map(|cell| (cell.row, cell.column))
                    .collect();
                let signature_bytes =
                    serde_json::to_vec(&(shape.rows, shape.columns, cell_coordinates))?;
                let signature = Sha256Digest::of_bytes(&signature_bytes).0;
                let table_id = tagged_content_id(
                    "table",
                    &serde_json::to_vec(&(
                        &fixture.source_pdf_sha256,
                        &fixture.document_state_id,
                        &page.page_id,
                        &region.region_id,
                        &signature,
                    ))?,
                );
                sketch.template_signatures.push(signature.clone());
                sketch.tables.push(SketchTable {
                    id: table_id,
                    source_region_id: region.region_id.clone(),
                    heading_path: shape.heading_path.clone(),
                    rows: shape.rows,
                    columns: shape.columns,
                    cells: table_cells,
                    structural_signature: signature,
                    reconstruction_quality: region.geometry_quality,
                });
            } else if !table_cells.is_empty() {
                return Err(CompilerError::TableShapeMissing(region.region_id.clone()));
            }
        }
    }

    sort_sketch(&mut sketch);
    sketch.sketch_id = sketch.recompute_id()?;
    sketch.validate()?;
    Ok(sketch)
}

fn anchor_for(
    fixture: &FixtureDocumentIr,
    page: &FixturePage,
    region: &FixtureRegion,
    evidence: &FixtureEvidence,
) -> EvidenceAnchor {
    let content_selector = match evidence.candidate {
        FixtureCandidate::TableCell { row, column } => ContentSelector::TableCell {
            table_region_id: region.region_id.clone(),
            row,
            column,
        },
        _ => ContentSelector::WholeEvidence,
    };
    EvidenceAnchor {
        schema_version: EVIDENCE_ANCHOR_SCHEMA.into(),
        source_pdf_sha256: fixture.source_pdf_sha256.clone(),
        document_state_id: fixture.document_state_id.clone(),
        document_ir_logical_sha256: fixture.document_ir_logical_sha256.clone(),
        evidence_ids: vec![evidence.evidence_id.clone()],
        page_id: page.page_id.clone(),
        region_id: Some(region.region_id.clone()),
        source_geometry: vec![region.geometry.clone()],
        geometry_quality: region.geometry_quality,
        content_selector,
    }
}

fn candidate_id(
    candidate: &FixtureCandidate,
    anchor: &EvidenceAnchor,
    raw_text: &str,
) -> Result<CandidateId, CompilerError> {
    let tag = match candidate {
        FixtureCandidate::Block => "block",
        FixtureCandidate::Subject { .. } => "subject",
        FixtureCandidate::Section { .. } => "section",
        FixtureCandidate::Condition { .. } => "condition",
        FixtureCandidate::Quantity { .. } => "quantity",
        FixtureCandidate::Symbol { .. } => "symbol",
        FixtureCandidate::Package => "package",
        FixtureCandidate::TableCell { .. } => "cell",
    };
    Ok(tagged_content_id(
        tag,
        &serde_json::to_vec(&(candidate, anchor, raw_text))?,
    ))
}

fn normalize_search_key(value: &str) -> String {
    value
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

pub(crate) fn sort_sketch(sketch: &mut DatasheetSketch) {
    sketch
        .subjects
        .sort_by(|left, right| left.id.cmp(&right.id));
    sketch
        .sections
        .sort_by(|left, right| left.id.cmp(&right.id));
    sketch.blocks.sort_by(|left, right| left.id.cmp(&right.id));
    sketch.tables.sort_by(|left, right| left.id.cmp(&right.id));
    sketch
        .quantities
        .sort_by(|left, right| left.id.cmp(&right.id));
    sketch.symbols.sort_by(|left, right| left.id.cmp(&right.id));
    sketch
        .packages
        .sort_by(|left, right| left.id.cmp(&right.id));
    sketch
        .conditions
        .sort_by(|left, right| left.id.cmp(&right.id));
    for section in &mut sketch.sections {
        section.block_ids.sort();
        section.block_ids.dedup();
        section.table_ids.sort();
        section.table_ids.dedup();
        section.deterministic_regime_candidates.sort();
        section.deterministic_regime_candidates.dedup();
    }
    sketch.template_signatures.sort();
    sketch.template_signatures.dedup();
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ClaimSelection {
    pub subject_candidate_id: CandidateId,
    pub predicate_term_id: String,
    pub regime: SpecificationRegime,
    pub value_role: ValueRole,
    pub value_candidate_id: CandidateId,
    pub condition_candidate_ids: Vec<CandidateId>,
    pub package_variant_candidate_ids: Vec<CandidateId>,
    pub extraction_confidence: f64,
}

pub fn assemble_claim_bundle(
    sketch: &DatasheetSketch,
    vocabulary: &PredicateVocabularySnapshot,
    selections: &[ClaimSelection],
) -> Result<ClaimBundle, CompilerError> {
    sketch.validate()?;
    vocabulary.digest.validate()?;
    if vocabulary.digest != sketch.producer.ontology_snapshot_digest {
        return Err(CompilerError::OntologyMismatch);
    }
    let subjects: BTreeMap<_, _> = sketch
        .subjects
        .iter()
        .map(|candidate| (&candidate.id, candidate))
        .collect();
    let values: BTreeMap<_, _> = sketch
        .quantities
        .iter()
        .map(|candidate| (&candidate.id, candidate))
        .collect();
    let conditions: BTreeMap<_, _> = sketch
        .conditions
        .iter()
        .map(|candidate| (&candidate.id, candidate))
        .collect();
    let predicates: BTreeSet<_> = vocabulary
        .terms
        .iter()
        .map(|term| term.id.as_str())
        .collect();
    let mut claims = Vec::with_capacity(selections.len());
    let mut selected_values = BTreeMap::new();
    let mut subject_hints = BTreeSet::new();

    for selection in selections {
        let mut selection = selection.clone();
        selection.condition_candidate_ids.sort();
        selection.condition_candidate_ids.dedup();
        selection.package_variant_candidate_ids.sort();
        selection.package_variant_candidate_ids.dedup();
        let subject = subjects
            .get(&selection.subject_candidate_id)
            .ok_or_else(|| {
                CompilerError::MissingCandidate(selection.subject_candidate_id.0.clone())
            })?;
        let value = values.get(&selection.value_candidate_id).ok_or_else(|| {
            CompilerError::MissingCandidate(selection.value_candidate_id.0.clone())
        })?;
        if !predicates.contains(selection.predicate_term_id.as_str()) {
            return Err(CompilerError::MissingPredicate(
                selection.predicate_term_id.clone(),
            ));
        }
        let mut anchors = vec![subject.evidence_anchor.clone(), value.anchor.clone()];
        for condition_id in &selection.condition_candidate_ids {
            let condition = conditions
                .get(condition_id)
                .ok_or_else(|| CompilerError::MissingCondition(condition_id.0.clone()))?;
            anchors.extend(condition.evidence_anchors.clone());
        }
        for package_id in &selection.package_variant_candidate_ids {
            let package = sketch
                .packages
                .iter()
                .find(|candidate| &candidate.id == package_id)
                .ok_or_else(|| CompilerError::MissingCandidate(package_id.0.clone()))?;
            anchors.push(package.anchor.clone());
        }
        anchors.sort_by_key(|anchor| serde_json::to_string(anchor).unwrap_or_default());
        anchors.dedup();
        let local_id = tagged_content_id(
            "claim",
            &serde_json::to_vec(&(&sketch.sketch_id, &selection))?,
        );
        selected_values.insert(value.id.clone(), (*value).clone());
        subject_hints.insert((
            subject.manufacturer_hint.clone(),
            subject.raw_designation.clone(),
        ));
        claims.push(ClaimCandidate {
            local_id,
            subject_candidate_id: selection.subject_candidate_id,
            predicate_term_id: selection.predicate_term_id,
            regime: selection.regime,
            value_role: selection.value_role,
            value_candidate_id: selection.value_candidate_id,
            condition_candidate_ids: selection.condition_candidate_ids,
            package_variant_candidate_ids: selection.package_variant_candidate_ids,
            evidence_anchors: anchors,
            extraction_confidence: selection.extraction_confidence,
            grounding_status: GroundingStatus::Grounded,
        });
    }
    claims.sort_by(|left, right| left.local_id.cmp(&right.local_id));
    let bundle = ClaimBundle {
        schema_version: CLAIM_BUNDLE_SCHEMA.into(),
        sketch_id: sketch.sketch_id.clone(),
        source_pdf_sha256: sketch.source_pdf_sha256.clone(),
        document_state_id: sketch.document_state_id.clone(),
        document_ir_logical_sha256: sketch.document_ir_logical_sha256.clone(),
        producer: sketch.producer.clone(),
        subject_hints: subject_hints
            .into_iter()
            .map(|(manufacturer_raw, mpn_raw)| SubjectHint {
                manufacturer_raw,
                mpn_raw,
            })
            .collect(),
        selected_values: selected_values.into_values().collect(),
        claims,
        warnings: vec![],
    };
    validate_claim_bundle_against_sketch_and_vocabulary(&bundle, sketch, vocabulary)?;
    Ok(bundle)
}

pub fn validate_claim_bundle_against_sketch_and_vocabulary(
    bundle: &ClaimBundle,
    sketch: &DatasheetSketch,
    vocabulary: &PredicateVocabularySnapshot,
) -> Result<(), CompilerError> {
    validate_claim_bundle_against_sketch(bundle, sketch)?;
    vocabulary.digest.validate()?;
    if vocabulary.digest != sketch.producer.ontology_snapshot_digest {
        return Err(CompilerError::OntologyMismatch);
    }
    let predicates: BTreeSet<_> = vocabulary
        .terms
        .iter()
        .map(|term| term.id.as_str())
        .collect();
    for claim in &bundle.claims {
        if !predicates.contains(claim.predicate_term_id.as_str()) {
            return Err(CompilerError::MissingPredicate(
                claim.predicate_term_id.clone(),
            ));
        }
    }
    Ok(())
}

pub fn validate_claim_bundle_against_sketch(
    bundle: &ClaimBundle,
    sketch: &DatasheetSketch,
) -> Result<(), CompilerError> {
    bundle.validate_structure()?;
    sketch.validate()?;
    if bundle.sketch_id != sketch.sketch_id
        || bundle.source_pdf_sha256 != sketch.source_pdf_sha256
        || bundle.document_state_id != sketch.document_state_id
        || bundle.document_ir_logical_sha256 != sketch.document_ir_logical_sha256
        || bundle.producer != sketch.producer
    {
        return Err(CompilerError::IdentityMismatch);
    }
    let subjects: BTreeSet<_> = sketch
        .subjects
        .iter()
        .map(|candidate| &candidate.id)
        .collect();
    let values: BTreeMap<_, _> = sketch
        .quantities
        .iter()
        .map(|candidate| (&candidate.id, candidate))
        .collect();
    let packages: BTreeMap<_, _> = sketch
        .packages
        .iter()
        .map(|candidate| (&candidate.id, candidate))
        .collect();
    let selected_values: BTreeMap<_, _> = bundle
        .selected_values
        .iter()
        .map(|candidate| (&candidate.id, candidate))
        .collect();
    for (id, selected) in &selected_values {
        if values.get(id).copied() != Some(*selected) {
            return Err(CompilerError::SelectedValueMismatch(id.0.clone()));
        }
    }
    let mut expected_subject_hints = BTreeSet::new();
    let sketch_anchors: Vec<_> = sketch
        .all_candidates_with_anchor()
        .into_iter()
        .map(|(_, anchor)| anchor)
        .chain(
            sketch
                .conditions
                .iter()
                .flat_map(|condition| &condition.evidence_anchors),
        )
        .chain(
            sketch
                .sections
                .iter()
                .filter_map(|section| section.heading_anchor.as_ref()),
        )
        .chain(
            sketch
                .tables
                .iter()
                .flat_map(|table| table.cells.iter().map(|cell| &cell.anchor)),
        )
        .collect();
    for claim in &bundle.claims {
        if !subjects.contains(&claim.subject_candidate_id) {
            return Err(CompilerError::MissingCandidate(
                claim.subject_candidate_id.0.clone(),
            ));
        }
        let value = values
            .get(&claim.value_candidate_id)
            .ok_or_else(|| CompilerError::MissingCandidate(claim.value_candidate_id.0.clone()))?;
        if !selected_values.contains_key(&claim.value_candidate_id) {
            return Err(CompilerError::MissingCandidate(
                claim.value_candidate_id.0.clone(),
            ));
        }
        let subject = sketch
            .subjects
            .iter()
            .find(|candidate| candidate.id == claim.subject_candidate_id)
            .expect("subject membership checked above");
        expected_subject_hints.insert((
            subject.manufacturer_hint.clone(),
            subject.raw_designation.clone(),
        ));
        let mut canonical_conditions = claim.condition_candidate_ids.clone();
        canonical_conditions.sort();
        canonical_conditions.dedup();
        let mut canonical_packages = claim.package_variant_candidate_ids.clone();
        canonical_packages.sort();
        canonical_packages.dedup();
        if canonical_conditions != claim.condition_candidate_ids
            || canonical_packages != claim.package_variant_candidate_ids
        {
            return Err(CompilerError::NonCanonicalClaimSelection);
        }
        let mut required_anchors = vec![&subject.evidence_anchor, &value.anchor];
        for condition in &claim.condition_candidate_ids {
            let condition = sketch
                .conditions
                .iter()
                .find(|candidate| &candidate.id == condition)
                .ok_or_else(|| CompilerError::MissingCondition(condition.0.clone()))?;
            required_anchors.extend(&condition.evidence_anchors);
        }
        for package in &claim.package_variant_candidate_ids {
            let package = packages
                .get(package)
                .ok_or_else(|| CompilerError::MissingCandidate(package.0.clone()))?;
            required_anchors.push(&package.anchor);
        }
        for anchor in &claim.evidence_anchors {
            if !sketch_anchors.contains(&anchor) {
                return Err(CompilerError::MissingBundleEvidence);
            }
        }
        if required_anchors
            .iter()
            .any(|required| !claim.evidence_anchors.contains(required))
        {
            return Err(CompilerError::MissingBundleEvidence);
        }
        let selection = ClaimSelection {
            subject_candidate_id: claim.subject_candidate_id.clone(),
            predicate_term_id: claim.predicate_term_id.clone(),
            regime: claim.regime.clone(),
            value_role: claim.value_role.clone(),
            value_candidate_id: claim.value_candidate_id.clone(),
            condition_candidate_ids: claim.condition_candidate_ids.clone(),
            package_variant_candidate_ids: claim.package_variant_candidate_ids.clone(),
            extraction_confidence: claim.extraction_confidence,
        };
        let expected_id = tagged_content_id(
            "claim",
            &serde_json::to_vec(&(&sketch.sketch_id, &selection))?,
        );
        if claim.local_id != expected_id {
            return Err(CompilerError::IdentityMismatch);
        }
    }
    let referenced_values: BTreeSet<_> = bundle
        .claims
        .iter()
        .map(|claim| &claim.value_candidate_id)
        .collect();
    if selected_values.keys().copied().collect::<BTreeSet<_>>() != referenced_values {
        return Err(CompilerError::SelectedValueSetMismatch);
    }
    if bundle
        .subject_hints
        .iter()
        .map(|hint| (hint.manufacturer_raw.clone(), hint.mpn_raw.clone()))
        .collect::<BTreeSet<_>>()
        != expected_subject_hints
        || bundle.subject_hints.len() != expected_subject_hints.len()
    {
        return Err(CompilerError::SubjectHintMismatch);
    }
    Ok(())
}

#[derive(Debug)]
pub enum CompilerError {
    AnchorIdentityMismatch,
    Contract(ContractError),
    DuplicateCandidate(String),
    DocumentSchemaMismatch,
    IdentityMismatch,
    IncompleteNormalization,
    Json(serde_json::Error),
    MissingBundleEvidence,
    MissingCandidate(String),
    MissingCondition(String),
    MissingConditionEvidence(String),
    MissingEvidence(String),
    MissingPage(String),
    MissingPredicate(String),
    MissingQuantitasIdentity,
    MissingRegion(String),
    NonCanonicalClaimSelection,
    OntologyMismatch,
    QuantitasRegistryMismatch,
    SelectedValueMismatch(String),
    SelectedValueSetMismatch,
    SubjectHintMismatch,
    TableShapeMissing(String),
}

impl std::fmt::Display for CompilerError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for CompilerError {}

impl From<ContractError> for CompilerError {
    fn from(error: ContractError) -> Self {
        Self::Contract(error)
    }
}

impl From<serde_json::Error> for CompilerError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}
