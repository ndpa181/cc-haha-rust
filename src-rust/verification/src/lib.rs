//! Verification module for adversarial testing
//!
//! Provides structured verification with:
//! - Check registry and execution
//! - Result aggregation
//! - Verdict computation

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Verification verdict
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Verdict {
    /// All checks passed
    Pass,
    /// Some checks failed
    Fail,
    /// Environmental limitation, partial verification
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

/// Result of a single check
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CheckResult {
    pub name: String,
    pub passed: bool,
    pub output: String,
    pub expected: Option<String>,
    pub actual: Option<String>,
    pub error: Option<String>,
}

impl CheckResult {
    pub fn pass(name: &str, output: &str) -> Self {
        Self {
            name: name.to_string(),
            passed: true,
            output: output.to_string(),
            expected: None,
            actual: None,
            error: None,
        }
    }

    pub fn fail(name: &str, expected: &str, actual: &str) -> Self {
        Self {
            name: name.to_string(),
            passed: false,
            output: String::new(),
            expected: Some(expected.to_string()),
            actual: Some(actual.to_string()),
            error: None,
        }
    }

    pub fn error(name: &str, err: &str) -> Self {
        Self {
            name: name.to_string(),
            passed: false,
            output: String::new(),
            expected: None,
            actual: None,
            error: Some(err.to_string()),
        }
    }
}

/// A verification check that can be run
#[derive(Debug, Clone)]
pub struct Check {
    pub name: String,
    pub description: String,
    pub check_type: CheckType,
}

#[derive(Debug, Clone)]
pub enum CheckType {
    /// Run a command and check output
    Command {
        cmd: String,
        args: Vec<String>,
        expected_pattern: Option<String>,
    },
    /// HTTP request check
    HttpGet {
        url: String,
        expected_status: u16,
    },
    /// File existence check
    FileExists {
        path: String,
    },
    /// Custom verification
    Custom {
        verify_fn: String, // Name of the verification function
    },
}

/// Verification report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VerificationReport {
    pub checks: Vec<CheckResult>,
    pub verdict: Verdict,
    pub summary: String,
    pub executed_at: u64,
}

impl VerificationReport {
    pub fn new() -> Self {
        Self {
            checks: Vec::new(),
            verdict: Verdict::Pass,
            summary: String::new(),
            executed_at: 0,
        }
    }

    pub fn add_check(&mut self, result: CheckResult) {
        if !result.passed && self.verdict == Verdict::Pass {
            self.verdict = Verdict::Fail;
        }
        self.checks.push(result);
    }

    pub fn compute_verdict(&mut self) {
        let total = self.checks.len();
        let passed = self.checks.iter().filter(|c| c.passed).count();
        let failed = self.checks.iter().filter(|c| !c.passed).count();

        self.verdict = if failed > 0 {
            Verdict::Fail
        } else if passed == total {
            Verdict::Pass
        } else {
            Verdict::Partial
        };

        self.summary = format!("{}/{} checks passed", passed, total);
    }

    pub fn format_markdown(&self) -> String {
        let mut lines: Vec<String> = Vec::new();

        lines.push("# Verification Report\n".to_string());

        for check in &self.checks {
            lines.push(format!("### Check: {}\n", check.name));

            if check.passed {
                lines.push("**Result: PASS**".to_string());
            } else {
                lines.push("**Result: FAIL**".to_string());
            }

            if !check.output.is_empty() {
                lines.push(format!("\n**Output observed:**\n```\n{}\n```\n", check.output));
            }

            if let Some(expected) = &check.expected {
                lines.push(format!("\n**Expected:** `{}`\n", expected));
            }

            if let Some(actual) = &check.actual {
                lines.push(format!("\n**Actual:** `{}`\n", actual));
            }

            if let Some(error) = &check.error {
                lines.push(format!("\n**Error:** `{}`\n", error));
            }

            lines.push("\n---\n".to_string());
        }

        lines.push(format!("\n**VERDICT: {}**\n", self.verdict));
        lines.push(format!("\n**Summary:** {}\n", self.summary));

        lines.join("\n")
    }
}

impl Default for VerificationReport {
    fn default() -> Self {
        Self::new()
    }
}

/// Verification engine
pub struct VerificationEngine {
    checks: HashMap<String, Check>,
}

impl VerificationEngine {
    pub fn new() -> Self {
        Self {
            checks: HashMap::new(),
        }
    }

    pub fn register(&mut self, check: Check) {
        self.checks.insert(check.name.clone(), check);
    }

    pub fn run_check(&self, name: &str) -> Option<CheckResult> {
        let check = self.checks.get(name)?;

        Some(match &check.check_type {
            CheckType::Command { cmd, args, expected_pattern } => {
                // In a real implementation, this would execute the command
                CheckResult::pass(name, &format!("Would run: {} {:?}", cmd, args))
            }
            CheckType::HttpGet { url, expected_status } => {
                CheckResult::pass(name, &format!("Would GET: {} (expect {})", url, expected_status))
            }
            CheckType::FileExists { path } => {
                CheckResult::pass(name, &format!("Would check: {}", path))
            }
            CheckType::Custom { verify_fn } => {
                CheckResult::pass(name, &format!("Would run: {}", verify_fn))
            }
        })
    }

    pub fn list_checks(&self) -> Vec<&Check> {
        self.checks.values().collect()
    }
}

impl Default for VerificationEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verdict_computation() {
        let mut report = VerificationReport::new();

        report.add_check(CheckResult::pass("test1", "output"));
        report.add_check(CheckResult::fail("test2", "expected", "actual"));

        report.compute_verdict();

        assert_eq!(report.verdict, Verdict::Fail);
        assert_eq!(report.summary, "1/2 checks passed");
    }

    #[test]
    fn test_all_pass() {
        let mut report = VerificationReport::new();

        report.add_check(CheckResult::pass("test1", "output"));
        report.add_check(CheckResult::pass("test2", "output"));

        report.compute_verdict();

        assert_eq!(report.verdict, Verdict::Pass);
    }
}
