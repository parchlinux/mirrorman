use serde::{Deserialize, Serialize};
use std::fs;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEntry {
    pub timestamp: String,
    pub action: String,
    pub package: String,
    pub version_info: String,
}

pub fn parse_pacman_log() -> Vec<LogEntry> {
    let content = match fs::read_to_string("/var/log/pacman.log") {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };
    parse_log_str(&content)
}

pub fn parse_log_str(content: &str) -> Vec<LogEntry> {
    let mut entries = Vec::new();

    for line in content.lines() {
        let line = line.trim();
        if line.is_empty() || !line.starts_with('[') {
            continue;
        }

        if let Some(close_bracket) = line.find(']') {
            let timestamp = &line[1..close_bracket];
            let rest = line[close_bracket + 1..].trim();

            if rest.contains("[ALPM]") {
                let msg = rest.replace("[ALPM]", "").trim().to_string();
                if let Some((action, detail)) = msg.split_once(' ') {
                    let action_str = match action {
                        "installed" => "Installed",
                        "upgraded" => "Upgraded",
                        "removed" => "Removed",
                        "reinstalled" => "Reinstalled",
                        "downgraded" => "Downgraded",
                        _ => continue,
                    };

                    let (pkg, ver) = if let Some((p, v)) = detail.split_once(' ') {
                        (p.to_string(), v.trim_matches(|c| c == '(' || c == ')').to_string())
                    } else {
                        (detail.to_string(), String::new())
                    };

                    entries.push(LogEntry {
                        timestamp: timestamp.to_string(),
                        action: action_str.to_string(),
                        package: pkg,
                        version_info: ver,
                    });
                }
            }
        }
    }

    entries.reverse(); // Most recent first
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_log_str() {
        let sample = "\
[2026-07-24T00:10:00+0330] [ALPM] transaction started
[2026-07-24T00:10:01+0330] [ALPM] installed ripgrep (14.1.0-1)
[2026-07-24T00:10:02+0330] [ALPM] upgraded gtk4 (4.14.0 -> 4.16.0)
[2026-07-24T00:10:03+0330] [ALPM] removed nano (7.2-1)
[2026-07-24T00:10:04+0330] [ALPM] transaction completed
";

        let entries = parse_log_str(sample);
        assert_eq!(entries.len(), 3);
        assert_eq!(entries[0].package, "nano");
        assert_eq!(entries[0].action, "Removed");
        assert_eq!(entries[1].package, "gtk4");
        assert_eq!(entries[1].action, "Upgraded");
        assert_eq!(entries[2].package, "ripgrep");
        assert_eq!(entries[2].action, "Installed");
    }
}
