use chrono::{DateTime, Duration, Utc};
use serde::Deserialize;
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct Mirror {
    pub url: String,
    pub country: String,
    pub country_code: String,
    pub protocol: String,
    pub speed: Option<f64>,
    pub last_sync: Option<String>,
    pub enabled: bool,
    pub ipv4: bool,
    pub ipv6: bool,
}

pub fn country_flag(code: &str) -> String {
    if code.len() != 2 { return String::new(); }
    let code = code.to_uppercase();
    let bytes = code.as_bytes();
    let a = bytes[0] as u32;
    let b = bytes[1] as u32;
    if a < 65 || a > 90 || b < 65 || b > 90 { return String::new(); }
    let ra = char::from_u32(0x1F1E6 + (a - 65)).unwrap_or(' ');
    let rb = char::from_u32(0x1F1E6 + (b - 65)).unwrap_or(' ');
    format!("{}{}", ra, rb)
}

#[derive(Debug, Deserialize)]
struct ApiResponse {
    urls: Vec<ApiMirror>,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
struct ApiMirror {
    url: Option<String>,
    protocol: Option<String>,
    country: Option<String>,
    country_code: Option<String>,
    last_sync: Option<String>,
    ipv4: Option<bool>,
    ipv6: Option<bool>,
}

const API_URL: &str = "https://archlinux.org/mirrors/status/json/";
const USER_AGENT: &str = "mirrorman/0.3";
const MIRRORLIST_FILE: &str = "/etc/pacman.d/mirrorlist";
const MIRRORLIST_BACKUP: &str = "/etc/pacman.d/mirrorlist.backup";

const IRANIAN_MIRRORS: &[&str] = &[
    "https://mirror.0-1.cloud/archlinux/$repo/os/$arch",
    "https://mirror.kernel.ir/archlinux/$repo/os/$arch",
    "https://mirror.mobinhost.com/archlinux/$repo/os/$arch",
    "http://repo.iut.ac.ir/repo/archlinux/$repo/os/$arch",
    "https://mirror.arvancloud.ir/archlinux/$repo/os/$arch",
];

pub struct MirrorManager {
    pub mirrors: Vec<Mirror>,
    pub countries: Vec<String>,
}

impl MirrorManager {
    pub fn new() -> Self {
        Self {
            mirrors: Vec::new(),
            countries: Vec::new(),
        }
    }

    pub fn fetch_mirrors(
        &mut self,
        country: Option<&str>,
        protocols: &[String],
        ip_versions: &[String],
        use_status: bool,
    ) -> Result<(), String> {
        let client = reqwest::blocking::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

        let response = client
            .get(API_URL)
            .send()
            .map_err(|e| format!("Network error: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("HTTP Error: {}", response.status()));
        }

        let body = response
            .text()
            .map_err(|e| format!("Failed to read response body: {e}"))?;

        let api: ApiResponse = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse API response: {e}"))?;

        let mut countries = std::collections::BTreeSet::new();
        let mut mirrors = Vec::new();

        for m in api.urls {
            let mirror_country = m.country.unwrap_or_default();
            countries.insert(mirror_country.clone());

            if let Some(ref c) = country {
                if c != &mirror_country {
                    continue;
                }
            }

            let protocol = m.protocol.unwrap_or_default();
            if !protocols.is_empty() && !protocols.contains(&protocol.to_lowercase()) {
                continue;
            }

            let url = match m.url {
                Some(u) => u,
                None => continue,
            };

            let ipv4 = ip_versions.contains(&"4".to_string());
            let ipv6 = ip_versions.contains(&"6".to_string());

            let last_sync = m.last_sync.clone();
            let country_code = m.country_code.unwrap_or_default();

            if use_status && !Self::is_mirror_up_to_date(last_sync.as_deref()) {
                continue;
            }

            mirrors.push(Mirror {
                url,
                country: mirror_country,
                country_code,
                protocol,
                speed: None,
                last_sync,
                enabled: true,
                ipv4,
                ipv6,
            });
        }

        countries.insert("Worldwide".to_string());
        self.countries = countries.into_iter().collect();
        self.countries.sort();
        self.mirrors = mirrors;

        Ok(())
    }

    fn is_mirror_up_to_date(last_sync: Option<&str>) -> bool {
        let last_sync = match last_sync {
            Some(s) => s,
            None => return false,
        };

        let sync_time = match last_sync.replace("Z", "+00:00").parse::<DateTime<Utc>>() {
            Ok(t) => t,
            Err(_) => return false,
        };

        (Utc::now() - sync_time) < Duration::hours(24)
    }

    pub fn fetch_countries_only(&self) -> Result<Vec<String>, String> {
        let client = reqwest::blocking::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(std::time::Duration::from_secs(10))
            .build()
            .map_err(|e| format!("Failed to create HTTP client: {e}"))?;

        let response = client
            .get(API_URL)
            .send()
            .map_err(|e| format!("Network error: {e}"))?;

        if !response.status().is_success() {
            return Err(format!("HTTP Error: {}", response.status()));
        }

        let body = response
            .text()
            .map_err(|e| format!("Failed to read response body: {e}"))?;

        let api: ApiResponse = serde_json::from_str(&body)
            .map_err(|e| format!("Failed to parse API response: {e}"))?;

        let mut countries: std::collections::BTreeSet<String> = api
            .urls
            .iter()
            .filter_map(|m| {
                let c = m.country.as_deref().unwrap_or("");
                if c.is_empty() || c == "Unknown" {
                    None
                } else {
                    Some(c.to_string())
                }
            })
            .collect();

        countries.insert("Worldwide".to_string());
        let mut list: Vec<_> = countries.into_iter().collect();
        list.sort();
        Ok(list)
    }

    pub fn test_all_speeds_concurrent(mirrors: &mut [Mirror], max_workers: usize) {
        if mirrors.is_empty() {
            return;
        }

        let results: Arc<Mutex<Vec<(usize, Option<f64>)>>> = Arc::new(Mutex::new(Vec::new()));
        let mut handles = Vec::new();
        let chunk_size = max_workers;

        for (idx, mirror) in mirrors.iter().enumerate() {
            if mirror.url.is_empty()
                || (!mirror.url.starts_with("http://") && !mirror.url.starts_with("https://"))
            {
                continue;
            }

            let url = mirror.url.clone();
            let results = Arc::clone(&results);
            let test_url = format!("{}core/os/x86_64/core.db", url.trim_end_matches('/'));

            let handle = std::thread::spawn(move || {
                let client = reqwest::blocking::Client::builder()
                    .user_agent(USER_AGENT)
                    .timeout(std::time::Duration::from_secs(5))
                    .build()
                    .ok()?;

                let start = Instant::now();
                match client.get(&test_url).send() {
                    Ok(resp) => {
                        let _ = resp.bytes();
                        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                        results.lock().unwrap().push((idx, Some(elapsed)));
                        Some(elapsed)
                    }
                    Err(_) => {
                        results.lock().unwrap().push((idx, None));
                        None
                    }
                }
            });

            handles.push(handle);

            if handles.len() >= chunk_size {
                for h in handles.drain(..) {
                    let _ = h.join();
                }
            }
        }

        for h in handles.drain(..) {
            let _ = h.join();
        }

        let final_results = results.lock().unwrap();
        for &(idx, speed) in final_results.iter() {
            if idx < mirrors.len() {
                mirrors[idx].speed = speed;
            }
        }
    }

    pub fn check_mirror_availability(mirrors: &mut [Mirror], max_workers: usize) {
        if mirrors.is_empty() { return; }

        let results: Arc<Mutex<Vec<(usize, Option<f64>)>>> = Arc::new(Mutex::new(Vec::new()));
        let mut handles = Vec::new();
        let chunk_size = max_workers;

        for (idx, mirror) in mirrors.iter().enumerate() {
            if mirror.url.is_empty()
                || (!mirror.url.starts_with("http://") && !mirror.url.starts_with("https://"))
            {
                continue;
            }

            let url = mirror.url.clone();
            let results = Arc::clone(&results);

            let handle = std::thread::spawn(move || {
                let client = reqwest::blocking::Client::builder()
                    .user_agent(USER_AGENT)
                    .timeout(std::time::Duration::from_secs(10))
                    .build()
                    .ok()?;

                let start = Instant::now();
                match client.head(&url).send() {
                    Ok(resp) => {
                        let elapsed = start.elapsed().as_secs_f64() * 1000.0;
                        if resp.status().is_success() || resp.status().as_u16() < 400 {
                            results.lock().unwrap().push((idx, Some(elapsed)));
                            Some(elapsed)
                        } else {
                            results.lock().unwrap().push((idx, None));
                            None
                        }
                    }
                    Err(_) => {
                        results.lock().unwrap().push((idx, None));
                        None
                    }
                }
            });

            handles.push(handle);

            if handles.len() >= chunk_size {
                for h in handles.drain(..) { let _ = h.join(); }
            }
        }

        for h in handles.drain(..) { let _ = h.join(); }

        let final_results = results.lock().unwrap();
        for &(idx, speed) in final_results.iter() {
            if idx < mirrors.len() {
                mirrors[idx].speed = speed;
            }
        }
    }

    pub fn add_iran_mirrors(&mut self) {
        for mirror_url in IRANIAN_MIRRORS {
            let url = mirror_url.replace("$repo/os/$arch", "");
            let protocol = if mirror_url.starts_with("https") {
                "https"
            } else {
                "http"
            };
            self.mirrors.push(Mirror {
                url,
                country: "IRAN".to_string(),
                country_code: "IR".to_string(),
                protocol: protocol.to_string(),
                speed: None,
                last_sync: None,
                enabled: true,
                ipv4: true,
                ipv6: false,
            });
        }
    }

    pub fn sort_by_speed(&mut self) {
        self.mirrors.sort_by(|a, b| match (a.speed, b.speed) {
            (Some(a), Some(b)) => a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });
    }

    pub fn sort_by_country(&mut self) {
        self.mirrors
            .sort_by(|a, b| a.country.cmp(&b.country));
    }

    pub fn sort_by_age(&mut self) {
        self.mirrors.sort_by(|a, b| match (&a.last_sync, &b.last_sync) {
            (Some(a), Some(b)) => b.cmp(a),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });
    }

    pub fn save_mirrorlist(&self) -> Result<(), String> {
        use std::io::Write;

        if MIRRORLIST_FILE != "/etc/pacman.d/mirrorlist" {
            return Err("Refusing to write: unexpected mirrorlist path".to_string());
        }

        // Backup existing mirrorlist
        if std::path::Path::new(MIRRORLIST_FILE).exists() {
            let _ = std::process::Command::new("pkexec")
                .args(["cp", MIRRORLIST_FILE, MIRRORLIST_BACKUP])
                .status();
        }

        let content = self.generate_mirrorlist_content();
        let temp_path = "/tmp/mirrorman_mirrorlist";

        {
            let mut f =
                std::fs::File::create(temp_path).map_err(|e| format!("Failed to create temp file: {e}"))?;
            f.write_all(content.as_bytes())
                .map_err(|e| format!("Failed to write mirrorlist: {e}"))?;
        }

        let status = std::process::Command::new("pkexec")
            .args(["cp", temp_path, MIRRORLIST_FILE])
            .status()
            .map_err(|e| format!("Failed to execute pkexec: {e}"))?;

        if !status.success() {
            let _ = std::fs::remove_file(temp_path);
            return Err("pkexec failed to copy mirrorlist".to_string());
        }

        let _ = std::fs::remove_file(temp_path);
        Ok(())
    }

    fn generate_mirrorlist_content(&self) -> String {
        let mut s = String::new();
        s.push_str("##\n## Parch Linux repository mirrorlist\n");
        s.push_str("## Generated by mirrorman\n##\n\n");

        let enabled_count = self.mirrors.iter().filter(|m| m.enabled).count();
        s.push_str(&format!("## {enabled_count} enabled mirror(s)\n\n"));

        for mirror in &self.mirrors {
            if mirror.enabled {
                let url = format!("{}$repo/os/$arch", mirror.url.trim_end_matches('/'));
                s.push_str(&format!("Server = {url}\n"));
            }
        }

        s
    }
}
