use crate::tr;
use adw::prelude::*;
use std::sync::Arc;
use std::sync::Mutex;

const PACMAN_CONF: &str = "/etc/pacman.conf";

struct PacmanConfig {
    ignore_pkg: Vec<String>,
    hold_pkg: Vec<String>,
    no_upgrade: Vec<String>,
    no_extract: Vec<String>,
    sync_first: Vec<String>,
    check_space: bool,
    ilovecandy: bool,
    parallel_downloads: i32,
    clean_method: String,
    architecture: String,
}

impl PacmanConfig {
    fn load() -> Self {
        let content = match std::fs::read_to_string(PACMAN_CONF) {
            Ok(c) => c,
            Err(_) => return Self::default(),
        };

        let mut cfg = Self::default();
        let mut in_options = false;

        for line in content.lines() {
            let stripped = line.trim();
            if stripped.eq_ignore_ascii_case("[options]") {
                in_options = true;
                continue;
            }
            if in_options && stripped.starts_with('[') && !stripped.eq_ignore_ascii_case("[options]") {
                in_options = false;
                continue;
            }
            if !in_options { continue; }
            if stripped.starts_with('#') || stripped.is_empty() { continue; }

            if let Some((key, val)) = stripped.split_once('=') {
                let key = key.trim();
                let val = val.trim();
                match key {
                    "IgnorePkg" => cfg.ignore_pkg = val.split_whitespace().map(|s| s.to_string()).collect(),
                    "HoldPkg" => cfg.hold_pkg = val.split_whitespace().map(|s| s.to_string()).collect(),
                    "NoUpgrade" => cfg.no_upgrade = val.split_whitespace().map(|s| s.to_string()).collect(),
                    "NoExtract" => cfg.no_extract = val.split_whitespace().map(|s| s.to_string()).collect(),
                    "SyncFirst" => cfg.sync_first = val.split_whitespace().map(|s| s.to_string()).collect(),
                    "ParallelDownloads" => { if let Ok(n) = val.parse() { cfg.parallel_downloads = n; } }
                    "CleanMethod" => cfg.clean_method = val.to_string(),
                    "Architecture" => cfg.architecture = val.to_string(),
                    _ => {}
                }
            } else {
                let key = stripped.trim();
                match key {
                    "CheckSpace" => cfg.check_space = true,
                    "ILoveCandy" => cfg.ilovecandy = true,
                    _ => {}
                }
            }
        }

        cfg
    }

    fn default() -> Self {
        Self {
            ignore_pkg: Vec::new(),
            hold_pkg: Vec::new(),
            no_upgrade: Vec::new(),
            no_extract: Vec::new(),
            sync_first: Vec::new(),
            check_space: false,
            ilovecandy: false,
            parallel_downloads: 5,
            clean_method: "KeepInstalled".to_string(),
            architecture: "auto".to_string(),
        }
    }
}

pub fn show_settings_sheet(bottom_sheet: &adw::BottomSheet) {
    let config = Arc::new(Mutex::new(PacmanConfig::load()));

    let toolbar_view = adw::ToolbarView::new();

    let header = adw::HeaderBar::new();
    let title_label = gtk4::Label::new(Some(tr!("Pacman Options")));
    title_label.add_css_class("title");
    header.set_title_widget(Some(&title_label));
    toolbar_view.add_top_bar(&header);

    let save_btn = gtk4::Button::with_label(tr!("Save"));
    save_btn.add_css_class("suggested-action");
    header.pack_end(&save_btn);

    let cancel_btn = gtk4::Button::with_label(tr!("Cancel"));
    {
        let bottom_sheet = bottom_sheet.clone();
        cancel_btn.connect_clicked(move |_| {
            bottom_sheet.set_open(false);
        });
    }
    header.pack_start(&cancel_btn);

    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    scroll.set_vexpand(true);
    scroll.set_propagate_natural_height(true);
    toolbar_view.set_content(Some(&scroll));

    let box_ = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    box_.set_margin_top(12);
    box_.set_margin_bottom(12);
    box_.set_margin_start(12);
    box_.set_margin_end(12);
    scroll.set_child(Some(&box_));

    let group = adw::PreferencesGroup::new();
    group.set_title(tr!("Package Management"));
    box_.append(&group);

    let cfg = config.lock().unwrap();

    let ignore_row = adw::ActionRow::new();
    ignore_row.set_title(tr!("IgnorePkg"));
    ignore_row.set_subtitle(tr!("Space-separated packages to ignore during upgrades"));
    let ignore_entry = gtk4::Entry::new();
    ignore_entry.set_text(&cfg.ignore_pkg.join(" "));
    ignore_row.add_suffix(&ignore_entry);
    group.add(&ignore_row);

    let hold_row = adw::ActionRow::new();
    hold_row.set_title(tr!("HoldPkg"));
    hold_row.set_subtitle(tr!("Space-separated packages to hold during upgrades"));
    let hold_entry = gtk4::Entry::new();
    hold_entry.set_text(&cfg.hold_pkg.join(" "));
    hold_row.add_suffix(&hold_entry);
    group.add(&hold_row);

    let noupgrade_row = adw::ActionRow::new();
    noupgrade_row.set_title(tr!("NoUpgrade"));
    noupgrade_row.set_subtitle(tr!("Space-separated files to protect from upgrade"));
    let noupgrade_entry = gtk4::Entry::new();
    noupgrade_entry.set_text(&cfg.no_upgrade.join(" "));
    noupgrade_row.add_suffix(&noupgrade_entry);
    group.add(&noupgrade_row);

    let noextract_row = adw::ActionRow::new();
    noextract_row.set_title(tr!("NoExtract"));
    noextract_row.set_subtitle(tr!("Space-separated files to skip during extraction"));
    let noextract_entry = gtk4::Entry::new();
    noextract_entry.set_text(&cfg.no_extract.join(" "));
    noextract_row.add_suffix(&noextract_entry);
    group.add(&noextract_row);

    let syncfirst_row = adw::ActionRow::new();
    syncfirst_row.set_title(tr!("SyncFirst"));
    syncfirst_row.set_subtitle(tr!("Space-separated packages to sync first"));
    let syncfirst_entry = gtk4::Entry::new();
    syncfirst_entry.set_text(&cfg.sync_first.join(" "));
    syncfirst_row.add_suffix(&syncfirst_entry);
    group.add(&syncfirst_row);

    let checkspace_row = adw::ActionRow::new();
    checkspace_row.set_title(tr!("CheckSpace"));
    checkspace_row.set_subtitle(tr!("Check for sufficient disk space before installing"));
    let checkspace_switch = gtk4::Switch::new();
    checkspace_switch.set_valign(gtk4::Align::Center);
    checkspace_switch.set_active(cfg.check_space);
    checkspace_row.add_suffix(&checkspace_switch);
    checkspace_row.set_activatable_widget(Some(&checkspace_switch));
    group.add(&checkspace_row);

    let candy_row = adw::ActionRow::new();
    candy_row.set_title(tr!("ILoveCandy"));
    candy_row.set_subtitle(tr!("Display a candy cane progress bar"));
    let candy_switch = gtk4::Switch::new();
    candy_switch.set_valign(gtk4::Align::Center);
    candy_switch.set_active(cfg.ilovecandy);
    candy_row.add_suffix(&candy_switch);
    candy_row.set_activatable_widget(Some(&candy_switch));
    group.add(&candy_row);

    let parallel_row = adw::ActionRow::new();
    parallel_row.set_title(tr!("ParallelDownloads"));
    parallel_row.set_subtitle(tr!("Number of parallel downloads (1-100)"));
    let parallel_spin = gtk4::SpinButton::with_range(1.0, 100.0, 1.0);
    parallel_spin.set_value(cfg.parallel_downloads as f64);
    parallel_row.add_suffix(&parallel_spin);
    group.add(&parallel_row);

    let cleanmethod_row = adw::ComboRow::new();
    cleanmethod_row.set_title(tr!("CleanMethod"));
    cleanmethod_row.set_subtitle(tr!("Cache cleaning method"));
    let cleanmethod_store = gtk4::StringList::new(&[tr!("KeepInstalled"), tr!("KeepCurrent")]);
    cleanmethod_row.set_model(Some(&cleanmethod_store));
    cleanmethod_row.set_selected(if cfg.clean_method == "KeepCurrent" { 1 } else { 0 });
    group.add(&cleanmethod_row);

    let arch_row = adw::ComboRow::new();
    arch_row.set_title(tr!("Architecture"));
    arch_row.set_subtitle(tr!("System architecture"));
    let arch_store = gtk4::StringList::new(&[tr!("auto"), tr!("x86_64"), tr!("x86_64_v3")]);
    arch_row.set_model(Some(&arch_store));
    arch_row.set_selected(match cfg.architecture.as_str() {
        "x86_64" => 1,
        "x86_64_v3" => 2,
        _ => 0,
    });
    group.add(&arch_row);

    let auto_group = adw::PreferencesGroup::new();
    auto_group.set_title(tr!("Auto Refresh Timer"));
    auto_group.set_description(Some(tr!("Automatically update mirrorlist on a systemd timer schedule")));
    box_.append(&auto_group);

    let auto_row = adw::ActionRow::new();
    auto_row.set_title(tr!("Enable Auto Refresh"));
    auto_row.set_subtitle(tr!("Run systemd background timer unit (mirrorman-refresh.timer)"));
    let auto_switch = gtk4::Switch::new();
    auto_switch.set_valign(gtk4::Align::Center);
    let timer_active = std::process::Command::new("systemctl")
        .args(["--user", "is-active", "mirrorman-refresh.timer"])
        .output()
        .map(|o| String::from_utf8_lossy(&o.stdout).trim() == "active")
        .unwrap_or(false);
    auto_switch.set_active(timer_active);
    auto_row.add_suffix(&auto_switch);
    auto_row.set_activatable_widget(Some(&auto_switch));
    auto_group.add(&auto_row);

    drop(cfg);

    let cfg_clone = config.clone();
    let bottom_sheet_save = bottom_sheet.clone();
    save_btn.connect_clicked(move |_| {
        if auto_switch.is_active() {
            let _ = std::process::Command::new("systemctl")
                .args(["--user", "enable", "--now", "mirrorman-refresh.timer"])
                .status();
        } else {
            let _ = std::process::Command::new("systemctl")
                .args(["--user", "disable", "--now", "mirrorman-refresh.timer"])
                .status();
        }

        let mut cfg = cfg_clone.lock().unwrap();
        cfg.ignore_pkg = ignore_entry.text().split_whitespace().map(|s| s.to_string()).collect();
        cfg.hold_pkg = hold_entry.text().split_whitespace().map(|s| s.to_string()).collect();
        cfg.no_upgrade = noupgrade_entry.text().split_whitespace().map(|s| s.to_string()).collect();
        cfg.no_extract = noextract_entry.text().split_whitespace().map(|s| s.to_string()).collect();
        cfg.sync_first = syncfirst_entry.text().split_whitespace().map(|s| s.to_string()).collect();
        cfg.check_space = checkspace_switch.is_active();
        cfg.ilovecandy = candy_switch.is_active();
        cfg.parallel_downloads = parallel_spin.value() as i32;
        cfg.clean_method = if cleanmethod_row.selected() == 1 { "KeepCurrent".to_string() } else { "KeepInstalled".to_string() };
        cfg.architecture = match arch_row.selected() {
            1 => "x86_64".to_string(),
            2 => "x86_64_v3".to_string(),
            _ => "auto".to_string(),
        };

        let _ = save_pacman_config(&cfg);
        bottom_sheet_save.set_open(false);
    });

    bottom_sheet.set_sheet(Some(&toolbar_view));
    bottom_sheet.set_open(true);
}

fn save_pacman_config(cfg: &PacmanConfig) -> Result<(), String> {

    let content = std::fs::read_to_string(PACMAN_CONF)
        .map_err(|e| format!("Failed to read pacman.conf: {e}"))?;

    let mut new_lines: Vec<String> = Vec::new();
    let mut in_options = false;
    let mut added = std::collections::HashSet::new();

    let updates: Vec<(&str, Option<String>)> = vec![
        ("IgnorePkg", if cfg.ignore_pkg.is_empty() { None } else { Some(format!("IgnorePkg = {}\n", cfg.ignore_pkg.join(" "))) }),
        ("HoldPkg", if cfg.hold_pkg.is_empty() { None } else { Some(format!("HoldPkg = {}\n", cfg.hold_pkg.join(" "))) }),
        ("NoUpgrade", if cfg.no_upgrade.is_empty() { None } else { Some(format!("NoUpgrade = {}\n", cfg.no_upgrade.join(" "))) }),
        ("NoExtract", if cfg.no_extract.is_empty() { None } else { Some(format!("NoExtract = {}\n", cfg.no_extract.join(" "))) }),
        ("SyncFirst", if cfg.sync_first.is_empty() { None } else { Some(format!("SyncFirst = {}\n", cfg.sync_first.join(" "))) }),
        ("CheckSpace", if cfg.check_space { Some("CheckSpace\n".to_string()) } else { None }),
        ("ILoveCandy", if cfg.ilovecandy { Some("ILoveCandy\n".to_string()) } else { None }),
        ("ParallelDownloads", Some(format!("ParallelDownloads = {}\n", cfg.parallel_downloads))),
        ("CleanMethod", Some(format!("CleanMethod = {}\n", cfg.clean_method))),
        ("Architecture", Some(format!("Architecture = {}\n", cfg.architecture))),
    ];

    for line in content.lines() {
        let stripped = line.trim().to_string();
        let lowered = stripped.to_lowercase();

        if lowered == "[options]" {
            in_options = true;
            new_lines.push(line.to_string());
            continue;
        }

        if in_options && stripped.starts_with('[') && lowered != "[options]" {
            for (key, val) in &updates {
                if let Some(v) = val {
                    if !added.contains(key) {
                        new_lines.push(v.trim_end().to_string());
                        added.insert(key);
                    }
                }
            }
            in_options = false;
            new_lines.push(line.to_string());
            continue;
        }

        if in_options {
            let key = line_key(line);
            let update = updates.iter().find(|(k, _)| *k == key.as_str());
            if let Some((key, Some(_))) = update {
                if !added.contains(key) {
                    added.insert(key);
                }
                continue;
            }
            if let Some((key, None)) = update {
                if !added.contains(key) {
                    added.insert(key);
                }
                continue;
            }
            new_lines.push(line.to_string());
        } else {
            new_lines.push(line.to_string());
        }
    }

    if in_options {
        for (key, val) in &updates {
            if let Some(v) = val {
                if !added.contains(key) {
                    new_lines.push(v.trim_end().to_string());
                    added.insert(key);
                }
            }
        }
    }

    let result = new_lines.join("\n") + "\n";
    crate::helper_client::HelperClient::save_pacman_conf(&result)
}

fn line_key(line: &str) -> String {
    let s = line.trim();
    if s.starts_with('#') { return String::new(); }
    s.split('=').next().unwrap_or("").trim().to_string()
}
