use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct DevMirror {
    pub name: String,
    pub url: String,
    pub country: String,
    pub country_code: String,
    pub official: bool,
    pub ping_url: String,
    #[serde(default)]
    pub trusted_host: Option<String>,
    #[serde(default)]
    pub speed: Option<f64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum DevEcosystemKind {
    Pip,
    Npm,
    Cargo,
    Go,
    Gem,
    Composer,
}

impl DevEcosystemKind {
    pub const ALL: [DevEcosystemKind; 6] = [
        DevEcosystemKind::Pip,
        DevEcosystemKind::Npm,
        DevEcosystemKind::Cargo,
        DevEcosystemKind::Go,
        DevEcosystemKind::Gem,
        DevEcosystemKind::Composer,
    ];

    pub fn as_str(&self) -> &'static str {
        match self {
            DevEcosystemKind::Pip => "pip",
            DevEcosystemKind::Npm => "npm",
            DevEcosystemKind::Cargo => "cargo",
            DevEcosystemKind::Go => "go",
            DevEcosystemKind::Gem => "gem",
            DevEcosystemKind::Composer => "composer",
        }
    }

    pub fn display_name(&self) -> &'static str {
        match self {
            DevEcosystemKind::Pip => "Python (pip)",
            DevEcosystemKind::Npm => "Node.js (npm)",
            DevEcosystemKind::Cargo => "Rust (cargo)",
            DevEcosystemKind::Go => "Go (GOPROXY)",
            DevEcosystemKind::Gem => "Ruby (gem)",
            DevEcosystemKind::Composer => "PHP (composer)",
        }
    }

    pub fn command_name(&self) -> &'static str {
        match self {
            DevEcosystemKind::Pip => "pip",
            DevEcosystemKind::Npm => "npm",
            DevEcosystemKind::Cargo => "cargo",
            DevEcosystemKind::Go => "go",
            DevEcosystemKind::Gem => "gem",
            DevEcosystemKind::Composer => "composer",
        }
    }

    pub fn icon_name(&self) -> &'static str {
        match self {
            DevEcosystemKind::Pip => "application-x-python-bytecode",
            DevEcosystemKind::Npm => "application-x-javascript",
            DevEcosystemKind::Cargo => "application-x-rust",
            DevEcosystemKind::Go => "application-x-go",
            DevEcosystemKind::Gem => "application-x-ruby",
            DevEcosystemKind::Composer => "application-x-php",
        }
    }

    pub fn parse(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "pip" | "python" | "pypi" => Some(DevEcosystemKind::Pip),
            "npm" | "node" | "nodejs" => Some(DevEcosystemKind::Npm),
            "cargo" | "rust" | "crates" => Some(DevEcosystemKind::Cargo),
            "go" | "golang" | "goproxy" => Some(DevEcosystemKind::Go),
            "gem" | "ruby" | "rubygems" => Some(DevEcosystemKind::Gem),
            "composer" | "php" | "packagist" => Some(DevEcosystemKind::Composer),
            _ => None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EcosystemStatus {
    pub kind: DevEcosystemKind,
    pub name: String,
    pub command: String,
    pub is_installed: bool,
    pub config_path: Option<String>,
    pub current_mirror: Option<String>,
    pub active_mirror_name: Option<String>,
    pub mirrors: Vec<DevMirror>,
}
