use std::collections::HashMap;
use std::fs;
use std::process::Command;

use mirrorman_core::helper_guard::{
    validate_pacman_args, validate_run_command, BLACKARCH_STRAP_SCRIPT,
};
use zbus::{connection::Builder, fdo, interface, Connection, MessageHeader, zvariant};

struct Helper;

fn run_output(cmd: &str, args: &[String]) -> fdo::Result<(bool, String, String)> {
    let output = Command::new(cmd)
        .args(args)
        .output()
        .map_err(|e| fdo::Error::Failed(format!("Failed to run command '{cmd}': {e}")))?;
    Ok((
        output.status.success(),
        String::from_utf8_lossy(&output.stdout).to_string(),
        String::from_utf8_lossy(&output.stderr).to_string(),
    ))
}

/// Ask polkit whether the caller may perform `action`. The caller's process id
/// is resolved from the D-Bus message header, so spoofing the sender name is
/// not enough to bypass this check.
async fn authorize(conn: &Connection, header: &MessageHeader<'_>, action: &str) -> fdo::Result<bool> {
    let Some(sender) = header.sender() else {
        return Err(fdo::Error::AccessDenied("no sender".to_string()));
    };
    let sender = sender.as_str().to_string();

    let reply = conn
        .call_method(
            Some("org.freedesktop.DBus"),
            "/org/freedesktop/DBus",
            Some("org.freedesktop.DBus"),
            "GetConnectionUnixProcessID",
            &(sender,),
        )
        .await
        .map_err(|e| fdo::Error::Failed(format!("failed to resolve caller pid: {e}")))?;
    let pid: u32 = reply
        .body()
        .deserialize()
        .map_err(|e| fdo::Error::Failed(format!("invalid pid reply: {e}")))?;

    let subject = (
        "unix-process".to_string(),
        HashMap::from([("pid".to_string(), zvariant::Value::U32(pid))]),
    );
    let details: HashMap<String, String> = HashMap::new();
    let flags: u32 = 0;
    let cancellation: String = String::new();

    let reply = conn
        .call_method(
            Some("org.freedesktop.PolicyKit1"),
            "/org/freedesktop/PolicyKit1/Authority",
            Some("org.freedesktop.PolicyKit1.Authority"),
            "CheckAuthorization",
            &(subject, action.to_string(), details, flags, cancellation),
        )
        .await
        .map_err(|e| fdo::Error::Failed(format!("polkit check failed: {e}")))?;
    let (is_authorized, _details): (bool, zvariant::OwnedValue) = reply
        .body()
        .deserialize()
        .map_err(|e| fdo::Error::Failed(format!("invalid polkit reply: {e}")))?;
    Ok(is_authorized)
}

async fn require_auth(
    conn: &Connection,
    header: &MessageHeader<'_>,
    action: &str,
) -> fdo::Result<()> {
    if !authorize(conn, header, action).await? {
        return Err(fdo::Error::AccessDenied(format!(
            "not authorized for {action}"
        )));
    }
    Ok(())
}

const MAX_CONTENT_LEN: usize = 1_000_000;

#[interface(name = "com.parchlinux.mirrorman.Helper")]
impl Helper {
    async fn save_mirrorlist(
        &self,
        content: String,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: MessageHeader<'_>,
    ) -> fdo::Result<bool> {
        require_auth(conn, &header, "com.parchlinux.mirrorman.edit-mirrorlist").await?;
        if content.len() > MAX_CONTENT_LEN {
            return Err(fdo::Error::Failed(
                "mirrorlist content too large".to_string(),
            ));
        }
        let mirrorlist_path = "/etc/pacman.d/mirrorlist";
        let backup_path = "/etc/pacman.d/mirrorlist.backup";
        if std::path::Path::new(mirrorlist_path).exists() {
            let _ = fs::copy(mirrorlist_path, backup_path);
        }
        fs::write(mirrorlist_path, content)
            .map(|_| true)
            .map_err(|e| fdo::Error::Failed(format!("Failed to write mirrorlist: {e}")))
    }

    async fn save_pacman_conf(
        &self,
        content: String,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: MessageHeader<'_>,
    ) -> fdo::Result<bool> {
        require_auth(conn, &header, "com.parchlinux.mirrorman.edit-pacman-conf").await?;
        if content.len() > MAX_CONTENT_LEN {
            return Err(fdo::Error::Failed(
                "pacman.conf content too large".to_string(),
            ));
        }
        fs::write("/etc/pacman.conf", content)
            .map(|_| true)
            .map_err(|e| fdo::Error::Failed(format!("Failed to write pacman.conf: {e}")))
    }

    async fn run_pacman(
        &self,
        args: Vec<String>,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: MessageHeader<'_>,
    ) -> fdo::Result<(bool, String, String)> {
        require_auth(conn, &header, "com.parchlinux.mirrorman.sync-repos").await?;
        validate_pacman_args(&args).map_err(fdo::Error::AccessDenied)?;
        run_output("/usr/bin/pacman", &args)
    }

    async fn run_command(
        &self,
        command: String,
        args: Vec<String>,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: MessageHeader<'_>,
    ) -> fdo::Result<(bool, String, String)> {
        require_auth(conn, &header, "com.parchlinux.mirrorman.sync-repos").await?;
        validate_run_command(&command, &args).map_err(fdo::Error::AccessDenied)?;
        let path = match command.as_str() {
            "cp" => "/usr/bin/cp",
            "pacman-key" => "/usr/bin/pacman-key",
            _ => return Err(fdo::Error::AccessDenied("command not whitelisted".to_string())),
        };
        run_output(path, &args)
    }

    async fn run_blackarch_strap(
        &self,
        #[zbus(connection)] conn: &Connection,
        #[zbus(header)] header: MessageHeader<'_>,
    ) -> fdo::Result<(bool, String, String)> {
        require_auth(conn, &header, "com.parchlinux.mirrorman.third-party").await?;
        run_output("/usr/bin/bash", &["-c".to_string(), BLACKARCH_STRAP_SCRIPT.to_string()])
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
