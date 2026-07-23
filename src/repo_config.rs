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

        let repo_pattern = regex_lite::Regex::new(r"^\s*(#?)\s*\[([^\]]+)\]").unwrap();

        for line in content.lines() {
            if let Some(caps) = repo_pattern.captures(line) {
                let repo_name = caps.get(2).unwrap().as_str().to_string();
                if repo_name == "options" {
                    continue;
                }
                let is_commented = caps.get(1).unwrap().as_str() == "#";
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
                // Add replacement lines from snippet
                for sl in &snippet_lines[1..] {
                    if sl.starts_with("Include =") || sl.starts_with("Server =") || sl.starts_with("SigLevel =") {
                        new_lines.push(sl.to_string());
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
    run_cmd(
        "bash",
        &[
            "-c",
            "cd /tmp && curl -O https://blackarch.org/strap.sh && echo '26849980b35a42e6e192c6d9ed8c46f0d6d06047  strap.sh' | sha1sum -c && chmod +x strap.sh && ./strap.sh && rm -f strap.sh",
        ],
    )?;
    Ok(())
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
