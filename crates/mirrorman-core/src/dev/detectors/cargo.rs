use std::path::PathBuf;

use super::{is_command_in_path, EcosystemDetector};
use crate::dev::models::{DevEcosystemKind, DevMirror};

pub struct CargoDetector;

impl CargoDetector {
    pub fn new() -> Self {
        Self
    }

    fn resolve_config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let legacy = PathBuf::from(&home).join(".cargo").join("config");
        if legacy.exists() {
            return legacy;
        }
        PathBuf::from(home).join(".cargo").join("config.toml")
    }
}

impl EcosystemDetector for CargoDetector {
    fn kind(&self) -> DevEcosystemKind {
        DevEcosystemKind::Cargo
    }

    fn is_installed(&self) -> bool {
        is_command_in_path("cargo")
    }

    fn get_config_path(&self) -> Option<PathBuf> {
        Some(Self::resolve_config_path())
    }

    fn get_current_mirror(&self) -> Option<String> {
        let path = Self::resolve_config_path();
        if !path.exists() {
            return Some("https://index.crates.io/".to_string());
        }

        let content = std::fs::read_to_string(&path).ok()?;
        parse_cargo_mirror(&content).or_else(|| Some("https://index.crates.io/".to_string()))
    }

    fn set_mirror(&self, mirror: &DevMirror) -> Result<(), String> {
        if mirror.official {
            return self.reset_mirror();
        }

        let path = Self::resolve_config_path();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)
                .map_err(|e| format!("Failed to create .cargo dir: {e}"))?;
        }

        let content = if path.exists() {
            std::fs::read_to_string(&path).unwrap_or_default()
        } else {
            String::new()
        };

        let updated = update_cargo_mirror(&content, &mirror.url)?;
        std::fs::write(&path, updated).map_err(|e| format!("Failed to write cargo config: {e}"))
    }

    fn reset_mirror(&self) -> Result<(), String> {
        let path = Self::resolve_config_path();
        if !path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let updated = reset_cargo_mirror(&content)?;
        if updated.trim().is_empty() {
            let _ = std::fs::remove_file(&path);
        } else {
            std::fs::write(&path, updated).map_err(|e| format!("Failed to write cargo config: {e}"))?;
        }
        Ok(())
    }
}

pub fn parse_cargo_mirror(content: &str) -> Option<String> {
    let table: toml::Table = content.parse().ok()?;
    let source = table.get("source")?.as_table()?;
    let crates_io = source.get("crates-io")?.as_table()?;
    let replace_with = crates_io.get("replace-with")?.as_str()?;
    let replaced_source = source.get(replace_with)?.as_table()?;
    replaced_source.get("registry")?.as_str().map(|s| s.to_string())
}

pub fn update_cargo_mirror(content: &str, new_registry: &str) -> Result<String, String> {
    let mut table: toml::Table = if content.trim().is_empty() {
        toml::Table::new()
    } else {
        content
            .parse()
            .map_err(|e| format!("Invalid TOML in cargo config: {e}"))?
    };

    let source_table = table
        .entry("source")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .ok_or_else(|| "Failed to access [source] table".to_string())?;

    let crates_io = source_table
        .entry("crates-io")
        .or_insert_with(|| toml::Value::Table(toml::Table::new()))
        .as_table_mut()
        .ok_or_else(|| "Failed to access [source.crates-io] table".to_string())?;

    crates_io.insert(
        "replace-with".to_string(),
        toml::Value::String("mirrorman".to_string()),
    );

    let mut mirrorman_source = toml::Table::new();
    mirrorman_source.insert(
        "registry".to_string(),
        toml::Value::String(new_registry.to_string()),
    );

    source_table.insert(
        "mirrorman".to_string(),
        toml::Value::Table(mirrorman_source),
    );

    toml::to_string_pretty(&table).map_err(|e| format!("Failed to serialize TOML: {e}"))
}

pub fn reset_cargo_mirror(content: &str) -> Result<String, String> {
    if content.trim().is_empty() {
        return Ok(String::new());
    }

    let mut table: toml::Table = content
        .parse()
        .map_err(|e| format!("Invalid TOML in cargo config: {e}"))?;

    if let Some(source) = table.get_mut("source").and_then(|v| v.as_table_mut()) {
        if let Some(crates_io) = source.get_mut("crates-io").and_then(|v| v.as_table_mut()) {
            crates_io.remove("replace-with");
            if crates_io.is_empty() {
                source.remove("crates-io");
            }
        }
        source.remove("mirrorman");
        if source.is_empty() {
            table.remove("source");
        }
    }

    toml::to_string_pretty(&table).map_err(|e| format!("Failed to serialize TOML: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cargo_update_and_parse() {
        let updated = update_cargo_mirror("", "sparse+https://mirrors.ustc.edu.cn/crates.io-index/").unwrap();
        assert!(updated.contains("[source.crates-io]"));
        assert!(updated.contains("replace-with = \"mirrorman\""));
        assert!(updated.contains("sparse+https://mirrors.ustc.edu.cn/crates.io-index/"));

        let parsed = parse_cargo_mirror(&updated);
        assert_eq!(parsed, Some("sparse+https://mirrors.ustc.edu.cn/crates.io-index/".to_string()));
    }

    #[test]
    fn test_cargo_reset() {
        let initial = update_cargo_mirror("", "sparse+https://mirrors.ustc.edu.cn/crates.io-index/").unwrap();
        let reset = reset_cargo_mirror(&initial).unwrap();
        assert_eq!(parse_cargo_mirror(&reset), None);
    }
}
