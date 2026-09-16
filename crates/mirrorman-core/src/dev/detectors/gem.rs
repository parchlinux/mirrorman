use std::path::PathBuf;

use super::{is_command_in_path, EcosystemDetector};
use crate::dev::models::{DevEcosystemKind, DevMirror};

pub struct GemDetector;

impl GemDetector {
    pub fn new() -> Self {
        Self
    }

    fn resolve_config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".gemrc")
    }
}

impl EcosystemDetector for GemDetector {
    fn kind(&self) -> DevEcosystemKind {
        DevEcosystemKind::Gem
    }

    fn is_installed(&self) -> bool {
        is_command_in_path("gem")
    }

    fn get_config_path(&self) -> Option<PathBuf> {
        Some(Self::resolve_config_path())
    }

    fn get_current_mirror(&self) -> Option<String> {
        let path = Self::resolve_config_path();
        if !path.exists() {
            return Some("https://rubygems.org".to_string());
        }

        let content = std::fs::read_to_string(&path).ok()?;
        parse_gem_source(&content).or_else(|| Some("https://rubygems.org".to_string()))
    }

    fn set_mirror(&self, mirror: &DevMirror) -> Result<(), String> {
        let path = Self::resolve_config_path();
        let content = if path.exists() {
            std::fs::read_to_string(&path).unwrap_or_default()
        } else {
            String::new()
        };

        let updated = update_gem_source(&content, &mirror.url);
        std::fs::write(&path, updated).map_err(|e| format!("Failed to write .gemrc: {e}"))
    }

    fn reset_mirror(&self) -> Result<(), String> {
        let path = Self::resolve_config_path();
        if !path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let updated = update_gem_source(&content, "https://rubygems.org");
        std::fs::write(&path, updated).map_err(|e| format!("Failed to write .gemrc: {e}"))
    }
}

pub fn parse_gem_source(content: &str) -> Option<String> {
    let mut in_sources = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') {
            continue;
        }
        if trimmed == "sources:" || trimmed == ":sources:" {
            in_sources = true;
            continue;
        }
        if in_sources {
            if trimmed.starts_with('-') {
                let url = trimmed.trim_start_matches('-').trim();
                return Some(url.to_string());
            } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                in_sources = false;
            }
        }
    }
    None
}

pub fn update_gem_source(content: &str, new_source: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    let mut in_sources = false;
    let mut sources_found = false;

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed == "sources:" || trimmed == ":sources:" {
            in_sources = true;
            sources_found = true;
            lines.push(line.to_string());
            lines.push(format!("- {new_source}"));
            continue;
        }

        if in_sources {
            if trimmed.starts_with('-') {
                // skip old source entries
                continue;
            } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                in_sources = false;
            }
        }

        lines.push(line.to_string());
    }

    if !sources_found {
        if !lines.iter().any(|l| l.trim() == "---") {
            lines.insert(0, "---".to_string());
        }
        lines.push(":sources:".to_string());
        lines.push(format!("- {new_source}"));
    }

    let mut out = lines.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gem_source_update() {
        let updated = update_gem_source("", "https://gems.ruby-china.com");
        assert!(updated.contains(":sources:"));
        assert!(updated.contains("- https://gems.ruby-china.com"));

        let existing = "---\n:sources:\n- https://old.gem/\ngem: --no-document\n";
        let updated2 = update_gem_source(existing, "https://rubygems.org");
        assert!(updated2.contains("- https://rubygems.org"));
        assert!(!updated2.contains("old.gem"));
        assert!(updated2.contains("gem: --no-document"));
    }
}
