use mirrorman::mirror_manager::MirrorManager;

#[test]
fn test_cli_auto_optimize_flow() {
    let mut mgr = MirrorManager::new();
    mgr.mirrors = vec![
        mirrorman::mirror_manager::Mirror {
            url: "https://mirror.de/".to_string(),
            country: "Germany".to_string(),
            country_code: "DE".to_string(),
            protocol: "https".to_string(),
            speed: Some(25.0),
            last_sync: None,
            enabled: false,
            ipv4: true,
            ipv6: false,
            completion_pct: Some(1.0),
            score: Some(1.0),
            duration_avg: Some(0.1),
            duration_stddev: Some(0.01),
        },
        mirrorman::mirror_manager::Mirror {
            url: "https://mirror.fr/".to_string(),
            country: "France".to_string(),
            country_code: "FR".to_string(),
            protocol: "https".to_string(),
            speed: Some(30.0),
            last_sync: None,
            enabled: false,
            ipv4: true,
            ipv6: false,
            completion_pct: Some(1.0),
            score: Some(1.2),
            duration_avg: Some(0.12),
            duration_stddev: Some(0.02),
        },
    ];

    let selected = mgr.auto_optimize();
    assert_eq!(selected.len(), 2);
    assert!(selected.iter().any(|m| m.country == "Germany"));
    assert!(selected.iter().any(|m| m.country == "France"));
}

#[test]
fn test_cli_mirrorlist_content_generation() {
    let mut mgr = MirrorManager::new();
    mgr.mirrors = vec![
        mirrorman::mirror_manager::Mirror {
            url: "https://mirror.arch.org/".to_string(),
            country: "Worldwide".to_string(),
            country_code: "WW".to_string(),
            protocol: "https".to_string(),
            speed: Some(15.0),
            last_sync: None,
            enabled: true,
            ipv4: true,
            ipv6: true,
            completion_pct: Some(1.0),
            score: Some(1.0),
            duration_avg: None,
            duration_stddev: None,
        },
    ];

    let content = mgr.generate_mirrorlist_content();
    assert!(content.contains("https://mirror.arch.org/$repo/os/$arch"));
    assert!(content.contains("1 enabled mirror"));
}
