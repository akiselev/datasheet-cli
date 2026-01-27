// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>

use serde_json::{Value, json};

pub struct PromptSpec {
    pub name: &'static str,
    pub description: &'static str,
    pub prompt: &'static str,
    pub schema: Value,
}

impl PromptSpec {
    pub fn new(name: &'static str, description: &'static str, prompt: &'static str) -> Self {
        Self {
            name,
            description,
            prompt,
            schema: json!({
                "type": "object",
                "additionalProperties": true,
            }),
        }
    }
}

const PROMPT_BOOT_CONFIG: &str = include_str!("../prompts/extract-boot-config.md");
const PROMPT_CHARACTERISTICS: &str = include_str!("../prompts/extract-characteristics.md");
const PROMPT_CUSTOM: &str = include_str!("../prompts/extract-custom.md");
const PROMPT_DRC_RULES: &str = include_str!("../prompts/extract-drc-rules.md");
const PROMPT_FEATURE_MATRIX: &str = include_str!("../prompts/extract-feature-matrix.md");
const PROMPT_FOOTPRINT: &str = include_str!("../prompts/extract-footprint.md");
const PROMPT_HIGH_SPEED: &str = include_str!("../prompts/extract-high-speed.md");
const PROMPT_LAYOUT_CONSTRAINTS: &str = include_str!("../prompts/extract-layout-constraints.md");
const PROMPT_PINOUT: &str = include_str!("../prompts/extract-pinout.md");
const PROMPT_POWER: &str = include_str!("../prompts/extract-power.md");
const PROMPT_REFERENCE_DESIGN: &str = include_str!("../prompts/extract-reference-design.md");

pub fn boot_config() -> PromptSpec {
    let mut spec = PromptSpec::new(
        "boot-config",
        "Boot configuration requirements",
        PROMPT_BOOT_CONFIG,
    );
    spec.schema = json!({
        "type": "object",
        "properties": {
            "boot_configuration": {
                "type": "array",
                "items": {"type": "object"}
            },
            "debug_interface": {"type": "object"}
        },
        "additionalProperties": true
    });
    spec
}

pub fn characteristics() -> PromptSpec {
    let mut spec = PromptSpec::new(
        "characteristics",
        "Electrical/thermal characteristics",
        PROMPT_CHARACTERISTICS,
    );
    spec.schema = json!({
        "type": "object",
        "properties": {
            "absolute_maximum_ratings": {
                "type": "array",
                "items": {"type": "object"}
            },
            "recommended_operating_conditions": {
                "type": "array",
                "items": {"type": "object"}
            },
            "electrical_specifications": {
                "type": "array",
                "items": {"type": "object"}
            },
            "thermal_data": {
                "type": "array",
                "items": {"type": "object"}
            }
        },
        "additionalProperties": true
    });
    spec
}

pub fn drc_rules() -> PromptSpec {
    let mut spec = PromptSpec::new("drc-rules", "PCB design rule constraints", PROMPT_DRC_RULES);
    spec.schema = json!({
        "type": "object",
        "properties": {
            "design_rules": {
                "type": "array",
                "items": {"type": "object"}
            }
        },
        "required": ["design_rules"],
        "additionalProperties": true
    });
    spec
}

pub fn feature_matrix() -> PromptSpec {
    let mut spec = PromptSpec::new(
        "feature-matrix",
        "Feature matrix and part decoding",
        PROMPT_FEATURE_MATRIX,
    );
    spec.schema = json!({
        "type": "object",
        "properties": {
            "part_number_decoding": {"type": "object", "additionalProperties": true},
            "variants": {
                "type": "array",
                "items": {"type": "object"}
            },
            "interface_support_summary": {"type": "object", "additionalProperties": true}
        },
        "required": ["variants"],
        "additionalProperties": true
    });
    spec
}

pub fn footprint() -> PromptSpec {
    let mut spec = PromptSpec::new("footprint", "PCB footprint extraction", PROMPT_FOOTPRINT);
    spec.schema = json!({
        "type": "object",
        "properties": {
            "part_details": {"type": "object", "additionalProperties": true},
            "packages": {
                "type": "array",
                "items": {"type": "object"}
            }
        },
        "required": ["packages"],
        "additionalProperties": true
    });
    spec
}

pub fn high_speed() -> PromptSpec {
    let mut spec = PromptSpec::new(
        "high-speed",
        "High-speed interface routing constraints",
        PROMPT_HIGH_SPEED,
    );
    spec.schema = json!({
        "type": "object",
        "properties": {
            "interfaces": {
                "type": "array",
                "items": {"type": "object"}
            }
        },
        "required": ["interfaces"],
        "additionalProperties": true
    });
    spec
}

pub fn layout_constraints() -> PromptSpec {
    let mut spec = PromptSpec::new(
        "layout-constraints",
        "PCB layout constraints",
        PROMPT_LAYOUT_CONSTRAINTS,
    );
    spec.schema = json!({
        "type": "object",
        "properties": {
            "placement_rules": {
                "type": "array",
                "items": {"type": "object"}
            },
            "routing_constraints": {
                "type": "array",
                "items": {"type": "object"}
            },
            "layer_stackup_notes": {
                "type": "array",
                "items": {"type": "string"}
            }
        },
        "additionalProperties": true
    });
    spec
}

pub fn pinout() -> PromptSpec {
    let mut spec = PromptSpec::new("pinout", "Pinout and configuration", PROMPT_PINOUT);
    // Keep schema shallow to avoid Gemini API nesting depth limits
    spec.schema = json!({
        "type": "object",
        "properties": {
            "part_details": {
                "type": "object",
                "additionalProperties": true
            },
            "packages": {
                "type": "array",
                "items": {
                    "type": "object",
                    "properties": {
                        "package_name": {"type": "string"},
                        "pins": {
                            "type": "array",
                            "items": {"type": "object"}
                        }
                    },
                    "additionalProperties": true
                }
            }
        },
        "required": ["packages"]
    });
    spec
}

pub fn power() -> PromptSpec {
    let mut spec = PromptSpec::new("power", "Power requirements", PROMPT_POWER);
    spec.schema = json!({
        "type": "object",
        "properties": {
            "power_rails": {
                "type": "array",
                "items": {"type": "object"}
            },
            "sequencing_rules": {
                "type": "array",
                "items": {"type": "object"}
            }
        },
        "required": ["power_rails"],
        "additionalProperties": true
    });
    spec
}

pub fn reference_design() -> PromptSpec {
    let mut spec = PromptSpec::new(
        "reference-design",
        "Reference design extraction",
        PROMPT_REFERENCE_DESIGN,
    );
    spec.schema = json!({
        "type": "object",
        "properties": {
            "required_components": {
                "type": "array",
                "items": {"type": "object"}
            },
            "critical_schematic_notes": {
                "type": "array",
                "items": {"type": "string"}
            }
        },
        "required": ["required_components"],
        "additionalProperties": true
    });
    spec
}

pub fn custom() -> PromptSpec {
    let spec = PromptSpec::new(
        "custom",
        "Custom extraction with user-provided prompt",
        PROMPT_CUSTOM,
    );
    // Schema will be overridden by user if provided, otherwise uses default from PromptSpec::new
    spec
}
