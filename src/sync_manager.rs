use std::process::Command;

pub struct SyncManager;

impl SyncManager {
    pub fn sync_repositories() -> Result<String, String> {
        Self::run_pacman(&["-Syy", "--noconfirm"], "sync")
    }

    pub fn clean_cache() -> Result<String, String> {
        Self::run_pacman(&["-Sc", "--noconfirm"], "clean cache")
    }

    fn run_pacman(args: &[&str], label: &str) -> Result<String, String> {
        let output = Command::new("pkexec")
            .args(["pacman"])
            .args(args)
            .output()
            .map_err(|e| format!("Failed to execute pacman: {e}"))?;

        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout).to_string();
            Ok(stdout)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr).to_string();
            Err(format!("Failed to {label}:\n{stderr}"))
        }
    }
}
