use std::fs;
use std::process::Command;
use zbus::{interface, connection::Builder, fdo};

struct Helper;

#[interface(name = "com.parchlinux.mirrorman.Helper")]
impl Helper {
    async fn save_mirrorlist(&self, content: String) -> fdo::Result<bool> {
        let mirrorlist_path = "/etc/pacman.d/mirrorlist";
        let backup_path = "/etc/pacman.d/mirrorlist.backup";
        if std::path::Path::new(mirrorlist_path).exists() {
            let _ = fs::copy(mirrorlist_path, backup_path);
        }
        match fs::write(mirrorlist_path, content) {
            Ok(_) => Ok(true),
            Err(e) => Err(fdo::Error::Failed(format!("Failed to write mirrorlist: {e}"))),
        }
    }

    async fn save_pacman_conf(&self, content: String) -> fdo::Result<bool> {
        let conf_path = "/etc/pacman.conf";
        match fs::write(conf_path, content) {
            Ok(_) => Ok(true),
            Err(e) => Err(fdo::Error::Failed(format!("Failed to write pacman.conf: {e}"))),
        }
    }

    async fn run_pacman(&self, args: Vec<String>) -> fdo::Result<(bool, String, String)> {
        let output = Command::new("/usr/bin/pacman")
            .args(&args)
            .output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                Ok((out.status.success(), stdout, stderr))
            }
            Err(e) => Err(fdo::Error::Failed(format!("Failed to run pacman: {e}"))),
        }
    }

    async fn run_command(&self, command: String, args: Vec<String>) -> fdo::Result<(bool, String, String)> {
        let allowed = ["pacman-key", "cp", "curl", "bash", "pacman"];
        if !allowed.contains(&command.as_str()) {
            return Err(fdo::Error::Failed(format!("Command '{command}' is not whitelisted")));
        }

        let output = Command::new(&command)
            .args(&args)
            .output();

        match output {
            Ok(out) => {
                let stdout = String::from_utf8_lossy(&out.stdout).to_string();
                let stderr = String::from_utf8_lossy(&out.stderr).to_string();
                Ok((out.status.success(), stdout, stderr))
            }
            Err(e) => Err(fdo::Error::Failed(format!("Failed to run command '{command}': {e}"))),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let helper = Helper;
    let _conn = Builder::system()?
        .name("com.parchlinux.mirrorman.Helper")?
        .serve_at("/com/parchlinux/mirrorman/Helper", helper)?
        .build()
        .await?;

    println!("mirrorman-helper D-Bus service active.");
    std::future::pending::<()>().await;
    Ok(())
}
