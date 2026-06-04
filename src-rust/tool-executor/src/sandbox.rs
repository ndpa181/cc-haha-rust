//! Sandbox configuration for tool execution
//!
//! Provides security boundaries for untrusted code execution:
//! - Filesystem access restrictions
//! - Network access control
//! - Resource limits
//! - Seccomp filters (Linux)

use std::collections::HashSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

/// Filesystem access mode
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FsAccess {
    /// Read-only access
    ReadOnly,
    /// Read-write access
    ReadWrite,
    /// No access
    Deny,
}

/// Configuration for filesystem sandboxing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemConfig {
    /// Allowed directory paths
    pub allowed_dirs: HashSet<PathBuf>,
    /// Denied directory paths
    pub denied_dirs: HashSet<PathBuf>,
    /// Temporary directory
    pub temp_dir: Option<PathBuf>,
    /// Create temp dir if missing
    pub create_temp: bool,
}

impl Default for FilesystemConfig {
    fn default() -> Self {
        let mut allowed_dirs = HashSet::new();
        allowed_dirs.insert(PathBuf::from("/tmp"));
        allowed_dirs.insert(PathBuf::from("/var/tmp"));

        Self {
            allowed_dirs,
            denied_dirs: HashSet::new(),
            temp_dir: Some(PathBuf::from("/tmp")),
            create_temp: true,
        }
    }
}

/// Network access configuration
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum NetworkAccess {
    /// No network access
    DenyAll,
    /// Allow outbound to specified hosts
    AllowList,
    /// Allow all network access
    AllowAll,
}

/// Configuration for network sandboxing
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    pub mode: NetworkAccess,
    /// Allowed host patterns (e.g., "*.example.com")
    pub allowed_hosts: Vec<String>,
    /// Allowed ports
    pub allowed_ports: Vec<u16>,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            mode: NetworkAccess::DenyAll,
            allowed_hosts: Vec::new(),
            allowed_ports: Vec::new(),
        }
    }
}

/// Complete sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Filesystem restrictions
    pub filesystem: FilesystemConfig,
    /// Network restrictions
    pub network: NetworkConfig,
    /// Maximum processes
    pub max_processes: Option<usize>,
    /// Maximum memory in bytes
    pub max_memory_bytes: Option<usize>,
    /// Enable seccomp (Linux only)
    pub seccomp: bool,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            filesystem: FilesystemConfig::default(),
            network: NetworkConfig::default(),
            max_processes: Some(100),
            max_memory_bytes: Some(512 * 1024 * 1024),
            seccomp: cfg!(all(target_os = "linux", not(test))),
        }
    }
}

impl SandboxConfig {
    /// Create a strict sandbox with minimal access
    pub fn strict() -> Self {
        Self {
            filesystem: FilesystemConfig {
                allowed_dirs: HashSet::from([PathBuf::from("/tmp")]),
                denied_dirs: HashSet::new(),
                temp_dir: Some(PathBuf::from("/tmp")),
                create_temp: true,
            },
            network: NetworkConfig::default(),
            max_processes: Some(10),
            max_memory_bytes: Some(128 * 1024 * 1024),
            seccomp: true,
        }
    }

    /// Create a permissive sandbox for trusted code
    pub fn permissive() -> Self {
        Self {
            filesystem: FilesystemConfig {
                allowed_dirs: HashSet::new(), // All allowed
                denied_dirs: HashSet::new(),
                temp_dir: None,
                create_temp: false,
            },
            network: NetworkConfig {
                mode: NetworkAccess::AllowAll,
                allowed_hosts: Vec::new(),
                allowed_ports: Vec::new(),
            },
            max_processes: None,
            max_memory_bytes: None,
            seccomp: false,
        }
    }

    /// Check if a path is allowed
    pub fn is_path_allowed(&self, path: &std::path::Path) -> bool {
        // Check denied paths first
        for denied in &self.filesystem.denied_dirs {
            if path.starts_with(denied) {
                return false;
            }
        }

        // If no allowed dirs specified, allow all
        if self.filesystem.allowed_dirs.is_empty() {
            return true;
        }

        // Check allowed paths
        for allowed in &self.filesystem.allowed_dirs {
            if path.starts_with(allowed) {
                return true;
            }
        }

        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_allowed() {
        let config = SandboxConfig::strict();
        assert!(config.is_path_allowed(PathBuf::from("/tmp/test").as_path()));
        assert!(!config.is_path_allowed(PathBuf::from("/etc/passwd").as_path()));
    }

    #[test]
    fn test_permissive_allows_all() {
        let config = SandboxConfig::permissive();
        assert!(config.is_path_allowed(PathBuf::from("/any/path").as_path()));
    }
}
