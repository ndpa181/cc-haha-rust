//! Fixture storage and retrieval with memory mapping

use std::collections::HashMap;
use std::fs::{self, File};
use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use memmap2::Mmap;
use serde::{Deserialize, Serialize};

use super::{VcrRequest, VcrResponse};

/// Metadata for a stored fixture
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FixtureMetadata {
    pub key: String,
    pub url: String,
    pub method: String,
    pub recorded_at: u64,
    pub size_bytes: u64,
}

/// A stored request/response pair
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fixture {
    pub key: String,
    pub request: VcrRequest,
    pub response: VcrResponse,
    #[serde(skip)]
    recorded_at: u64,
}

impl Fixture {
    pub fn new(key: String, request: VcrRequest, response: VcrResponse) -> Self {
        Self {
            key,
            request,
            response,
            recorded_at: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        }
    }

    /// Replace placeholders with actual values
    pub fn hydrate(&self, placeholders: &HashMap<String, String>) -> VcrResponse {
        let mut hydrated_body = self.response.body.clone();

        for (placeholder, value) in placeholders {
            hydrated_body = hydrated_body.replace(placeholder, value);
        }

        // Handle num_files="[NUM]" patterns
        hydrated_body = hydrated_body.replace("num_files=\"[NUM]\"", "num_files=\"1\"");

        // Handle duration_ms="[DURATION]" patterns
        hydrated_body = hydrated_body.replace("duration_ms=\"[DURATION]\"", "duration_ms=\"0\"");

        VcrResponse {
            status: self.response.status,
            headers: self.response.headers.clone(),
            body: hydrated_body,
        }
    }

    pub fn metadata(&self) -> FixtureMetadata {
        FixtureMetadata {
            key: self.key.clone(),
            url: self.request.url.clone(),
            method: self.request.method.clone(),
            recorded_at: self.recorded_at,
            size_bytes: 0, // Computed externally
        }
    }
}

/// Memory-mapped fixture storage
pub struct FixtureStore {
    dir: PathBuf,
    use_mmap: bool,
    index_path: PathBuf,
}

impl FixtureStore {
    pub fn new(dir: &Path, use_mmap: bool) -> io::Result<Self> {
        fs::create_dir_all(dir)?;

        let index_path = dir.join("index.json");

        Ok(Self {
            dir: dir.to_path_buf(),
            use_mmap,
            index_path,
        })
    }

    /// Get a fixture by key
    pub fn get(&self, key: &str) -> io::Result<Option<Fixture>> {
        let fixture_path = self.dir.join(format!("{}.json", key));

        if !fixture_path.exists() {
            return Ok(None);
        }

        if self.use_mmap {
            self.get_mmap(&fixture_path)
        } else {
            self.get_streaming(&fixture_path)
        }
    }

    fn get_mmap(&self, path: &Path) -> io::Result<Option<Fixture>> {
        let file = File::open(path)?;
        let metadata = file.metadata()?;
        let size = metadata.len() as usize;

        if size == 0 {
            return Ok(None);
        }

        // For small files, just read directly
        if size < 1024 * 1024 {
            return self.get_streaming(path);
        }

        // Memory map the file
        let mmap = unsafe { Mmap::map(&file)? };
        let json_str = std::str::from_utf8(&mmap)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let fixture: Fixture = serde_json::from_str(json_str)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        Ok(Some(fixture))
    }

    fn get_streaming(&self, path: &Path) -> io::Result<Option<Fixture>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);

        let fixture: Fixture = match serde_json::from_reader(reader) {
            Ok(f) => f,
            Err(_) => return Ok(None),
        };

        Ok(Some(fixture))
    }

    /// Store a fixture
    pub fn put(&self, key: &str, fixture: &Fixture) -> io::Result<()> {
        let fixture_path = self.dir.join(format!("{}.json", key));

        let json = serde_json::to_string_pretty(fixture)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let mut file = File::create(&fixture_path)?;
        file.write_all(json.as_bytes())?;
        file.sync_all()?;

        // Update index
        self.update_index(key, &fixture.metadata())
    }

    fn update_index(&self, key: &str, metadata: &FixtureMetadata) -> io::Result<()> {
        let mut index = self.read_index().unwrap_or_default();
        index.insert(key.to_string(), metadata.clone());

        let json = serde_json::to_string_pretty(&index)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;

        let mut file = File::create(&self.index_path)?;
        file.write_all(json.as_bytes())?;

        Ok(())
    }

    fn read_index(&self) -> io::Result<HashMap<String, FixtureMetadata>> {
        if !self.index_path.exists() {
            return Ok(HashMap::new());
        }

        let file = File::open(&self.index_path)?;
        let reader = BufReader::new(file);

        serde_json::from_reader(reader)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
    }

    /// List all fixtures
    pub fn list(&self) -> io::Result<Vec<FixtureMetadata>> {
        let index = self.read_index()?;
        Ok(index.into_values().collect())
    }

    /// Clear all fixtures
    pub fn clear(&mut self) -> io::Result<()> {
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().map_or(false, |e| e == "json") {
                fs::remove_file(path)?;
            }
        }
        Ok(())
    }

    /// Get number of fixtures
    pub fn len(&self) -> io::Result<usize> {
        let index = self.read_index()?;
        Ok(index.len())
    }

    /// Get total size of all fixtures
    pub fn total_size(&self) -> io::Result<u64> {
        let mut total = 0u64;
        for entry in fs::read_dir(&self.dir)? {
            let entry = entry?;
            if entry.path().extension().map_or(false, |e| e == "json") {
                total += entry.metadata()?.len();
            }
        }
        Ok(total)
    }
}
