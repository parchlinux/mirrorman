use zbus::blocking::Connection;

pub struct HelperClient;

impl HelperClient {
    pub fn save_mirrorlist(content: &str) -> Result<(), String> {
        if let Ok(conn) = Connection::system() {
            if let Ok(reply) = conn.call_method(
                Some("com.parchlinux.mirrorman.Helper"),
                "/com/parchlinux/mirrorman/Helper",
                Some("com.parchlinux.mirrorman.Helper"),
                "SaveMirrorlist",
                &(content,),
            ) {
                if let Ok(success) = reply.body().deserialize::<bool>() {
                    if success {
                        return Ok(());
                    }
                }
            }
        }
        Self::fallback_save_mirrorlist(content)
    }

    pub fn save_pacman_conf(content: &str) -> Result<(), String> {
        if let Ok(conn) = Connection::system() {
            if let Ok(reply) = conn.call_method(
                Some("com.parchlinux.mirrorman.Helper"),
                "/com/parchlinux/mirrorman/Helper",
                Some("com.parchlinux.mirrorman.Helper"),
                "SavePacmanConf",
                &(content,),
            ) {
                if let Ok(success) = reply.body().deserialize::<bool>() {
                    if success {
                        return Ok(());
                    }
                }
            }
        }
        Self::fallback_save_pacman_conf(content)
    }

    pub fn run_pacman(args: &[&str]) -> Result<(bool, String, String), String> {
        if let Ok(conn) = Connection::system() {
            let vec_args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
            if let Ok(reply) = conn.call_method(
                Some("com.parchlinux.mirrorman.Helper"),
                "/com/parchlinux/mirrorman/Helper",
                Some("com.parchlinux.mirrorman.Helper"),
                "RunPacman",
                &(vec_args,),
            ) {
                if let Ok(tuple) = reply.body().deserialize::<(bool, String, String)>() {
                    return Ok(tuple);
                }
            }
        }
        Self::fallback_run_pacman(args)
    }

    pub fn run_command(command: &str, args: &[&str]) -> Result<(bool, String, String), String> {
        if let Ok(conn) = Connection::system() {
            let vec_args: Vec<String> = args.iter().map(|s| s.to_string()).collect();
            if let Ok(reply) = conn.call_method(
                Some("com.parchlinux.mirrorman.Helper"),
                "/com/parchlinux/mirrorman/Helper",
                Some("com.parchlinux.mirrorman.Helper"),
                "RunCommand",
                &(command, vec_args),
            ) {
                if let Ok(tuple) = reply.body().deserialize::<(bool, String, String)>() {
                    return Ok(tuple);
                }
            }
        }
        Self::fallback_run_command(command, args)
    }

    fn fallback_save_mirrorlist(content: &str) -> Result<(), String> {
        use std::io::Write;
        let temp_path = "/tmp/mirrorman_mirrorlist";
        let mut f = std::fs::File::create(temp_path).map_err(|e| format!("Failed to create temp file: {e}"))?;
        f.write_all(content.as_bytes()).map_err(|e| format!("Failed to write mirrorlist: {e}"))?;
        let status = std::process::Command::new("pkexec")
            .args(["cp", temp_path, "/etc/pacman.d/mirrorlist"])
            .status()
            .map_err(|e| format!("pkexec failed: {e}"))?;
        let _ = std::fs::remove_file(temp_path);
        if status.success() { Ok(()) } else { Err("pkexec failed to save mirrorlist".to_string()) }
    }

    fn fallback_save_pacman_conf(content: &str) -> Result<(), String> {
        use std::io::Write;
        let temp_path = "/tmp/mirrorman_pacman_conf";
        let mut f = std::fs::File::create(temp_path).map_err(|e| format!("Failed to create temp file: {e}"))?;
        f.write_all(content.as_bytes()).map_err(|e| format!("Failed to write config: {e}"))?;
        let status = std::process::Command::new("pkexec")
            .args(["cp", temp_path, "/etc/pacman.conf"])
            .status()
            .map_err(|e| format!("pkexec failed: {e}"))?;
        let _ = std::fs::remove_file(temp_path);
        if status.success() { Ok(()) } else { Err("pkexec failed to save pacman.conf".to_string()) }
    }

    fn fallback_run_pacman(args: &[&str]) -> Result<(bool, String, String), String> {
        let output = std::process::Command::new("pkexec")
            .arg("pacman")
            .args(args)
            .output()
            .map_err(|e| format!("Failed to execute pkexec pacman: {e}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Ok((output.status.success(), stdout, stderr))
    }

    fn fallback_run_command(command: &str, args: &[&str]) -> Result<(bool, String, String), String> {
        let output = std::process::Command::new("pkexec")
            .arg(command)
            .args(args)
            .output()
            .map_err(|e| format!("Failed to execute pkexec {command}: {e}"))?;
        let stdout = String::from_utf8_lossy(&output.stdout).to_string();
        let stderr = String::from_utf8_lossy(&output.stderr).to_string();
        Ok((output.status.success(), stdout, stderr))
    }
}
