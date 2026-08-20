use mirrorman_core::helper_client::HelperClient;
use mirrorman_core::helper_guard::{
    validate_pacman_args, validate_pacman_key_args, validate_run_command,
};

#[test]
fn test_helper_client_invalid_command_whitelist() {
    let result = HelperClient::run_command("invalid_custom_binary", &["--test"]);
    assert!(result.is_err(), "non-whitelisted command must be rejected client-side");
}

#[test]
fn test_helper_client_shell_commands_rejected() {
    let result = HelperClient::run_command("bash", &["-c", "rm -rf /"]);
    assert!(result.is_err(), "bash must never be executable via run_command");
}

#[test]
fn test_helper_client_curl_rejected() {
    let result = HelperClient::run_command("curl", &["-O", "https://blackarch.org/strap.sh"]);
    assert!(result.is_err(), "curl must never be executable via run_command");
}

#[test]
fn test_guard_pacman_args() {
    assert!(validate_pacman_args(&["-Syy".to_string()]).is_ok());
    assert!(validate_pacman_args(&["--root".to_string(), "/tmp".to_string()]).is_err());
    assert!(validate_pacman_args(&["-S".to_string(), "foo;rm -rf /".to_string()]).is_err());
}

#[test]
fn test_guard_pacman_key_args() {
    assert!(validate_pacman_key_args(&[
        "--recv-key".to_string(),
        "3056513887B78AEB".to_string(),
    ])
    .is_ok());
    assert!(validate_pacman_key_args(&["--recv-key".to_string(), "bash".to_string()]).is_err());
}

#[test]
fn test_guard_run_command() {
    assert!(validate_run_command(
        "cp",
        &["/etc/pacman.d/mirrorlist".to_string(), "/etc/pacman.d/mirrorlist.bak".to_string()]
    )
    .is_ok());
    assert!(validate_run_command(
        "pacman-key",
        &["--lsign-key".to_string(), "3056513887B78AEB".to_string()]
    )
    .is_ok());
    assert!(validate_run_command("bash", &["-c".to_string(), "true".to_string()]).is_err());
    assert!(validate_run_command("pacman", &["-Syy".to_string()]).is_err());
}
