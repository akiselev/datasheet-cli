// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>

use anyhow::{anyhow, Context, Result};
use base64::engine::general_purpose::STANDARD;
use base64::Engine;
use clap::ValueEnum;
use serde_json::Value;
use std::env;
use std::path::Path;
use std::time::Duration;

#[derive(Clone, Copy, Debug, ValueEnum)]
pub enum LlmProvider {
    #[clap(name="gemini")]
    Gemini,
}

impl LlmProvider {
    pub fn default_base_url(self) -> &'static str {
        "https://generativelanguage.googleapis.com/v1beta"
    }

    pub fn api_key_env_vars(self) -> &'static [&'static str] {
        &["GOOGLE_API_KEY", "GEMINI_API_KEY"]
    }
}

#[derive(Clone, Debug)]
pub struct Attachment {
    pub mime_type: String,
    pub data: Vec<u8>,
}

impl Attachment {
    pub fn from_path(path: &Path) -> Result<Self> {
        let data = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
        Ok(Self {
            mime_type: "application/pdf".to_string(),
            data,
        })
    }
}

/// A reference to a file uploaded to Gemini via the File API
#[derive(Clone, Debug)]
pub struct FileReference {
    pub mime_type: String,
    pub file_uri: String,
}

/// How to send the attachment to Gemini
#[derive(Clone, Debug)]
pub enum AttachmentSource {
    /// Send inline as base64 (original behavior)
    Inline(Attachment),
    /// Reference a pre-uploaded file via URI
    FileUri(FileReference),
}

pub struct LlmRequest {
    pub model: String,
    pub prompt: String,
    pub schema: Value,
    pub attachment: AttachmentSource,
    pub temperature: Option<f32>,
}

pub struct LlmResponse {
    pub json: Value,
}

pub trait LlmClient {
    fn generate_json(&self, request: LlmRequest) -> Result<LlmResponse>;
}

/// Back-compat: extract.rs expects this name.
pub fn resolve_api_key(provider: LlmProvider, cli_key: Option<String>) -> Result<String> {
    if let Some(key) = cli_key {
        if !key.trim().is_empty() {
            return Ok(key);
        }
    }

    // Back-compat with your older code path.
    if let Ok(key) = env::var("DATASHEET_API_KEY") {
        if !key.trim().is_empty() {
            return Ok(key);
        }
    }

    for &var in provider.api_key_env_vars() {
        if let Ok(key) = env::var(var) {
            if !key.trim().is_empty() {
                return Ok(key);
            }
        }
    }

    Err(anyhow!(
        "missing API key (use --api-key or set one of: {})",
        provider.api_key_env_vars().join(", ")
    ))
}

pub fn build_client(
    _provider: LlmProvider,
    api_key: String,
    base_url: Option<String>,
) -> Result<Box<dyn LlmClient>> {
    Ok(Box::new(GeminiLlm::new(api_key, base_url)?))
}



struct GeminiLlm {
    api_key: String,
    base_url: String,
    client: reqwest::blocking::Client,
}

impl GeminiLlm {
    fn new(api_key: String, base_url: Option<String>) -> Result<Self> {
        let base_url = base_url.unwrap_or_else(|| LlmProvider::Gemini.default_base_url().to_string());
        
        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(300))
            .build()
            .context("building reqwest client")?;
        
        Ok(Self {
            api_key,
            base_url,
            client,
        })
    }
}

impl LlmClient for GeminiLlm {
    fn generate_json(&self, request: LlmRequest) -> Result<LlmResponse> {
        eprintln!("[DEBUG] Model: {}", request.model);

        // Build the file part based on attachment source
        let file_part = match &request.attachment {
            AttachmentSource::Inline(attachment) => {
                let encoded_pdf = STANDARD.encode(&attachment.data);
                eprintln!("[DEBUG] PDF size: {} bytes (inline)", attachment.data.len());
                eprintln!("[DEBUG] Base64 length: {} chars", encoded_pdf.len());
                serde_json::json!({
                    "inline_data": {
                        "mime_type": attachment.mime_type,
                        "data": encoded_pdf
                    }
                })
            }
            AttachmentSource::FileUri(file_ref) => {
                eprintln!("[DEBUG] Using cached file URI: {}", file_ref.file_uri);
                serde_json::json!({
                    "file_data": {
                        "mime_type": file_ref.mime_type,
                        "file_uri": file_ref.file_uri
                    }
                })
            }
        };

        // Build the request body following Gemini API format
        let body = serde_json::json!({
            "contents": [{
                "parts": [
                    file_part,
                    {
                        "text": request.prompt
                    }
                ]
            }],
            "generationConfig": {
                "temperature": request.temperature.unwrap_or(1.0),
                "responseMimeType": "application/json",
                "responseJsonSchema": request.schema
            }
        });
        
        // Construct the URL: {base_url}/models/{model}:generateContent?key={api_key}
        let url = format!(
            "{}/models/{}:generateContent?key={}",
            self.base_url,
            request.model,
            self.api_key
        );
        
        eprintln!("[DEBUG] Calling: {}", url.replace(&self.api_key, "***"));
        
        let resp = self.client
            .post(&url)
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .context("sending request to Gemini")?;
        
        let status = resp.status();
        let response_text = resp.text().context("reading response text")?;
        
        if !status.is_success() {
            return Err(anyhow!(
                "Gemini API error (status {}): {}",
                status,
                response_text
            ));
        }
        
        eprintln!("[DEBUG] Response: {}", &response_text[..response_text.len().min(500)]);
        
        let response_json: Value = serde_json::from_str(&response_text)
            .context("parsing Gemini response")?;
        
        // Extract the text from candidates[0].content.parts[0].text
        let text = response_json
            .get("candidates")
            .and_then(|c| c.get(0))
            .and_then(|c| c.get("content"))
            .and_then(|c| c.get("parts"))
            .and_then(|p| p.get(0))
            .and_then(|p| p.get("text"))
            .and_then(|t| t.as_str())
            .ok_or_else(|| anyhow!("unexpected Gemini response format: {}", response_json))?;
        
        let json: Value = serde_json::from_str(text)
            .context("parsing model JSON from Gemini text response")?;
        
        Ok(LlmResponse { json })
    }
}
