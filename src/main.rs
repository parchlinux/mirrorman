use mirrorman::i18n;
use mirrorman::log_viewer;
use mirrorman::mirror_manager;
use mirrorman::pacman_settings;
use mirrorman::repo_config;
use mirrorman::sync_manager;
use mirrorman::templates;
use mirrorman::tr;
use mirrorman::utils;

use std::sync::Arc;
use std::sync::Mutex;

use adw::prelude::*;
use adw::ResponseAppearance;
use gtk4::glib;
use mirror_manager::{country_flag, Mirror, MirrorManager};

static APP_ID: &str = "com.parchlinux.mirrorman";

fn build_ui(app: &adw::Application) {
    let window = adw::ApplicationWindow::new(app);
    window.set_title(Some(tr!("Parch Repository Manager")));
    window.set_icon_name(Some("com.parchlinux.mirrorman"));
    window.set_default_size(960, 640);
    window.set_size_request(600, 450);

    // ── Shared state ──
    let mm = Arc::new(Mutex::new(MirrorManager::new()));
    let rc = Arc::new(Mutex::new(repo_config::RepoConfig::new()));

    // ── Channel: background threads → main thread ──
    let (tx, rx) = std::sync::mpsc::channel::<String>();

    // ── Shared reference to mirror_list so channel handler can rebuild it ──
    let mirror_list_holder: Arc<Mutex<Option<gtk4::ListBox>>> = Arc::new(Mutex::new(None));

    // ── Build UI ──
    let toolbar_view = adw::ToolbarView::new();
    let bottom_sheet = adw::BottomSheet::new();
    bottom_sheet.set_content(Some(&toolbar_view));
    bottom_sheet.set_show_drag_handle(true);
    bottom_sheet.set_modal(true);
    window.set_content(Some(&bottom_sheet));

    let header = adw::HeaderBar::new();
    toolbar_view.add_top_bar(&header);

    let header_refresh_btn = gtk4::Button::new();
    header_refresh_btn.set_icon_name("view-refresh-symbolic");
    header_refresh_btn.set_tooltip_text(Some(tr!("Refresh Mirrors")));
    header.pack_start(&header_refresh_btn);

    let templates_btn = gtk4::Button::new();
    templates_btn.set_icon_name("folder-saved-search-symbolic");
    templates_btn.set_tooltip_text(Some(tr!("Mirrorlist Templates")));
    header.pack_end(&templates_btn);

    let history_btn = gtk4::Button::new();
    history_btn.set_icon_name("document-open-recent-symbolic");
    history_btn.set_tooltip_text(Some(tr!("Transaction History")));
    header.pack_end(&history_btn);

    let settings_btn = gtk4::Button::new();
    settings_btn.set_icon_name("preferences-system-symbolic");
    settings_btn.set_tooltip_text(Some(tr!("Pacman Settings")));
    header.pack_end(&settings_btn);

    let about_btn = gtk4::Button::new();
    about_btn.set_icon_name("help-about-symbolic");
    about_btn.set_tooltip_text(Some(tr!("About")));
    header.pack_end(&about_btn);

    let sidebar_toggle_btn = gtk4::Button::new();
    sidebar_toggle_btn.set_icon_name("sidebar-show-symbolic");
    sidebar_toggle_btn.set_tooltip_text(Some(tr!("Toggle Sidebar")));
    header.pack_start(&sidebar_toggle_btn);

    window.set_default_size(960, 640);
    window.set_size_request(360, 480);

    let split_view = adw::OverlaySplitView::new();
    split_view.set_min_sidebar_width(280.0);
    split_view.set_max_sidebar_width(340.0);
    split_view.set_sidebar_width_fraction(0.3);
    toolbar_view.set_content(Some(&split_view));

    {
        let split_view = split_view.clone();
        sidebar_toggle_btn.connect_clicked(move |_| {
            let show = split_view.shows_sidebar();
            split_view.set_show_sidebar(!show);
        });
    }

    let left_sidebar = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    left_sidebar.add_css_class("sidebar");
    split_view.set_sidebar(Some(&left_sidebar));

    let sidebar_scroll = gtk4::ScrolledWindow::new();
    sidebar_scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    sidebar_scroll.set_vexpand(true);
    left_sidebar.append(&sidebar_scroll);

    let sidebar_box = gtk4::Box::new(gtk4::Orientation::Vertical, 18);
    sidebar_box.set_margin_top(18);
    sidebar_box.set_margin_bottom(18);
    sidebar_box.set_margin_start(12);
    sidebar_box.set_margin_end(12);
    sidebar_scroll.set_child(Some(&sidebar_box));

    let filter_clamp = adw::Clamp::new();
    filter_clamp.set_maximum_size(400);
    sidebar_box.append(&filter_clamp);

    let filter_group = adw::PreferencesGroup::new();
    filter_group.set_title(tr!("Mirror Filters"));
    filter_group.set_description(Some(tr!("Configure mirror selection criteria")));
    filter_clamp.set_child(Some(&filter_group));

    let country_row = adw::ComboRow::new();
    country_row.set_title(tr!("Country"));
    let country_store = gtk4::StringList::new(&["Worldwide"]);
    country_row.set_model(Some(&country_store));
    country_row.set_selected(0);
    filter_group.add(&country_row);

    let protocol_row = adw::ActionRow::new();
    protocol_row.set_title(tr!("Protocol"));
    let protocol_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    protocol_box.set_margin_top(6);
    protocol_box.set_margin_bottom(6);
    let http_check = gtk4::CheckButton::with_label("HTTP");
    http_check.set_active(true);
    let https_check = gtk4::CheckButton::with_label("HTTPS");
    https_check.set_active(true);
    protocol_box.append(&http_check);
    protocol_box.append(&https_check);
    protocol_row.add_suffix(&protocol_box);
    filter_group.add(&protocol_row);

    let ip_row = adw::ActionRow::new();
    ip_row.set_title(tr!("IP Version"));
    let ip_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 12);
    ip_box.set_margin_top(6);
    ip_box.set_margin_bottom(6);
    let ipv4_check = gtk4::CheckButton::with_label("IPv4");
    ipv4_check.set_active(true);
    let ipv6_check = gtk4::CheckButton::with_label("IPv6");
    ip_box.append(&ipv4_check);
    ip_box.append(&ipv6_check);
    ip_row.add_suffix(&ip_box);
    filter_group.add(&ip_row);

    let status_row = adw::ActionRow::new();
    status_row.set_title(tr!("Up-to-date only"));
    status_row.set_subtitle(tr!("Show only synchronized mirrors"));
    let status_check = gtk4::Switch::new();
    status_check.set_valign(gtk4::Align::Center);
    status_check.set_active(true);
    status_row.add_suffix(&status_check);
    status_row.set_activatable_widget(Some(&status_check));
    filter_group.add(&status_row);

    let btn_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    btn_box.set_margin_top(12);
    btn_box.set_homogeneous(true);
    sidebar_box.append(&btn_box);

    let refresh_btn = gtk4::Button::new();
    let refresh_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    refresh_box.set_halign(gtk4::Align::Center);
    refresh_box.append(&gtk4::Image::from_icon_name("view-refresh-symbolic"));
    refresh_box.append(&gtk4::Label::new(Some(tr!("Fetch"))));
    refresh_btn.set_child(Some(&refresh_box));
    refresh_btn.add_css_class("suggested-action");
    btn_box.append(&refresh_btn);

    let rank_btn = gtk4::Button::new();
    let rank_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    rank_box.set_halign(gtk4::Align::Center);
    rank_box.append(&gtk4::Image::from_icon_name("emblem-default-symbolic"));
    rank_box.append(&gtk4::Label::new(Some(tr!("Test & Rank"))));
    rank_btn.set_child(Some(&rank_box));
    rank_btn.set_sensitive(false);
    btn_box.append(&rank_btn);

    let loading_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 10);
    loading_box.set_margin_top(8);
    loading_box.set_halign(gtk4::Align::Center);
    let loading_spinner = gtk4::Spinner::new();
    let loading_label = gtk4::Label::new(Some(""));
    loading_label.add_css_class("dim-label");
    loading_label.add_css_class("caption");
    loading_box.append(&loading_spinner);
    loading_box.append(&loading_label);
    sidebar_box.append(&loading_box);

    let sep1 = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    sep1.set_margin_top(6);
    sep1.set_margin_bottom(6);
    sidebar_box.append(&sep1);

    let repo_group = adw::PreferencesGroup::new();
    repo_group.set_title(tr!("Repositories"));
    repo_group.set_description(Some(tr!("Enable or disable repositories")));
    sidebar_box.append(&repo_group);

    let repo_list = gtk4::ListBox::new();
    repo_list.set_selection_mode(gtk4::SelectionMode::None);
    repo_list.add_css_class("boxed-list");
    repo_group.add(&repo_list);

    let sep2 = gtk4::Separator::new(gtk4::Orientation::Horizontal);
    sep2.set_margin_top(6);
    sep2.set_margin_bottom(6);
    sidebar_box.append(&sep2);

    let third_group = adw::PreferencesGroup::new();
    third_group.set_title(tr!("Third-Party Repositories"));
    third_group.set_description(Some(tr!("Enable or disable additional repositories")));
    sidebar_box.append(&third_group);

    let third_list = gtk4::ListBox::new();
    third_list.set_selection_mode(gtk4::SelectionMode::None);
    third_list.add_css_class("boxed-list");
    third_group.add(&third_list);

    let add_repo_btn = gtk4::Button::new();
    let add_repo_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    add_repo_box.set_halign(gtk4::Align::Center);
    add_repo_box.append(&gtk4::Image::from_icon_name("list-add-symbolic"));
    add_repo_box.append(&gtk4::Label::new(Some(tr!("Add Repository"))));
    add_repo_btn.set_child(Some(&add_repo_box));
    add_repo_btn.set_margin_top(6);
    add_repo_btn.set_halign(gtk4::Align::Center);
    sidebar_box.append(&add_repo_btn);

    let sys_grid = gtk4::Grid::new();
    sys_grid.set_column_spacing(8);
    sys_grid.set_row_spacing(8);
    sys_grid.set_column_homogeneous(true);
    sys_grid.set_margin_top(12);
    sidebar_box.append(&sys_grid);

    let sync_btn = gtk4::Button::new();
    let sync_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    sync_box.set_halign(gtk4::Align::Center);
    sync_box.append(&gtk4::Image::from_icon_name("emblem-synchronizing-symbolic"));
    sync_box.append(&gtk4::Label::new(Some(tr!("Sync"))));
    sync_btn.set_child(Some(&sync_box));
    sync_btn.set_tooltip_text(Some(tr!("Save mirrorlist and sync repositories")));
    sys_grid.attach(&sync_btn, 0, 0, 1, 1);

    let clean_btn = gtk4::Button::new();
    let clean_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    clean_box.set_halign(gtk4::Align::Center);
    clean_box.append(&gtk4::Image::from_icon_name("user-trash-symbolic"));
    clean_box.append(&gtk4::Label::new(Some(tr!("Clean"))));
    clean_btn.set_child(Some(&clean_box));
    clean_btn.set_tooltip_text(Some(tr!("Clean package cache")));
    sys_grid.attach(&clean_btn, 1, 0, 1, 1);

    let backup_btn = gtk4::Button::new();
    let backup_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    backup_box.set_halign(gtk4::Align::Center);
    backup_box.append(&gtk4::Image::from_icon_name("document-save-symbolic"));
    backup_box.append(&gtk4::Label::new(Some(tr!("Backup"))));
    backup_btn.set_child(Some(&backup_box));
    backup_btn.set_tooltip_text(Some(tr!("Backup mirrorlist with timestamp")));
    sys_grid.attach(&backup_btn, 0, 1, 1, 1);

    let update_btn = gtk4::Button::new();
    let update_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    update_box.set_halign(gtk4::Align::Center);
    update_box.append(&gtk4::Image::from_icon_name("system-software-update-symbolic"));
    update_box.append(&gtk4::Label::new(Some(tr!("Update"))));
    update_btn.set_child(Some(&update_box));
    update_btn.add_css_class("destructive-action");
    update_btn.set_tooltip_text(Some(tr!("Update all system packages")));
    sys_grid.attach(&update_btn, 1, 1, 1, 1);

    let right_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    right_box.add_css_class("view");
    split_view.set_content(Some(&right_box));

    let mirror_toolbar = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    mirror_toolbar.add_css_class("toolbar");
    mirror_toolbar.set_margin_top(10);
    mirror_toolbar.set_margin_bottom(10);
    mirror_toolbar.set_margin_start(10);
    mirror_toolbar.set_margin_end(10);
    right_box.append(&mirror_toolbar);

    let left_controls = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    mirror_toolbar.append(&left_controls);

    let enable_btn = gtk4::Button::new();
    let ebox = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    ebox.append(&gtk4::Image::from_icon_name("emblem-ok-symbolic"));
    ebox.append(&gtk4::Label::new(Some(tr!("Enable"))));
    enable_btn.set_child(Some(&ebox));
    enable_btn.add_css_class("suggested-action");
    enable_btn.set_sensitive(false);
    left_controls.append(&enable_btn);

    let disable_btn = gtk4::Button::new();
    let dbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    dbox.append(&gtk4::Image::from_icon_name("process-stop-symbolic"));
    dbox.append(&gtk4::Label::new(Some(tr!("Disable"))));
    disable_btn.set_child(Some(&dbox));
    disable_btn.set_sensitive(false);
    left_controls.append(&disable_btn);

    let best_setup_btn = gtk4::Button::new();
    let bbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    bbox.append(&gtk4::Image::from_icon_name("starred-symbolic"));
    bbox.append(&gtk4::Label::new(Some(tr!("Best Setup"))));
    best_setup_btn.set_child(Some(&bbox));
    best_setup_btn.add_css_class("suggested-action");
    best_setup_btn.set_tooltip_text(Some(tr!("Auto-select top optimal mirrors across countries")));
    left_controls.append(&best_setup_btn);

    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    mirror_toolbar.append(&spacer);

    mirror_toolbar.append(&gtk4::Label::new(Some(tr!("Sort by:"))));

    let sort_dropdown = gtk4::DropDown::from_strings(&[
        &tr!("Speed"),
        &tr!("Health"),
        &tr!("Country"),
        &tr!("Age"),
    ]);
    sort_dropdown.set_valign(gtk4::Align::Center);
    sort_dropdown.set_sensitive(false);
    mirror_toolbar.append(&sort_dropdown);

    let avail_btn = gtk4::Button::from_icon_name("emblem-ok-symbolic");
    avail_btn.set_tooltip_text(Some(tr!("Check mirror availability via HEAD request")));
    mirror_toolbar.append(&avail_btn);

    let iran_btn = gtk4::Button::from_icon_name("network-server-symbolic");
    iran_btn.set_tooltip_text(Some(tr!("Add Iranian mirrors")));
    mirror_toolbar.append(&iran_btn);

    let share_btn = gtk4::Button::from_icon_name("edit-copy-symbolic");
    share_btn.set_tooltip_text(Some(tr!("Copy mirror configuration to clipboard")));
    mirror_toolbar.append(&share_btn);

    let mirror_scroll = gtk4::ScrolledWindow::new();
    mirror_scroll.set_vexpand(true);
    mirror_scroll.set_policy(gtk4::PolicyType::Never, gtk4::PolicyType::Automatic);
    right_box.append(&mirror_scroll);

    let status_page = adw::StatusPage::new();
    status_page.set_title(tr!("No Mirrors Loaded"));
    status_page.set_description(Some(tr!("Configure your filters and click 'Fetch' to load available mirrors")));
    status_page.set_icon_name(Some("network-server-symbolic"));
    mirror_scroll.set_child(Some(&status_page));

    let mirror_list = gtk4::ListBox::new();
    mirror_list.set_selection_mode(gtk4::SelectionMode::Single);
    mirror_list.add_css_class("boxed-list");
    mirror_list.set_margin_top(6);
    mirror_list.set_margin_bottom(12);
    mirror_list.set_margin_start(12);
    mirror_list.set_margin_end(12);
    *mirror_list_holder.lock().unwrap() = Some(mirror_list.clone());

    // ── Populate repo lists ──
    {
        let config = rc.lock().unwrap();
        for name in &config.standard_repos {
            let row = adw::ActionRow::new();
            row.set_title(name);
            let sw = gtk4::Switch::new();
            sw.set_active(*config.repositories.get(name).unwrap_or(&false));
            sw.set_valign(gtk4::Align::Center);
            row.add_suffix(&sw);
            row.set_activatable_widget(Some(&sw));
            repo_list.append(&row);
        }
        for (i, name) in config.third_party_repos.iter().enumerate() {
            let row = adw::ActionRow::new();
            row.set_title([tr!("Chaotic-AUR"), tr!("BlackArch"), tr!("ArchLinuxCN")][i]);
            let sw = gtk4::Switch::new();
            sw.set_active(*config.repositories.get(name).unwrap_or(&false));
            sw.set_valign(gtk4::Align::Center);
            row.add_suffix(&sw);
            row.set_activatable_widget(Some(&sw));
            third_list.append(&row);
        }
    }

    // ── Helper: show dialog ──
    fn inform(win: &adw::ApplicationWindow, title: &str, body: &str) {
        let d = adw::AlertDialog::new(Some(title), Some(body));
        d.add_response("ok", tr!("OK"));
        d.set_response_appearance("ok", adw::ResponseAppearance::Suggested);
        d.present(Some(win));
    }

    fn alert(win: &adw::ApplicationWindow, title: &str, body: &str) {
        let d = adw::AlertDialog::new(Some(title), Some(body));
        d.add_response("ok", tr!("OK"));
        d.present(Some(win));
    }

    // ── Loading state helper ──
    fn set_loading(
        spinner: &gtk4::Spinner, label: &gtk4::Label,
        refresh: &gtk4::Button, hrefresh: &gtk4::Button,
        rank: &gtk4::Button, sync: &gtk4::Button, clean: &gtk4::Button,
        avail: &gtk4::Button,
        http: &gtk4::CheckButton, https: &gtk4::CheckButton,
        ipv4: &gtk4::CheckButton, ipv6: &gtk4::CheckButton,
        status: &gtk4::Switch, country: &adw::ComboRow,
        loading: bool, msg: &str,
    ) {
        spinner.set_spinning(loading);
        label.set_text(msg);
        refresh.set_sensitive(!loading);
        hrefresh.set_sensitive(!loading);
        rank.set_sensitive(!loading);
        sync.set_sensitive(!loading);
        clean.set_sensitive(!loading);
        avail.set_sensitive(!loading);
        http.set_sensitive(!loading);
        https.set_sensitive(!loading);
        ipv4.set_sensitive(!loading);
        ipv6.set_sensitive(!loading);
        status.set_sensitive(!loading);
        country.set_sensitive(!loading);
    }

    // ── Channel message handler (main thread) ──
    let win = window.clone();
    let list_holder = mirror_list_holder.clone();
    let mm_arc = mm.clone();
    let mirror_scroll_h = mirror_scroll.clone();
    let country_store_h = country_store.clone();
    let l_spinner = loading_spinner.clone();
    let l_label = loading_label.clone();
    let l_refresh = refresh_btn.clone();
    let l_hrefresh = header_refresh_btn.clone();
    let l_rank = rank_btn.clone();
    let l_sync = sync_btn.clone();
    let l_clean = clean_btn.clone();
    let l_avail = avail_btn.clone();
    let l_sort_dropdown = sort_dropdown.clone();
    let l_http = http_check.clone();
    let l_https = https_check.clone();
    let l_ipv4 = ipv4_check.clone();
    let l_ipv6 = ipv6_check.clone();
    let l_status = status_check.clone();
    let l_country = country_row.clone();
    let _msg_handler = glib::timeout_add_local(
        std::time::Duration::from_millis(100),
        move || {
            while let Ok(msg) = rx.try_recv() {
                let parts: Vec<&str> = msg.splitn(2, ':').collect();
                let cmd = parts[0];
                let rest = parts.get(1).copied().unwrap_or("");
                match cmd {
                    "fetch_ok" => {
                        let mgr = mm_arc.lock().unwrap();
                        if let Some(list) = list_holder.lock().unwrap().as_ref() {
                            if mgr.mirrors.len() > 0 {
                                mirror_scroll_h.set_child(Some(list));
                            }
                            refresh_list_ui(list, &mgr.mirrors);
                        }
                        let has_mirrors = mgr.mirrors.len() > 0;
                        l_sort_dropdown.set_sensitive(has_mirrors);
                        set_loading(&l_spinner, &l_label, &l_refresh, &l_hrefresh, &l_rank, &l_sync, &l_clean, &l_avail, &l_http, &l_https, &l_ipv4, &l_ipv6, &l_status, &l_country, false, "");
                    }
                    "fetch_err" => {
                        alert(&win, tr!("Fetch Failed"), rest);
                        set_loading(&l_spinner, &l_label, &l_refresh, &l_hrefresh, &l_rank, &l_sync, &l_clean, &l_avail, &l_http, &l_https, &l_ipv4, &l_ipv6, &l_status, &l_country, false, "");
                    }
                    "rank_ok" => {
                        let mgr = mm_arc.lock().unwrap();
                        if let Some(list) = list_holder.lock().unwrap().as_ref() {
                            refresh_list_ui(list, &mgr.mirrors);
                        }
                        set_loading(&l_spinner, &l_label, &l_refresh, &l_hrefresh, &l_rank, &l_sync, &l_clean, &l_avail, &l_http, &l_https, &l_ipv4, &l_ipv6, &l_status, &l_country, false, "");
                    }
                    "rank_err" => {
                        alert(&win, tr!("Ranking Error"), rest);
                        set_loading(&l_spinner, &l_label, &l_refresh, &l_hrefresh, &l_rank, &l_sync, &l_clean, &l_avail, &l_http, &l_https, &l_ipv4, &l_ipv6, &l_status, &l_country, false, "");
                    }
                    "cntry" => {
                        let list: Vec<&str> = rest.split(',').collect();
                        while country_store_h.n_items() > 1 {
                            country_store_h.remove(1);
                        }
                        for c in list {
                            if !c.is_empty() {
                                country_store_h.append(c);
                            }
                        }
                    }
                    "err" => alert(&win, tr!("Error"), rest),
                    _ => {}
                }
            }
            glib::ControlFlow::Continue
        },
    );

    // ── Fetch click ──
    {
        let mm = mm.clone();
        let tx = tx.clone();
        let win = window.clone();
        let loading_spinner = loading_spinner.clone();
        let loading_label = loading_label.clone();
        let refresh_btn = refresh_btn.clone();
        let header_refresh_btn = header_refresh_btn.clone();
        let rank_btn = rank_btn.clone();
        let sync_btn = sync_btn.clone();
        let clean_btn = clean_btn.clone();
        let avail_btn = avail_btn.clone();
        let http_check = http_check.clone();
        let https_check = https_check.clone();
        let ipv4_check = ipv4_check.clone();
        let ipv6_check = ipv6_check.clone();
        let status_check = status_check.clone();
        let country_row = country_row.clone();
        let country_store = country_store.clone();

        refresh_btn.clone().connect_clicked(move |_| {
            let protocols: Vec<String> = [
                http_check.is_active().then(|| "http"),
                https_check.is_active().then(|| "https"),
            ].into_iter().flatten().map(String::from).collect();
            if protocols.is_empty() {
                alert(&win, tr!("No Protocols"), tr!("Select at least one protocol"));
                return;
            }
            let ip_versions: Vec<String> = [
                ipv4_check.is_active().then(|| "4"),
                ipv6_check.is_active().then(|| "6"),
            ].into_iter().flatten().map(String::from).collect();
            if ip_versions.is_empty() {
                alert(&win, tr!("No IP Versions"), tr!("Select at least one IP version"));
                return;
            }
            let country = country_store.string(country_row.selected())
                .filter(|c| c.as_str() != "Worldwide")
                .map(|c| c.to_string());
            let use_status = status_check.is_active();

            set_loading(
                &loading_spinner, &loading_label,
                &refresh_btn, &header_refresh_btn,
                &rank_btn, &sync_btn, &clean_btn, &avail_btn,
                &http_check, &https_check,
                &ipv4_check, &ipv6_check,
                &status_check, &country_row,
                true, tr!("Fetching mirrors..."),
            );

            let tx = tx.clone();
            let mm = mm.clone();
            std::thread::spawn(move || {
                let mut mgr = mm.lock().unwrap();
                match mgr.fetch_mirrors(country.as_deref(), &protocols, &ip_versions, use_status) {
                    Ok(()) => {
                        let count = mgr.mirrors.len();
                        let _ = tx.send(format!("fetch_ok:{}", count));
                    }
                    Err(e) => {
                        let _ = tx.send(format!("fetch_err:{}", e));
                    }
                }
            });
        });
    }

    // ── Header refresh ──
    header_refresh_btn.connect_clicked({
        let r = refresh_btn.clone();
        move |_| { r.activate(); }
    });

    // ── Rank click ──
    {
        let mm = mm.clone();
        let tx = tx.clone();
        let loading_spinner = loading_spinner.clone();
        let loading_label = loading_label.clone();
        let refresh_btn = refresh_btn.clone();
        let header_refresh_btn = header_refresh_btn.clone();
        let rank_btn = rank_btn.clone();
        let sync_btn = sync_btn.clone();
        let clean_btn = clean_btn.clone();
        let avail_btn = avail_btn.clone();
        let http_check = http_check.clone();
        let https_check = https_check.clone();
        let ipv4_check = ipv4_check.clone();
        let ipv6_check = ipv6_check.clone();
        let status_check = status_check.clone();
        let country_row = country_row.clone();

        rank_btn.clone().connect_clicked(move |_| {
            let mgr = mm.lock().unwrap();
            let old_mirrors = mgr.mirrors.clone();
            drop(mgr);

            set_loading(
                &loading_spinner, &loading_label,
                &refresh_btn, &header_refresh_btn,
                &rank_btn, &sync_btn, &clean_btn, &avail_btn,
                &http_check, &https_check,
                &ipv4_check, &ipv6_check,
                &status_check, &country_row,
                true, tr!("Testing mirrors..."),
            );

            let tx = tx.clone();
            let mm = mm.clone();
            std::thread::spawn(move || {
                let mut mirrors = old_mirrors;
                MirrorManager::test_all_speeds_concurrent(&mut mirrors, 50);
                {
                    let mut mgr = mm.lock().unwrap();
                    mgr.mirrors = mirrors;
                    mgr.sort_by_speed();
                }
                let _ = tx.send("rank_ok:".to_string());
            });
        });
    }

    // ── Availability Click ──
    {
        let mm = mm.clone();
        let tx = tx.clone();
        let loading_spinner = loading_spinner.clone();
        let loading_label = loading_label.clone();
        let refresh_btn = refresh_btn.clone();
        let header_refresh_btn = header_refresh_btn.clone();
        let rank_btn = rank_btn.clone();
        let sync_btn = sync_btn.clone();
        let clean_btn = clean_btn.clone();
        let avail_btn = avail_btn.clone();
        let http_check = http_check.clone();
        let https_check = https_check.clone();
        let ipv4_check = ipv4_check.clone();
        let ipv6_check = ipv6_check.clone();
        let status_check = status_check.clone();
        let country_row = country_row.clone();

        avail_btn.clone().connect_clicked(move |_| {
            let mgr = mm.lock().unwrap();
            if mgr.mirrors.is_empty() { return; }
            let old_mirrors = mgr.mirrors.clone();
            drop(mgr);

            set_loading(
                &loading_spinner, &loading_label,
                &refresh_btn, &header_refresh_btn,
                &rank_btn, &sync_btn, &clean_btn, &avail_btn,
                &http_check, &https_check,
                &ipv4_check, &ipv6_check,
                &status_check, &country_row,
                true, tr!("Checking availability..."),
            );

            let tx = tx.clone();
            let mm = mm.clone();
            std::thread::spawn(move || {
                let mut mirrors = old_mirrors;
                MirrorManager::check_mirror_availability(&mut mirrors, 50);
                {
                    let mut mgr = mm.lock().unwrap();
                    mgr.mirrors = mirrors;
                }
                let _ = tx.send("rank_ok:".to_string());
            });
        });
    }

    // ── Iran Blackout ──
    {
        let mm = mm.clone();
        let mirror_list = mirror_list.clone();
        let mirror_scroll = mirror_scroll.clone();
        let win = window.clone();
        let l_sort_dropdown = sort_dropdown.clone();
        let l_rank = rank_btn.clone();
        iran_btn.connect_clicked(move |_| {
            let mut mgr = mm.lock().unwrap();
            mgr.add_iran_mirrors();
            let count = mgr.mirrors.len();
            mirror_scroll.set_child(Some(&mirror_list));
            refresh_list_ui(&mirror_list, &mgr.mirrors);
            l_sort_dropdown.set_sensitive(true);
            l_rank.set_sensitive(true);
            inform(&win, tr!("Iran Blackout Added"),
                &format!("{}\n\n{} {}", tr!("Added 3 Iranian mirrors."), tr!("Total mirrors:"), count));
        });
    }

    // ── Share ──
    {
        let mm = mm.clone();
        let win = window.clone();
        share_btn.connect_clicked(move |_| {
            let content = {
                let mgr = mm.lock().unwrap();
                sync_manager::SyncManager::generate_share_content(&mgr.mirrors)
            };
            if content.is_empty() {
                alert(&win, tr!("Nothing to Share"), tr!("No mirrors configured yet"));
                return;
            }
            if let Some(display) = gtk4::gdk::Display::default() {
                display.clipboard().set_text(&content);
            }
            inform(&win, tr!("Copied!"), tr!("Mirror configuration copied to clipboard"));
        });
    }

    // ── Enable / Disable / Selection ──
    let sel_idx = Arc::new(std::sync::Mutex::new(None::<usize>));
    {
        let mm = mm.clone();
        let list = mirror_list.clone();
        let sel = sel_idx.clone();
        enable_btn.connect_clicked(move |_| {
            let idx = *sel.lock().unwrap();
            if let Some(idx) = idx {
                if let Ok(mut mgr) = mm.try_lock() {
                    if idx < mgr.mirrors.len() {
                        mgr.mirrors[idx].enabled = true;
                        refresh_list_ui(&list, &mgr.mirrors);
                    }
                }
            }
        });
    }
    {
        let mm = mm.clone();
        let list = mirror_list.clone();
        let sel = sel_idx.clone();
        disable_btn.connect_clicked(move |_| {
            let idx = *sel.lock().unwrap();
            if let Some(idx) = idx {
                if let Ok(mut mgr) = mm.try_lock() {
                    if idx < mgr.mirrors.len() {
                        mgr.mirrors[idx].enabled = false;
                        refresh_list_ui(&list, &mgr.mirrors);
                    }
                }
            }
        });
    }

    {
        let sel = sel_idx.clone();
        mirror_list.connect_row_selected(move |_, row| {
            *sel.lock().unwrap() = row.map(|r| r.index() as usize);
            enable_btn.set_sensitive(row.is_some());
            disable_btn.set_sensitive(row.is_some());
        });
    }

    // ── Sort DropDown ──
    {
        let mm = mm.clone();
        let list = mirror_list.clone();
        sort_dropdown.connect_selected_notify(move |dd| {
            if let Ok(mut mgr) = mm.try_lock() {
                match dd.selected() {
                    0 => mgr.sort_by_speed(),
                    1 => mgr.sort_by_score(),
                    2 => mgr.sort_by_country(),
                    3 => mgr.sort_by_age(),
                    _ => {}
                }
                refresh_list_ui(&list, &mgr.mirrors);
            }
        });
    }

    // ── Best Setup ──
    {
        let mm = mm.clone();
        let list = mirror_list.clone();
        let win = window.clone();
        let mirror_scroll = mirror_scroll.clone();
        let l_sort_dropdown = sort_dropdown.clone();
        let l_rank = rank_btn.clone();
        best_setup_btn.connect_clicked(move |_| {
            let mut mgr = mm.lock().unwrap();
            if mgr.mirrors.is_empty() {
                alert(&win, tr!("No Mirrors"), tr!("Fetch mirrors first before running Best Setup."));
                return;
            }
            let selected = mgr.auto_optimize();
            mirror_scroll.set_child(Some(&list));
            refresh_list_ui(&list, &mgr.mirrors);
            l_sort_dropdown.set_sensitive(true);
            l_rank.set_sensitive(true);
            let mirror_urls: Vec<String> = selected.iter().map(|m| format!("• {} ({})", m.url, m.country)).collect();
            inform(
                &win,
                tr!("Best Setup Applied"),
                &format!("{}\n\n{}", tr!("Selected top optimal mirrors across countries:"), mirror_urls.join("\n"))
            );
        });
    }

    // ── Sync ──
    {
        let mm = mm.clone();
        let loading_spinner = loading_spinner.clone();
        let loading_label = loading_label.clone();
        let refresh_btn = refresh_btn.clone();
        let header_refresh_btn = header_refresh_btn.clone();
        let rank_btn = rank_btn.clone();
        let sync_btn = sync_btn.clone();
        let clean_btn = clean_btn.clone();
        let avail_btn = avail_btn.clone();
        let http_check = http_check.clone();
        let https_check = https_check.clone();
        let ipv4_check = ipv4_check.clone();
        let ipv6_check = ipv6_check.clone();
        let status_check = status_check.clone();
        let country_row = country_row.clone();
        let win = window.clone();

        sync_btn.clone().connect_clicked(move |_| {
            let current = MirrorManager::read_current_mirrorlist();
            let proposed = {
                let mgr = mm.lock().unwrap();
                mgr.generate_mirrorlist_content()
            };

            let win = win.clone();
            let mm = mm.clone();
            let loading_spinner = loading_spinner.clone();
            let loading_label = loading_label.clone();
            let refresh_btn = refresh_btn.clone();
            let header_refresh_btn = header_refresh_btn.clone();
            let rank_btn = rank_btn.clone();
            let sync_btn = sync_btn.clone();
            let clean_btn = clean_btn.clone();
            let avail_btn = avail_btn.clone();
            let http_check = http_check.clone();
            let https_check = https_check.clone();
            let ipv4_check = ipv4_check.clone();
            let ipv6_check = ipv6_check.clone();
            let status_check = status_check.clone();
            let country_row = country_row.clone();

            let win_for_diff = win.clone();
            show_diff_dialog(&win_for_diff, &current, &proposed, move || {
                let win_dialog = win.clone();
                set_loading(
                    &loading_spinner, &loading_label,
                    &refresh_btn, &header_refresh_btn,
                    &rank_btn, &sync_btn, &clean_btn, &avail_btn,
                    &http_check, &https_check,
                    &ipv4_check, &ipv6_check,
                    &status_check, &country_row,
                    true, "Saving mirrorlist...",
                );

                let dialog = adw::AlertDialog::new(
                    Some(tr!("Syncing Repositories")),
                    Some(tr!("Saving mirrorlist and refreshing package databases...")),
                );
                dialog.add_response("cancel", tr!("Cancel"));
                let progress = gtk4::ProgressBar::new();
                progress.set_pulse_step(0.1);
                dialog.set_extra_child(Some(&progress));
                dialog.present(Some(&win_dialog));

                let sync_result: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));

            let result_check = sync_result.clone();
            let p_loading_spinner = loading_spinner.clone();
            let p_loading_label = loading_label.clone();
            let p_refresh_btn = refresh_btn.clone();
            let p_header_refresh_btn = header_refresh_btn.clone();
            let p_rank_btn = rank_btn.clone();
            let p_sync_btn = sync_btn.clone();
            let p_clean_btn = clean_btn.clone();
            let p_avail_btn = avail_btn.clone();
            let p_http_check = http_check.clone();
            let p_https_check = https_check.clone();
            let p_ipv4_check = ipv4_check.clone();
            let p_ipv6_check = ipv6_check.clone();
            let p_status_check = status_check.clone();
            let p_country_row = country_row.clone();
            let p_win = win.clone();
            let _pulse = glib::timeout_add_local(
                std::time::Duration::from_millis(100),
                move || {
                    progress.pulse();
                    if let Some(ref msg) = *result_check.lock().unwrap() {
                        dialog.close();
                        set_loading(
                            &p_loading_spinner, &p_loading_label,
                            &p_refresh_btn, &p_header_refresh_btn,
                            &p_rank_btn, &p_sync_btn, &p_clean_btn, &p_avail_btn,
                            &p_http_check, &p_https_check,
                            &p_ipv4_check, &p_ipv6_check,
                            &p_status_check, &p_country_row,
                            false, "",
                        );
                        if msg == "ok" {
                            inform(&p_win, tr!("Success"), tr!("Mirrorlist saved and repositories synced successfully"));
                        } else {
                            alert(&p_win, tr!("Sync Failed"), msg);
                        }
                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                },
            );

            let sync_result = sync_result.clone();
            let mm = mm.clone();
            std::thread::spawn(move || {
                let result = {
                    let mgr = mm.lock().unwrap();
                    mgr.save_mirrorlist()
                };
                let msg = match result {
                    Ok(()) => {
                        match sync_manager::SyncManager::sync_repositories() {
                            Ok(_) => "ok".to_string(),
                            Err(e) => format!("sync_err:{e}"),
                        }
                    }
                    Err(e) => format!("err:{e}"),
                };
                *sync_result.lock().unwrap() = Some(msg);
            });
            });
        });
    }

    // ── Update ──
    {
        let win = window.clone();
        update_btn.connect_clicked(move |_| {
            let d = adw::AlertDialog::new(
                Some(tr!("Update System?")),
                Some(tr!("This will update all system packages. Continue?")),
            );
            d.add_response("cancel", tr!("Cancel"));
            d.add_response("update", tr!("Update"));
            d.set_response_appearance("update", ResponseAppearance::Destructive);
            d.connect_response(None, move |_, resp| {
                if resp == "update" { utils::open_terminal_with_command("pkexec pacman -Syu"); }
            });
            d.present(Some(&win));
        });
    }

    // ── Clean cache ──
    {
        let win = window.clone();
        clean_btn.connect_clicked(move |_| {
            let dialog = adw::AlertDialog::new(
                Some(tr!("Cleaning Cache")),
                Some(tr!("Removing old package files from cache...")),
            );
            let progress = gtk4::ProgressBar::new();
            progress.set_pulse_step(0.1);
            dialog.set_extra_child(Some(&progress));
            dialog.present(Some(&win));

            let result: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
            let result_check = result.clone();
            let win_c = win.clone();
            let _pulse = glib::timeout_add_local(
                std::time::Duration::from_millis(100),
                move || {
                    progress.pulse();
                    if let Some(ref msg) = *result_check.lock().unwrap() {
                        dialog.close();
                        if msg == "ok" {
                            inform(&win_c, tr!("Success"), tr!("Package cache cleaned successfully"));
                        } else {
                            alert(&win_c, tr!("Cache Clean Failed"), msg);
                        }
                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                },
            );

            std::thread::spawn(move || {
                let msg = match sync_manager::SyncManager::clean_cache() {
                    Ok(_) => "ok".to_string(),
                    Err(e) => e,
                };
                *result.lock().unwrap() = Some(msg);
            });
        });
    }

    // ── Backup ──
    {
        let win = window.clone();
        backup_btn.connect_clicked(move |_| {
            let dialog = adw::AlertDialog::new(
                Some(tr!("Backing Up")),
                Some(tr!("Creating timestamped backup of mirrorlist...")),
            );
            let progress = gtk4::ProgressBar::new();
            progress.set_pulse_step(0.1);
            dialog.set_extra_child(Some(&progress));
            dialog.present(Some(&win));

            let result: Arc<Mutex<Option<String>>> = Arc::new(Mutex::new(None));
            let result_check = result.clone();
            let win_c = win.clone();
            let _pulse = glib::timeout_add_local(
                std::time::Duration::from_millis(100),
                move || {
                    progress.pulse();
                    if let Some(ref msg) = *result_check.lock().unwrap() {
                        dialog.close();
                        if msg == "ok" {
                            inform(&win_c, tr!("Success"), tr!("Mirrorlist backup created successfully"));
                        } else {
                            alert(&win_c, tr!("Backup Failed"), msg);
                        }
                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                },
            );

            std::thread::spawn(move || {
                let msg = match sync_manager::SyncManager::backup_mirrorlist() {
                    Ok(_) => "ok".to_string(),
                    Err(e) => e,
                };
                *result.lock().unwrap() = Some(msg);
            });
        });
    }

    // ── Add Repository ──
    {
        let rc = rc.clone();
        let repo_list = repo_list.clone();
        let win = window.clone();
        add_repo_btn.connect_clicked(move |_| {
            let dialog = adw::AlertDialog::new(
                Some(tr!("Add Repository")),
                Some(tr!("Enter a repository name and server URL")),
            );

            let content = gtk4::Box::new(gtk4::Orientation::Vertical, 8);
            let name_entry = gtk4::Entry::new();
            name_entry.set_placeholder_text(Some(tr!("Repository name (e.g. myrepo)")));
            let url_entry = gtk4::Entry::new();
            url_entry.set_placeholder_text(Some(tr!("Server URL (e.g. https://mirror.example.com/archlinux/$repo/os/$arch)")));

            let siglevel_label = gtk4::Label::new(Some(tr!("SigLevel")));
            siglevel_label.set_halign(gtk4::Align::Start);
            let siglevel_model = gtk4::StringList::new(&[
                "",
                "Never",
                "Optional",
                "Required",
                "Optional TrustAll",
                "Required TrustAll",
                "Optional TrustedOnly",
                "Required TrustedOnly",
            ]);
            let siglevel_dropdown = gtk4::DropDown::new(Some(siglevel_model.clone()), None::<&gtk4::Expression>);
            siglevel_dropdown.set_selected(4); // "Optional TrustAll"
            content.append(&name_entry);
            content.append(&url_entry);
            content.append(&siglevel_label);
            content.append(&siglevel_dropdown);
            dialog.set_extra_child(Some(&content));

            dialog.add_response("cancel", tr!("Cancel"));
            dialog.add_response("add", tr!("Add"));
            dialog.set_response_appearance("add", ResponseAppearance::Suggested);

            let name_entry = name_entry.clone();
            let url_entry = url_entry.clone();
            let siglevel_dropdown = siglevel_dropdown.clone();
            let siglevel_model = siglevel_model.clone();
            let rc = rc.clone();
            let repo_list = repo_list.clone();
            let win_resp = win.clone();
            dialog.connect_response(None, move |_, resp| {
                if resp != "add" {
                    return;
                }
                let name = name_entry.text().to_string();
                let url = url_entry.text().to_string();
                let siglevel = siglevel_model
                    .string(siglevel_dropdown.selected())
                    .map(|s| s.to_string())
                    .unwrap_or_default();
                match rc.lock().unwrap().add_repository(&name, &url, &siglevel) {
                    Ok(()) => {
                        let row = adw::ActionRow::new();
                        row.set_title(&name);
                        let sw = gtk4::Switch::new();
                        sw.set_active(true);
                        sw.set_valign(gtk4::Align::Center);
                        row.add_suffix(&sw);
                        row.set_activatable_widget(Some(&sw));
                        repo_list.append(&row);

                        let rc2 = rc.clone();
                        let name_c = name.clone();
                        sw.connect_state_set(move |_, active| {
                            let mut cfg = rc2.lock().unwrap();
                            let _ = cfg.toggle_repo_in_config(&name_c, active, false);
                            glib::Propagation::Proceed
                        });

                        inform(&win_resp, tr!("Repository Added"), &format!("{} {}\n{}", tr!("Added repository:"), name, tr!("Sync repositories to use it")));
                    }
                    Err(e) => {
                        alert(&win_resp, tr!("Error"), &e);
                    }
                }
            });

            dialog.present(Some(&win));
        });
    }

    // ── Templates ──
    {
        let win = window.clone();
        let mm = mm.clone();
        let list_holder = mirror_list_holder.clone();
        templates_btn.connect_clicked(move |_| {
            show_templates_dialog(&win, mm.clone(), list_holder.clone());
        });
    }

    // ── History ──
    {
        let win = window.clone();
        history_btn.connect_clicked(move |_| {
            show_history_window(&win);
        });
    }

    // ── Settings ──
    {
        let bottom_sheet = bottom_sheet.clone();
        settings_btn.connect_clicked(move |_| {
            pacman_settings::show_settings_sheet(&bottom_sheet);
        });
    }

    // ── About ──
    {
        let win = window.clone();
        about_btn.connect_clicked(move |_| {
            let a = adw::AboutDialog::new();
            a.set_application_name(tr!("Parch Repository Manager"));
            a.set_application_icon("com.parchlinux.mirrorman");
            a.set_version("0.5.0-beta.1");
            a.set_developer_name(tr!("Parch GNU/Linux Team"));
            a.set_website("https://parchlinux.com");
            a.set_copyright(tr!("Copyright 2026 Parch GNU/Linux Team"));
            a.set_license_type(gtk4::License::Gpl30);
            a.set_release_notes(tr!(
"<p>Version 0.5.0 (2026)</p>
<ul>
<li>Mirror Health Dashboard with Score and Reliability metrics</li>
<li>One-Click Best Setup for automatic multi-country mirror optimization</li>
<li>Privilege Overhaul using mirrorman-helper D-Bus system service</li>
<li>Mirrorlist Diff Preview before saving</li>
<li>Template Profile storage and loading</li>
<li>Transaction History log viewer</li>
<li>Auto Refresh background timer integration</li>
</ul>"
            ));
            a.present(Some(&win));
        });
    }

    // ── Repo list toggles ──
    for is_third in [false, true] {
        let list: &gtk4::ListBox = if is_third { &third_list } else { &repo_list };
        let rc = rc.clone();
        let tx = tx.clone();

        for i in 0.. {
            let row = match list.row_at_index(i) {
                Some(r) => r,
                None => break,
            };
            let row = row.downcast::<adw::ActionRow>().expect("ActionRow");
            let row_title = row.title().to_string();
            let name = if is_third {
                let cfg = rc.lock().unwrap();
                cfg.third_party_repos.get(i as usize).cloned().unwrap_or(row_title)
            } else {
                row_title
            };
            if let Some(sw) = row.activatable_widget().and_downcast::<gtk4::Switch>() {
                let rc = rc.clone();
                let tx = tx.clone();
                let name_c = name.clone();
                sw.connect_state_set(move |_, active| {
                    if is_third && active {
                        let cfg = rc.lock().unwrap();
                        if let Err(e) = cfg.enable_third_party(&name_c) {
                            let _ = tx.send(format!("err:Enable failed: {e}"));
                            return glib::Propagation::Stop;
                        }
                    }
                    let mut cfg = rc.lock().unwrap();
                    if let Err(e) = cfg.toggle_repo_in_config(&name_c, active, is_third) {
                        let _ = tx.send(format!("err:{e}"));
                        return glib::Propagation::Stop;
                    }
                    glib::Propagation::Proceed
                });
            }
        }
    }

    // ── Load country list at startup ──
    {
        let tx = tx.clone();
        std::thread::spawn(move || {
            let mgr = MirrorManager::new();
            match mgr.fetch_countries_only() {
                Ok(countries) => {
                    let msg = format!("cntry:{}", countries.join(","));
                    let _ = tx.send(msg);
                }
                Err(_) => {}
            }
        });
    }

    window.present();
}

fn refresh_list_ui(list: &gtk4::ListBox, mirrors: &[Mirror]) {
    while let Some(c) = list.first_child() { list.remove(&c); }
    for m in mirrors {
        let row = adw::ActionRow::new();
        row.set_title(&m.url);
        let ip_display = {
            let mut ips = vec![];
            if m.ipv4 { ips.push(tr!("IPv4")); }
            if m.ipv6 { ips.push(tr!("IPv6")); }
            if ips.is_empty() { String::new() } else { format!("🌐 {}", ips.join("/")) }
        };
        let speed_str = match m.speed {
            Some(s) if s < 100.0 => format!("🟢 {:.0}ms", s),
            Some(s) if s < 300.0 => format!("🟡 {:.0}ms", s),
            Some(s) => format!("🔴 {:.0}ms", s),
            None => format!("{} {}", "⚪", tr!("Not tested")),
        };
        let health_str = match (m.score, m.completion_pct) {
            (Some(score), Some(cp)) => {
                let cp_pct = if cp <= 1.0 { cp * 100.0 } else { cp };
                let health_icon = if score < 1.5 && cp_pct >= 95.0 {
                    "🟢"
                } else if score < 3.0 || cp_pct >= 80.0 {
                    "🟡"
                } else {
                    "🔴"
                };
                format!("{health_icon} Score: {:.1} • {:.0}%", score, cp_pct)
            }
            (Some(score), None) => format!("Score: {:.1}", score),
            (None, Some(cp)) => {
                let cp_pct = if cp <= 1.0 { cp * 100.0 } else { cp };
                format!("{:.0}%", cp_pct)
            }
            (None, None) => String::new(),
        };
        let mut parts = vec![
            format!("{} {}", country_flag(&m.country_code), m.country),
            format!("🔗 {}", m.protocol.to_uppercase()),
            ip_display,
            speed_str,
        ];
        if !health_str.is_empty() {
            parts.push(health_str);
        }
        parts.push(format!("🕒 {}",
            m.last_sync.as_ref().and_then(|s| s.split('T').next()).unwrap_or(tr!("Unknown"))));

        row.set_subtitle(&parts.join(" • "));
        let box_ = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
        if m.enabled {
            let ic = gtk4::Image::from_icon_name("emblem-ok-symbolic");
            ic.add_css_class("success");
            box_.append(&ic);
            let lb = gtk4::Label::new(Some(tr!("Enabled")));
            lb.add_css_class("success");
            box_.append(&lb);
        } else {
            let ic = gtk4::Image::from_icon_name("window-close-symbolic");
            ic.add_css_class("error");
            box_.append(&ic);
            let lb = gtk4::Label::new(Some(tr!("Disabled")));
            lb.add_css_class("dim-label");
            box_.append(&lb);
        }
        row.add_suffix(&box_);
        list.append(&row);
    }
}

fn show_diff_dialog<F: Fn() + 'static>(parent: &adw::ApplicationWindow, current: &str, proposed: &str, on_confirm: F) {
    let win = adw::Window::new();
    win.set_transient_for(Some(parent));
    win.set_modal(true);
    win.set_title(Some(tr!("Mirrorlist Diff Preview")));
    win.set_default_size(900, 600);

    let toolbar_view = adw::ToolbarView::new();
    win.set_content(Some(&toolbar_view));

    let header = adw::HeaderBar::new();
    toolbar_view.add_top_bar(&header);

    let apply_btn = gtk4::Button::with_label(tr!("Apply & Save"));
    apply_btn.add_css_class("suggested-action");
    header.pack_end(&apply_btn);

    let cancel_btn = gtk4::Button::with_label(tr!("Cancel"));
    {
        let win = win.clone();
        cancel_btn.connect_clicked(move |_| win.destroy());
    }
    header.pack_start(&cancel_btn);

    let paned = gtk4::Paned::new(gtk4::Orientation::Horizontal);
    paned.set_position(450);
    paned.set_shrink_start_child(false);
    paned.set_shrink_end_child(false);
    toolbar_view.set_content(Some(&paned));

    let left_box = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    left_box.set_margin_top(8);
    left_box.set_margin_bottom(8);
    left_box.set_margin_start(8);
    left_box.set_margin_end(8);
    let left_label = gtk4::Label::new(Some(tr!("Current Mirrorlist (/etc/pacman.d/mirrorlist)")));
    left_label.add_css_class("heading");
    left_box.append(&left_label);

    let left_scroll = gtk4::ScrolledWindow::new();
    left_scroll.set_vexpand(true);
    let left_text = gtk4::TextView::new();
    left_text.set_editable(false);
    left_text.set_monospace(true);
    left_text.buffer().set_text(current);
    left_scroll.set_child(Some(&left_text));
    left_box.append(&left_scroll);
    paned.set_start_child(Some(&left_box));

    let right_box = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    right_box.set_margin_top(8);
    right_box.set_margin_bottom(8);
    right_box.set_margin_start(8);
    right_box.set_margin_end(8);
    let right_label = gtk4::Label::new(Some(tr!("Proposed Mirrorlist")));
    right_label.add_css_class("heading");
    right_box.append(&right_label);

    let right_scroll = gtk4::ScrolledWindow::new();
    right_scroll.set_vexpand(true);
    let right_text = gtk4::TextView::new();
    right_text.set_editable(false);
    right_text.set_monospace(true);
    right_text.buffer().set_text(proposed);
    right_scroll.set_child(Some(&right_text));
    right_box.append(&right_scroll);
    paned.set_end_child(Some(&right_box));

    let win_confirm = win.clone();
    apply_btn.connect_clicked(move |_| {
        win_confirm.destroy();
        on_confirm();
    });

    win.present();
}

fn show_templates_dialog(
    parent: &adw::ApplicationWindow,
    mm: Arc<Mutex<MirrorManager>>,
    mirror_list_holder: Arc<Mutex<Option<gtk4::ListBox>>>,
) {
    let win = adw::Window::new();
    win.set_transient_for(Some(parent));
    win.set_modal(true);
    win.set_title(Some(tr!("Mirrorlist Templates")));
    win.set_default_size(600, 500);

    let toolbar_view = adw::ToolbarView::new();
    win.set_content(Some(&toolbar_view));

    let header = adw::HeaderBar::new();
    toolbar_view.add_top_bar(&header);

    let save_tpl_btn = gtk4::Button::with_label(tr!("Save Profile"));
    save_tpl_btn.add_css_class("suggested-action");
    header.pack_start(&save_tpl_btn);

    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_vexpand(true);
    toolbar_view.set_content(Some(&scroll));

    let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 12);
    main_box.set_margin_top(12);
    main_box.set_margin_bottom(12);
    main_box.set_margin_start(12);
    main_box.set_margin_end(12);
    scroll.set_child(Some(&main_box));

    let group = adw::PreferencesGroup::new();
    group.set_title(tr!("Saved Profiles"));
    main_box.append(&group);

    let tpl_list = gtk4::ListBox::new();
    tpl_list.set_selection_mode(gtk4::SelectionMode::None);
    tpl_list.add_css_class("boxed-list");
    group.add(&tpl_list);

    let populate_list = {
        let tpl_list = tpl_list.clone();
        let mm = mm.clone();
        let list_holder = mirror_list_holder.clone();
        let win = win.clone();
        move || {
            while let Some(c) = tpl_list.first_child() {
                tpl_list.remove(&c);
            }
            let templates = templates::MirrorTemplate::list_all();
            if templates.is_empty() {
                let row = adw::ActionRow::new();
                row.set_title(tr!("No Saved Profiles"));
                row.set_subtitle(tr!("Click 'Save Profile' to save your active mirrorlist configuration."));
                tpl_list.append(&row);
                return;
            }
            for tpl in templates {
                let row = adw::ActionRow::new();
                row.set_title(&tpl.name);
                let enabled_count = tpl.mirrors.iter().filter(|m| m.enabled).count();
                row.set_subtitle(&format!("{} • {} {}", tpl.created_at, enabled_count, tr!("mirrors")));

                let load_btn = gtk4::Button::with_label(tr!("Load"));
                load_btn.add_css_class("flat");
                load_btn.set_valign(gtk4::Align::Center);
                {
                    let mm = mm.clone();
                    let tpl_mirrors = tpl.mirrors.clone();
                    let list_holder = list_holder.clone();
                    let win = win.clone();
                    load_btn.connect_clicked(move |_| {
                        let mut mgr = mm.lock().unwrap();
                        let mut new_mirrors = Vec::new();
                        for tm in &tpl_mirrors {
                            new_mirrors.push(crate::mirror_manager::Mirror {
                                url: tm.url.clone(),
                                country: tm.country.clone(),
                                country_code: String::new(),
                                protocol: tm.protocol.clone(),
                                speed: None,
                                last_sync: None,
                                enabled: tm.enabled,
                                ipv4: true,
                                ipv6: false,
                                completion_pct: None,
                                score: None,
                                duration_avg: None,
                                duration_stddev: None,
                            });
                        }
                        mgr.mirrors = new_mirrors;
                        if let Some(list) = list_holder.lock().unwrap().as_ref() {
                            refresh_list_ui(list, &mgr.mirrors);
                        }
                        win.destroy();
                    });
                }
                row.add_suffix(&load_btn);

                let del_btn = gtk4::Button::from_icon_name("user-trash-symbolic");
                del_btn.add_css_class("flat");
                del_btn.add_css_class("error");
                del_btn.set_valign(gtk4::Align::Center);
                {
                    let tpl_name = tpl.name.clone();
                    let win = win.clone();
                    del_btn.connect_clicked(move |_| {
                        let _ = templates::MirrorTemplate::delete(&tpl_name);
                        win.destroy();
                    });
                }
                row.add_suffix(&del_btn);
                tpl_list.append(&row);
            }
        }
    };

    populate_list();

    {
        let win_parent = win.clone();
        let mm = mm.clone();
        save_tpl_btn.connect_clicked(move |_| {
            let entry = gtk4::Entry::new();
            entry.set_placeholder_text(Some(tr!("Profile Name (e.g. Fast European Mirrors)")));
            let dialog = adw::AlertDialog::new(
                Some(tr!("Save Profile")),
                Some(tr!("Enter a name for this profile:")),
            );
            dialog.set_extra_child(Some(&entry));
            dialog.add_response("cancel", tr!("Cancel"));
            dialog.add_response("save", tr!("Save"));
            dialog.set_response_appearance("save", ResponseAppearance::Suggested);

            let mm = mm.clone();
            let win_to_destroy = win_parent.clone();
            dialog.connect_response(None, move |_, response| {
                if response == "save" {
                    let name = entry.text();
                    let mgr = mm.lock().unwrap();
                    let _ = templates::MirrorTemplate::save(name.as_str(), &mgr.mirrors);
                    win_to_destroy.destroy();
                }
            });
            dialog.present(Some(&win_parent));
        });
    }

    win.present();
}

fn show_history_window(parent: &adw::ApplicationWindow) {
    let win = adw::Window::new();
    win.set_transient_for(Some(parent));
    win.set_modal(true);
    win.set_title(Some(tr!("Transaction History")));
    win.set_default_size(800, 600);

    let toolbar_view = adw::ToolbarView::new();
    win.set_content(Some(&toolbar_view));

    let header = adw::HeaderBar::new();
    toolbar_view.add_top_bar(&header);

    let search_entry = gtk4::SearchEntry::new();
    search_entry.set_placeholder_text(Some(tr!("Search packages...")));
    header.set_title_widget(Some(&search_entry));

    let main_box = gtk4::Box::new(gtk4::Orientation::Vertical, 6);
    main_box.set_margin_top(8);
    main_box.set_margin_bottom(8);
    main_box.set_margin_start(8);
    main_box.set_margin_end(8);
    toolbar_view.set_content(Some(&main_box));

    let scroll = gtk4::ScrolledWindow::new();
    scroll.set_vexpand(true);
    main_box.append(&scroll);

    let list_box = gtk4::ListBox::new();
    list_box.set_selection_mode(gtk4::SelectionMode::None);
    list_box.add_css_class("boxed-list");
    scroll.set_child(Some(&list_box));

    let entries = log_viewer::parse_pacman_log();

    fn update_history_list(list_box: &gtk4::ListBox, entries: &[log_viewer::LogEntry], query: &str) {
        while let Some(c) = list_box.first_child() {
            list_box.remove(&c);
        }
        let query = query.to_lowercase();

        let mut count = 0;
        for entry in entries {
            if !query.is_empty()
                && !entry.package.to_lowercase().contains(&query)
                && !entry.action.to_lowercase().contains(&query)
            {
                continue;
            }
            if count >= 200 {
                break;
            }
            count += 1;

            let row = adw::ActionRow::new();
            row.set_title(&entry.package);
            row.set_subtitle(&format!("{} • {}", entry.timestamp, entry.version_info));

            let badge = gtk4::Label::new(Some(&entry.action));
            match entry.action.as_str() {
                "Installed" => badge.add_css_class("success"),
                "Upgraded" => badge.add_css_class("accent"),
                "Removed" => badge.add_css_class("error"),
                _ => badge.add_css_class("dim-label"),
            }
            badge.set_valign(gtk4::Align::Center);
            row.add_suffix(&badge);
            list_box.append(&row);
        }
    }

    update_history_list(&list_box, &entries, "");

    let list_box_c = list_box.clone();
    search_entry.connect_search_changed(move |e| {
        update_history_list(&list_box_c, &entries, e.text().as_str());
    });

    win.present();
}

fn main() {
    gtk4::init().expect("Failed to initialize GTK.");
    i18n::init();

    let app = adw::Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);

    app.run();
}

