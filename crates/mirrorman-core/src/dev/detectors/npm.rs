use std::path::PathBuf;

use super::{is_command_in_path, EcosystemDetector};
use crate::dev::models::{DevEcosystemKind, DevMirror};

pub struct NpmDetector;

impl NpmDetector {
    pub fn new() -> Self {
        Self
    }

    fn resolve_config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        PathBuf::from(home).join(".npmrc")
    }
}

impl EcosystemDetector for NpmDetector {
    fn kind(&self) -> DevEcosystemKind {
        DevEcosystemKind::Npm
    }

    fn is_installed(&self) -> bool {
        is_command_in_path("npm")
    }

    fn get_config_path(&self) -> Option<PathBuf> {
        Some(Self::resolve_config_path())
    }

    fn get_current_mirror(&self) -> Option<String> {
        let path = Self::resolve_config_path();
        if !path.exists() {
            return Some("https://registry.npmjs.org/".to_string());
        }

        let content = std::fs::read_to_string(&path).ok()?;
        parse_npm_registry(&content).or_else(|| Some("https://registry.npmjs.org/".to_string()))
    }

    fn set_mirror(&self, mirror: &DevMirror) -> Result<(), String> {
        let path = Self::resolve_config_path();
        let content = if path.exists() {
            std::fs::read_to_string(&path).unwrap_or_default()
        } else {
            String::new()
        };

        let updated = update_npm_registry(&content, &mirror.url);
        std::fs::write(&path, updated).map_err(|e| format!("Failed to write .npmrc: {e}"))
    }

    fn reset_mirror(&self) -> Result<(), String> {
        let path = Self::resolve_config_path();
        if !path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let updated = reset_npm_registry(&content);
        if updated.trim().is_empty() {
            let _ = std::fs::remove_file(&path);
        } else {
            std::fs::write(&path, updated).map_err(|e| format!("Failed to write .npmrc: {e}"))?;
        }
        Ok(())
    }
}

pub fn parse_npm_registry(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if let Some((k, v)) = trimmed.split_once('=') {
            if k.trim().eq_ignore_ascii_case("registry") {
                return Some(v.trim().to_string());
            }
        }
    }
    None
}

pub fn update_npm_registry(content: &str, new_registry: &str) -> String {
    let mut found = false;
    let mut lines: Vec<String> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('#') && !trimmed.starts_with(';') {
            if let Some((k, _)) = trimmed.split_once('=') {
                if k.trim().eq_ignore_ascii_case("registry") {
                    lines.push(format!("registry={new_registry}"));
                    found = true;
                    continue;
                }
            }
        }
        lines.push(line.to_string());
    }

    if !found {
        lines.push(format!("registry={new_registry}"));
    }

    let mut out = lines.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

pub fn reset_npm_registry(content: &str) -> String {
    let mut lines: Vec<String> = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if !trimmed.starts_with('#') && !trimmed.starts_with(';') {
            if let Some((k, _)) = trimmed.split_once('=') {
                if k.trim().eq_ignore_ascii_case("registry") {
                    continue;
                }
            }
        }
        lines.push(line.to_string());
    }

    let mut out = lines.join("\n");
    if !out.is_empty() && !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_npm_registry() {
        let updated = update_npm_registry("", "https://registry.npmmirror.com/");
        assert_eq!(updated.trim(), "registry=https://registry.npmmirror.com/");

        let existing = "save-exact=true\nregistry=https://old.ir/\n";
        let updated2 = update_npm_registry(existing, "https://package-mirror.liara.ir/repository/npm/");
        assert!(updated2.contains("save-exact=true"));
        assert!(updated2.contains("registry=https://package-mirror.liara.ir/repository/npm/"));
        assert!(!updated2.contains("old.ir"));
    }

    #[test]
    fn test_reset_npm_registry() {
        let existing = "registry=https://custom.npm/\nsave-exact=true\n";
        let reset = reset_npm_registry(existing);
        assert!(!reset.contains("registry="));
        assert!(reset.contains("save-exact=true"));
    }
}
