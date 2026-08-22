// SPDX-License-Identifier: MIT OR Apache-2.0
//! Deterministic exact-text quantity parsing backed by a pinned Quantitas registry.

use datasheet_core::{NormalizedQuantity, ParsedQuantityShape};
use quantitas::{
    Dimension, ExactScale, QuantityKindId, RegistrySnapshot, UnitDef, UnitId, UnitProvenance,
    UnitRegistry,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

pub const FDX2_QUANTITAS_REVISION: &str = "734d78cd6ff516afee54201bc70cd59fd34e67e3";
pub const FDX2_REGISTRY_ID: &str = "datasheet-fdx2-analog-power-units/1";

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum QuantityMeaning {
    Scalar {
        value_si: String,
    },
    Range {
        minimum_si: String,
        maximum_si: String,
    },
    Inequality {
        operator: InequalityOperator,
        value_si: String,
    },
    Tolerance {
        magnitude_si: String,
    },
    Symbolic {
        expression: String,
    },
    Unparsed,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum InequalityOperator {
    LessThanOrEqual,
    GreaterThanOrEqual,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, JsonSchema)]
pub struct ParsedQuantity {
    pub raw_text: String,
    pub parsed_shape: ParsedQuantityShape,
    pub meaning: QuantityMeaning,
    pub raw_unit: Option<String>,
    pub canonical_unit: Option<String>,
    pub quantity_kind: Option<String>,
    pub dimensionality: Option<String>,
    pub normalized_quantity: Option<NormalizedQuantity>,
    pub warnings: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum QuantityParseError {
    ConflictingUnits { left: String, right: String },
    InvalidRegistry(String),
}

impl std::fmt::Display for QuantityParseError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for QuantityParseError {}

#[derive(Clone, Debug)]
pub struct QuantityParser {
    registry: UnitRegistry,
}

impl QuantityParser {
    pub fn fdx2() -> Result<Self, QuantityParseError> {
        Ok(Self {
            registry: fdx2_registry()?,
        })
    }

    #[must_use]
    pub fn registry_identity(&self) -> &'static str {
        FDX2_REGISTRY_ID
    }

    pub fn parse(
        &self,
        raw_text: &str,
        inherited_unit: Option<&str>,
    ) -> Result<ParsedQuantity, QuantityParseError> {
        let trimmed = raw_text.trim();
        if trimmed.is_empty() || matches!(trimmed, "-" | "—" | "–" | "N/A" | "NA") {
            return Ok(unparsed(raw_text, "missing_or_unavailable"));
        }
        if is_symbolic_relative(trimmed) {
            return Ok(ParsedQuantity {
                raw_text: raw_text.into(),
                parsed_shape: ParsedQuantityShape::Symbolic,
                meaning: QuantityMeaning::Symbolic {
                    expression: trimmed.into(),
                },
                raw_unit: trailing_unit(trimmed),
                canonical_unit: None,
                quantity_kind: None,
                dimensionality: None,
                normalized_quantity: None,
                warnings: vec!["symbolic_relative_not_scalarized".into()],
            });
        }

        if let Some(rest) = trimmed.strip_prefix('±') {
            return self.parse_tolerance(raw_text, rest.trim(), inherited_unit);
        }
        if let Some(rest) = trimmed
            .strip_prefix("<=")
            .or_else(|| trimmed.strip_prefix('≤'))
        {
            return self.parse_inequality(
                raw_text,
                rest.trim(),
                inherited_unit,
                InequalityOperator::LessThanOrEqual,
            );
        }
        if let Some(rest) = trimmed
            .strip_prefix(">=")
            .or_else(|| trimmed.strip_prefix('≥'))
        {
            return self.parse_inequality(
                raw_text,
                rest.trim(),
                inherited_unit,
                InequalityOperator::GreaterThanOrEqual,
            );
        }
        if let Some(rest) = trimmed.strip_suffix(" max") {
            return self.parse_inequality(
                raw_text,
                rest.trim(),
                inherited_unit,
                InequalityOperator::LessThanOrEqual,
            );
        }

        let normalized_range = trimmed.replace(['–', '—'], " to ");
        if let Some((left, right)) = split_range(&normalized_range) {
            return self.parse_range(raw_text, left, right, inherited_unit);
        }

        match self.parse_scalar(trimmed, inherited_unit) {
            Ok(scalar) => Ok(ParsedQuantity {
                raw_text: raw_text.into(),
                parsed_shape: ParsedQuantityShape::Scalar,
                meaning: QuantityMeaning::Scalar {
                    value_si: scalar.value_si.clone(),
                },
                raw_unit: Some(scalar.raw_unit.clone()),
                canonical_unit: Some(scalar.canonical_unit.clone()),
                quantity_kind: Some(scalar.quantity_kind.clone()),
                dimensionality: Some(scalar.dimensionality.clone()),
                normalized_quantity: Some(NormalizedQuantity {
                    canonical_value: scalar.value_si,
                    canonical_unit: scalar.canonical_unit,
                    registry_identity: FDX2_REGISTRY_ID.into(),
                }),
                warnings: vec![],
            }),
            Err(_) => Ok(unparsed(raw_text, "unsupported_quantity_syntax")),
        }
    }

    fn parse_range(
        &self,
        raw_text: &str,
        left: &str,
        right: &str,
        inherited_unit: Option<&str>,
    ) -> Result<ParsedQuantity, QuantityParseError> {
        let left_unit = trailing_unit(left);
        let right_unit = trailing_unit(right);
        let common_unit = right_unit
            .as_deref()
            .or(left_unit.as_deref())
            .or(inherited_unit);
        let left = self
            .parse_scalar(left, common_unit)
            .map_err(QuantityParseError::InvalidRegistry)?;
        let right = self
            .parse_scalar(right, common_unit)
            .map_err(QuantityParseError::InvalidRegistry)?;
        if left.quantity_kind != right.quantity_kind {
            return Err(QuantityParseError::ConflictingUnits {
                left: left.raw_unit,
                right: right.raw_unit,
            });
        }
        let canonical_value = format!("{}..{}", left.value_si, right.value_si);
        Ok(ParsedQuantity {
            raw_text: raw_text.into(),
            parsed_shape: ParsedQuantityShape::Range,
            meaning: QuantityMeaning::Range {
                minimum_si: left.value_si,
                maximum_si: right.value_si,
            },
            raw_unit: Some(right.raw_unit),
            canonical_unit: Some(right.canonical_unit.clone()),
            quantity_kind: Some(right.quantity_kind),
            dimensionality: Some(right.dimensionality),
            normalized_quantity: Some(NormalizedQuantity {
                canonical_value,
                canonical_unit: right.canonical_unit,
                registry_identity: FDX2_REGISTRY_ID.into(),
            }),
            warnings: vec![],
        })
    }

    fn parse_inequality(
        &self,
        raw_text: &str,
        body: &str,
        inherited_unit: Option<&str>,
        operator: InequalityOperator,
    ) -> Result<ParsedQuantity, QuantityParseError> {
        let scalar = self
            .parse_scalar(body, inherited_unit)
            .map_err(QuantityParseError::InvalidRegistry)?;
        Ok(ParsedQuantity {
            raw_text: raw_text.into(),
            parsed_shape: ParsedQuantityShape::Inequality,
            meaning: QuantityMeaning::Inequality {
                operator,
                value_si: scalar.value_si.clone(),
            },
            raw_unit: Some(scalar.raw_unit),
            canonical_unit: Some(scalar.canonical_unit.clone()),
            quantity_kind: Some(scalar.quantity_kind),
            dimensionality: Some(scalar.dimensionality),
            normalized_quantity: Some(NormalizedQuantity {
                canonical_value: scalar.value_si,
                canonical_unit: scalar.canonical_unit,
                registry_identity: FDX2_REGISTRY_ID.into(),
            }),
            warnings: vec![],
        })
    }

    fn parse_tolerance(
        &self,
        raw_text: &str,
        body: &str,
        inherited_unit: Option<&str>,
    ) -> Result<ParsedQuantity, QuantityParseError> {
        let scalar = self
            .parse_scalar(body, inherited_unit)
            .map_err(QuantityParseError::InvalidRegistry)?;
        Ok(ParsedQuantity {
            raw_text: raw_text.into(),
            parsed_shape: ParsedQuantityShape::Tolerance,
            meaning: QuantityMeaning::Tolerance {
                magnitude_si: scalar.value_si.clone(),
            },
            raw_unit: Some(scalar.raw_unit),
            canonical_unit: Some(scalar.canonical_unit.clone()),
            quantity_kind: Some(scalar.quantity_kind),
            dimensionality: Some(scalar.dimensionality),
            normalized_quantity: Some(NormalizedQuantity {
                canonical_value: scalar.value_si,
                canonical_unit: scalar.canonical_unit,
                registry_identity: FDX2_REGISTRY_ID.into(),
            }),
            warnings: vec![],
        })
    }

    fn parse_scalar(&self, text: &str, inherited_unit: Option<&str>) -> Result<Scalar, String> {
        let (number, explicit_unit) = split_number_and_unit(text)?;
        let unit = if explicit_unit.is_empty() {
            inherited_unit.ok_or_else(|| "quantity has no unit".to_string())?
        } else {
            explicit_unit
        };
        let kind = unit_kind(unit).ok_or_else(|| format!("unsupported unit {unit}"))?;
        let parse_text = format!("{number} {unit}");
        let (literal, display) = self
            .registry
            .parse(&parse_text, QuantityKindId::new(kind.0))
            .map_err(|error| error.to_string())?;
        let quantity = self
            .registry
            .canonicalize(&literal)
            .map_err(|error| error.to_string())?;
        Ok(Scalar {
            value_si: canonical_number(quantity.value_si()),
            raw_unit: display.symbol,
            canonical_unit: kind.1.into(),
            quantity_kind: quantity.kind().as_str().into(),
            dimensionality: quantity.dimension().to_string(),
        })
    }
}

struct Scalar {
    value_si: String,
    raw_unit: String,
    canonical_unit: String,
    quantity_kind: String,
    dimensionality: String,
}

fn unparsed(raw_text: &str, warning: &str) -> ParsedQuantity {
    ParsedQuantity {
        raw_text: raw_text.into(),
        parsed_shape: ParsedQuantityShape::TextUnparsed,
        meaning: QuantityMeaning::Unparsed,
        raw_unit: trailing_unit(raw_text),
        canonical_unit: None,
        quantity_kind: None,
        dimensionality: None,
        normalized_quantity: None,
        warnings: vec![warning.into()],
    }
}

fn split_range(input: &str) -> Option<(&str, &str)> {
    input
        .split_once(" to ")
        .map(|(left, right)| (left.trim(), right.trim()))
}

fn split_number_and_unit(input: &str) -> Result<(&str, &str), String> {
    let input = input.trim();
    let mut end = 0;
    for (index, character) in input.char_indices() {
        if character.is_ascii_digit()
            || character == '.'
            || ((character == '-' || character == '+') && index == 0)
        {
            end = index + character.len_utf8();
        } else {
            break;
        }
    }
    if end == 0 {
        return Err("missing numeric magnitude".into());
    }
    input[..end]
        .parse::<f64>()
        .map_err(|_| "invalid numeric magnitude".to_string())?;
    Ok((&input[..end], input[end..].trim()))
}

fn trailing_unit(input: &str) -> Option<String> {
    split_number_and_unit(input)
        .ok()
        .and_then(|(_, unit)| (!unit.is_empty()).then(|| unit.to_string()))
}

fn is_symbolic_relative(input: &str) -> bool {
    let upper = input.to_ascii_uppercase();
    upper.contains("VDD")
        || upper.contains("VOUT")
        || upper.contains("VIN") && input.contains(['+', '-', '*'])
        || input.contains('*')
}

fn canonical_number(value: f64) -> String {
    if value == 0.0 {
        "0".into()
    } else {
        // Quantitas owns conversion; this stable decimal rendering removes only binary-float
        // noise from exact registry scales while retaining twelve fractional digits.
        let rendered = format!("{value:.12}");
        rendered
            .trim_end_matches('0')
            .trim_end_matches('.')
            .to_string()
    }
}

fn unit_kind(unit: &str) -> Option<(&'static str, &'static str)> {
    match unit {
        "V" => Some(("electronics:ElectricPotential", "V")),
        "A" | "nA" => Some(("si:ElectricCurrent", "A")),
        "Ω" | "kΩ" | "ohm" | "kohm" => Some(("electronics:Resistance", "ohm")),
        "Hz" | "MHz" => Some(("si:Frequency", "Hz")),
        "%" => Some(("datasheet:RelativeTolerance", "1")),
        "°C" | "degC" => Some(("si:ThermodynamicTemperature", "K")),
        "°C/W" | "degC/W" => Some(("electronics:ThermalResistance", "K/W")),
        _ => None,
    }
}

fn fdx2_registry() -> Result<UnitRegistry, QuantityParseError> {
    let voltage = Dimension::MASS
        .checked_product(Dimension::LENGTH.checked_powi(2).map_err(registry_error)?)
        .and_then(|value| value.checked_quotient(Dimension::TIME.checked_powi(3)?))
        .and_then(|value| value.checked_quotient(Dimension::CURRENT))
        .map_err(registry_error)?;
    let resistance = voltage
        .checked_quotient(Dimension::CURRENT)
        .map_err(registry_error)?;
    let frequency = Dimension::DIMENSIONLESS
        .checked_quotient(Dimension::TIME)
        .map_err(registry_error)?;
    let power = Dimension::MASS
        .checked_product(Dimension::LENGTH.checked_powi(2).map_err(registry_error)?)
        .and_then(|value| value.checked_quotient(Dimension::TIME.checked_powi(3)?))
        .map_err(registry_error)?;
    let thermal_resistance = Dimension::TEMPERATURE
        .checked_quotient(power)
        .map_err(registry_error)?;
    let provenance = UnitProvenance {
        authority: "Quantitas FDX2 minimized conformance registry".into(),
        version: FDX2_QUANTITAS_REVISION.into(),
        persistent_id: Some("https://github.com/akiselev/quantitas".into()),
    };
    let mut registry = UnitRegistry::empty();
    for (id, symbol, dimension, scale, offset, kinds, interval) in [
        unit(
            "volt",
            "V",
            voltage,
            ExactScale::ONE,
            None,
            "electronics:ElectricPotential",
        ),
        unit(
            "ampere",
            "A",
            Dimension::CURRENT,
            ExactScale::ONE,
            None,
            "si:ElectricCurrent",
        ),
        unit(
            "nanoampere",
            "nA",
            Dimension::CURRENT,
            ExactScale::new(1, 1, -9).map_err(registry_error)?,
            None,
            "si:ElectricCurrent",
        ),
        unit(
            "ohm",
            "Ω",
            resistance,
            ExactScale::ONE,
            None,
            "electronics:Resistance",
        ),
        unit(
            "kiloohm",
            "kΩ",
            resistance,
            ExactScale::new(1, 1, 3).map_err(registry_error)?,
            None,
            "electronics:Resistance",
        ),
        unit(
            "hertz",
            "Hz",
            frequency,
            ExactScale::ONE,
            None,
            "si:Frequency",
        ),
        unit(
            "megahertz",
            "MHz",
            frequency,
            ExactScale::new(1, 1, 6).map_err(registry_error)?,
            None,
            "si:Frequency",
        ),
        unit(
            "percent",
            "%",
            Dimension::DIMENSIONLESS,
            ExactScale::new(1, 1, -2).map_err(registry_error)?,
            None,
            "datasheet:RelativeTolerance",
        ),
        (
            "degree-celsius",
            "°C",
            Dimension::TEMPERATURE,
            ExactScale::ONE,
            Some(ExactScale::new(27_315, 100, 0).map_err(registry_error)?),
            vec![QuantityKindId::thermodynamic_temperature()],
            Some(UnitId::new("fdx2:degree-celsius-interval")),
        ),
        unit(
            "degree-celsius-interval",
            "delta_degC",
            Dimension::TEMPERATURE,
            ExactScale::ONE,
            None,
            "si:TemperatureDifference",
        ),
        unit(
            "thermal-resistance",
            "°C/W",
            thermal_resistance,
            ExactScale::ONE,
            None,
            "electronics:ThermalResistance",
        ),
    ] {
        registry
            .insert(UnitDef {
                id: UnitId::new(format!("fdx2:{id}")),
                symbol: symbol.into(),
                dimension,
                scale_to_si: scale,
                offset_to_si: offset,
                admitted_kinds: kinds,
                interval_form: interval,
                provenance: provenance.clone(),
            })
            .map_err(registry_error)?;
    }
    registry
        .freeze(RegistrySnapshot {
            source: "fixture://datasheet-fdx2/analog-power-units".into(),
            version: "1".into(),
            retrieval_date: "2026-08-21".into(),
            content_digest: FDX2_REGISTRY_ID.into(),
            generator_version: format!("datasheet-compiler/{}", env!("CARGO_PKG_VERSION")),
        })
        .map_err(registry_error)
}

type UnitTuple = (
    &'static str,
    &'static str,
    Dimension,
    ExactScale,
    Option<ExactScale>,
    Vec<QuantityKindId>,
    Option<UnitId>,
);

fn unit(
    id: &'static str,
    symbol: &'static str,
    dimension: Dimension,
    scale: ExactScale,
    offset: Option<ExactScale>,
    kind: &'static str,
) -> UnitTuple {
    (
        id,
        symbol,
        dimension,
        scale,
        offset,
        vec![QuantityKindId::new(kind)],
        None,
    )
}

fn registry_error(error: impl std::fmt::Display) -> QuantityParseError {
    QuantityParseError::InvalidRegistry(error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn required_forms_parse_without_losing_raw_text() {
        let parser = QuantityParser::fdx2().unwrap();
        for input in [
            "3.3 V",
            "60 nA",
            "-0.3 V",
            "1.8 to 5.5 V",
            "1.8–5.5 V",
            "<= 5.5 V",
            "5.5 V max",
            "±2 %",
            "25 °C",
            "-40 °C to 125 °C",
            "180 °C/W",
            "10 kΩ",
            "1 MHz",
        ] {
            let parsed = parser.parse(input, None).unwrap();
            assert_eq!(parsed.raw_text, input);
            assert_ne!(
                parsed.parsed_shape,
                ParsedQuantityShape::TextUnparsed,
                "{input}"
            );
            assert!(parsed.normalized_quantity.is_some(), "{input}");
        }
    }

    #[test]
    fn symbolic_relative_forms_are_never_scalarized() {
        let parser = QuantityParser::fdx2().unwrap();
        for input in ["VDD + 0.3 V", "VOUT - 0.5 V", "0.7 * VDD"] {
            let parsed = parser.parse(input, None).unwrap();
            assert_eq!(parsed.parsed_shape, ParsedQuantityShape::Symbolic);
            assert!(parsed.normalized_quantity.is_none());
        }
    }

    #[test]
    fn missing_tokens_never_become_zero() {
        let parser = QuantityParser::fdx2().unwrap();
        for input in ["", "-", "—", "N/A", "NA"] {
            let parsed = parser.parse(input, Some("V")).unwrap();
            assert_eq!(parsed.parsed_shape, ParsedQuantityShape::TextUnparsed);
            assert!(parsed.normalized_quantity.is_none());
        }
    }
}
