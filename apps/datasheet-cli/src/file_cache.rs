// SPDX-License-Identifier: GPL-3.0-only
// SPDX-FileCopyrightText: 2026 Alexander Kiselev <alex@akiselev.com>

//! Gemini File API cache for avoiding repeated PDF uploads.
//!
//! This module implements caching for the Gemini File API, storing file hashes
//! mapped to their Gemini file URIs. Files uploaded to Gemini expire after 48 hours,
//! so the cache automatically cleans up expired entries.

use anyhow::{anyhow, Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// How long Gemini keeps uploaded files (48 hours)
const GEMINI_FILE_TTL_SECS: u64 = 48 * 60 * 60;

/// Safety margin before expiration to avoid race conditions (1 hour)
const EXPIRY_MARGIN_SECS: u64 = 60 * 60;

/// Information about a file uploaded to Gemini
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedFile {
    /// The Gemini file name (e.g., "files/abc123")
    pub name: String,
    /// The Gemini file URI used in API requests
    pub uri: String,
    /// Unix timestamp when the file expires
    pub expires_at: u64,
    /// Original file size in bytes (for validation)
    pub file_size: u64,
}

impl CachedFile {
    /// Check if this cached file has expired or is about to expire
    pub fn is_expired(&self) -> bool {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        // Add margin to avoid using files that are about to expire
        now + EXPIRY_MARGIN_SECS >= self.expires_at
    }
}

/// Cache mapping file content hashes to Gemini file info
#[derive(Debug, Default, Serialize, Deserialize)]
pub struct CacheData {
    /// Map of SHA256 hash (hex) -> cached file info
    pub files: HashMap<String, CachedFile>,
}

/// Manages the file cache for Gemini uploads
pub struct FileCache {
    cache_dir: PathBuf,
    cache_file: PathBuf,
    data: CacheData,
    api_key: String,
    base_url: String,
    client: reqwest::blocking::Client,
}

impl FileCache {
    /// Create a new file cache manager
    pub fn new(api_key: String, base_url: Option<String>) -> Result<Self> {
        let cache_dir = get_cache_dir()?;
        let cache_file = cache_dir.join("gemini_files.json");

        // Load existing cache or create empty one
        let data = if cache_file.exists() {
            let content = fs::read_to_string(&cache_file)
                .context("reading cache file")?;
            serde_json::from_str(&content).unwrap_or_default()
        } else {
            CacheData::default()
        };

        let base_url = base_url.unwrap_or_else(|| {
            "https://generativelanguage.googleapis.com/v1beta".to_string()
        });

        let client = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(600)) // 10 min for large uploads
            .build()
            .context("building reqwest client")?;

        let mut cache = Self {
            cache_dir,
            cache_file,
            data,
            api_key,
            base_url,
            client,
        };

        // Clean up expired entries on load
        cache.cleanup_expired();

        Ok(cache)
    }

    /// Get or upload a file to Gemini, returning the cached file info
    pub fn get_or_upload(&mut self, path: &Path) -> Result<CachedFile> {
        let file_data = fs::read(path)
            .with_context(|| format!("reading {}", path.display()))?;
        let hash = compute_hash(&file_data);

        // Check if we have a valid cached entry
        if let Some(cached) = self.data.files.get(&hash) {
            if !cached.is_expired() {
                // Verify the file still exists on Gemini
                match self.check_file_exists(&cached.name) {
                    Ok(true) => {
                        eprintln!("[CACHE] Using cached file: {}", cached.uri);
                        return Ok(cached.clone());
                    }
                    Ok(false) => {
                        eprintln!("[CACHE] Cached file no longer exists on Gemini, re-uploading");
                    }
                    Err(e) => {
                        eprintln!("[CACHE] Error checking file: {}, re-uploading", e);
                    }
                }
            } else {
                eprintln!("[CACHE] Cached file expired, re-uploading");
            }
        }

        // Upload the file to Gemini
        let display_name = path
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| "datasheet.pdf".to_string());

        let cached_file = self.upload_file(&file_data, &display_name)?;

        // Store in cache and save
        self.data.files.insert(hash, cached_file.clone());
        self.save()?;

        Ok(cached_file)
    }

    /// Upload a file to Gemini using the resumable upload API
    fn upload_file(&self, data: &[u8], display_name: &str) -> Result<CachedFile> {
        let file_size = data.len() as u64;
        eprintln!("[CACHE] Uploading {} bytes to Gemini...", file_size);

        // Step 1: Start resumable upload to get upload URL
        // The upload endpoint uses a different path structure than the main API.
        // Base URL is like "https://generativelanguage.googleapis.com/v1beta"
        // Upload URL should be "https://generativelanguage.googleapis.com/upload/v1beta/files"
        let host = self.base_url
            .strip_suffix("/v1beta")
            .or_else(|| self.base_url.strip_suffix("/v1"))
            .unwrap_or(&self.base_url);
        let start_url = format!(
            "{}/upload/v1beta/files?key={}",
            host, self.api_key
        );

        let start_body = serde_json::json!({
            "file": {
                "display_name": display_name
            }
        });

        let start_resp = self.client
            .post(&start_url)
            .header("X-Goog-Upload-Protocol", "resumable")
            .header("X-Goog-Upload-Command", "start")
            .header("X-Goog-Upload-Header-Content-Length", file_size.to_string())
            .header("X-Goog-Upload-Header-Content-Type", "application/pdf")
            .header("Content-Type", "application/json")
            .json(&start_body)
            .send()
            .context("starting resumable upload")?;

        if !start_resp.status().is_success() {
            let status = start_resp.status();
            let body = start_resp.text().unwrap_or_default();
            return Err(anyhow!("Failed to start upload ({}): {}", status, body));
        }

        // Get upload URL from response headers
        let upload_url = start_resp
            .headers()
            .get("x-goog-upload-url")
            .ok_or_else(|| anyhow!("Missing x-goog-upload-url header"))?
            .to_str()
            .context("parsing upload URL")?
            .to_string();

        // Step 2: Upload the actual bytes
        let upload_resp = self.client
            .post(&upload_url)
            .header("Content-Length", file_size.to_string())
            .header("X-Goog-Upload-Offset", "0")
            .header("X-Goog-Upload-Command", "upload, finalize")
            .body(data.to_vec())
            .send()
            .context("uploading file data")?;

        if !upload_resp.status().is_success() {
            let status = upload_resp.status();
            let body = upload_resp.text().unwrap_or_default();
            return Err(anyhow!("Failed to upload file ({}): {}", status, body));
        }

        let upload_result: serde_json::Value = upload_resp.json()
            .context("parsing upload response")?;

        // Extract file info from response
        let file_obj = upload_result
            .get("file")
            .ok_or_else(|| anyhow!("Missing 'file' in upload response"))?;

        let name = file_obj
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'name' in file response"))?
            .to_string();

        let uri = file_obj
            .get("uri")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing 'uri' in file response"))?
            .to_string();

        // Calculate expiration time (48 hours from now)
        let expires_at = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            + GEMINI_FILE_TTL_SECS;

        eprintln!("[CACHE] Uploaded successfully: {}", uri);

        Ok(CachedFile {
            name,
            uri,
            expires_at,
            file_size,
        })
    }

    /// Check if a file still exists on Gemini
    fn check_file_exists(&self, name: &str) -> Result<bool> {
        let url = format!(
            "{}/{}?key={}",
            self.base_url, name, self.api_key
        );

        let resp = self.client
            .get(&url)
            .send()
            .context("checking file existence")?;

        if resp.status().is_success() {
            // Parse response to check state
            let info: serde_json::Value = resp.json()
                .context("parsing file info")?;

            // Check if file is in ACTIVE state
            let state = info
                .get("state")
                .and_then(|v| v.as_str())
                .unwrap_or("UNKNOWN");

            Ok(state == "ACTIVE")
        } else if resp.status() == reqwest::StatusCode::NOT_FOUND {
            Ok(false)
        } else {
            Err(anyhow!("Unexpected status checking file: {}", resp.status()))
        }
    }

    /// Remove expired entries from the cache
    fn cleanup_expired(&mut self) {
        let before_count = self.data.files.len();
        self.data.files.retain(|_, cached| !cached.is_expired());
        let removed = before_count - self.data.files.len();
        if removed > 0 {
            eprintln!("[CACHE] Cleaned up {} expired entries", removed);
            // Save after cleanup
            let _ = self.save();
        }
    }

    /// Save the cache to disk
    fn save(&self) -> Result<()> {
        // Ensure cache directory exists
        fs::create_dir_all(&self.cache_dir)
            .context("creating cache directory")?;

        let content = serde_json::to_string_pretty(&self.data)
            .context("serializing cache")?;

        fs::write(&self.cache_file, content)
            .context("writing cache file")?;

        Ok(())
    }
}

/// Compute SHA256 hash of data and return as hex string
fn compute_hash(data: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(data);
    let result = hasher.finalize();
    hex::encode(result)
}

/// Get the cache directory path
fn get_cache_dir() -> Result<PathBuf> {
    // Try to get platform-specific cache directory
    if let Some(cache_dir) = dirs::cache_dir() {
        return Ok(cache_dir.join("datasheet-cli"));
    }

    // Fallback to .cache in current directory
    Ok(PathBuf::from(".cache").join("datasheet-cli"))
}

/// Simple hex encoding (avoiding another dependency)
mod hex {
    pub fn encode(bytes: impl AsRef<[u8]>) -> String {
        bytes
            .as_ref()
            .iter()
            .map(|b| format!("{:02x}", b))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_computation() {
        let data = b"test data";
        let hash = compute_hash(data);
        assert_eq!(hash.len(), 64); // SHA256 produces 32 bytes = 64 hex chars
    }

    #[test]
    fn test_expiry_check() {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_secs();

        // Not expired (well beyond the 1 hour margin)
        let cached = CachedFile {
            name: "test".to_string(),
            uri: "test".to_string(),
            expires_at: now + 2 * 3600, // 2 hours from now
            file_size: 100,
        };
        assert!(!cached.is_expired());

        // Expired
        let cached = CachedFile {
            name: "test".to_string(),
            uri: "test".to_string(),
            expires_at: now - 1, // Already passed
            file_size: 100,
        };
        assert!(cached.is_expired());

        // Within margin (should be treated as expired)
        let cached = CachedFile {
            name: "test".to_string(),
            uri: "test".to_string(),
            expires_at: now + 30 * 60, // 30 min from now (within 1 hour margin)
            file_size: 100,
        };
        assert!(cached.is_expired());
    }
}
