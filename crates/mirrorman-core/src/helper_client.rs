use crate::helper_guard::{
    validate_pacman_args, validate_run_command, BLACKARCH_STRAP_SCRIPT,
};
use zbus::blocking::Connection;

pub struct HelperClient;

const MAX_CONTENT_LEN: usize = 1_000_000;

fn call_helper(
    method: &str,
    body: &(impl serde::Serialize + zbus::zvariant::DynamicType),
) -> Result<zbus::Message, zbus::Error> {
    let conn = Connection::system()?;
    conn.call_method(
        Some("com.parchlinux.mirrorman.Helper"),
        "/com/parchlinux/mirrorman/Helper",
        Some("com.parchlinux.mirrorman.Helper"),
        method,
        body,
    )
}

impl HelperClient {
    pub fn save_mirrorlist(content: &str) -> Result<(), String> {
        if content.len() > MAX_CONTENT_LEN {
            return Err("mirrorlist content too large".to_string());
        }
        if let Ok(reply) = call_helper("SaveMirrorlist", &(content,)) {
            if let Ok(true) = reply.body().deserialize::<bool>() {
                return Ok(());
            }
        }
        Self::fallback_save_mirrorlist(content)
    }

    pub fn save_pacman_conf(content: &str) -> Result<(), String> {
        if content.len() > MAX_CONTENT_LEN {
            return Err("pacman.conf content too large".to_string());
        }
        if let Ok(reply) = call_helper("SavePacmanConf", &(content,)) {
            if let Ok(true) = reply.body().deserialize::<bool>() {
                return Ok(());
            }
        }
        Self::fallback_save_pacman_conf(content)
    }

    pub fn run_pacman(args: &[&str]) -> Result<(bool, String, String), String> {
        let vec_args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        validate_pacman_args(&vec_args)?;
        if let Ok(reply) = call_helper("RunPacman", &(vec_args,)) {
            if let Ok(tuple) = reply.body().deserialize::<(bool, String, String)>() {
                return Ok(tuple);
            }
        }
        Self::fallback_run_pacman(args)
    }

    pub fn run_command(command: &str, args: &[&str]) -> Result<(bool, String, String), String> {
        let vec_args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
        validate_run_command(command, &vec_args)?;
        if let Ok(reply) = call_helper("RunCommand", &(command, vec_args)) {
            if let Ok(tuple) = reply.body().deserialize::<(bool, String, String)>() {
                return Ok(tuple);
            }
        }
        Self::fallback_run_command(command, args)
    }

    /// Runs the pinned BlackArch `strap.sh` bootstrap through the dedicated
    /// helper operation. Never falls through to arbitrary shell execution.
    pub fn run_blackarch_strap() -> Result<(bool, String, String), String> {
        if let Ok(reply) = call_helper("RunBlackArchStrap", &()) {
            if let Ok(tuple) = reply.body().deserialize::<(bool, String, String)>() {
                return Ok(tuple);
            }
        }
        let output = std::process::Command::new("pkexec")
            .args(["bash", "-c", BLACKARCH_STRAP_SCRIPT])
            .output()
            .map_err(|e| format!("Failed to execute pkexec bash: {e}"))?;
        Ok((
            output.status.success(),
            String::from_utf8_lossy(&output.stdout).to_string(),
            String::from_utf8_lossy(&output.stderr).to_string(),
        ))
    }

    fn fallback_save_mirrorlist(content: &str) -> Result<(), String> {
        use std::io::Write;
        let temp_path = "/tmp/mirrorman_mirrorlist";
        let mut f = std::fs::File::create(temp_path)
            .map_err(|e| format!("Failed to create temp file: {e}"))?;
        f.write_all(content.as_bytes())
            .map_err(|e| format!("Failed to write mirrorlist: {e}"))?;
        let status = std::process::Command::new("pkexec")
            .args(["cp", temp_path, "/etc/pacman.d/mirrorlist"])
            .status()
            .map_err(|e| format!("pkexec failed: {e}"))?;
        let _ = std::fs::remove_file(temp_path);
        if status.success() {
            Ok(())
        } else {
            Err("pkexec failed to save mirrorlist".to_string())
        }
    }

    fn fallback_save_pacman_conf(content: &str) -> Result<(), String> {
        use std::io::Write;
        let temp_path = "/tmp/mirrorman_pacman_conf";
        let mut f = std::fs::File::create(temp_path)
            .map_err(|e| format!("Failed to create temp file: {e}"))?;
        f.write_all(content.as_bytes())
            .map_err(|e| format!("Failed to write config: {e}"))?;
        let status = std::process::Command::new("pkexec")
            .args(["cp", temp_path, "/etc/pacman.conf"])
            .status()
            .map_err(|e| format!("pkexec failed: {e}"))?;
        let _ = std::fs::remove_file(temp_path);
        if status.success() {
            Ok(())
        } else {
            Err("pkexec failed to save pacman.conf".to_string())
        }
    }

    fn fallback_run_pacman(args: &[&str]) -> Result<(bool, String, String), String> {
        validate_pacman_args(&args.iter().map(|s| s.to_string()).collect::<Vec<_>>())?;
        let output = std::process::Command::new("pkexec")
            .arg("pacman")
            .args(args)
            .output()
            .map_err(|e| format!("Failed to execute pkexec pacman: {e}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Ok((output.status.success(), stdout, stderr))
    }

    fn fallback_run_command(
        command: &str,
        args: &[&str],
    ) -> Result<(bool, String, String), String> {
        validate_run_command(command, &args.iter().map(|s| s.to_string()).collect::<Vec<_>>())?;
        let path = match command {
            "cp" => "cp",
            "pacman-key" => "pacman-key",
            _ => return Err(format!("command '{command}' is not whitelisted")),
        };
        let output = std::process::Command::new("pkexec")
            .arg(path)
            .args(args)
            .output()
            .map_err(|e| format!("Failed to execute pkexec {command}: {e}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Ok((output.status.success(), stdout, stderr))
    }
}
