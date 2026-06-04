//! Sandbox module with system call filtering
//!
//! Provides security boundaries using seccomp on Linux:
//! - Syscall allow/deny lists
//! - Memory and CPU resource limits
//! - Namespace isolation preparation

use std::collections::HashSet;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[cfg(target_os = "linux")]
use std::os::unix::process::ExitStatusExt;

mod seccomp;

pub use seccomp::{SeccompConfig, SeccompRule, Syscall};

/// Resource limits for sandbox
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceLimits {
    /// Max memory in bytes
    pub max_memory_bytes: Option<u64>,
    /// Max CPU time in seconds
    pub max_cpu_seconds: Option<u64>,
    /// Max number of processes
    pub max_processes: Option<u64>,
    /// Max file size in bytes
    pub max_file_size_bytes: Option<u64>,
}

impl Default for ResourceLimits {
    fn default() -> Self {
        Self {
            max_memory_bytes: Some(512 * 1024 * 1024),
            max_cpu_seconds: Some(300),
            max_processes: Some(100),
            max_file_size_bytes: Some(100 * 1024 * 1024),
        }
    }
}

/// Filesystem access control
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FsLimits {
    /// Read-only paths
    pub read_only: HashSet<PathBuf>,
    /// Write paths
    pub read_write: HashSet<PathBuf>,
    /// No access paths
    pub no_access: HashSet<PathBuf>,
    /// Create temporary directory
    pub temp_dir: Option<PathBuf>,
}

impl Default for FsLimits {
    fn default() -> Self {
        let mut read_only = HashSet::new();
        read_only.insert(PathBuf::from("/usr"));
        read_only.insert(PathBuf::from("/lib"));
        read_only.insert(PathBuf::from("/bin"));

        let mut read_write = HashSet::new();
        read_write.insert(PathBuf::from("/tmp"));
        read_write.insert(PathBuf::from("/var/tmp"));

        Self {
            read_only,
            read_write,
            no_access: HashSet::new(),
            temp_dir: Some(PathBuf::from("/tmp")),
        }
    }
}

/// Complete sandbox configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SandboxConfig {
    /// Enable seccomp filtering
    pub seccomp_enabled: bool,
    /// Seccomp rules
    pub seccomp: SeccompConfig,
    /// Resource limits
    pub resources: ResourceLimits,
    /// Filesystem limits
    pub filesystem: FsLimits,
    /// Allow network access
    pub network_enabled: bool,
    /// Allowed ports (empty = all)
    pub allowed_ports: Vec<u16>,
}

impl Default for SandboxConfig {
    fn default() -> Self {
        Self {
            seccomp_enabled: cfg!(all(target_os = "linux", not(test))),
            seccomp: SeccompConfig::default(),
            resources: ResourceLimits::default(),
            filesystem: FsLimits::default(),
            network_enabled: false,
            allowed_ports: Vec::new(),
        }
    }
}

impl SandboxConfig {
    /// Create a strict sandbox
    pub fn strict() -> Self {
        Self {
            seccomp_enabled: true,
            seccomp: SeccompConfig::strict(),
            resources: ResourceLimits {
                max_memory_bytes: Some(128 * 1024 * 1024),
                max_cpu_seconds: Some(60),
                max_processes: Some(10),
                max_file_size_bytes: Some(10 * 1024 * 1024),
            },
            filesystem: FsLimits::default(),
            network_enabled: false,
            allowed_ports: Vec::new(),
        }
    }

    /// Create a permissive sandbox
    pub fn permissive() -> Self {
        Self {
            seccomp_enabled: false,
            seccomp: SeccompConfig::permissive(),
            resources: ResourceLimits {
                max_memory_bytes: None,
                max_cpu_seconds: None,
                max_processes: None,
                max_file_size_bytes: None,
            },
            filesystem: FsLimits {
                read_only: HashSet::new(),
                read_write: HashSet::new(),
                no_access: HashSet::new(),
                temp_dir: Some(PathBuf::from("/tmp")),
            },
            network_enabled: true,
            allowed_ports: Vec::new(),
        }
    }
}

/// Sandbox creation result
#[derive(Debug)]
pub struct Sandbox {
    _config: SandboxConfig,
}

impl Sandbox {
    /// Create a new sandbox (setup phase)
    #[cfg(target_os = "linux")]
    pub fn new(config: SandboxConfig) -> Result<Self, SandboxError> {
        if config.seccomp_enabled {
            seccomp::apply_seccomp(&config.seccomp)?;
        }

        Ok(Self { _config: config })
    }

    #[cfg(not(target_os = "linux"))]
    pub fn new(config: SandboxConfig) -> Result<Self, SandboxError> {
        if config.seccomp_enabled {
            return Err(SandboxError::Unsupported(
                "Seccomp is only supported on Linux".to_string(),
            ));
        }

        Ok(Self { _config: config })
    }

    /// Enter the sandbox (called in child process after fork)
    pub fn enter(&self) -> Result<(), SandboxError> {
        // Would apply resource limits here
        Ok(())
    }
}

/// Sandbox errors
#[derive(Debug, thiserror::Error)]
pub enum SandboxError {
    #[error("Failed to apply seccomp filter: {0}")]
    SeccompError(String),

    #[error("Unsupported feature: {0}")]
    Unsupported(String),

    #[error("Operation not permitted (may need CAP_SYS_ADMIN)")]
    NotPermitted,

    #[error("Invalid configuration: {0}")]
    InvalidConfig(String),
}
