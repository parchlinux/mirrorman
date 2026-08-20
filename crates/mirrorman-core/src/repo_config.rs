use std::collections::HashMap;

const PACMAN_CONF: &str = "/etc/pacman.conf";

pub struct RepoConfig {
    pub pacman_conf: &'static str,
    pub standard_repos: Vec<String>,
    pub third_party_repos: Vec<String>,
    pub custom_repos: Vec<String>,
    pub repositories: HashMap<String, bool>,
}

impl RepoConfig {
    pub fn new() -> Self {
        let standard_repos = vec![
            "core".to_string(),
            "extra".to_string(),
            "multilib".to_string(),
        ];
        let third_party_repos = vec![
            "chaotic-aur".to_string(),
            "blackarch".to_string(),
            "archlinuxcn".to_string(),
        ];

        let mut repositories = HashMap::new();
        for repo in standard_repos.iter().chain(third_party_repos.iter()) {
            repositories.insert(repo.clone(), false);
        }

        let mut config = Self {
            pacman_conf: PACMAN_CONF,
            standard_repos,
            third_party_repos,
            custom_repos: Vec::new(),
            repositories,
        };

        config.load_pacman_conf();
        config
    }

    fn load_pacman_conf(&mut self) {
        let content = match std::fs::read_to_string(self.pacman_conf) {
            Ok(c) => c,
            Err(_) => return,
        };

        let repo_pattern = regex_lite::Regex::new(r"^\s*(#?)\s*\[([^\]]+)\]")
            .expect("valid regex");

        for line in content.lines() {
            if let Some(caps) = repo_pattern.captures(line) {
                let repo_name = caps.get(2).expect("regex capture group 2").as_str().to_string();
                if repo_name == "options" {
                    continue;
                }
                let is_commented = caps.get(1).expect("regex capture group 1").as_str() == "#";
                let enabled = !is_commented;

                if self.repositories.contains_key(&repo_name) {
                    self.repositories.insert(repo_name, enabled);
                }
            }
        }
    }

    pub fn toggle_repo_in_config(
        &mut self,
        repo_name: &str,
        enable: bool,
        is_third_party: bool,
    ) -> Result<(), String> {

        let config_text =
            std::fs::read_to_string(self.pacman_conf).map_err(|e| format!("Failed to read pacman.conf: {e}"))?;

        let section_snippet = if is_third_party {
            get_third_party_section(repo_name)
        } else {
            None
        };

        let modified = toggle_repo_text(&config_text, repo_name, enable, section_snippet.as_deref());
        crate::helper_client::HelperClient::save_pacman_conf(&modified)?;

        self.repositories.insert(repo_name.to_string(), enable);
        Ok(())
    }

    pub fn add_repository(&mut self, repo_name: &str, repo_url: &str, siglevel: &str) -> Result<(), String> {
        if repo_name.is_empty() || repo_url.is_empty() {
            return Err("Repository name and URL are required".to_string());
        }
        if self.repositories.contains_key(repo_name) {
            return Err(format!("Repository already exists: '{repo_name}'"));
        }

        let config_text = if let Ok(c) = std::fs::read_to_string(self.pacman_conf) {
            c
        } else {
            String::new()
        };

        let sig_line = if siglevel.is_empty() {
            String::new()
        } else {
            format!("SigLevel = {siglevel}\n")
        };
        let modified = format!("{config_text}\n[{repo_name}]\nServer = {repo_url}\n{sig_line}");
        crate::helper_client::HelperClient::save_pacman_conf(&modified)?;

        self.repositories.insert(repo_name.to_string(), true);
        self.custom_repos.push(repo_name.to_string());
        Ok(())
    }

    pub fn remove_repository(&mut self, repo_name: &str) -> Result<(), String> {
        if !self.custom_repos.iter().any(|r| r == repo_name) {
            return Err(format!("'{repo_name}' is not a custom repository"));
        }

        let config_text = std::fs::read_to_string(self.pacman_conf)
            .map_err(|e| format!("Failed to read pacman.conf: {e}"))?;

        let modified = remove_repo_text(&config_text, repo_name);
        crate::helper_client::HelperClient::save_pacman_conf(&modified)?;

        self.repositories.remove(repo_name);
        self.custom_repos.retain(|r| r != repo_name);
        Ok(())
    }

    pub fn enable_third_party(&self, repo_name: &str) -> Result<(), String> {
        match repo_name {
            "chaotic-aur" => enable_chaotic_aur(),
            "blackarch" => enable_blackarch(),
            "archlinuxcn" => enable_archlinuxcn(),
            _ => Err(format!("Unknown third-party repo: {repo_name}")),
        }
    }
}

fn toggle_repo_text(config_text: &str, repo_name: &str, enable: bool, section_snippet: Option<&str>) -> String {
    let mut new_lines = Vec::new();
    let mut found_section = false;
    let mut in_section = false;
    let section_header = format!("[{repo_name}]");
    let snippet_lines: Vec<&str> = section_snippet.map(|s| s.lines().collect()).unwrap_or_default();

    for line in config_text.lines() {
        let stripped = line.trim();
        let header_check = stripped.trim_start_matches('#').trim();
        if header_check == section_header {
            found_section = true;
            in_section = true;
            if enable {
                new_lines.push(line.trim_start_matches('#').to_string());
                // Add replacement lines from snippet (skip the header line)
                if snippet_lines.len() > 1 {
                    for sl in &snippet_lines[1..] {
                        if sl.starts_with("Include =") || sl.starts_with("Server =") || sl.starts_with("SigLevel =") {
                            new_lines.push(sl.to_string());
                        }
                    }
                }
            } else {
                if !line.starts_with('#') {
                    new_lines.push(format!("#{line}"));
                } else {
                    new_lines.push(line.to_string());
                }
            }
            continue;
        }

        if in_section {
            let header_uncommented = stripped.trim_start_matches('#').trim();
            if header_uncommented.starts_with('[') && header_uncommented != section_header {
                in_section = false;
                new_lines.push(line.to_string());
                continue;
            }
            if header_uncommented.starts_with("Include =") || header_uncommented.starts_with("Server =") || header_uncommented.starts_with("SigLevel =") {
                if !enable {
                    if !line.starts_with('#') {
                        new_lines.push(format!("#{line}"));
                    } else {
                        new_lines.push(line.to_string());
                    }
                } else if snippet_lines.is_empty() {
                    new_lines.push(line.trim_start_matches('#').to_string());
                }
                // When enabling with snippet: skip old line (replaced above)
                // When enabling without snippet (standard repos): keep existing line
            } else {
                new_lines.push(line.to_string());
            }
        } else {
            new_lines.push(line.to_string());
        }
    }

    if enable && !found_section {
        if let Some(snippet) = section_snippet {
            new_lines.push(String::new());
            for snippet_line in snippet.lines() {
                new_lines.push(snippet_line.to_string());
            }
        }
    }

    new_lines.push(String::new());
    new_lines.join("\n")
}

fn remove_repo_text(config_text: &str, repo_name: &str) -> String {
    let section_header = format!("[{repo_name}]");
    let mut new_lines = Vec::new();
    let mut in_section = false;
    let mut skip_next_blank = false;

    for line in config_text.lines() {
        let stripped = line.trim();
        let header_check = stripped.trim_start_matches('#').trim();
        if header_check == section_header {
            in_section = true;
            skip_next_blank = true;
            continue;
        }

        if in_section {
            let header_uncommented = stripped.trim_start_matches('#').trim();
            if header_uncommented.starts_with('[') && header_uncommented != section_header {
                in_section = false;
                if skip_next_blank && new_lines.last().map_or(false, |l: &String| l.is_empty()) {
                    new_lines.pop();
                }
                new_lines.push(line.to_string());
                continue;
            }
            continue;
        }

        new_lines.push(line.to_string());
    }

    if skip_next_blank && new_lines.last().map_or(false, |l: &String| l.is_empty()) {
        new_lines.pop();
    }
    new_lines.push(String::new());
    new_lines.join("\n")
}

fn get_third_party_section(repo_name: &str) -> Option<String> {
    match repo_name {
        "chaotic-aur" => Some("[chaotic-aur]\nSigLevel = Optional TrustAll\nInclude = /etc/pacman.d/chaotic-mirrorlist\n".to_string()),
        "blackarch" => Some(
            "[blackarch]\nSigLevel = Optional\nServer = https://blackarch.org/blackarch/$repo/os/$arch\n".to_string(),
        ),
        "archlinuxcn" => {
            Some("[archlinuxcn]\nSigLevel = Optional TrustAll\nServer = https://repo.archlinuxcn.org/$arch\n".to_string())
        }
        _ => None,
    }
}

fn enable_chaotic_aur() -> Result<(), String> {
    run_cmd(
        "pacman-key",
        &[
            "--recv-key",
            "3056513887B78AEB",
            "--keyserver",
            "keyserver.ubuntu.com",
        ],
    )?;
    run_cmd("pacman-key", &["--lsign-key", "3056513887B78AEB"])?;
    run_cmd(
        "pacman",
        &[
            "-U",
            "--noconfirm",
            "https://cdn-mirror.chaotic.cx/chaotic-aur/chaotic-keyring.pkg.tar.zst",
            "https://cdn-mirror.chaotic.cx/chaotic-aur/chaotic-mirrorlist.pkg.tar.zst",
        ],
    )?;
    Ok(())
}

fn enable_blackarch() -> Result<(), String> {
    let (success, _, stderr) = crate::helper_client::HelperClient::run_blackarch_strap()?;
    if success {
        Ok(())
    } else {
        Err(format!("BlackArch strap failed: {stderr}"))
    }
}

fn enable_archlinuxcn() -> Result<(), String> {
    run_cmd(
        "pacman-key",
        &[
            "--recv-key",
            "4D41FD3D9E72E7966A573093E8CA6AEB220E236C",
            "--keyserver",
            "keyserver.ubuntu.com",
        ],
    )?;
    run_cmd(
        "pacman-key",
        &["--lsign-key", "4D41FD3D9E72E7966A573093E8CA6AEB220E236C"],
    )?;
    run_cmd("pacman", &["-S", "archlinuxcn-keyring", "--noconfirm"])?;
    Ok(())
}

fn run_cmd(command: &str, args: &[&str]) -> Result<(), String> {
    if command == "pacman" {
        let (success, _, stderr) = crate::helper_client::HelperClient::run_pacman(args)?;
        if success {
            Ok(())
        } else {
            Err(format!("pacman failed: {stderr}"))
        }
    } else {
        let (success, _, stderr) = crate::helper_client::HelperClient::run_command(command, args)?;
        if success {
            Ok(())
        } else {
            Err(format!("Command '{command}' failed: {stderr}"))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn toggle_enable_commented_section() {
        let config = "[options]\n#ParallelDownloads = 5\n\n#[core]\nInclude = /etc/pacman.d/mirrorlist\n\n[extra]\nInclude = /etc/pacman.d/mirrorlist\n";
        let result = toggle_repo_text(config, "core", true, None);
        assert!(result.contains("[core]"));
        assert!(!result.contains("#[core]"));
        assert!(result.contains("Include = /etc/pacman.d/mirrorlist"));
    }

    #[test]
    fn toggle_disable_uncommented_section() {
        let config = "[options]\n\n[core]\nInclude = /etc/pacman.d/mirrorlist\n\n[extra]\nInclude = /etc/pacman.d/mirrorlist\n";
        let result = toggle_repo_text(config, "core", false, None);
        assert!(result.contains("#[core]"));
        assert!(result.contains("#Include = /etc/pacman.d/mirrorlist"));
    }

    #[test]
    fn toggle_already_enabled_no_change() {
        let config = "[options]\n\n[core]\nInclude = /etc/pacman.d/mirrorlist\n";
        let result = toggle_repo_text(config, "core", true, None);
        assert!(result.contains("[core]\nInclude = /etc/pacman.d/mirrorlist"));
    }

    #[test]
    fn toggle_already_disabled_no_change() {
        let config = "[options]\n\n#[core]\n#Include = /etc/pacman.d/mirrorlist\n";
        let result = toggle_repo_text(config, "core", false, None);
        assert!(result.contains("#[core]"));
        assert!(result.contains("#Include = /etc/pacman.d/mirrorlist"));
    }

    #[test]
    fn toggle_third_party_with_snippet() {
        let config = "[options]\n\n[core]\nInclude = /etc/pacman.d/mirrorlist\n";
        let snippet = "[chaotic-aur]\nSigLevel = Optional TrustAll\nInclude = /etc/pacman.d/chaotic-mirrorlist\n";
        let result = toggle_repo_text(config, "chaotic-aur", true, Some(snippet));
        assert!(result.contains("[chaotic-aur]"));
        assert!(result.contains("SigLevel = Optional TrustAll"));
        assert!(result.contains("Include = /etc/pacman.d/chaotic-mirrorlist"));
    }

    #[test]
    fn toggle_nonexistent_section_adds_it() {
        let config = "[options]\n\n[core]\nInclude = /etc/pacman.d/mirrorlist\n";
        let snippet = "[myrepo]\nServer = https://example.com/$repo/os/$arch\n";
        let result = toggle_repo_text(config, "myrepo", true, Some(snippet));
        assert!(result.contains("[myrepo]"));
        assert!(result.contains("Server = https://example.com/$repo/os/$arch"));
    }

    #[test]
    fn toggle_preserves_other_sections() {
        let config = "[options]\n\n[core]\nInclude = /etc/pacman.d/mirrorlist\n\n[extra]\nInclude = /etc/pacman.d/mirrorlist\n\n[multilib]\nInclude = /etc/pacman.d/mirrorlist\n";
        let result = toggle_repo_text(config, "core", false, None);
        assert!(result.contains("[extra]\nInclude = /etc/pacman.d/mirrorlist"));
        assert!(result.contains("[multilib]\nInclude = /etc/pacman.d/mirrorlist"));
    }

    #[test]
    fn toggle_with_siglevel() {
        let config = "[options]\n\n[core]\nInclude = /etc/pacman.d/mirrorlist\n";
        let snippet = "[myrepo]\nSigLevel = Required\nServer = https://example.com/\n";
        let result = toggle_repo_text(config, "myrepo", true, Some(snippet));
        assert!(result.contains("SigLevel = Required"));
        assert!(result.contains("Server = https://example.com/"));
    }

    // --- remove_repo_text tests ---

    #[test]
    fn remove_existing_repo() {
        let config = "[options]\n\n[core]\nInclude = /etc/pacman.d/mirrorlist\n\n[extra]\nInclude = /etc/pacman.d/mirrorlist\n\n[chaotic-aur]\nSigLevel = Optional TrustAll\nInclude = /etc/pacman.d/chaotic-mirrorlist\n";
        let result = remove_repo_text(config, "chaotic-aur");
        assert!(!result.contains("[chaotic-aur]"));
        assert!(!result.contains("chaotic-mirrorlist"));
        assert!(result.contains("[core]\nInclude = /etc/pacman.d/mirrorlist"));
        assert!(result.contains("[extra]\nInclude = /etc/pacman.d/mirrorlist"));
    }

    #[test]
    fn remove_repo_preserves_surrounding() {
        let config = "[options]\n\n[core]\nInclude = /etc/pacman.d/mirrorlist\n\n[myrepo]\nServer = https://example.com/\n\n[multilib]\nInclude = /etc/pacman.d/mirrorlist\n";
        let result = remove_repo_text(config, "myrepo");
        assert!(!result.contains("[myrepo]"));
        assert!(result.contains("[core]\nInclude = /etc/pacman.d/mirrorlist"));
        assert!(result.contains("[multilib]\nInclude = /etc/pacman.d/mirrorlist"));
    }

    #[test]
    fn remove_nonexistent_repo_unchanged() {
        let config = "[options]\n\n[core]\nInclude = /etc/pacman.d/mirrorlist\n";
        let result = remove_repo_text(config, "nonexistent");
        assert_eq!(result.trim(), config.trim());
    }

    #[test]
    fn remove_commented_section() {
        let config = "[options]\n\n#[chaotic-aur]\n#SigLevel = Optional TrustAll\n#Include = /etc/pacman.d/chaotic-mirrorlist\n\n[core]\nInclude = /etc/pacman.d/mirrorlist\n";
        let result = remove_repo_text(config, "chaotic-aur");
        assert!(!result.contains("chaotic-aur"));
        assert!(result.contains("[core]\nInclude = /etc/pacman.d/mirrorlist"));
    }

    #[test]
    fn remove_repo_at_end() {
        let config = "[options]\n\n[core]\nInclude = /etc/pacman.d/mirrorlist\n\n[myrepo]\nServer = https://example.com/\n";
        let result = remove_repo_text(config, "myrepo");
        assert!(!result.contains("[myrepo]"));
        assert!(result.contains("[core]\nInclude = /etc/pacman.d/mirrorlist"));
    }
}
