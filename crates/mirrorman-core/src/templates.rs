use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MirrorTemplate {
    pub name: String,
    pub created_at: String,
    pub mirrors: Vec<TemplateMirror>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TemplateMirror {
    pub url: String,
    pub country: String,
    pub protocol: String,
    pub enabled: bool,
}

impl MirrorTemplate {
    fn get_templates_dir() -> PathBuf {
        let home = std::env::var("HOME").unwrap_or_else(|_| "/home".to_string());
        let config_base = std::env::var("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from(format!("{home}/.config")));
        let mut dir = config_base;
        dir.push("mirrorman");
        dir.push("templates");
        let _ = fs::create_dir_all(&dir);
        dir
    }

    pub fn list_all() -> Vec<Self> {
        let dir = Self::get_templates_dir();
        let mut templates = Vec::new();
        if let Ok(entries) = fs::read_dir(dir) {
            for entry in entries.flatten() {
                if entry.path().extension().and_then(|s| s.to_str()) == Some("json") {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        if let Ok(tpl) = serde_json::from_str::<Self>(&content) {
                            templates.push(tpl);
                        }
                    }
                }
            }
        }
        templates.sort_by(|a, b| a.name.cmp(&b.name));
        templates
    }

    pub fn save(name: &str, mirrors: &[crate::mirror_manager::Mirror]) -> Result<(), String> {
        if name.trim().is_empty() {
            return Err("Template name cannot be empty".to_string());
        }
        let mut dir = Self::get_templates_dir();
        let safe_filename = format!("{}.json", name.trim().replace('/', "_").replace(' ', "_"));
        dir.push(safe_filename);

        let template_mirrors = mirrors
            .iter()
            .map(|m| TemplateMirror {
                url: m.url.clone(),
                country: m.country.clone(),
                protocol: m.protocol.clone(),
                enabled: m.enabled,
            })
            .collect();

        let tpl = Self {
            name: name.trim().to_string(),
            created_at: chrono::Local::now().format("%Y-%m-%d %H:%M:%S").to_string(),
            mirrors: template_mirrors,
        };

        let json = serde_json::to_string_pretty(&tpl).map_err(|e| e.to_string())?;
        fs::write(dir, json).map_err(|e| format!("Failed to write template: {e}"))
    }

    pub fn delete(name: &str) -> Result<(), String> {
        let mut dir = Self::get_templates_dir();
        let safe_filename = format!("{}.json", name.trim().replace('/', "_").replace(' ', "_"));
        dir.push(safe_filename);
        if dir.exists() {
            fs::remove_file(dir).map_err(|e| format!("Failed to delete template: {e}"))?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_template_save_and_delete() {
        let mirrors = vec![crate::mirror_manager::Mirror {
            url: "https://test.mirror/".to_string(),
            country: "Germany".to_string(),
            country_code: "DE".to_string(),
            protocol: "https".to_string(),
            speed: Some(20.0),
            last_sync: None,
            enabled: true,
            ipv4: true,
            ipv6: false,
            completion_pct: None,
            score: None,
            duration_avg: None,
            duration_stddev: None,
        }];

        let test_name = "Test_Profile_UnitTest";
        let res = MirrorTemplate::save(test_name, &mirrors);
        assert!(res.is_ok());

        let templates = MirrorTemplate::list_all();
        assert!(templates.iter().any(|t| t.name == test_name));

        let del_res = MirrorTemplate::delete(test_name);
        assert!(del_res.is_ok());
    }
}
