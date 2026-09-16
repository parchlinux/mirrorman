use std::path::PathBuf;

use super::{is_command_in_path, EcosystemDetector};
use crate::dev::models::{DevEcosystemKind, DevMirror};

pub struct PipDetector;

impl PipDetector {
    pub fn new() -> Self {
        Self
    }

    fn resolve_config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let legacy = PathBuf::from(&home).join(".pip").join("pip.conf");
        if legacy.exists() {
            return legacy;
        }

        let xdg_config = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(&home).join(".config"));
        xdg_config.join("pip").join("pip.conf")
    }
}

impl EcosystemDetector for PipDetector {
    fn kind(&self) -> DevEcosystemKind {
        DevEcosystemKind::Pip
    }

    fn is_installed(&self) -> bool {
        is_command_in_path("pip") || is_command_in_path("pip3")
    }

    fn get_config_path(&self) -> Option<PathBuf> {
        Some(Self::resolve_config_path())
    }

    fn get_current_mirror(&self) -> Option<String> {
        let path = Self::resolve_config_path();
        if !path.exists() {
            return Some("https://pypi.org/simple".to_string());
        }

        let content = std::fs::read_to_string(&path).ok()?;
        parse_pip_index_url(&content).or_else(|| Some("https://pypi.org/simple".to_string()))
    }

    fn set_mirror(&self, mirror: &DevMirror) -> Result<(), String> {
        let path = Self::resolve_config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create pip config dir: {e}"))?;
        }

        let content = if path.exists() {
            std::fs::read_to_string(&path).unwrap_or_default()
        } else {
            String::new()
        };

        let updated = update_pip_config(&content, &mirror.url, mirror.trusted_host.as_deref());
        std::fs::write(&path, updated).map_err(|e| format!("Failed to write pip config: {e}"))
    }

    fn reset_mirror(&self) -> Result<(), String> {
        let path = Self::resolve_config_path();
        if !path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let updated = reset_pip_config(&content);
        if updated.trim().is_empty() {
            let _ = std::fs::remove_file(&path);
        } else {
            std::fs::write(&path, updated).map_err(|e| format!("Failed to write pip config: {e}"))?;
        }
        Ok(())
    }
}

pub fn parse_pip_index_url(content: &str) -> Option<String> {
    let mut in_global = false;
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.starts_with(';') {
            continue;
        }
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let sec = &trimmed[1..trimmed.len() - 1].trim();
            in_global = sec.eq_ignore_ascii_case("global");
            continue;
        }
        if in_global {
            if let Some((k, v)) = trimmed.split_once('=') {
                if k.trim().eq_ignore_ascii_case("index-url") {
                    return Some(v.trim().to_string());
                }
            }
        }
    }
    None
}

pub fn update_pip_config(content: &str, new_index_url: &str, trusted_host: Option<&str>) -> String {
    let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let mut global_start: Option<usize> = None;
    let mut global_end: Option<usize> = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let sec = &trimmed[1..trimmed.len() - 1].trim();
            if sec.eq_ignore_ascii_case("global") {
                global_start = Some(i);
            } else if global_start.is_some() && global_end.is_none() {
                global_end = Some(i);
            }
        }
    }

    if global_start.is_none() {
        // No [global] section, create one at the beginning
        let mut out = String::new();
        out.push_str("[global]\n");
        out.push_str(&format!("index-url = {new_index_url}\n"));
        if let Some(th) = trusted_host {
            out.push_str(&format!("trusted-host = {th}\n"));
        }
        out.push('\n');
        out.push_str(content.trim_start());
        return out;
    }

    let start = global_start.unwrap();
    let end = global_end.unwrap_or(lines.len());

    // Remove existing index-url and trusted-host in [global]
    let mut filtered_section = Vec::new();
    for line in &lines[start + 1..end] {
        let trimmed = line.trim();
        let is_target_key = if let Some((k, _)) = trimmed.split_once('=') {
            let key = k.trim().to_ascii_lowercase();
            key == "index-url" || key == "trusted-host"
        } else {
            false
        };
        if !is_target_key {
            filtered_section.push(line.clone());
        }
    }

    // Add new keys
    filtered_section.push(format!("index-url = {new_index_url}"));
    if let Some(th) = trusted_host {
        filtered_section.push(format!("trusted-host = {th}"));
    }

    let mut result = Vec::new();
    result.extend_from_slice(&lines[..=start]);
    result.extend(filtered_section);
    result.extend_from_slice(&lines[end..]);

    let mut out = result.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

pub fn reset_pip_config(content: &str) -> String {
    let lines: Vec<String> = content.lines().map(|s| s.to_string()).collect();
    let mut global_start: Option<usize> = None;
    let mut global_end: Option<usize> = None;

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('[') && trimmed.ends_with(']') {
            let sec = &trimmed[1..trimmed.len() - 1].trim();
            if sec.eq_ignore_ascii_case("global") {
                global_start = Some(i);
            } else if global_start.is_some() && global_end.is_none() {
                global_end = Some(i);
            }
        }
    }

    if let Some(start) = global_start {
        let end = global_end.unwrap_or(lines.len());
        let mut filtered_section = Vec::new();
        for line in &lines[start + 1..end] {
            let trimmed = line.trim();
            let is_target_key = if let Some((k, _)) = trimmed.split_once('=') {
                let key = k.trim().to_ascii_lowercase();
                key == "index-url" || key == "trusted-host"
            } else {
                false
            };
            if !is_target_key {
                filtered_section.push(line.clone());
            }
        }

        let mut result = Vec::new();
        result.extend_from_slice(&lines[..start]);
        if !filtered_section.iter().all(|l| l.trim().is_empty()) {
            result.push(lines[start].clone());
            result.extend(filtered_section);
        }
        result.extend_from_slice(&lines[end..]);

        let mut out = result.join("\n");
        if !out.is_empty() && !out.ends_with('\n') {
            out.push('\n');
        }
        return out;
    }

    content.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_update_pip_config_from_empty() {
        let updated = update_pip_config("", "https://mirror.ir/simple", Some("mirror.ir"));
        assert!(updated.contains("[global]"));
        assert!(updated.contains("index-url = https://mirror.ir/simple"));
        assert!(updated.contains("trusted-host = mirror.ir"));
    }

    #[test]
    fn test_update_pip_config_preserves_other_sections() {
        let initial = "[install]\ntimeout = 60\n\n[global]\nindex-url = https://old.mirror/simple\n";
        let updated = update_pip_config(initial, "https://new.mirror/simple", None);
        assert!(updated.contains("[install]"));
        assert!(updated.contains("timeout = 60"));
        assert!(updated.contains("index-url = https://new.mirror/simple"));
        assert!(!updated.contains("old.mirror"));
    }

    #[test]
    fn test_reset_pip_config() {
        let initial = "[global]\nindex-url = https://mirror.ir/simple\ntrusted-host = mirror.ir\n";
        let reset = reset_pip_config(initial);
        assert!(!reset.contains("index-url"));
        assert!(!reset.contains("trusted-host"));
    }
}
