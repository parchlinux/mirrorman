pub mod pip;
pub mod npm;
pub mod cargo;
pub mod go;
pub mod gem;
pub mod composer;

use std::path::PathBuf;
use super::models::{DevEcosystemKind, DevMirror, EcosystemStatus};

pub trait EcosystemDetector: Send + Sync {
    fn kind(&self) -> DevEcosystemKind;
    fn is_installed(&self) -> bool;
    fn get_config_path(&self) -> Option<PathBuf>;
    fn get_current_mirror(&self) -> Option<String>;
    fn set_mirror(&self, mirror: &DevMirror) -> Result<(), String>;
    fn reset_mirror(&self) -> Result<(), String>;

    fn get_status(&self, candidate_mirrors: Vec<DevMirror>) -> EcosystemStatus {
        let is_installed = self.is_installed();
        let current_mirror = if is_installed {
            self.get_current_mirror()
        } else {
            None
        };
        let config_path = self.get_config_path().map(|p| p.to_string_lossy().to_string());

        let active_mirror_name = if let Some(ref curr) = current_mirror {
            candidate_mirrors
                .iter()
                .find(|m| m.url.trim_end_matches('/') == curr.trim_end_matches('/'))
                .map(|m| m.name.clone())
        } else {
            None
        };

        EcosystemStatus {
            kind: self.kind(),
            name: self.kind().display_name().to_string(),
            command: self.kind().command_name().to_string(),
            is_installed,
            config_path,
            current_mirror,
            active_mirror_name,
            mirrors: candidate_mirrors,
        }
    }
}

pub fn is_command_in_path(cmd: &str) -> bool {
    if let Ok(path) = std::env::var("PATH") {
        for dir in std::env::split_paths(&path) {
            let full = dir.join(cmd);
            if full.is_file() {
                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(meta) = full.metadata() {
                        if meta.permissions().mode() & 0o111 != 0 {
                            return true;
                        }
                    }
                }
                #[cfg(not(unix))]
                return true;
            }
        }
    }
    false
}
