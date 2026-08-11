use crate::mirror_manager::{Mirror, MirrorManager};
use clap::{Parser, Subcommand};
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
    crate::helper_client::HelperClient::save_mirrorlist(&mgr.generate_mirrorlist_content())
}

fn print_mirror_table(mirrors: &[Mirror]) {
    println!("{:<3} {:<6} {:>8}  {:<55} {}", "#", "Proto", "Speed", "URL", "Country");
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
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mirror_manager::Mirror;

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
}
