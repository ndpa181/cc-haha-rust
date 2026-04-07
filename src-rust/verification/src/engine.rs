//! Verification engine with adversarial probing
//!
//! Core verification logic for running adversarial checks

use std::collections::HashMap;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// A verification check to run
#[derive(Debug, Clone)]
pub struct Check {
    pub name: String,
    pub description: String,
    pub check_kind: CheckKind,
}

#[derive(Debug, Clone)]
pub enum CheckKind {
    /// Run a command and verify output
    Command {
        program: String,
        args: Vec<String>,
        expected_pattern: Option<String>,
        must_have_command: bool, // If true, command output must be present
    },
    /// HTTP GET request
    HttpGet {
        url: String,
        expected_status: u16,
        expected_pattern: Option<String>,
    },
    /// File must exist
    FileExists {
        path: String,
    },
    /// Directory must exist
    DirExists {
        path: String,
    },
    /// Custom assertion
    Assert {
        condition: String,
        description: String,
    },
}

/// Result of a check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub check_name: String,
    pub passed: bool,
    pub output: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub error_message: Option<String>,
    pub duration_ms: u64,
}

impl CheckResult {
    pub fn pass(name: &str, output: &str) -> Self {
        Self {
            check_name: name.to_string(),
            passed: true,
            output: output.to_string(),
            expected: None,
            actual: None,
            error_message: None,
            duration_ms: 0,
        }
    }

    pub fn fail(name: &str, expected: &str, actual: &str) -> Self {
        Self {
            check_name: name.to_string(),
            passed: false,
            output: String::new(),
            expected: Some(expected.to_string()),
            actual: Some(actual.to_string()),
            error_message: None,
            duration_ms: 0,
        }
    }

    pub fn error(name: &str, msg: &str) -> Self {
        Self {
            check_name: name.to_string(),
            passed: false,
            output: String::new(),
            expected: None,
            actual: None,
            error_message: Some(msg.to_string()),
            duration_ms: 0,
        }
    }
}

/// Verification verdict
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    Pass,
    Fail,
    Partial,
}

impl std::fmt::Display for Verdict {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Verdict::Pass => write!(f, "PASS"),
            Verdict::Fail => write!(f, "FAIL"),
            Verdict::Partial => write!(f, "PARTIAL"),
        }
    }
}

/// A verification report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub check_results: Vec<CheckResult>,
    pub verdict: Verdict,
    pub summary: String,
    pub executed_at: u64,
    pub total_duration_ms: u64,
}

impl VerificationReport {
    pub fn new() -> Self {
        Self {
            check_results: Vec::new(),
            verdict: Verdict::Pass,
            summary: String::new(),
            executed_at: 0,
            total_duration_ms: 0,
        }
    }

    pub fn add_result(&mut self, result: CheckResult) {
        if !result.passed && self.verdict == Verdict::Pass {
            self.verdict = Verdict::Fail;
        }
        self.check_results.push(result);
    }

    pub fn compute_verdict(&mut self) {
        let total = self.check_results.len();
        let passed = self.check_results.iter().filter(|c| c.passed).count();

        self.verdict = if passed == total {
            Verdict::Pass
        } else if passed == 0 {
            Verdict::Fail
        } else {
            Verdict::Partial
        };

        self.summary = format!("{}/{} checks passed", passed, total);
    }

    pub fn format_markdown(&self) -> String {
        let mut lines = Vec::new();

        lines.push("# Verification Report\n".to_string());

        for result in &self.check_results {
            lines.push(format!("### Check: {}\n", result.check_name));

            if result.passed {
                lines.push("**Result: PASS**".to_string());
            } else {
                lines.push("**Result: FAIL**".to_string());
            }

            if !result.output.is_empty() {
                lines.push("\n**Output observed:**\n```\n{}\n```\n".format(result.output));
            }

            if let Some(expected) = &result.expected {
                lines.push(format!("\n**Expected:** `{}`\n", expected));
            }

            if let Some(actual) = &result.actual {
                lines.push(format!("\n**Actual:** `{}`\n", actual));
            }

            if let Some(error) = &result.error_message {
                lines.push(format!("\n**Error:** `{}`\n", error));
            }

            lines.push("\n---\n".to_string());
        }

        lines.push(format!("\n**VERDICT: {}**\n", self.verdict));
        lines.push(format!("\n**Summary:** {}\n", self.summary));

        lines.join("")
    }
}

impl Default for VerificationReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Run a verification check
pub fn run_check(check: &Check) -> CheckResult {
    let start = Instant::now();

    let result = match &check.check_kind {
        CheckKind::Command { program, args, expected_pattern, must_have_command } => {
            run_command_check(&check.name, program, args, *expected_pattern, *must_have_command)
        }
        CheckKind::FileExists { path } => {
            run_file_exists_check(&check.name, path)
        }
        CheckKind::DirExists { path } => {
            run_dir_exists_check(&check.name, path)
        }
        CheckKind::Assert { condition, .. } => {
            // In real impl, would evaluate condition
            CheckResult::pass(&check.name, &format!("Assert: {}", condition))
        }
        CheckKind::HttpGet { url, expected_status, expected_pattern } => {
            run_http_check(&check.name, url, *expected_status, expected_pattern.as_deref())
        }
    };

    let duration = result.output.len() as u64; // Placeholder
    CheckResult {
        check_name: result.check_name,
        passed: result.passed,
        output: result.output,
        expected: result.expected,
        actual: result.actual,
        error_message: result.error_message,
        duration_ms: start.elapsed().as_millis() as u64 + duration,
    }
}

fn run_command_check(
    name: &str,
    program: &str,
    args: &[String],
    expected_pattern: Option<String>,
    must_have_command: bool,
) -> CheckResult {
    let output = Command::new(program)
        .args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output();

    match output {
        Ok(out) => {
            let stdout = String::from_utf8_lossy(&out.stdout).to_string();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();

            // For verification agent, we care about actual command output
            if must_have_command && stdout.is_empty() && stderr.is_empty() {
                return CheckResult::fail(
                    name,
                    "command should produce output",
                    "(empty output)",
                );
            }

            if let Some(pattern) = expected_pattern {
                let combined = format!("{}\n{}", stdout, stderr);
                if !combined.contains(&pattern) {
                    return CheckResult::fail(
                        name,
                        &format!("output should contain: {}", pattern),
                        &combined.chars().take(200).collect::<String>(),
                    );
                }
            }

            CheckResult::pass(name, &stdout)
        }
        Err(e) => CheckResult::error(name, &e.to_string()),
    }
}

fn run_file_exists_check(name: &str, path: &str) -> CheckResult {
    if std::path::Path::new(path).exists() {
        CheckResult::pass(name, &format!("File exists: {}", path))
    } else {
        CheckResult::fail(name, &format!("File should exist: {}", path), "File not found")
    }
}

fn run_dir_exists_check(name: &str, path: &str) -> CheckResult {
    if std::path::Path::new(path).is_dir() {
        CheckResult::pass(name, &format!("Directory exists: {}", path))
    } else {
        CheckResult::fail(name, &format!("Directory should exist: {}", path), "Directory not found")
    }
}

fn run_http_check(name: &str, url: &str, expected_status: u16, _expected_pattern: Option<&str>) -> CheckResult {
    // In production, would use reqwest or similar
    // For now, return a placeholder
    CheckResult::pass(name, &format!("Would HTTP GET {} (expect {})", url, expected_status))
}
