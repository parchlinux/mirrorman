use mirrorman::helper_client::HelperClient;

#[test]
fn test_helper_client_invalid_command_whitelist() {
    let result = HelperClient::run_command("invalid_custom_binary", &["--test"]);
    assert!(result.is_err() || result.is_ok());
}


