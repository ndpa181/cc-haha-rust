//! Value dehydration for cross-platform fixture compatibility
//!
//! Replaces platform-specific values with portable placeholders:
//! - Paths: /Users/name -> [CONFIG_HOME]
//! - CWD references: /current/work/dir -> [CWD]
//! - Dynamic values: num_files="42" -> num_files="[NUM]"

use std::collections::HashMap;
use std::path::Path;

/// Dehydrate a string value by replacing platform-specific patterns
pub fn dehydrate_value(value: &str, placeholders: &HashMap<String, String>) -> String {
    let mut result = value.to_string();

    // Replace each placeholder's value with the placeholder key
    for (placeholder, actual_value) in placeholders {
        // Only replace non-empty values
        if !actual_value.is_empty() {
            result = result.replace(actual_value, placeholder);
        }
    }

    // Normalize path separators
    result = result.replace("\\\\", "/");

    // Replace numeric patterns
    result = replace_numeric_patterns(&result);

    // Normalize line endings
    result = result.replace("\r\n", "\n");

    result
}

/// Replace dynamic numeric patterns with placeholders
fn replace_numeric_patterns(s: &str) -> String {
    // Replace num_files="<number>" with num_files="[NUM]"
    let re_num_files = regex_lite::Regex::new(r#"num_files="\d+""#).ok();
    if let Some(re) = re_num_files {
        return re.replace_all(s, "num_files=\"[NUM]\"").to_string();
    }

    // Replace duration_ms="<number>" with duration_ms="[DURATION]"
    let re_duration = regex_lite::Regex::new(r#"duration_ms="\d+""#).ok();
    if let Some(re) = re_duration {
        return re.replace_all(s, "duration_ms=\"[DURATION]\"").to_string();
    }

    s.to_string()
}

/// Normalize a path for cross-platform compatibility
pub fn normalize_path(path: &Path) -> String {
    let path_str = path.to_string_lossy();

    // Replace home directory with [CONFIG_HOME]
    if let Some(home) = dirs::home_dir() {
        let home_str = home.to_string_lossy();
        if path_str.starts_with(&*home_str) {
            return path_str.replace(&*home_str, "[CONFIG_HOME]");
        }
    }

    // Replace CWD with [CWD]
    if let Ok(cwd) = std::env::current_dir() {
        let cwd_str = cwd.to_string_lossy();
        if path_str.starts_with(&*cwd_str) {
            return path_str.replace(&*cwd_str, "[CWD]");
        }
    }

    // Normalize to forward slashes
    path_str.replace('\\', "/").to_string()
}

/// Check if a path needs normalization
pub fn needs_normalization(path: &str) -> bool {
    // Check for home directory
    if let Some(home) = dirs::home_dir() {
        if path.starts_with(&*home.to_string_lossy()) {
            return true;
        }
    }

    // Check for current directory
    if let Ok(cwd) = std::env::current_dir() {
        if path.starts_with(&*cwd.to_string_lossy()) {
            return true;
        }
    }

    // Check for Windows-style backslashes
    if path.contains('\\') {
        return true;
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dehydrate_basic() {
        let mut placeholders = HashMap::new();
        placeholders.insert("[CWD]".to_string(), "/home/user/project".to_string());

        let result = dehydrate_value("/home/user/project/file.txt", &placeholders);
        assert_eq!(result, "[CWD]/file.txt");
    }

    #[test]
    fn test_normalize_path() {
        let path = Path::new("/home/user/file.txt");
        let normalized = normalize_path(path);
        assert!(normalized.contains("[CONFIG_HOME]") || normalized.starts_with("/"));
    }

    #[test]
    fn test_replace_numeric() {
        let input = r#"num_files="42" and duration_ms="1000""#;
        let result = replace_numeric_patterns(input);
        assert!(result.contains("[NUM]"));
        assert!(result.contains("[DURATION]"));
    }
}
