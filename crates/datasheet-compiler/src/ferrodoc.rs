// SPDX-License-Identifier: MIT OR Apache-2.0
//! Portable bridge for the landed Ferrodoc FP0/FDX0 JSON contracts.
//!
//! These wire types deliberately mirror only the public evidence/state fields consumed by
//! Datasheet. Keeping the bridge schema-based avoids coupling either repository's runtime.

use std::collections::{BTreeMap, BTreeSet};

use datasheet_core::{
    GeometryQuality, PredicateVocabularySnapshot, ProducerIdentity, Sha256Digest, SourceGeometry,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::baseline::{
    BaselineError, DeterministicBaseline, Fdx2Cell, Fdx2Document, Fdx2Footnote, Fdx2Page,
    Fdx2Table, Fdx2TextEvidence, compile_analog_power_baseline, infer_regime,
};

pub const FERRODOC_EVIDENCE_DELTA_SCHEMA: &str = "ferrodoc-evidence-delta/1";
pub const FERRODOC_DOCUMENT_STATE_SCHEMA: &str = "ferrodoc-document-state/1";
pub const FERRODOC_CONFORMANCE_REVISION: &str = "2a4d886703753a8893bfa17ade8a554df61f59ab";
pub const FERRODOC_EVIDENCE_DELTA_SCHEMA_SHA256: &str =
    "ab98db6810db12bdf22b04e6b113bbbe4cbe43aa187702967ad1db30c1e0ff16";
pub const FERRODOC_DOCUMENT_STATE_SCHEMA_SHA256: &str =
    "a9a81cd6b65acf1a256529337c985ad512d34a7b79274f48a926490c35f1aa2b";

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FerrodocSchemaVersion {
    pub major: u16,
    pub minor: u16,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FerrodocDeltaProducer {
    pub name: String,
    pub version: String,
    pub build: String,
    pub model_digest: Option<String>,
    pub configuration_digest: String,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema)]
pub struct FerrodocPageRegionRef {
    pub page_id: String,
    pub region_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FerrodocRefinementScope {
    Document,
    Pages {
        page_ids: BTreeSet<String>,
    },
    Regions {
        regions: BTreeSet<FerrodocPageRegionRef>,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FerrodocRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub space: String,
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FerrodocPageRect {
    pub page_index: u32,
    pub rect: FerrodocRect,
    pub source_transform: [f64; 6],
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FerrodocProvenance {
    pub schema_version: FerrodocSchemaVersion,
    pub input_digest: String,
    pub engine_id: String,
    pub engine_version: String,
    pub model_digest: Option<String>,
    pub parameters: BTreeMap<String, serde_json::Value>,
    pub stage: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FerrodocSourceLayer {
    pub id: String,
    pub kind: serde_json::Value,
    pub provenance: FerrodocProvenance,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FerrodocTextSourceSpan {
    pub evidence_id: String,
    pub start: u32,
    pub end: u32,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FerrodocTableCell {
    pub row: u32,
    pub column: u32,
    pub row_span: u32,
    pub column_span: u32,
    pub text: String,
    #[serde(default)]
    pub geometry: Option<FerrodocPageRect>,
    #[serde(default)]
    pub geometry_quality: FerrodocGeometryQuality,
    #[serde(default)]
    pub source_spans: Vec<FerrodocTextSourceSpan>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum FerrodocEvidenceContent {
    Text {
        text: String,
    },
    Table {
        rows: u32,
        columns: u32,
        cells: Vec<FerrodocTableCell>,
    },
    Formula {
        latex: String,
    },
    Image {
        artifact: String,
        alt: Option<String>,
    },
    Unknown {
        media_type: String,
        value: serde_json::Value,
    },
}

#[derive(
    Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Serialize, Deserialize, JsonSchema,
)]
#[serde(rename_all = "snake_case")]
pub enum FerrodocGeometryQuality {
    Glyph,
    Word,
    Line,
    Region,
    PageOnly,
    #[default]
    Unknown,
}

impl From<FerrodocGeometryQuality> for GeometryQuality {
    fn from(value: FerrodocGeometryQuality) -> Self {
        match value {
            FerrodocGeometryQuality::Glyph => Self::ExactGlyph,
            FerrodocGeometryQuality::Word => Self::ExactWord,
            FerrodocGeometryQuality::Line => Self::ExactLine,
            FerrodocGeometryQuality::Region => Self::ExactRegion,
            FerrodocGeometryQuality::PageOnly => Self::PageOnly,
            FerrodocGeometryQuality::Unknown => Self::Unknown,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FerrodocEvidence {
    pub id: String,
    pub layer_id: String,
    pub content: FerrodocEvidenceContent,
    pub geometry: Option<FerrodocPageRect>,
    #[serde(default)]
    pub geometry_quality: FerrodocGeometryQuality,
    pub confidence: Option<f64>,
    pub provenance: FerrodocProvenance,
    #[serde(default)]
    pub engine_metadata: BTreeMap<String, serde_json::Value>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FerrodocSelectedView {
    pub evidence_ids: Vec<String>,
    pub reason: String,
    pub explanation: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FerrodocRegion {
    pub id: String,
    pub kind: String,
    pub geometry: FerrodocPageRect,
    #[serde(default)]
    pub evidence: Vec<FerrodocEvidence>,
    pub selected: Option<FerrodocSelectedView>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FerrodocPage {
    pub id: String,
    pub index: u32,
    pub bounds: FerrodocPageRect,
    #[serde(default)]
    pub layers: Vec<FerrodocSourceLayer>,
    #[serde(default)]
    pub artifacts: Vec<serde_json::Value>,
    #[serde(default)]
    pub regions: Vec<FerrodocRegion>,
    #[serde(default)]
    pub reading_order: Vec<serde_json::Value>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FerrodocEvidenceDelta {
    pub delta_schema: String,
    pub source_pdf_sha256: String,
    pub ir_schema: FerrodocSchemaVersion,
    pub stage: String,
    pub producer: FerrodocDeltaProducer,
    pub scope: FerrodocRefinementScope,
    pub input_state_id: Option<String>,
    #[serde(default)]
    pub required_evidence_ids: BTreeSet<String>,
    #[serde(default)]
    pub new_pages: Vec<FerrodocPage>,
    #[serde(default)]
    pub page_additions: Vec<serde_json::Value>,
    #[serde(default)]
    pub selection_hints: Vec<serde_json::Value>,
    #[serde(default)]
    pub diagnostics: Vec<serde_json::Value>,
    #[serde(default)]
    pub coverage_delta: Vec<serde_json::Value>,
}

impl FerrodocEvidenceDelta {
    pub fn logical_id(&self) -> Result<String, FerrodocBridgeError> {
        self.validate()?;
        let projection = (
            &self.source_pdf_sha256,
            self.ir_schema,
            &self.stage,
            &self.producer,
            &self.new_pages,
            &self.page_additions,
            &self.selection_hints,
        );
        Ok(derive_ferrodoc_id(
            "delta",
            &[
                FERRODOC_EVIDENCE_DELTA_SCHEMA.as_bytes(),
                &serde_json::to_vec(&projection)?,
            ],
        ))
    }

    fn validate(&self) -> Result<(), FerrodocBridgeError> {
        if self.delta_schema != FERRODOC_EVIDENCE_DELTA_SCHEMA || self.ir_schema.major != 1 {
            return Err(FerrodocBridgeError::UnsupportedSchema);
        }
        validate_digest(&self.source_pdf_sha256)?;
        validate_digest(&self.producer.build)?;
        validate_digest(&self.producer.configuration_digest)?;
        if self.producer.name.trim().is_empty() || self.producer.version.trim().is_empty() {
            return Err(FerrodocBridgeError::InvalidDelta("empty producer".into()));
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FerrodocMaterializedCheckpoint {
    pub document_ir_logical_sha256: String,
    pub artifact_id: String,
    pub representation: String,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FerrodocDocumentStateManifest {
    pub state_schema: String,
    pub source_pdf_sha256: String,
    pub ir_schema: FerrodocSchemaVersion,
    pub evidence_delta_ids: BTreeSet<String>,
    pub reconciliation_policy_id: String,
    #[serde(default)]
    pub coverage: Vec<serde_json::Value>,
    pub materialized_ir_checkpoint: Option<FerrodocMaterializedCheckpoint>,
    #[serde(default)]
    pub parent_state_ids: BTreeSet<String>,
}

impl FerrodocDocumentStateManifest {
    pub fn logical_id(&self) -> Result<String, FerrodocBridgeError> {
        if self.state_schema != FERRODOC_DOCUMENT_STATE_SCHEMA || self.ir_schema.major != 1 {
            return Err(FerrodocBridgeError::UnsupportedSchema);
        }
        validate_digest(&self.source_pdf_sha256)?;
        validate_digest(&self.reconciliation_policy_id)?;
        for delta_id in &self.evidence_delta_ids {
            validate_stable_id("delta", delta_id)?;
        }
        for parent_id in &self.parent_state_ids {
            validate_stable_id("state", parent_id)?;
        }
        if let Some(checkpoint) = &self.materialized_ir_checkpoint {
            validate_digest(&checkpoint.document_ir_logical_sha256)?;
            validate_stable_id("artifact", &checkpoint.artifact_id)?;
            if checkpoint.representation.trim().is_empty() {
                return Err(FerrodocBridgeError::WrongState);
            }
        }
        let projection = (
            FERRODOC_DOCUMENT_STATE_SCHEMA,
            &self.source_pdf_sha256,
            self.ir_schema,
            &self.evidence_delta_ids,
            &self.reconciliation_policy_id,
        );
        Ok(derive_ferrodoc_id(
            "state",
            &[
                FERRODOC_DOCUMENT_STATE_SCHEMA.as_bytes(),
                &serde_json::to_vec(&projection)?,
            ],
        ))
    }
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct FerrodocRegionRefinementTarget {
    pub base_document_state_id: String,
    pub page_id: String,
    pub region_id: String,
    pub requested_capabilities: BTreeSet<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct FerrodocConformanceInput {
    pub upstream_revision: String,
    pub evidence_delta_schema_sha256: String,
    pub document_state_schema_sha256: String,
    pub evidence_delta: FerrodocEvidenceDelta,
    pub document_state_manifest: FerrodocDocumentStateManifest,
    pub targets: Vec<FerrodocRegionRefinementTarget>,
}

impl FerrodocConformanceInput {
    pub fn validate(&self) -> Result<String, FerrodocBridgeError> {
        if self.upstream_revision != FERRODOC_CONFORMANCE_REVISION
            || self.evidence_delta_schema_sha256 != FERRODOC_EVIDENCE_DELTA_SCHEMA_SHA256
            || self.document_state_schema_sha256 != FERRODOC_DOCUMENT_STATE_SCHEMA_SHA256
        {
            return Err(FerrodocBridgeError::ContractLockMismatch);
        }
        let delta_id = self.evidence_delta.logical_id()?;
        let state_id = self.document_state_manifest.logical_id()?;
        if self.evidence_delta.input_state_id.is_some()
            || !self.evidence_delta.page_additions.is_empty()
            || !matches!(self.evidence_delta.scope, FerrodocRefinementScope::Document)
        {
            return Err(FerrodocBridgeError::UnresolvedInputStatePrecondition);
        }
        if self.evidence_delta.source_pdf_sha256 != self.document_state_manifest.source_pdf_sha256
            || self.evidence_delta.ir_schema != self.document_state_manifest.ir_schema
            || self.document_state_manifest.evidence_delta_ids != BTreeSet::from([delta_id])
        {
            return Err(FerrodocBridgeError::WrongState);
        }
        validate_pages(&self.evidence_delta)?;
        for target in &self.targets {
            self.validate_target(target, &state_id)?;
        }
        Ok(state_id)
    }

    pub fn validate_target(
        &self,
        target: &FerrodocRegionRefinementTarget,
        state_id: &str,
    ) -> Result<(), FerrodocBridgeError> {
        if target.base_document_state_id != state_id || target.requested_capabilities.is_empty() {
            return Err(FerrodocBridgeError::StaleTarget);
        }
        validate_stable_id("state", &target.base_document_state_id)?;
        validate_stable_id("page", &target.page_id)?;
        validate_stable_id("region", &target.region_id)?;
        let page = self
            .evidence_delta
            .new_pages
            .iter()
            .find(|page| page.id == target.page_id)
            .ok_or(FerrodocBridgeError::WrongPageQualifiedTarget)?;
        if !page
            .regions
            .iter()
            .any(|region| region.id == target.region_id)
        {
            return Err(FerrodocBridgeError::WrongPageQualifiedTarget);
        }
        Ok(())
    }
}

pub fn adapt_ferrodoc_document(
    input: &FerrodocConformanceInput,
) -> Result<Fdx2Document, FerrodocBridgeError> {
    let state_id = input.validate()?;
    let checkpoint = input
        .document_state_manifest
        .materialized_ir_checkpoint
        .as_ref()
        .map(|checkpoint| Sha256Digest(checkpoint.document_ir_logical_sha256.clone()));
    let mut pages = vec![];
    for page in &input.evidence_delta.new_pages {
        let mut output = Fdx2Page {
            page_id: page.id.clone(),
            title_blocks: vec![],
            headings: vec![],
            paragraphs: vec![],
            tables: vec![],
        };
        let mut heading_path = vec![];
        let mut last_table = None;
        for region in &page.regions {
            match region.kind.as_str() {
                "heading" => {
                    let evidence = selected_text(region)?;
                    let mapped = map_text(page, region, evidence)?;
                    if infer_regime(&evidence_text(evidence)?).is_some() {
                        heading_path = vec![evidence_text(evidence)?];
                        output.headings.push(mapped);
                    } else if output.title_blocks.is_empty() {
                        output.title_blocks.push(mapped);
                    } else {
                        output.headings.push(mapped);
                    }
                    last_table = None;
                }
                "paragraph" => {
                    output
                        .paragraphs
                        .push(map_text(page, region, selected_text(region)?)?);
                    last_table = None;
                }
                "table" => {
                    let evidence = selected_table(region)?;
                    let FerrodocEvidenceContent::Table {
                        rows,
                        columns,
                        cells,
                    } = &evidence.content
                    else {
                        unreachable!("selected_table checked content");
                    };
                    let mapped_cells = cells
                        .iter()
                        .map(|cell| map_cell(page, evidence, cell))
                        .collect::<Result<Vec<_>, _>>()?;
                    output.tables.push(Fdx2Table {
                        region_id: region.id.clone(),
                        heading_path: heading_path.clone(),
                        rows: *rows,
                        columns: *columns,
                        header_rows: 1,
                        cells: mapped_cells,
                        footnotes: vec![],
                        reconstruction_quality: evidence.geometry_quality.into(),
                    });
                    last_table = Some(output.tables.len() - 1);
                }
                "footnote" => {
                    let evidence = selected_text(region)?;
                    if let Some(table_index) = last_table {
                        let text = evidence_text(evidence)?;
                        let marker = leading_marker(&text).ok_or_else(|| {
                            FerrodocBridgeError::InvalidDelta("footnote has no marker".into())
                        })?;
                        output.tables[table_index].footnotes.push(Fdx2Footnote {
                            marker,
                            evidence: map_text(page, region, evidence)?,
                        });
                    } else {
                        output.paragraphs.push(map_text(page, region, evidence)?);
                    }
                }
                _ => {}
            }
        }
        pages.push(output);
    }
    Ok(Fdx2Document {
        schema_version: format!(
            "ferrodoc-document-ir/{}.{}",
            input.evidence_delta.ir_schema.major, input.evidence_delta.ir_schema.minor
        ),
        source_pdf_sha256: Sha256Digest(input.evidence_delta.source_pdf_sha256.clone()),
        document_state_id: state_id,
        document_ir_logical_sha256: checkpoint,
        pages,
    })
}

pub fn compile_ferrodoc_analog_power_baseline(
    input: &FerrodocConformanceInput,
    producer: ProducerIdentity,
    vocabulary: &PredicateVocabularySnapshot,
    category_prior: Option<&str>,
) -> Result<DeterministicBaseline, FerrodocBridgeError> {
    let document = adapt_ferrodoc_document(input)?;
    compile_analog_power_baseline(&document, producer, vocabulary, category_prior)
        .map_err(FerrodocBridgeError::Baseline)
}

fn validate_pages(delta: &FerrodocEvidenceDelta) -> Result<(), FerrodocBridgeError> {
    let mut page_ids = BTreeSet::new();
    for page in &delta.new_pages {
        validate_stable_id("page", &page.id)?;
        if !page_ids.insert(&page.id) || page.bounds.page_index != page.index {
            return Err(FerrodocBridgeError::InvalidDelta(
                "duplicate/invalid page".into(),
            ));
        }
        let layer_ids: BTreeSet<_> = page.layers.iter().map(|layer| layer.id.as_str()).collect();
        for layer in &page.layers {
            validate_stable_id("layer", &layer.id)?;
        }
        let page_evidence: BTreeMap<_, _> = page
            .regions
            .iter()
            .flat_map(|region| &region.evidence)
            .map(|evidence| (evidence.id.as_str(), evidence))
            .collect();
        let page_evidence_count: usize = page
            .regions
            .iter()
            .map(|region| region.evidence.len())
            .sum();
        if page_evidence.len() != page_evidence_count {
            return Err(FerrodocBridgeError::InvalidDelta(
                "duplicate page evidence".into(),
            ));
        }
        let mut region_ids = BTreeSet::new();
        for region in &page.regions {
            validate_stable_id("region", &region.id)?;
            if !region_ids.insert(&region.id) || region.geometry.page_index != page.index {
                return Err(FerrodocBridgeError::InvalidDelta(
                    "duplicate/invalid region".into(),
                ));
            }
            let evidence_ids: BTreeSet<_> = region
                .evidence
                .iter()
                .map(|item| item.id.as_str())
                .collect();
            if evidence_ids.len() != region.evidence.len() {
                return Err(FerrodocBridgeError::InvalidDelta(
                    "duplicate evidence".into(),
                ));
            }
            for evidence in &region.evidence {
                validate_stable_id("evidence", &evidence.id)?;
                if !layer_ids.contains(evidence.layer_id.as_str())
                    || evidence
                        .geometry
                        .as_ref()
                        .is_some_and(|geometry| geometry.page_index != page.index)
                    || evidence.provenance.input_digest != delta.source_pdf_sha256
                {
                    return Err(FerrodocBridgeError::InvalidDelta(
                        "invalid evidence owner".into(),
                    ));
                }
                if let FerrodocEvidenceContent::Table { cells, .. } = &evidence.content {
                    for cell in cells {
                        if cell.source_spans.is_empty()
                            || cell
                                .source_spans
                                .iter()
                                .any(|span| !page_evidence.contains_key(span.evidence_id.as_str()))
                        {
                            return Err(FerrodocBridgeError::InvalidDelta(
                                "unresolved cell source span".into(),
                            ));
                        }
                        let mut reconstructed = String::new();
                        for span in &cell.source_spans {
                            validate_stable_id("evidence", &span.evidence_id)?;
                            if span.start > span.end {
                                return Err(FerrodocBridgeError::InvalidDelta(
                                    "invalid cell source span".into(),
                                ));
                            }
                            let source = page_evidence
                                .get(span.evidence_id.as_str())
                                .expect("checked source evidence");
                            let FerrodocEvidenceContent::Text { text } = &source.content else {
                                return Err(FerrodocBridgeError::InvalidDelta(
                                    "cell source span is not text".into(),
                                ));
                            };
                            let selected = text
                                .get(span.start as usize..span.end as usize)
                                .ok_or_else(|| {
                                    FerrodocBridgeError::InvalidDelta(
                                        "cell source span is not a UTF-8 range".into(),
                                    )
                                })?;
                            reconstructed.push_str(selected);
                        }
                        if reconstructed != cell.text {
                            return Err(FerrodocBridgeError::InvalidDelta(
                                "cell text differs from source spans".into(),
                            ));
                        }
                    }
                }
            }
            if let Some(selected) = &region.selected {
                if selected.evidence_ids.is_empty()
                    || selected
                        .evidence_ids
                        .iter()
                        .any(|id| !evidence_ids.contains(id.as_str()))
                {
                    return Err(FerrodocBridgeError::InvalidDelta(
                        "invalid selected view".into(),
                    ));
                }
            }
        }
    }
    Ok(())
}

fn selected_text(region: &FerrodocRegion) -> Result<&FerrodocEvidence, FerrodocBridgeError> {
    selected_evidence(region, |content| {
        matches!(content, FerrodocEvidenceContent::Text { .. })
    })
}

fn selected_table(region: &FerrodocRegion) -> Result<&FerrodocEvidence, FerrodocBridgeError> {
    selected_evidence(region, |content| {
        matches!(content, FerrodocEvidenceContent::Table { .. })
    })
}

fn selected_evidence(
    region: &FerrodocRegion,
    accepts: impl Fn(&FerrodocEvidenceContent) -> bool,
) -> Result<&FerrodocEvidence, FerrodocBridgeError> {
    let selected = region
        .selected
        .as_ref()
        .ok_or(FerrodocBridgeError::NoSelectedEvidence)?;
    selected
        .evidence_ids
        .iter()
        .find_map(|id| {
            region
                .evidence
                .iter()
                .find(|evidence| &evidence.id == id && accepts(&evidence.content))
        })
        .ok_or(FerrodocBridgeError::NoSelectedEvidence)
}

fn evidence_text(evidence: &FerrodocEvidence) -> Result<String, FerrodocBridgeError> {
    match &evidence.content {
        FerrodocEvidenceContent::Text { text } => Ok(text.clone()),
        _ => Err(FerrodocBridgeError::NoSelectedEvidence),
    }
}

fn map_text(
    page: &FerrodocPage,
    region: &FerrodocRegion,
    evidence: &FerrodocEvidence,
) -> Result<Fdx2TextEvidence, FerrodocBridgeError> {
    Ok(Fdx2TextEvidence {
        evidence_id: evidence.id.clone(),
        region_id: region.id.clone(),
        text: evidence_text(evidence)?,
        geometry: normalized_geometry(page, evidence.geometry.as_ref())?,
        geometry_quality: evidence.geometry_quality.into(),
    })
}

fn map_cell(
    page: &FerrodocPage,
    table_evidence: &FerrodocEvidence,
    cell: &FerrodocTableCell,
) -> Result<Fdx2Cell, FerrodocBridgeError> {
    let span = cell.source_spans.first().ok_or_else(|| {
        FerrodocBridgeError::InvalidDelta("table cell lacks source evidence".into())
    })?;
    Ok(Fdx2Cell {
        evidence_id: span.evidence_id.clone(),
        row: cell.row,
        column: cell.column,
        row_span: cell.row_span,
        column_span: cell.column_span,
        text: cell.text.clone(),
        markers: bracket_markers(&cell.text),
        geometry: normalized_geometry(
            page,
            cell.geometry.as_ref().or(table_evidence.geometry.as_ref()),
        )?,
        geometry_quality: cell.geometry_quality.into(),
    })
}

fn normalized_geometry(
    page: &FerrodocPage,
    geometry: Option<&FerrodocPageRect>,
) -> Result<Vec<SourceGeometry>, FerrodocBridgeError> {
    let Some(geometry) = geometry else {
        return Ok(vec![]);
    };
    let width = page.bounds.rect.width;
    let height = page.bounds.rect.height;
    if width <= 0.0 || height <= 0.0 || geometry.page_index != page.index {
        return Err(FerrodocBridgeError::InvalidGeometry);
    }
    let rect = &geometry.rect;
    let normalized = SourceGeometry {
        left: rect.x / width,
        top: rect.y / height,
        right: (rect.x + rect.width) / width,
        bottom: (rect.y + rect.height) / height,
    };
    normalized
        .validate()
        .map_err(|_| FerrodocBridgeError::InvalidGeometry)?;
    Ok(vec![normalized])
}

fn bracket_markers(text: &str) -> Vec<String> {
    let mut markers = vec![];
    let mut rest = text;
    while let Some(start) = rest.find('[') {
        let Some(end) = rest[start + 1..].find(']') else {
            break;
        };
        let marker = &rest[start + 1..start + 1 + end];
        if !marker.is_empty()
            && marker
                .chars()
                .all(|character| character.is_ascii_alphanumeric())
        {
            markers.push(marker.into());
        }
        rest = &rest[start + end + 2..];
    }
    markers.sort();
    markers.dedup();
    markers
}

fn leading_marker(text: &str) -> Option<String> {
    let rest = text.trim().strip_prefix('[')?;
    let (marker, _) = rest.split_once(']')?;
    (!marker.is_empty()).then(|| marker.to_string())
}

fn validate_digest(value: &str) -> Result<(), FerrodocBridgeError> {
    Sha256Digest(value.into())
        .validate()
        .map_err(|_| FerrodocBridgeError::InvalidDigest)
}

fn validate_stable_id(prefix: &str, value: &str) -> Result<(), FerrodocBridgeError> {
    let digest = value
        .strip_prefix(prefix)
        .and_then(|rest| rest.strip_prefix('_'))
        .ok_or(FerrodocBridgeError::InvalidStableId)?;
    validate_digest(digest).map_err(|_| FerrodocBridgeError::InvalidStableId)
}

fn derive_ferrodoc_id(prefix: &str, parts: &[&[u8]]) -> String {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(prefix.as_bytes());
    bytes.push(0);
    for part in parts {
        bytes.extend_from_slice(&(part.len() as u64).to_be_bytes());
        bytes.extend_from_slice(part);
    }
    let digest = Sha256::digest(bytes);
    format!("{prefix}_{digest:x}")
}

#[derive(Debug)]
pub enum FerrodocBridgeError {
    Baseline(BaselineError),
    ContractLockMismatch,
    InvalidDelta(String),
    InvalidDigest,
    InvalidGeometry,
    InvalidStableId,
    Json(serde_json::Error),
    NoSelectedEvidence,
    StaleTarget,
    UnsupportedSchema,
    UnresolvedInputStatePrecondition,
    WrongPageQualifiedTarget,
    WrongState,
}

impl std::fmt::Display for FerrodocBridgeError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for FerrodocBridgeError {}

impl From<serde_json::Error> for FerrodocBridgeError {
    fn from(error: serde_json::Error) -> Self {
        Self::Json(error)
    }
}
