use std::path::PathBuf;
use std::process::Command;

use super::{is_command_in_path, EcosystemDetector};
use crate::dev::models::{DevEcosystemKind, DevMirror};

pub struct GoDetector;

impl GoDetector {
    pub fn new() -> Self {
        Self
    }

    fn resolve_config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let base = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(home).join(".config"));
        base.join("go").join("env")
    }
}

impl EcosystemDetector for GoDetector {
    fn kind(&self) -> DevEcosystemKind {
        DevEcosystemKind::Go
    }

    fn is_installed(&self) -> bool {
        is_command_in_path("go")
    }

    fn get_config_path(&self) -> Option<PathBuf> {
        Some(Self::resolve_config_path())
    }

    fn get_current_mirror(&self) -> Option<String> {
        // Try running go env GOPROXY if available
        if is_command_in_path("go") {
            if let Ok(output) = Command::new("go").args(["env", "GOPROXY"]).output() {
                if output.status.success() {
                    let s = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !s.is_empty() {
                        return Some(s);
                    }
                }
            }
        }

        // Check ~/.config/go/env
        let path = Self::resolve_config_path();
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                if let Some(val) = parse_go_env_file(&content) {
                    return Some(val);
                }
            }
        }

        Some("https://proxy.golang.org,direct".to_string())
    }

    fn set_mirror(&self, mirror: &DevMirror) -> Result<(), String> {
        let val = &mirror.url;

        // Try go env -w
        if is_command_in_path("go") {
            let _ = Command::new("go")
                .args(["env", "-w", &format!("GOPROXY={val}")])
                .output();
        }

        // Also write directly to ~/.config/go/env for persistence
        let path = Self::resolve_config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let content = if path.exists() {
            std::fs::read_to_string(&path).unwrap_or_default()
        } else {
            String::new()
        };

        let updated = update_go_env_file(&content, val);
        std::fs::write(&path, updated).map_err(|e| format!("Failed to write go env: {e}"))
    }

    fn reset_mirror(&self) -> Result<(), String> {
        if is_command_in_path("go") {
            let _ = Command::new("go")
                .args(["env", "-u", "GOPROXY"])
                .output();
        }

        let path = Self::resolve_config_path();
        if path.exists() {
            let content = std::fs::read_to_string(&path).unwrap_or_default();
            let updated = reset_go_env_file(&content);
            if updated.trim().is_empty() {
                let _ = std::fs::remove_file(&path);
            } else {
                let _ = std::fs::write(&path, updated);
            }
        }

        Ok(())
    }
}

pub fn parse_go_env_file(content: &str) -> Option<String> {
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some((k, v)) = trimmed.split_once('=') {
            if k.trim() == "GOPROXY" {
                return Some(v.trim().trim_matches('"').trim_matches('\'').to_string());
            }
        }
    }
    None
}

pub fn update_go_env_file(content: &str, val: &str) -> String {
    let mut found = false;
    let mut lines = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some((k, _)) = trimmed.split_once('=') {
            if k.trim() == "GOPROXY" {
                lines.push(format!("GOPROXY={val}"));
                found = true;
                continue;
            }
        }
        lines.push(line.to_string());
    }

    if !found {
        lines.push(format!("GOPROXY={val}"));
    }

    let mut out = lines.join("\n");
    if !out.ends_with('\n') {
        out.push('\n');
    }
    out
}

pub fn reset_go_env_file(content: &str) -> String {
    let mut lines = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some((k, _)) = trimmed.split_once('=') {
            if k.trim() == "GOPROXY" {
                continue;
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
    fn test_go_env_update() {
        let updated = update_go_env_file("", "https://goproxy.cn,direct");
        assert_eq!(updated.trim(), "GOPROXY=https://goproxy.cn,direct");

        let existing = "GONOSUMDB=github.com/myorg/*\nGOPROXY=direct\n";
        let updated2 = update_go_env_file(existing, "https://package-mirror.liara.ir/repository/go/,direct");
        assert!(updated2.contains("GONOSUMDB="));
        assert!(updated2.contains("GOPROXY=https://package-mirror.liara.ir/repository/go/,direct"));
    }

    #[test]
    fn test_go_env_reset() {
        let existing = "GOPROXY=https://goproxy.cn,direct\nGONOSUMDB=*\n";
        let reset = reset_go_env_file(existing);
        assert!(!reset.contains("GOPROXY="));
        assert!(reset.contains("GONOSUMDB=*"));
    }
}
