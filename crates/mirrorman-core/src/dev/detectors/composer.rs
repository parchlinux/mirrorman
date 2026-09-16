use std::path::PathBuf;

use super::{is_command_in_path, EcosystemDetector};
use crate::dev::models::{DevEcosystemKind, DevMirror};

pub struct ComposerDetector;

impl ComposerDetector {
    pub fn new() -> Self {
        Self
    }

    fn resolve_config_path() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
        let base = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(home).join(".config"));
        base.join("composer").join("config.json")
    }
}

impl EcosystemDetector for ComposerDetector {
    fn kind(&self) -> DevEcosystemKind {
        DevEcosystemKind::Composer
    }

    fn is_installed(&self) -> bool {
        is_command_in_path("composer")
    }

    fn get_config_path(&self) -> Option<PathBuf> {
        Some(Self::resolve_config_path())
    }

    fn get_current_mirror(&self) -> Option<String> {
        let path = Self::resolve_config_path();
        if !path.exists() {
            return Some("https://repo.packagist.org".to_string());
        }

        let content = std::fs::read_to_string(&path).ok()?;
        parse_composer_mirror(&content).or_else(|| Some("https://repo.packagist.org".to_string()))
    }

    fn set_mirror(&self, mirror: &DevMirror) -> Result<(), String> {
        let path = Self::resolve_config_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }

        let content = if path.exists() {
            std::fs::read_to_string(&path).unwrap_or_default()
        } else {
            String::new()
        };

        let updated = update_composer_mirror(&content, &mirror.url)?;
        std::fs::write(&path, updated).map_err(|e| format!("Failed to write composer config: {e}"))
    }

    fn reset_mirror(&self) -> Result<(), String> {
        let path = Self::resolve_config_path();
        if !path.exists() {
            return Ok(());
        }

        let content = std::fs::read_to_string(&path).unwrap_or_default();
        let updated = reset_composer_mirror(&content)?;
        std::fs::write(&path, updated).map_err(|e| format!("Failed to write composer config: {e}"))
    }
}

pub fn parse_composer_mirror(content: &str) -> Option<String> {
    let val: serde_json::Value = serde_json::from_str(content).ok()?;
    val.get("repositories")?
        .get("packagist")?
        .get("url")?
        .as_str()
        .map(|s| s.to_string())
}

pub fn update_composer_mirror(content: &str, new_url: &str) -> Result<String, String> {
    let mut val: serde_json::Value = if content.trim().is_empty() {
        serde_json::json!({})
    } else {
        serde_json::from_str(content).map_err(|e| format!("Invalid JSON: {e}"))?
    };

    let repos = val
        .as_object_mut()
        .ok_or_else(|| "Root is not a JSON object".to_string())?
        .entry("repositories")
        .or_insert_with(|| serde_json::json!({}))
        .as_object_mut()
        .ok_or_else(|| "'repositories' is not an object".to_string())?;

    repos.insert(
        "packagist".to_string(),
        serde_json::json!({
            "type": "composer",
            "url": new_url
        }),
    );

    serde_json::to_string_pretty(&val).map_err(|e| format!("Serialization error: {e}"))
}

pub fn reset_composer_mirror(content: &str) -> Result<String, String> {
    if content.trim().is_empty() {
        return Ok(String::new());
    }

    let mut val: serde_json::Value =
        serde_json::from_str(content).map_err(|e| format!("Invalid JSON: {e}"))?;

    if let Some(root) = val.as_object_mut() {
        if let Some(repos) = root.get_mut("repositories").and_then(|r| r.as_object_mut()) {
            repos.remove("packagist");
            if repos.is_empty() {
                root.remove("repositories");
            }
        }
    }

    serde_json::to_string_pretty(&val).map_err(|e| format!("Serialization error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_composer_update_and_reset() {
        let updated = update_composer_mirror("", "https://package-mirror.liara.ir/repository/composer/").unwrap();
        assert_eq!(
            parse_composer_mirror(&updated),
            Some("https://package-mirror.liara.ir/repository/composer/".to_string())
        );

        let reset = reset_composer_mirror(&updated).unwrap();
        assert_eq!(parse_composer_mirror(&reset), None);
    }
}
