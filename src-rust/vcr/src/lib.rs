//! VCR - Video Cassette Recorder for API response caching
//!
//! Records and replays HTTP interactions with deterministic fixture matching:
//! - SHA-1 based fixture naming
//! - Cross-platform path normalization
//! - Memory-mapped file cache for performance
//! - Automatic dehydration/hydration of platform-specific values

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use memmap2::Mmap;
use serde::{Deserialize, Serialize};
use sha1::{Digest, Sha1};

mod dehydrate;
mod fixture;

pub use dehydrate::{dehydrate_value, normalize_path};
pub use fixture::{Fixture, FixtureMetadata, FixtureStore};

/// VCR Configuration
#[derive(Debug, Clone)]
pub struct VcrConfig {
    /// Directory for storing fixtures
    pub fixtures_dir: PathBuf,
    /// Enable memory-mapped cache
    pub use_mmap: bool,
    /// Maximum fixture size for mmap (larger uses streaming)
    pub mmap_threshold: usize,
    /// Placeholder replacements for cross-platform compatibility
    pub placeholders: HashMap<String, String>,
}

impl Default for VcrConfig {
    fn default() -> Self {
        let mut placeholders = HashMap::new();
        placeholders.insert("[CWD]".to_string(), std::env::current_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string());
        placeholders.insert("[CONFIG_HOME]".to_string(), dirs::config_dir()
            .unwrap_or_default()
            .to_string_lossy()
            .to_string());

        Self {
            fixtures_dir: PathBuf::from(".fixtures"),
            use_mmap: true,
            mmap_threshold: 1024 * 1024, // 1MB
            placeholders,
        }
    }
}

/// Request to record or match
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcrRequest {
    pub method: String,
    pub url: String,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
}

impl VcrRequest {
    /// Generate a unique hash for this request
    pub fn hash(&self) -> String {
        let mut hasher = Sha1::new();
        hasher.update(self.method.as_bytes());
        hasher.update(self.url.as_bytes());
        for (k, v) in &self.headers {
            hasher.update(k.as_bytes());
            hasher.update(v.as_bytes());
        }
        if let Some(body) = &self.body {
            hasher.update(body.as_bytes());
        }
        format!("{:x}", hasher.finalize())
    }
}

/// Recorded response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VcrResponse {
    pub status: u16,
    pub headers: HashMap<String, String>,
    pub body: String,
}

/// VCR mode of operation
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VcrMode {
    /// Record new responses, playback existing
    Auto,
    /// Only playback, fail on cache miss
    Playback,
    /// Disable caching, pass through to network
    Disabled,
}

/// VCR instance for recording and playback
pub struct Vcr {
    config: VcrConfig,
    mode: VcrMode,
    store: FixtureStore,
}

impl Vcr {
    /// Create a new VCR instance
    pub fn new(config: VcrConfig, mode: VcrMode) -> io::Result<Self> {
        // Ensure fixtures directory exists
        fs::create_dir_all(&config.fixtures_dir)?;

        let store = FixtureStore::new(&config.fixtures_dir, config.use_mmap)?;

        Ok(Self {
            config,
            mode,
            store,
        })
    }

    /// Set the VCR mode
    pub fn set_mode(&mut self, mode: VcrMode) {
        self.mode = mode;
    }

    /// Get the current mode
    pub fn mode(&self) -> VcrMode {
        self.mode
    }

    /// Look up a cached response or record a new one
    pub fn get_or_record<F>(&mut self, request: &VcrRequest, network_call: F) -> io::Result<VcrResponse>
    where
        F: FnOnce(&VcrRequest) -> io::Result<VcrResponse>,
    {
        let key = request.hash();

        // Try to find existing fixture
        if let Some(mut fixture) = self.store.get(&key)? {
            // Hydrate the response (replace placeholders)
            let response = fixture.hydrate(&self.config.placeholders);
            return Ok(response);
        }

        // In playback mode, fail on cache miss
        if self.mode == VcrMode::Playback {
            return Err(io::Error::new(
                io::ErrorKind::NotFound,
                format!("Fixture not found for request: {}", request.url),
            ));
        }

        // Record new response
        let response = network_call(request)?;

        // Dehydrate the response (replace platform-specific values)
        let dehydrated_response = VcrResponse {
            status: response.status,
            headers: response.headers,
            body: dehydrate_value(&response.body, &self.config.placeholders),
        };

        // Create and save fixture
        let fixture = Fixture::new(
            key.clone(),
            request.clone(),
            dehydrated_response,
        );
        self.store.put(&key, &fixture)?;

        Ok(response)
    }

    /// Clear all cached fixtures
    pub fn clear(&mut self) -> io::Result<()> {
        self.store.clear()
    }

    /// List all cached fixtures
    pub fn list(&self) -> io::Result<Vec<FixtureMetadata>> {
        self.store.list()
    }

    /// Get statistics about the cache
    pub fn stats(&self) -> io::Result<CacheStats> {
        let entries = self.store.len()?;
        let total_size = self.store.total_size()?;

        Ok(CacheStats {
            entries,
            total_bytes: total_size,
        })
    }
}

/// Cache statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub entries: usize,
    pub total_bytes: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_hash() {
        let req1 = VcrRequest {
            method: "GET".to_string(),
            url: "https://api.example.com/test".to_string(),
            headers: HashMap::new(),
            body: None,
        };

        let req2 = VcrRequest {
            method: "GET".to_string(),
            url: "https://api.example.com/test".to_string(),
            headers: HashMap::new(),
            body: None,
        };

        assert_eq!(req1.hash(), req2.hash());
    }

    #[test]
    fn test_different_urls_different_hashes() {
        let req1 = VcrRequest {
            method: "GET".to_string(),
            url: "https://api.example.com/a".to_string(),
            headers: HashMap::new(),
            body: None,
        };

        let req2 = VcrRequest {
            method: "GET".to_string(),
            url: "https://api.example.com/b".to_string(),
            headers: HashMap::new(),
            body: None,
        };

        assert_ne!(req1.hash(), req2.hash());
    }
}
