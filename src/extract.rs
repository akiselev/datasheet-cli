// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>

use crate::file_cache::FileCache;
use crate::llm::{AttachmentSource, FileReference, LlmProvider, LlmRequest, build_client, resolve_api_key};
use crate::prompts;
use anyhow::{Context, Result, anyhow};
use clap::{Args, ValueEnum};
use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};

const __DEFAULT__: &str = "__DEFAULT__";

#[derive(Args, Debug)]
pub struct ExtractArgs {
    /// Task to run
    #[arg(value_enum)]
    pub task: ExtractTask,

    /// Input PDF path
    pub pdf: PathBuf,

    /// LLM provider (always Gemini)
    #[arg(long, default_value = "gemini", hide = true)]
    pub provider: LlmProvider,

    /// Model name (default: task-specific model, see each task's default_model())
    /// Examples: gemini-2.0-flash-exp, gemini-2.5-flash, gemini-1.5-pro
    #[arg(long, default_value = __DEFAULT__)]
    pub model: String,

    /// API key (falls back to GOOGLE_API_KEY or GEMINI_API_KEY)
    #[arg(long)]
    pub api_key: Option<String>,

    /// Optional base URL override for Gemini API
    #[arg(long)]
    pub base_url: Option<String>,

    /// Output file (defaults to stdout)
    #[arg(long)]
    pub out: Option<PathBuf>,

    /// Sampling temperature (currently not plumbed for OpenRouter in this implementation)
    #[arg(long)]
    pub temperature: Option<f32>,

    /// Show formatted (pretty-printed) JSON output
    #[arg(long, short = 'f', visible_alias = "pretty")]
    pub formatted: bool,

    /// Custom prompt text or path to prompt file (only for 'custom' task)
    /// If the value is a valid file path, the file contents will be used as the prompt
    #[arg(long)]
    pub prompt: Option<String>,

    /// Custom JSON schema file path or inline JSON (only for 'custom' task)
    /// Must be a valid JSON Schema object
    #[arg(long)]
    pub schema: Option<String>,

    /// Disable file caching (re-upload PDF every request)
    /// By default, PDFs are uploaded once to Gemini's File API and cached for 48 hours
    #[arg(long)]
    pub no_cache: bool,
}

#[derive(ValueEnum, Debug, Clone, Copy)]
pub enum ExtractTask {
    BootConfig,
    Characteristics,
    Custom,
    DrcRules,
    FeatureMatrix,
    Footprint,
    HighSpeed,
    LayoutConstraints,
    Pinout,
    Power,
    ReferenceDesign,
}

impl ExtractTask {
    pub fn prompt(self) -> prompts::PromptSpec {
        match self {
            ExtractTask::BootConfig => prompts::boot_config(),
            ExtractTask::Characteristics => prompts::characteristics(),
            ExtractTask::Custom => prompts::custom(),
            ExtractTask::DrcRules => prompts::drc_rules(),
            ExtractTask::FeatureMatrix => prompts::feature_matrix(),
            ExtractTask::Footprint => prompts::footprint(),
            ExtractTask::HighSpeed => prompts::high_speed(),
            ExtractTask::LayoutConstraints => prompts::layout_constraints(),
            ExtractTask::Pinout => prompts::pinout(),
            ExtractTask::Power => prompts::power(),
            ExtractTask::ReferenceDesign => prompts::reference_design(),
        }
    }

    pub fn default_model(self) -> &'static str {
        "gemini-3-pro-preview"
    }
}

pub fn run_extract(args: &ExtractArgs) -> Result<()> {
    if !args.pdf.exists() {
        return Err(anyhow!("PDF not found: {}", args.pdf.display()));
    }

    // Validate that --prompt and --schema are only used with Custom task
    if !matches!(args.task, ExtractTask::Custom) {
        if args.prompt.is_some() {
            return Err(anyhow!(
                "--prompt can only be used with 'custom' task. Use 'datasheet extract custom <PDF> --prompt \"...\"'"
            ));
        }
        if args.schema.is_some() {
            return Err(anyhow!(
                "--schema can only be used with 'custom' task. Use 'datasheet extract custom <PDF> --schema \"...\"'"
            ));
        }
    }

    let mut prompt_spec = args.task.prompt();
    let task_label = format!("{} ({})", prompt_spec.name, prompt_spec.description);

    // For custom task, allow overriding prompt and schema
    let prompt_text: String;
    if matches!(args.task, ExtractTask::Custom) {
        // Load custom prompt if provided (from file or inline)
        if let Some(custom_prompt) = &args.prompt {
            prompt_text = load_text_or_file(custom_prompt)
                .context("loading custom prompt")?;
        } else {
            prompt_text = prompt_spec.prompt.to_string();
        }

        // Load custom schema if provided (from file or inline JSON)
        if let Some(custom_schema) = &args.schema {
            let schema_text = load_text_or_file(custom_schema)
                .context("loading custom schema")?;
            prompt_spec.schema = serde_json::from_str(&schema_text)
                .context("parsing custom schema as JSON")?;
        }
    } else {
        prompt_text = prompt_spec.prompt.to_string();
    }

    let api_key = resolve_api_key(args.provider, args.api_key.clone())?;
    let client = build_client(args.provider, api_key.clone(), args.base_url.clone())?;

    // Get attachment source - use file cache unless disabled
    let attachment = if args.no_cache {
        // Read file directly and send inline
        let data = fs::read(&args.pdf)
            .with_context(|| format!("reading {}", args.pdf.display()))?;
        AttachmentSource::Inline(crate::llm::Attachment {
            mime_type: "application/pdf".to_string(),
            data,
        })
    } else {
        // Use file cache to upload/retrieve the file
        let mut cache = FileCache::new(api_key, args.base_url.clone())
            .context("initializing file cache")?;
        let cached = cache.get_or_upload(&args.pdf)
            .context("getting or uploading file to Gemini")?;
        AttachmentSource::FileUri(FileReference {
            mime_type: "application/pdf".to_string(),
            file_uri: cached.uri,
        })
    };

    // Use task-specific default if user didn't specify a model
    let model = if args.model == __DEFAULT__ {
        args.task.default_model().to_string()
    } else {
        args.model.clone()
    };

    let response = client.generate_json(LlmRequest {
        model,
        prompt: prompt_text,
        schema: prompt_spec.schema,
        attachment,
        temperature: args.temperature,
    })?;

    write_output(&response.json, args.out.as_deref(), args.formatted)
        .with_context(|| format!("writing {task_label} output for {}", args.pdf.display()))?;

    Ok(())
}

/// Load text from a string or file path.
/// If the input looks like a valid file path and the file exists, read from the file.
/// Otherwise, treat the input as inline text.
fn load_text_or_file(input: &str) -> Result<String> {
    let path = Path::new(input);
    if path.exists() && path.is_file() {
        fs::read_to_string(path)
            .with_context(|| format!("reading file: {}", path.display()))
    } else {
        Ok(input.to_string())
    }
}

fn write_output(value: &Value, out: Option<&Path>, formatted: bool) -> Result<()> {
    let rendered = if formatted {
        serde_json::to_string_pretty(value)?
    } else {
        serde_json::to_string(value)?
    };

    if let Some(path) = out {
        fs::write(path, rendered)?;
    } else {
        println!("{rendered}");
    }
    Ok(())
}
