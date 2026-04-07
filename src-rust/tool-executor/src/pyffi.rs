//! PyO3 FFI bindings for tool-executor
//!
//! Exposes ToolExecutor to Python via PyO3

use pyo3::prelude::*;
use pyo3::types::PyDict;
use std::collections::HashMap;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use crate::{ToolExecutor, ToolExecutorConfig, ExecutionResult};

/// Python-friendly execution result
#[pyclass]
pub struct PyExecutionResult {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
    pub exit_code: Option<i32>,
    pub error: Option<String>,
    pub duration_ms: u64,
}

#[pymethods]
impl PyExecutionResult {
    #[getter]
    fn success(&self) -> bool { self.success }

    #[getter]
    fn stdout(&self) -> &str { &self.stdout }

    #[getter]
    fn stderr(&self) -> &str { &self.stderr }

    #[getter]
    fn exit_code(&self) -> Option<i32> { self.exit_code }

    #[getter]
    fn error(&self) -> Option<&str> { self.error.as_deref() }

    #[getter]
    fn duration_ms(&self) -> u64 { self.duration_ms }
}

impl From<ExecutionResult> for PyExecutionResult {
    fn from(result: ExecutionResult) -> Self {
        Self {
            success: result.success,
            stdout: result.output.stdout,
            stderr: result.output.stderr,
            exit_code: result.output.exit_code,
            error: result.error.map(|e| e.to_string()),
            duration_ms: result.duration_ms,
        }
    }
}

/// Python-friendly tool executor
#[pyclass]
pub struct PyToolExecutor {
    inner: ToolExecutor,
}

#[pymethods]
impl PyToolExecutor {
    #[new]
    fn new(
        working_dir: Option<String>,
        memory_limit_mb: Option<usize>,
        time_limit_secs: Option<f64>,
        env_vars: Option<HashMap<String, String>>,
    ) -> Self {
        let mut config = ToolExecutorConfig::default();

        if let Some(dir) = working_dir {
            config.working_dir = PathBuf::from(dir);
        }

        if let Some(limit) = memory_limit_mb {
            config.memory_limit = Some(limit * 1024 * 1024);
        }

        if let Some(secs) = time_limit_secs {
            config.time_limit = Some(Duration::from_secs_f64(secs));
        }

        if let Some(vars) = env_vars {
            config.env_vars = vars;
        }

        Self {
            inner: ToolExecutor::new(config),
        }
    }

    /// Execute a command synchronously
    fn execute(&self, program: &str, args: Vec<String>, input: Option<String>) -> PyResult<PyExecutionResult> {
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();

        // Note: This is blocking in async context, but PyO3 GIL ensures safety
        // For production, use asyncio.to_thread
        let rt = tokio::runtime::Runtime::new().unwrap();
        let result = rt.block_on(self.inner.execute(program, &args_refs, input.as_deref()));

        Ok(PyExecutionResult::from(result))
    }

    /// Execute with timeout
    fn execute_with_timeout(
        &self,
        program: &str,
        args: Vec<String>,
        timeout_secs: f64,
    ) -> PyResult<PyExecutionResult> {
        let args_refs: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
        let rt = tokio::runtime::Runtime::new().unwrap();

        let timeout = Duration::from_secs_f64(timeout_secs);

        let start = Instant::now();
        let result = rt.block_on(async {
            tokio::time::timeout(timeout, self.inner.execute(program, &args_refs, None))
                .await
                .unwrap_or_else(|_| crate::ExecutionResult {
                    success: false,
                    output: crate::ToolOutput::default(),
                    error: Some(crate::ToolError::Timeout),
                    duration_ms: timeout.as_millis() as u64,
                })
        });

        Ok(PyExecutionResult::from(result))
    }
}

/// Create a default executor
#[pyfunction]
fn create_executor() -> PyToolExecutor {
    PyToolExecutor::new(None, None, None, None)
}

/// Execute a command with defaults
#[pyfunction]
fn execute_command(program: &str, args: Vec<String>) -> PyResult<PyExecutionResult> {
    let executor = create_executor();
    executor.execute(program, args, None)
}

/// Python module definition
#[pymodule]
fn tool_executor(_py: Python, m: &PyModule) -> PyResult<()> {
    m.add_class::<PyToolExecutor>()?;
    m.add_class::<PyExecutionResult>()?;
    m.add_function(wrap_pyfunction!(create_executor, m)?)?;
    m.add_function(wrap_pyfunction!(execute_command, m)?)?;
    Ok(())
}
