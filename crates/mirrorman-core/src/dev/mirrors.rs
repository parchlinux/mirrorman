use std::collections::HashMap;
use std::path::PathBuf;

use super::models::{DevEcosystemKind, DevMirror};

const DEFAULT_MIRRORS_JSON: &str = include_str!("data/dev_mirrors.json");

pub fn user_custom_mirrors_path() -> PathBuf {
    let base = std::env::var("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| {
            let home = std::env::var("HOME").unwrap_or_else(|_| "/tmp".to_string());
            PathBuf::from(home).join(".config")
        });
    base.join("mirrorman").join("dev_mirrors.json")
}

pub fn load_curated_mirrors() -> HashMap<String, Vec<DevMirror>> {
    let mut map: HashMap<String, Vec<DevMirror>> =
        serde_json::from_str(DEFAULT_MIRRORS_JSON).unwrap_or_default();

    // Check for user overrides
    let custom_path = user_custom_mirrors_path();
    if custom_path.exists() {
        if let Ok(content) = std::fs::read_to_string(&custom_path) {
            if let Ok(custom_map) = serde_json::from_str::<HashMap<String, Vec<DevMirror>>>(&content) {
                for (eco, custom_mirrors) in custom_map {
                    let entry = map.entry(eco).or_default();
                    for cm in custom_mirrors {
                        if !entry.iter().any(|m| m.url == cm.url) {
                            entry.push(cm);
                        }
                    }
                }
            }
        }
    }

    map
}

pub fn get_mirrors_for_ecosystem(kind: DevEcosystemKind) -> Vec<DevMirror> {
    let map = load_curated_mirrors();
    map.get(kind.as_str()).cloned().unwrap_or_default()
}
