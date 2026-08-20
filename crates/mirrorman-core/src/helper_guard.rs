//! Whitelist validation for privileged helper operations.
//!
//! Shared by the root `mirrorman-helper` D-Bus service and the unprivileged
//! client so both sides enforce the same allow-lists. Nothing here runs any
//! command; it only decides whether a proposed invocation is safe enough to
//! forward to a root context.

/// Pacman operations the helper may perform. The first argument must be one of
/// these combined flags; anything after it is validated per-argument.
const ALLOWED_PACMAN_OPS: &[&str] = &["-S", "-Sy", "-Syy", "-Su", "-Syu", "-Syyu", "-Sc", "-U"];

/// Additional flags allowed after the operation. Crucially absent: `--root`,
/// `--config`, `--dbpath`, `--cachedir`, `--hookdir`, `--logfile`, `-r`, `-b`.
const ALLOWED_PACMAN_FLAGS: &[&str] = &["--noconfirm", "--needed", "--refresh", "--clean"];

/// Validate the argument vector for a `pacman` invocation.
pub fn validate_pacman_args(args: &[String]) -> Result<(), String> {
    let Some(first) = args.first() else {
        return Err("pacman requires at least one operation flag".to_string());
    };
    if !ALLOWED_PACMAN_OPS.contains(&first.as_str()) {
        return Err(format!("pacman operation '{first}' is not allowed"));
    }
    for a in &args[1..] {
        if a.starts_with('-') {
            if !ALLOWED_PACMAN_FLAGS.contains(&a.as_str()) {
                return Err(format!("pacman flag '{a}' is not allowed"));
            }
            continue;
        }
        if !is_safe_pacman_value(a) {
            return Err(format!("pacman argument '{a}' is not allowed"));
        }
    }
    Ok(())
}

/// Package names, HTTP(S) URLs and absolute file paths are the only acceptable
/// "value" arguments to whitelisted pacman operations.
fn is_safe_pacman_value(s: &str) -> bool {
    if s.is_empty() || s.len() >= 2048 {
        return false;
    }
    if s.starts_with("http://") || s.starts_with("https://") {
        return !s.chars().any(char::is_whitespace);
    }
    if s.starts_with('/') {
        return !s.contains("..");
    }
    // package spec: alphanumerics plus common separators (`pkg>=1.0` epoch
    // colons, provider `@`, repo `:`).
    s.chars()
        .all(|c| c.is_ascii_alphanumeric() || "-._+@:=<>".contains(c))
}

/// The `cp` command is only allowed to back up the mirrorlist into
/// `/etc/pacman.d/`.
pub fn validate_cp_args(args: &[String]) -> Result<(), String> {
    if args.len() != 2 {
        return Err("cp requires exactly two arguments".to_string());
    }
    if args[0] != "/etc/pacman.d/mirrorlist" {
        return Err("cp source must be /etc/pacman.d/mirrorlist".to_string());
    }
    let dest = &args[1];
    if !dest.starts_with("/etc/pacman.d/") || dest.contains("..") || dest.len() >= 2048 {
        return Err("cp destination must be a path under /etc/pacman.d/".to_string());
    }
    Ok(())
}

fn is_hex_key(s: &str) -> bool {
    (s.len() >= 16 && s.len() <= 64) && s.chars().all(|c| c.is_ascii_hexdigit())
}

/// `pacman-key` is limited to importing/signing a single PGP key id, with an
/// optional `--keyserver` host.
pub fn validate_pacman_key_args(args: &[String]) -> Result<(), String> {
    if args.len() == 2 && (args[0] == "--recv-key" || args[0] == "--lsign-key") {
        if !is_hex_key(&args[1]) {
            return Err("invalid key id".to_string());
        }
        return Ok(());
    }
    if args.len() == 4 && args[0] == "--recv-key" && args[2] == "--keyserver" {
        if !is_hex_key(&args[1]) {
            return Err("invalid key id".to_string());
        }
        if !args[3]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-' || c == ':')
        {
            return Err("invalid keyserver".to_string());
        }
        return Ok(());
    }
    Err("unsupported pacman-key invocation".to_string())
}

/// Validate a `RunCommand` request. Only fixed, low-risk invocations are
/// allowed; shell interpreters and network tools are never permitted here.
pub fn validate_run_command(command: &str, args: &[String]) -> Result<(), String> {
    match command {
        "cp" => validate_cp_args(args),
        "pacman-key" => validate_pacman_key_args(args),
        _ => Err(format!("command '{command}' is not whitelisted")),
    }
}

/// The exact, pinned BlackArch bootstrap script. It downloads `strap.sh`,
/// verifies its SHA1, and runs it — all with fixed arguments, so it is safe to
/// allow as a dedicated operation rather than as arbitrary `bash -c`.
pub const BLACKARCH_STRAP_SCRIPT: &str = "cd /tmp && curl -O https://blackarch.org/strap.sh && echo '26849980b35a42e6e192c6d9ed8c46f0d6d06047  strap.sh' | sha1sum -c && chmod +x strap.sh && ./strap.sh && rm -f strap.sh";

#[cfg(test)]
mod tests {
    use super::*;

    fn v(s: &[&str]) -> Vec<String> {
        s.iter().map(|x| x.to_string()).collect()
    }

    #[test]
    fn pacman_sync_allowed() {
        assert!(validate_pacman_args(&v(&["-Syy", "--noconfirm"])).is_ok());
        assert!(validate_pacman_args(&v(&["-Syu", "--noconfirm"])).is_ok());
    }

    #[test]
    fn pacman_install_allowed() {
        assert!(validate_pacman_args(&v(&["-S", "archlinuxcn-keyring", "--noconfirm"])).is_ok());
        assert!(validate_pacman_args(&v(&["-S", "linux", "linux-headers"])).is_ok());
        assert!(validate_pacman_args(&v(&["-S", "pkg>=1.0", "--needed"])).is_ok());
    }

    #[test]
    fn pacman_url_install_allowed() {
        assert!(validate_pacman_args(&v(&[
            "-U",
            "--noconfirm",
            "https://cdn-mirror.chaotic.cx/chaotic-aur/chaotic-keyring.pkg.tar.zst",
        ]))
        .is_ok());
    }

    #[test]
    fn pacman_dangerous_rejected() {
        assert!(validate_pacman_args(&v(&["--root", "/tmp"])).is_err());
        assert!(validate_pacman_args(&v(&["-Syy", "--config", "/tmp/x.conf"])).is_err());
        assert!(validate_pacman_args(&v(&["-Syy", "--dbpath", "/tmp"])).is_err());
        assert!(validate_pacman_args(&v(&["-r", "/tmp"])).is_err());
        assert!(validate_pacman_args(&v(&["-S", "foo; rm -rf /"])).is_err());
        assert!(validate_pacman_args(&v(&["-S", "--noconfirm", "--root", "/"])).is_err());
        assert!(validate_pacman_args(&v(&[])).is_err());
    }

    #[test]
    fn cp_backup_allowed() {
        assert!(validate_cp_args(&v(&[
            "/etc/pacman.d/mirrorlist",
            "/etc/pacman.d/mirrorlist.backup.20260724_101010",
        ]))
        .is_ok());
    }

    #[test]
    fn cp_abuse_rejected() {
        assert!(validate_cp_args(&v(&["/etc/shadow", "/tmp/x"])).is_err());
        assert!(validate_cp_args(&v(&["/etc/pacman.d/mirrorlist", "/tmp/x"])).is_err());
        assert!(validate_cp_args(&v(&["/etc/pacman.d/mirrorlist", "/etc/pacman.d/../shadow"])).is_err());
        assert!(validate_cp_args(&v(&["/etc/pacman.d/mirrorlist"])).is_err());
    }

    #[test]
    fn pacman_key_allowed() {
        assert!(validate_pacman_key_args(&v(&[
            "--recv-key",
            "4D41FD3D9E72E7966A573093E8CA6AEB220E236C",
            "--keyserver",
            "keyserver.ubuntu.com",
        ]))
        .is_ok());
        assert!(validate_pacman_key_args(&v(&[
            "--lsign-key",
            "4D41FD3D9E72E7966A573093E8CA6AEB220E236C",
        ]))
        .is_ok());
    }

    #[test]
    fn pacman_key_abuse_rejected() {
        assert!(validate_pacman_key_args(&v(&["--recv-key", "bash"])).is_err());
        assert!(validate_pacman_key_args(&v(&["--recv-key", "AB", "--keyserver", "evil.example"])).is_err());
        assert!(validate_pacman_key_args(&v(&["--lsign-key", "--init"])).is_err());
    }

    #[test]
    fn run_command_whitelist() {
        assert!(validate_run_command("cp", &v(&["/etc/pacman.d/mirrorlist", "/etc/pacman.d/mirrorlist.bak"])).is_ok());
        assert!(validate_run_command("bash", &v(&["-c", "rm -rf /"])).is_err());
        assert!(validate_run_command("curl", &v(&["-O", "https://blackarch.org/strap.sh"])).is_err());
        assert!(validate_run_command("pacman", &v(&["-Syy"])).is_err());
        assert!(validate_run_command("systemctl", &v(&["poweroff"])).is_err());
    }
}
