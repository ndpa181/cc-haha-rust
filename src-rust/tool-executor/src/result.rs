//! Tool execution result types

use serde::{Deserialize, Serialize};

/// Captured output from a tool execution
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ToolOutput {
    /// Standard output
    pub stdout: String,
    /// Standard error
    pub stderr: String,
    /// Exit code if available
    pub exit_code: Option<i32>,
}

/// Errors that can occur during tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ToolError {
    /// Failed to spawn the process
    Spawn(String),
    /// Execution failed
    Execution(String),
    /// Process timed out
    Timeout,
    /// Memory limit exceeded
    MemoryLimit,
    /// Sandbox violation
    SandboxViolation(String),
    /// Input/output error
    Io(String),
}

impl std::fmt::Display for ToolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ToolError::Spawn(s) => write!(f, "Failed to spawn: {}", s),
            ToolError::Execution(s) => write!(f, "Execution failed: {}", s),
            ToolError::Timeout => write!(f, "Process timed out"),
            ToolError::MemoryLimit => write!(f, "Memory limit exceeded"),
            ToolError::SandboxViolation(s) => write!(f, "Sandbox violation: {}", s),
            ToolError::Io(s) => write!(f, "I/O error: {}", s),
        }
    }
}

impl std::error::Error for ToolError {}

/// Complete result of a tool execution
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExecutionResult {
    /// Whether execution succeeded
    pub success: bool,
    /// Captured output
    pub output: ToolOutput,
    /// Error if any
    pub error: Option<ToolError>,
    /// Execution time in milliseconds
    pub duration_ms: u64,
}
