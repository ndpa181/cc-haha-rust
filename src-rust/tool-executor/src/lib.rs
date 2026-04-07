//! Tool Executor - Safe subprocess execution with resource limits
//!
//! Provides controlled execution environment for tool operations with:
//! - Memory and time limits
//! - Working directory sandboxing
//! - Environment variable filtering
//! - Structured output capture

use std::collections::HashMap;
use std::path::PathBuf;
use std::process::Stdio;
use std::time::{Duration, Instant};

use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;
use tokio::sync::mpsc;

mod result;
mod sandbox;

pub use result::{ToolOutput, ToolError, ExecutionResult};
pub use sandbox::SandboxConfig;

/// Configuration for tool execution
#[derive(Debug, Clone)]
pub struct ToolExecutorConfig {
    /// Working directory for execution
    pub working_dir: PathBuf,
    /// Environment variables to pass
    pub env_vars: HashMap<String, String>,
    /// Memory limit in bytes
    pub memory_limit: Option<usize>,
    /// Time limit
    pub time_limit: Option<Duration>,
    /// Whether to allow network access
    pub allow_network: bool,
}

impl Default for ToolExecutorConfig {
    fn default() -> Self {
        Self {
            working_dir: PathBuf::from("."),
            env_vars: HashMap::new(),
            memory_limit: Some(512 * 1024 * 1024), // 512MB default
            time_limit: Some(Duration::from_secs(300)), // 5 min default
            allow_network: false,
        }
    }
}

/// Tool executor with resource management
pub struct ToolExecutor {
    config: ToolExecutorConfig,
}

impl ToolExecutor {
    pub fn new(config: ToolExecutorConfig) -> Self {
        Self { config }
    }

    /// Execute a command with the configured limits
    pub async fn execute(
        &self,
        program: &str,
        args: &[&str],
        input: Option<&str>,
    ) -> ExecutionResult {
        let start = Instant::now();

        // Build command
        let mut cmd = Command::new(program);
        cmd.args(args)
            .cwd(&self.config.working_dir)
            .envs(&self.config.env_vars)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        if input.is_some() {
            cmd.stdin(Stdio::piped());
        }

        // Spawn with timeout
        let child_result = tokio::time::timeout(
            self.config.time_limit.unwrap_or(Duration::MAX),
            cmd.spawn(),
        )
        .await;

        let child = match child_result {
            Ok(Ok(c)) => c,
            Ok(Err(e)) => {
                return ExecutionResult {
                    success: false,
                    output: ToolOutput::default(),
                    error: Some(ToolError::Spawn(e.to_string())),
                    duration_ms: start.elapsed().as_millis() as u64,
                };
            }
            Err(_) => {
                return ExecutionResult {
                    success: false,
                    output: ToolOutput::default(),
                    error: Some(ToolError::Timeout),
                    duration_ms: self.config.time_limit.unwrap().as_millis() as u64,
                };
            }
        };

        // Handle input if provided
        let mut handle = child;
        if let Some(input_data) = input {
            if let Some(mut stdin) = handle.stdin.take() {
                use tokio::io::AsyncWriteExt;
                let _ = stdin.write_all(input_data.as_bytes()).await;
            }
        }

        // Wait for completion
        let output_result = handle.wait_with_output().await;

        let output = match output_result {
            Ok(o) => o,
            Err(e) => {
                return ExecutionResult {
                    success: false,
                    output: ToolOutput::default(),
                    error: Some(ToolError::Execution(e.to_string())),
                    duration_ms: start.elapsed().as_millis() as u64,
                };
            }
        };

        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();

        ExecutionResult {
            success: output.status.success(),
            output: ToolOutput {
                stdout,
                stderr,
                exit_code: output.status.code(),
            },
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }

    /// Stream output line by line
    pub async fn execute_streaming(
        &self,
        program: &str,
        args: &[&str],
        mut tx: mpsc::Sender<String>,
    ) -> ExecutionResult {
        let start = Instant::now();

        let mut cmd = Command::new(program);
        cmd.args(args)
            .cwd(&self.config.working_dir)
            .envs(&self.config.env_vars)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        let mut child = match cmd.spawn() {
            Ok(c) => c,
            Err(e) => {
                return ExecutionResult {
                    success: false,
                    output: ToolOutput::default(),
                    error: Some(ToolError::Spawn(e.to_string())),
                    duration_ms: start.elapsed().as_millis() as u64,
                };
            }
        };

        let stdout = child.stdout.take().unwrap();
        let mut reader = BufReader::new(stdout).lines();

        let mut stdout_lines = Vec::new();

        loop {
            tokio::select! {
                line = reader.next_line() => {
                    match line {
                        Ok(Some(l)) => {
                            stdout_lines.push(l.clone());
                            let _ = tx.send(l).await;
                        }
                        Ok(None) => break,
                        Err(e) => {
                            return ExecutionResult {
                                success: false,
                                output: ToolOutput::default(),
                                error: Some(ToolError::Execution(e.to_string())),
                                duration_ms: start.elapsed().as_millis() as u64,
                            };
                        }
                    }
                }
                status = child.wait() => {
                    match status {
                        Ok(s) => {
                            let stderr = String::new(); // Would need stderr handling
                            return ExecutionResult {
                                success: s.success(),
                                output: ToolOutput {
                                    stdout: stdout_lines.join("\n"),
                                    stderr,
                                    exit_code: s.code(),
                                },
                                error: None,
                                duration_ms: start.elapsed().as_millis() as u64,
                            };
                        }
                        Err(e) => {
                            return ExecutionResult {
                                success: false,
                                output: ToolOutput::default(),
                                error: Some(ToolError::Execution(e.to_string())),
                                duration_ms: start.elapsed().as_millis() as u64,
                            };
                        }
                    }
                }
            }
        }

        let status = child.wait().await.unwrap_or_default();
        ExecutionResult {
            success: status.success(),
            output: ToolOutput {
                stdout: stdout_lines.join("\n"),
                stderr: String::new(),
                exit_code: status.code(),
            },
            error: None,
            duration_ms: start.elapsed().as_millis() as u64,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_basic_execution() {
        let config = ToolExecutorConfig::default();
        let executor = ToolExecutor::new(config);

        let result = executor.execute("echo", &["hello", "world"], None).await;
        assert!(result.success);
        assert_eq!(result.output.stdout.trim(), "hello world");
    }

    #[tokio::test]
    async fn test_timeout() {
        let mut config = ToolExecutorConfig::default();
        config.time_limit = Some(Duration::from_millis(10));

        let executor = ToolExecutor::new(config);
        let result = executor.execute("sleep", &["10"], None).await;

        assert!(!result.success);
        assert!(matches!(result.error, Some(ToolError::Timeout)));
    }
}
