use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug, Clone, Serialize, Deserialize)]
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
    pub completion_pct: Option<f64>,
    pub score: Option<f64>,
    pub duration_avg: Option<f64>,
    pub duration_stddev: Option<f64>,
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

/// Builds the URL used to time a mirror (downloads a small repo database).
pub fn speed_test_url(url: &str) -> String {
    format!("{}/core/os/x86_64/core.db", url.trim_end_matches('/'))
}

/// Builds the URL used for a cheap HEAD availability probe.
pub fn availability_url(url: &str) -> String {
    format!("{}/lastsync", url.trim_end_matches('/'))
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
    completion_pct: Option<f64>,
    score: Option<f64>,
    duration_avg: Option<f64>,
    duration_stddev: Option<f64>,
}

const API_URL: &str = "https://archlinux.org/mirrors/status/json/";
const USER_AGENT: &str = "mirrorman/0.4.2";
const MIRRORLIST_FILE: &str = "/etc/pacman.d/mirrorlist";
pub const MIRRORLIST_BACKUP: &str = "/etc/pacman.d/mirrorlist.backup";

const IRANIAN_MIRRORS: &[&str] = &[
    "https://mirror.mobinhost.com/archlinux/$repo/os/$arch",
    "http://repo.iut.ac.ir/repo/archlinux/$repo/os/$arch",
    "https://mirror.arvancloud.ir/archlinux/$repo/os/$arch",
];

/// True when a mirror's country should be kept for the given selection.
/// An empty selection means "all countries".
pub fn country_selected(countries: &[String], mirror_country: &str) -> bool {
    countries.is_empty() || countries.iter().any(|c| c == mirror_country)
}

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
        countries: &[String],
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

        let mut countries_set = std::collections::BTreeSet::new();
        let mut mirrors = Vec::new();

        let want_ipv4 = ip_versions.contains(&"4".to_string());
        let want_ipv6 = ip_versions.contains(&"6".to_string());
        if !want_ipv4 && !want_ipv6 {
            return Err("Select at least one IP version".to_string());
        }

        for m in api.urls {
            let mirror_country = m.country.unwrap_or_default();
            countries_set.insert(mirror_country.clone());

            if !country_selected(countries, &mirror_country) {
                continue;
            }

            let protocol = m.protocol.unwrap_or_default();
            if !protocols.is_empty() && !protocols.contains(&protocol.to_lowercase()) {
                continue;
            }

            let url = match m.url {
                Some(u) => u,
                None => continue,
            };

            let mirror_ipv4 = m.ipv4.unwrap_or(false);
            let mirror_ipv6 = m.ipv6.unwrap_or(false);
            let has_wanted_ip = (want_ipv4 && mirror_ipv4) || (want_ipv6 && mirror_ipv6);
            if !has_wanted_ip {
                continue;
            }

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
                ipv4: mirror_ipv4,
                ipv6: mirror_ipv6,
                completion_pct: m.completion_pct,
                score: m.score,
                duration_avg: m.duration_avg,
                duration_stddev: m.duration_stddev,
            });
        }

        countries_set.insert("Worldwide".to_string());
        self.countries = countries_set.into_iter().collect();
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
            let test_url = speed_test_url(&url);

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
                let check_url = availability_url(&url);
                match client.head(&check_url).send() {
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
                if speed.is_none() {
                    mirrors[idx].enabled = false;
                }
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
                completion_pct: None,
                score: None,
                duration_avg: None,
                duration_stddev: None,
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

    pub fn sort_by_score(&mut self) {
        self.mirrors.sort_by(|a, b| match (a.score, b.score) {
            (Some(a), Some(b)) => a.partial_cmp(&b).unwrap_or(std::cmp::Ordering::Equal),
            (Some(_), None) => std::cmp::Ordering::Less,
            (None, Some(_)) => std::cmp::Ordering::Greater,
            (None, None) => std::cmp::Ordering::Equal,
        });
    }

    pub fn sort_by_reliability(&mut self) {
        self.mirrors.sort_by(|a, b| {
            let cp_a = a.completion_pct.unwrap_or(0.0);
            let cp_b = b.completion_pct.unwrap_or(0.0);
            let std_a = a.duration_stddev.unwrap_or(999.0);
            let std_b = b.duration_stddev.unwrap_or(999.0);

            cp_b.partial_cmp(&cp_a)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| std_a.partial_cmp(&std_b).unwrap_or(std::cmp::Ordering::Equal))
        });
    }

    pub fn auto_optimize(&mut self) -> Vec<Mirror> {
        self.auto_optimize_with_count(5)
    }

    pub fn auto_optimize_with_count(&mut self, max_count: usize) -> Vec<Mirror> {
        let max_count = max_count.max(1);
        let mut indices: Vec<usize> = (0..self.mirrors.len()).collect();
        indices.sort_by(|&i, &j| {
            let a = &self.mirrors[i];
            let b = &self.mirrors[j];
            match (a.score, b.score) {
                (Some(sa), Some(sb)) => sa.partial_cmp(&sb).unwrap_or(std::cmp::Ordering::Equal),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => match (a.duration_avg, b.duration_avg) {
                    (Some(da), Some(db)) => da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal),
                    (Some(_), None) => std::cmp::Ordering::Less,
                    (None, Some(_)) => std::cmp::Ordering::Greater,
                    (None, None) => std::cmp::Ordering::Equal,
                },
            }
        });

        let mut selected_indices = std::collections::HashSet::new();
        let mut seen_countries = std::collections::HashSet::new();

        for idx in indices {
            let m = &self.mirrors[idx];
            if !seen_countries.contains(&m.country) {
                seen_countries.insert(m.country.clone());
                selected_indices.insert(idx);
                if selected_indices.len() >= max_count {
                    break;
                }
            }
        }

        let mut selected_mirrors = Vec::new();
        for (idx, m) in self.mirrors.iter_mut().enumerate() {
            if selected_indices.contains(&idx) {
                m.enabled = true;
                selected_mirrors.push(m.clone());
            } else {
                m.enabled = false;
            }
        }

        selected_mirrors
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

    pub fn read_current_mirrorlist() -> String {
        std::fs::read_to_string(MIRRORLIST_FILE).unwrap_or_default()
    }

    pub fn save_mirrorlist(&self) -> Result<(), String> {
        if MIRRORLIST_FILE != "/etc/pacman.d/mirrorlist" {
            return Err("Refusing to write: unexpected mirrorlist path".to_string());
        }

        let enabled_count = self.mirrors.iter().filter(|m| m.enabled).count();
        if enabled_count == 0 {
            return Err(
                "Refusing to save an empty mirrorlist: no mirrors are enabled. Run 'refresh' and 'best-setup' first, or use --force."
                    .to_string(),
            );
        }

        let content = self.generate_mirrorlist_content();
        crate::helper_client::HelperClient::save_mirrorlist(&content)
    }

    pub fn generate_mirrorlist_content(&self) -> String {
        let mut s = String::new();
        s.push_str("##\n## Parch Linux repository mirrorlist\n");
        s.push_str("## Generated by mirrorman\n##\n\n");

        let enabled_count = self.mirrors.iter().filter(|m| m.enabled).count();
        s.push_str(&format!("## {enabled_count} enabled mirror(s)\n\n"));

        for mirror in &self.mirrors {
            if mirror.enabled {
                let url = format!("{}/$repo/os/$arch", mirror.url.trim_end_matches('/'));
                s.push_str(&format!("Server = {url}\n"));
            }
        }

        s
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mirror_sorting_by_score() {
        let mut mgr = MirrorManager::new();
        mgr.mirrors = vec![
            Mirror {
                url: "http://mirror1.org/".to_string(),
                country: "Germany".to_string(),
                country_code: "DE".to_string(),
                protocol: "https".to_string(),
                speed: Some(150.0),
                last_sync: None,
                enabled: true,
                ipv4: true,
                ipv6: false,
                completion_pct: Some(1.0),
                score: Some(2.5),
                duration_avg: None,
                duration_stddev: None,
            },
            Mirror {
                url: "http://mirror2.org/".to_string(),
                country: "France".to_string(),
                country_code: "FR".to_string(),
                protocol: "https".to_string(),
                speed: Some(100.0),
                last_sync: None,
                enabled: true,
                ipv4: true,
                ipv6: false,
                completion_pct: Some(1.0),
                score: Some(1.1),
                duration_avg: None,
                duration_stddev: None,
            },
        ];

        mgr.sort_by_score();
        assert_eq!(mgr.mirrors[0].score, Some(1.1));
        assert_eq!(mgr.mirrors[1].score, Some(2.5));
    }

    #[test]
    fn test_auto_optimize_country_diversity() {
        let mut mgr = MirrorManager::new();
        mgr.mirrors = vec![
            Mirror {
                url: "http://m1.de/".to_string(),
                country: "Germany".to_string(),
                country_code: "DE".to_string(),
                protocol: "https".to_string(),
                speed: Some(50.0),
                last_sync: None,
                enabled: false,
                ipv4: true,
                ipv6: false,
                completion_pct: Some(1.0),
                score: Some(1.0),
                duration_avg: None,
                duration_stddev: None,
            },
            Mirror {
                url: "http://m2.de/".to_string(),
                country: "Germany".to_string(),
                country_code: "DE".to_string(),
                protocol: "https".to_string(),
                speed: Some(60.0),
                last_sync: None,
                enabled: false,
                ipv4: true,
                ipv6: false,
                completion_pct: Some(1.0),
                score: Some(1.2),
                duration_avg: None,
                duration_stddev: None,
            },
            Mirror {
                url: "http://m1.fr/".to_string(),
                country: "France".to_string(),
                country_code: "FR".to_string(),
                protocol: "https".to_string(),
                speed: Some(70.0),
                last_sync: None,
                enabled: false,
                ipv4: true,
                ipv6: false,
                completion_pct: Some(1.0),
                score: Some(1.1),
                duration_avg: None,
                duration_stddev: None,
            },
        ];

        let selected = mgr.auto_optimize();
        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0].country, "Germany");
        assert_eq!(selected[1].country, "France");
    }

    #[test]
    fn test_generate_mirrorlist_content() {
        let mut mgr = MirrorManager::new();
        mgr.mirrors = vec![
            Mirror {
                url: "https://arch.mirror.org/".to_string(),
                country: "Germany".to_string(),
                country_code: "DE".to_string(),
                protocol: "https".to_string(),
                speed: Some(10.0),
                last_sync: None,
                enabled: true,
                ipv4: true,
                ipv6: false,
                completion_pct: Some(1.0),
                score: Some(1.0),
                duration_avg: None,
                duration_stddev: None,
            },
        ];

        let content = mgr.generate_mirrorlist_content();
        assert!(content.contains("Server = https://arch.mirror.org/$repo/os/$arch"));
    }

    #[test]
    fn test_speed_test_url_has_slash() {
        assert_eq!(
            speed_test_url("https://mirror.aarnet.edu.au/pub/archlinux/"),
            "https://mirror.aarnet.edu.au/pub/archlinux/core/os/x86_64/core.db"
        );
        assert_eq!(
            speed_test_url("https://mirror.mobinhost.com/archlinux"),
            "https://mirror.mobinhost.com/archlinux/core/os/x86_64/core.db"
        );
    }

    #[test]
    fn test_availability_url_probes_lastsync() {
        assert_eq!(
            availability_url("https://mirror.aarnet.edu.au/pub/archlinux/"),
            "https://mirror.aarnet.edu.au/pub/archlinux/lastsync"
        );
    }

    #[test]
    fn test_country_selected_empty_means_all() {
        let none: Vec<String> = vec![];
        assert!(country_selected(&none, "Germany"));
        assert!(country_selected(&none, "AnyCountry"));
    }

    #[test]
    fn test_country_selected_multi() {
        let selected = vec!["Germany".to_string(), "France".to_string()];
        assert!(country_selected(&selected, "Germany"));
        assert!(country_selected(&selected, "France"));
        assert!(!country_selected(&selected, "Italy"));
        assert!(!country_selected(&selected, ""));
    }
}
