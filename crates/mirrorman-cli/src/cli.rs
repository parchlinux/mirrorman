use mirrorman_core::mirror_manager::{Mirror, MirrorManager};
use clap::{Parser, Subcommand, CommandFactory};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Command-line interface for the Parch Mirror Manager.
#[derive(Parser, Debug)]
#[command(name = "mirrorman-cli", version, about = "Pacman mirror and repository manager CLI")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand, Debug)]
pub enum Command {
    /// Fetch the latest mirror status from the Arch Linux API
    Refresh {
        /// Restrict to these countries (repeatable). Default: all countries
        #[arg(long, short = 'c')]
        country: Vec<String>,
        /// Restrict to these protocols (http, https). Default: http and https
        #[arg(long)]
        protocol: Vec<String>,
        /// Include IPv4-capable mirrors (default when neither --ipv4 nor --ipv6 is given)
        #[arg(long)]
        ipv4: bool,
        /// Include IPv6-capable mirrors
        #[arg(long)]
        ipv6: bool,
        /// Include mirrors that are not up to date
        #[arg(long)]
        no_status: bool,
        /// Print the fetched mirrors as JSON
        #[arg(long)]
        json: bool,
    },
    /// Test the response times of the cached mirrors
    Test {
        /// Number of concurrent speed tests
        #[arg(long, default_value_t = 50)]
        workers: usize,
        /// Print the ranked mirrors as JSON
        #[arg(long)]
        json: bool,
    },
    /// Auto-select the best mirrors across countries and enable them
    BestSetup {
        /// Maximum number of countries/mirrors to select
        #[arg(long, default_value_t = 5)]
        count: usize,
        /// Print the selected mirrors as JSON
        #[arg(long)]
        json: bool,
    },
    /// Save the mirrorlist to /etc/pacman.d/mirrorlist via the D-Bus helper
    Save {
        /// Save even when no mirror is enabled
        #[arg(long)]
        force: bool,
    },
    /// Fetch, auto-select and save the mirrorlist in one step
    Auto {
        /// Save even when no mirror is enabled
        #[arg(long)]
        force: bool,
    },
    /// List the mirrors cached by a previous refresh
    List {
        /// Print the mirrors as JSON
        #[arg(long)]
        json: bool,
    },
    /// Show a summary of the current system mirrorlist
    Status,
    /// Create a timestamped backup of the current mirrorlist
    Backup,
    /// Clean pacman package cache (removes uninstalled + keeps N most recent)
    Clean {
        /// Number of most recent versions to keep per package
        #[arg(long, short = 'n', default_value_t = 2)]
        keep: u32,
        /// Dry run — show what would be removed without doing it
        #[arg(long)]
        dry_run: bool,
    },
    /// Save mirrorlist and sync pacman repositories (equivalent to GUI Sync)
    Sync {
        /// Save even when no mirror is enabled
        #[arg(long)]
        force: bool,
    },
    /// Show the diff between current and proposed mirrorlist
    Diff,
    /// Test a single mirror URL for speed
    TestMirror {
        /// The mirror URL to test
        url: String,
    },
    /// Generate shell completion scripts
    Completions {
        /// The shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,
    },
    /// Export current mirrorlist to a file path
    Export {
        /// Output file path (default: stdout)
        #[arg(long, short = 'o')]
        output: Option<PathBuf>,
    },
    /// Manage developer CLI package managers (pip, npm, cargo, go, gem, composer)
    Dev {
        #[command(subcommand)]
        subcommand: DevCommand,
    },
}

#[derive(Subcommand, Debug)]
pub enum DevCommand {
    /// Scan detected developer package managers and show current mirror
    Scan {
        /// Print as JSON
        #[arg(long)]
        json: bool,
    },
    /// List candidate mirrors for an ecosystem
    List {
        /// The package manager (pip, npm, cargo, go, gem, composer)
        ecosystem: String,
        /// Print as JSON
        #[arg(long)]
        json: bool,
    },
    /// Test response latency of candidate mirrors
    Test {
        /// The package manager (pip, npm, cargo, go, gem, composer)
        ecosystem: String,
        /// Number of concurrent workers
        #[arg(long, default_value_t = 10)]
        workers: usize,
        /// Print as JSON
        #[arg(long)]
        json: bool,
    },
    /// Set the active mirror for a package manager
    Set {
        /// The package manager (pip, npm, cargo, go, gem, composer)
        ecosystem: String,
        /// Mirror name or URL
        mirror: String,
    },
    /// Reset package manager to official upstream default
    Reset {
        /// The package manager (pip, npm, cargo, go, gem, composer)
        ecosystem: String,
    },
    /// Auto-select and configure the fastest responsive mirror for all installed tools
    Auto {
        /// Number of concurrent workers per ecosystem
        #[arg(long, default_value_t = 10)]
        workers: usize,
        /// Print as JSON
        #[arg(long)]
        json: bool,
    },
}

#[derive(Serialize, Deserialize)]
pub struct CachedState {
    pub fetched_at: Option<String>,
    pub mirrors: Vec<Mirror>,
}

/// Cache location; overridable via MIRRORMAN_CACHE (used by tests).
fn cache_path() -> PathBuf {
    if let Ok(p) = std::env::var("MIRRORMAN_CACHE") {
        return PathBuf::from(p);
    }
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home".to_string());
    let cache_base = std::env::var("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|_| PathBuf::from(format!("{home}/.cache")));
    let mut dir = cache_base;
    dir.push("mirrorman");
    let _ = std::fs::create_dir_all(&dir);
    dir.push("mirrors.json");
    dir
}

pub fn load_cached() -> Result<MirrorManager, String> {
    let path = cache_path();
    let content = std::fs::read_to_string(&path)
        .map_err(|_| "No cached mirror data. Run 'mirrorman-cli refresh' first.".to_string())?;
    let state: CachedState =
        serde_json::from_str(&content).map_err(|e| format!("Failed to parse mirror cache: {e}"))?;
    let mut mgr = MirrorManager::new();
    mgr.mirrors = state.mirrors;
    Ok(mgr)
}

pub fn save_cache(mgr: &MirrorManager) -> Result<(), String> {
    let state = CachedState {
        fetched_at: Some(chrono::Local::now().to_rfc3339()),
        mirrors: mgr.mirrors.clone(),
    };
    let json = serde_json::to_string_pretty(&state).map_err(|e| e.to_string())?;
    std::fs::write(cache_path(), json).map_err(|e| format!("Failed to write mirror cache: {e}"))
}

pub fn default_protocols(protocols: &[String]) -> Vec<String> {
    if protocols.is_empty() {
        vec!["http".to_string(), "https".to_string()]
    } else {
        protocols.to_vec()
    }
}

pub fn default_ip_versions(ipv4: bool, ipv6: bool) -> Vec<String> {
    let both = !ipv4 && !ipv6;
    let mut versions = Vec::new();
    if ipv4 || both {
        versions.push("4".to_string());
    }
    if ipv6 || both {
        versions.push("6".to_string());
    }
    versions
}

pub fn do_refresh(
    countries: &[String],
    protocols: &[String],
    ip_versions: &[String],
    use_status: bool,
) -> Result<MirrorManager, String> {
    let mut mgr = MirrorManager::new();
    mgr.fetch_mirrors(countries, protocols, ip_versions, use_status)?;
    save_cache(&mgr)?;
    Ok(mgr)
}

pub fn do_test(mgr: &mut MirrorManager, workers: usize) -> Result<(), String> {
    if mgr.mirrors.is_empty() {
        return Err("No mirrors to test. Run 'mirrorman-cli refresh' first.".to_string());
    }
    MirrorManager::test_all_speeds_concurrent(&mut mgr.mirrors, workers);
    mgr.sort_by_speed();
    save_cache(mgr)
}

pub fn do_best_setup(mgr: &mut MirrorManager, count: usize) -> Result<Vec<Mirror>, String> {
    if mgr.mirrors.is_empty() {
        return Err("No mirrors available. Run 'mirrorman-cli refresh' first.".to_string());
    }
    let selected = mgr.auto_optimize_with_count(count);
    save_cache(mgr)?;
    Ok(selected)
}

pub fn do_save(mgr: &MirrorManager, force: bool) -> Result<(), String> {
    let enabled_count = mgr.mirrors.iter().filter(|m| m.enabled).count();
    if enabled_count == 0 && !force {
        return Err(
            "No enabled mirrors to save. Run 'mirrorman-cli best-setup' first, or use --force."
                .to_string(),
        );
    }
    mirrorman_core::helper_client::HelperClient::save_mirrorlist(&mgr.generate_mirrorlist_content())
}

fn print_mirror_table(mirrors: &[Mirror]) {
    println!("{:<3} {:<6} {:>8}  {:<55} Country", "#", "Proto", "Speed", "URL");
    for (i, m) in mirrors.iter().enumerate() {
        let speed = m
            .speed
            .map(|s| format!("{s:.0}ms"))
            .unwrap_or_else(|| "-".to_string());
        let enabled = if m.enabled { "enabled" } else { "off" };
        println!(
            "{:<3} {:<6} {:>8}  {:<55} {} [{}]",
            i + 1,
            m.protocol,
            speed,
            m.url,
            m.country,
            enabled
        );
    }
}

pub fn execute(cli: Cli) -> Result<(), String> {
    match cli.command {
        Command::Refresh {
            country,
            protocol,
            ipv4,
            ipv6,
            no_status,
            json,
        } => {
            println!("[+] Fetching mirror status...");
            let protocols = default_protocols(&protocol);
            let ip_versions = default_ip_versions(ipv4, ipv6);
            let mgr = do_refresh(&country, &protocols, &ip_versions, !no_status)?;
            println!("[+] Fetched {} mirrors.", mgr.mirrors.len());
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&mgr.mirrors).map_err(|e| e.to_string())?
                );
            } else {
                print_mirror_table(&mgr.mirrors);
            }
            Ok(())
        }
        Command::Test { workers, json } => {
            let mut mgr = load_cached()?;
            println!("[+] Testing {} mirrors with {workers} workers...", mgr.mirrors.len());
            do_test(&mut mgr, workers)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&mgr.mirrors).map_err(|e| e.to_string())?
                );
            } else {
                println!("[+] Ranked by speed:");
                print_mirror_table(&mgr.mirrors);
            }
            Ok(())
        }
        Command::BestSetup { count, json } => {
            let mut mgr = load_cached()?;
            println!("[+] Running auto-optimization (Best Setup)...");
            let selected = do_best_setup(&mut mgr, count)?;
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&selected).map_err(|e| e.to_string())?
                );
            } else {
                println!("[+] Selected {} optimal mirrors:", selected.len());
                for m in &selected {
                    println!("    - {} ({})", m.url, m.country);
                }
            }
            Ok(())
        }
        Command::Save { force } => {
            let mgr = load_cached()?;
            println!("[+] Saving mirrorlist to /etc/pacman.d/mirrorlist...");
            do_save(&mgr, force)?;
            println!("[+] Mirrorlist saved successfully!");
            Ok(())
        }
        Command::Auto { force } => {
            println!("[+] Fetching mirror status...");
            let protocols = default_protocols(&[]);
            let ip_versions = default_ip_versions(false, false);
            let mut mgr = do_refresh(&[], &protocols, &ip_versions, true)?;
            println!("[+] Fetched {} mirrors.", mgr.mirrors.len());
            println!("[+] Running auto-optimization (Best Setup)...");
            let selected = mgr.auto_optimize();
            println!("[+] Selected {} optimal mirrors:", selected.len());
            for m in &selected {
                println!("    - {} ({})", m.url, m.country);
            }
            println!("[+] Saving mirrorlist to /etc/pacman.d/mirrorlist...");
            do_save(&mgr, force)?;
            println!("[+] Mirrorlist saved successfully!");
            Ok(())
        }
        Command::List { json } => match load_cached() {
            Ok(mgr) => {
                if json {
                    println!(
                        "{}",
                        serde_json::to_string_pretty(&mgr.mirrors).map_err(|e| e.to_string())?
                    );
                } else if mgr.mirrors.is_empty() {
                    println!("No mirrors cached. Run 'mirrorman-cli refresh' first.");
                } else {
                    print_mirror_table(&mgr.mirrors);
                }
                Ok(())
            }
            Err(e) => {
                eprintln!("{e}");
                Ok(())
            }
        },
        Command::Status => {
            let current = MirrorManager::read_current_mirrorlist();
            let servers = current
                .lines()
                .filter(|l| l.trim_start().starts_with("Server ="))
                .count();
            println!("Mirrorlist status (/etc/pacman.d/mirrorlist):");
            println!("  File present:        {}", !current.is_empty());
            println!("  Server entries:      {servers}");
            Ok(())
        }
        Command::Backup => {
            let backup_path = mirrorman_core::sync_manager::SyncManager::backup_mirrorlist()?;
            println!("[+] Backup created: {backup_path}");
            Ok(())
        }
        Command::Clean { keep, dry_run } => {
            let mut cmd = std::process::Command::new("paccache");
            cmd.args(["-r", "-k", &keep.to_string()]);
            if dry_run {
                cmd.arg("--dryrun");
            }
            let output = match cmd.output() {
                Ok(o) => o,
                Err(e) => {
                    return Err(format!(
                        "Failed to run paccache ({e}). Please ensure 'pacman-contrib' is installed."
                    ));
                }
            };
            let stdout = String::from_utf8_lossy(&output.stdout);
            let stderr = String::from_utf8_lossy(&output.stderr);
            if !stdout.is_empty() {
                print!("{stdout}");
            }
            if !stderr.is_empty() {
                eprint!("{stderr}");
            }
            if output.status.success() {
                if !dry_run {
                    println!("[+] Cache cleaned (keeping {keep} versions per package).");
                }
                Ok(())
            } else {
                Err(format!("paccache exited with status {}", output.status))
            }
        }
        Command::Sync { force } => {
            println!("[+] Saving mirrorlist...");
            let mgr = load_cached()?;
            do_save(&mgr, force)?;
            println!("[+] Syncing repositories...");
            let out = mirrorman_core::sync_manager::SyncManager::sync_repositories()?;
            if !out.is_empty() {
                println!("{out}");
            }
            println!("[+] Repositories synced successfully!");
            Ok(())
        }
        Command::Diff => {
            let proposed = {
                let mgr = load_cached()?;
                mgr.generate_mirrorlist_content()
            };
            let current = MirrorManager::read_current_mirrorlist();
            if current == proposed {
                println!("[+] No differences — mirrorlist is up to date.");
                return Ok(());
            }
            use std::io::Write;
            let mut diff_cmd = std::process::Command::new("diff")
                .args(["--color=auto", "-u", "/etc/pacman.d/mirrorlist", "/dev/stdin"])
                .stdin(std::process::Stdio::piped())
                .stdout(std::process::Stdio::piped())
                .stderr(std::process::Stdio::piped())
                .spawn()
                .map_err(|e| format!("Failed to run diff: {e}"))?;
            if let Some(ref mut stdin) = diff_cmd.stdin {
                stdin.write_all(proposed.as_bytes()).map_err(|e| e.to_string())?;
            }
            let output = diff_cmd.wait_with_output().map_err(|e| e.to_string())?;
            let stdout = String::from_utf8_lossy(&output.stdout);
            if stdout.is_empty() {
                println!("[+] No differences — mirrorlist is up to date.");
            } else {
                print!("{stdout}");
            }
            Ok(())
        }
        Command::TestMirror { url } => {
            println!("[+] Testing {url}...");
            let start = std::time::Instant::now();
            let output = std::process::Command::new("curl")
                .args(["-o", "/dev/null", "-s", "-w", "%{time_total}", &url])
                .output()
                .map_err(|e| format!("Failed to test mirror: {e}"))?;
            let elapsed = start.elapsed();
            let stdout = String::from_utf8_lossy(&output.stdout);
            println!("[+] Response time: {elapsed:?} (curl: {stdout})");
            Ok(())
        }
        Command::Completions { shell } => {
            let mut cli = Cli::command();
            let bin_name = cli.get_name().to_string();
            clap_complete::generate(shell, &mut cli, bin_name, &mut std::io::stdout());
            Ok(())
        }
        Command::Export { output } => {
            let content = MirrorManager::read_current_mirrorlist();
            if content.is_empty() {
                return Err("No mirrorlist content to export.".to_string());
            }
            match output {
                Some(path) => {
                    std::fs::write(&path, &content)
                        .map_err(|e| format!("Failed to write to {}: {e}", path.display()))?;
                    println!("[+] Exported to {}", path.display());
                }
                None => print!("{content}"),
            }
            Ok(())
        }
        Command::Dev { subcommand } => execute_dev(subcommand),
    }
}

pub fn execute_dev(cmd: DevCommand) -> Result<(), String> {
    match cmd {
        DevCommand::Scan { json } => {
            let statuses = mirrorman_core::dev::scan_ecosystems();
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&statuses).unwrap_or_default()
                );
            } else {
                println!(
                    "{:<18} {:<12} {:<45} {:<30}",
                    "Ecosystem", "Installed", "Current Mirror", "Config Path"
                );
                println!("{}", "-".repeat(105));
                for s in statuses {
                    let installed_str = if s.is_installed { "Yes" } else { "No" };
                    let mirror_str = s.current_mirror.as_deref().unwrap_or("-");
                    let config_str = s.config_path.as_deref().unwrap_or("-");
                    println!(
                        "{:<18} {:<12} {:<45} {:<30}",
                        s.name, installed_str, mirror_str, config_str
                    );
                }
            }
            Ok(())
        }
        DevCommand::List { ecosystem, json } => {
            let kind = mirrorman_core::dev::DevEcosystemKind::parse(&ecosystem)
                .ok_or_else(|| format!("Unknown ecosystem '{ecosystem}'. Available: pip, npm, cargo, go, gem, composer"))?;
            let mirrors = mirrorman_core::dev::mirrors::get_mirrors_for_ecosystem(kind);
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&mirrors).unwrap_or_default()
                );
            } else {
                println!("Available mirrors for {}:", kind.display_name());
                println!("{:<25} {:<8} {:<10} {:<50}", "Name", "Country", "Type", "URL");
                println!("{}", "-".repeat(95));
                for m in mirrors {
                    let type_str = if m.official { "Official" } else { "Mirror" };
                    println!(
                        "{:<25} {:<8} {:<10} {:<50}",
                        m.name, m.country_code, type_str, m.url
                    );
                }
            }
            Ok(())
        }
        DevCommand::Test { ecosystem, workers, json } => {
            let kind = mirrorman_core::dev::DevEcosystemKind::parse(&ecosystem)
                .ok_or_else(|| format!("Unknown ecosystem '{ecosystem}'. Available: pip, npm, cargo, go, gem, composer"))?;
            println!("[*] Benchmarking mirrors for {}...", kind.display_name());
            let tested = mirrorman_core::dev::test_ecosystem_mirrors(kind, workers);
            if json {
                println!(
                    "{}",
                    serde_json::to_string_pretty(&tested).unwrap_or_default()
                );
            } else {
                println!(
                    "{:<25} {:<8} {:<14} {:<50}",
                    "Name", "Country", "Latency", "URL"
                );
                println!("{}", "-".repeat(97));
                for m in tested {
                    let latency_str = match m.speed {
                        Some(ms) => format!("{:.1} ms", ms),
                        None => "Failed/Timeout".to_string(),
                    };
                    println!(
                        "{:<25} {:<8} {:<14} {:<50}",
                        m.name, m.country_code, latency_str, m.url
                    );
                }
            }
            Ok(())
        }
        DevCommand::Set { ecosystem, mirror } => {
            let kind = mirrorman_core::dev::DevEcosystemKind::parse(&ecosystem)
                .ok_or_else(|| format!("Unknown ecosystem '{ecosystem}'. Available: pip, npm, cargo, go, gem, composer"))?;
            let applied = mirrorman_core::dev::set_ecosystem_mirror(kind, &mirror)?;
            println!(
                "[+] Successfully updated {} mirror to '{}' ({})",
                kind.display_name(),
                applied.name,
                applied.url
            );
            Ok(())
        }
        DevCommand::Reset { ecosystem } => {
            let kind = mirrorman_core::dev::DevEcosystemKind::parse(&ecosystem)
                .ok_or_else(|| format!("Unknown ecosystem '{ecosystem}'. Available: pip, npm, cargo, go, gem, composer"))?;
            mirrorman_core::dev::reset_ecosystem_mirror(kind)?;
            println!("[+] Successfully reset {} mirror to official upstream default.", kind.display_name());
            Ok(())
        }
        DevCommand::Auto { workers, json } => {
            println!("[*] Scanning installed tools and selecting lowest-latency mirrors...");
            let results = mirrorman_core::dev::auto_select_fastest(workers);
            if json {
                let serialized: Vec<serde_json::Value> = results
                    .iter()
                    .map(|(kind, res)| {
                        serde_json::json!({
                            "ecosystem": kind.as_str(),
                            "success": res.is_ok(),
                            "mirror": res.as_ref().ok()
                        })
                    })
                    .collect();
                println!("{}", serde_json::to_string_pretty(&serialized).unwrap_or_default());
            } else {
                for (kind, res) in results {
                    match res {
                        Ok(mirror) => {
                            let speed_str = mirror.speed.map(|s| format!("{:.1} ms", s)).unwrap_or_else(|| "-".to_string());
                            println!(
                                "[+] {}: set to '{}' ({}) [{}]",
                                kind.display_name(),
                                mirror.name,
                                mirror.url,
                                speed_str
                            );
                        }
                        Err(e) => {
                            eprintln!("[-] {}: failed ({e})", kind.display_name());
                        }
                    }
                }
            }
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mirrorman_core::mirror_manager::Mirror;

    fn sample_mirror(enabled: bool) -> Mirror {
        Mirror {
            url: "https://mirror.de/".to_string(),
            country: "Germany".to_string(),
            country_code: "DE".to_string(),
            protocol: "https".to_string(),
            speed: Some(20.0),
            last_sync: None,
            enabled,
            ipv4: true,
            ipv6: false,
            completion_pct: None,
            score: None,
            duration_avg: None,
            duration_stddev: None,
        }
    }

    #[test]
    fn test_default_protocols_empty() {
        let none: Vec<String> = vec![];
        assert_eq!(default_protocols(&none), vec!["http", "https"]);
        let custom = vec!["http".to_string()];
        assert_eq!(default_protocols(&custom), vec!["http"]);
    }

    #[test]
    fn test_default_ip_versions() {
        assert_eq!(default_ip_versions(false, false), vec!["4", "6"]);
        assert_eq!(default_ip_versions(true, false), vec!["4"]);
        assert_eq!(default_ip_versions(false, true), vec!["6"]);
        assert_eq!(default_ip_versions(true, true), vec!["4", "6"]);
    }

    #[test]
    fn test_cache_roundtrip() {
        let tmp = std::env::temp_dir().join(format!("mirrorman-test-{}", std::process::id()));
        std::env::set_var("MIRRORMAN_CACHE", &tmp);
        let mut mgr = MirrorManager::new();
        mgr.mirrors = vec![sample_mirror(true)];
        save_cache(&mgr).unwrap();
        let loaded = load_cached().unwrap();
        assert_eq!(loaded.mirrors.len(), 1);
        assert_eq!(loaded.mirrors[0].url, "https://mirror.de/");
        let _ = std::fs::remove_file(&tmp);
    }

    #[test]
    fn test_save_requires_enabled_mirrors() {
        let mut mgr = MirrorManager::new();
        mgr.mirrors = vec![sample_mirror(false)];
        let err = do_save(&mgr, false).unwrap_err();
        assert!(err.contains("No enabled mirrors"));
    }

    #[test]
    fn test_dev_scan_succeeds() {
        let res = execute_dev(DevCommand::Scan { json: false });
        assert!(res.is_ok());
    }

    #[test]
    fn test_dev_list_pip_succeeds() {
        let res = execute_dev(DevCommand::List { ecosystem: "pip".to_string(), json: false });
        assert!(res.is_ok());
    }

    #[test]
    fn test_dev_list_unknown_fails() {
        let res = execute_dev(DevCommand::List { ecosystem: "unknown_pkg_mgr".to_string(), json: false });
        assert!(res.is_err());
    }
}
