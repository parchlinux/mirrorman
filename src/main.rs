#[macro_use]
mod i18n;
mod mirror_manager;
mod pacman_settings;
mod repo_config;
mod sync_manager;
mod utils;

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
    window.set_default_size(1200, 800);

    // ── Shared state ──
    let mm = Arc::new(Mutex::new(MirrorManager::new()));
    let rc = Arc::new(Mutex::new(repo_config::RepoConfig::new()));

    // ── Channel: background threads → main thread ──
    let (tx, rx) = std::sync::mpsc::channel::<String>();

    // ── Shared reference to mirror_list so channel handler can rebuild it ──
    let mirror_list_holder: Arc<Mutex<Option<gtk4::ListBox>>> = Arc::new(Mutex::new(None));

    // ── Build UI ──
    let toolbar_view = adw::ToolbarView::new();
    window.set_content(Some(&toolbar_view));

    let header = adw::HeaderBar::new();
    toolbar_view.add_top_bar(&header);

    let header_refresh_btn = gtk4::Button::new();
    header_refresh_btn.set_icon_name("view-refresh-symbolic");
    header_refresh_btn.set_tooltip_text(Some(tr!("Refresh Mirrors")));
    header.pack_start(&header_refresh_btn);

    let settings_btn = gtk4::Button::new();
    settings_btn.set_icon_name("preferences-system-symbolic");
    settings_btn.set_tooltip_text(Some(tr!("Pacman Settings")));
    header.pack_end(&settings_btn);

    let about_btn = gtk4::Button::new();
    about_btn.set_icon_name("help-about-symbolic");
    about_btn.set_tooltip_text(Some(tr!("About")));
    header.pack_end(&about_btn);

    let paned = gtk4::Paned::new(gtk4::Orientation::Horizontal);
    paned.set_position(320);
    paned.set_shrink_start_child(false);
    paned.set_shrink_end_child(false);
    toolbar_view.set_content(Some(&paned));

    let left_sidebar = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    left_sidebar.add_css_class("sidebar");
    paned.set_start_child(Some(&left_sidebar));

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

    let sys_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    sys_box.set_margin_top(12);
    sys_box.set_homogeneous(true);
    sidebar_box.append(&sys_box);

    let sync_btn = gtk4::Button::new();
    let sync_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    sync_box.set_halign(gtk4::Align::Center);
    sync_box.append(&gtk4::Image::from_icon_name("emblem-synchronizing-symbolic"));
    sync_box.append(&gtk4::Label::new(Some(tr!("Sync"))));
    sync_btn.set_child(Some(&sync_box));
    sync_btn.set_tooltip_text(Some(tr!("Save mirrorlist and sync repositories")));
    sys_box.append(&sync_btn);

    let clean_btn = gtk4::Button::new();
    let clean_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    clean_box.set_halign(gtk4::Align::Center);
    clean_box.append(&gtk4::Image::from_icon_name("user-trash-symbolic"));
    clean_box.append(&gtk4::Label::new(Some(tr!("Clean"))));
    clean_btn.set_child(Some(&clean_box));
    clean_btn.set_tooltip_text(Some(tr!("Clean package cache")));
    sys_box.append(&clean_btn);

    let update_btn = gtk4::Button::new();
    let update_box = gtk4::Box::new(gtk4::Orientation::Horizontal, 8);
    update_box.set_halign(gtk4::Align::Center);
    update_box.append(&gtk4::Image::from_icon_name("system-software-update-symbolic"));
    update_box.append(&gtk4::Label::new(Some(tr!("Update"))));
    update_btn.set_child(Some(&update_box));
    update_btn.add_css_class("destructive-action");
    update_btn.set_tooltip_text(Some(tr!("Update all system packages")));
    sys_box.append(&update_btn);

    let right_box = gtk4::Box::new(gtk4::Orientation::Vertical, 0);
    right_box.add_css_class("view");
    paned.set_end_child(Some(&right_box));

    let mirror_toolbar = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    mirror_toolbar.add_css_class("toolbar");
    mirror_toolbar.set_margin_top(12);
    mirror_toolbar.set_margin_bottom(12);
    mirror_toolbar.set_margin_start(12);
    mirror_toolbar.set_margin_end(12);
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

    let spacer = gtk4::Box::new(gtk4::Orientation::Horizontal, 0);
    spacer.set_hexpand(true);
    mirror_toolbar.append(&spacer);

    mirror_toolbar.append(&gtk4::Label::new(Some(tr!("Sort by:"))));

    let sort_speed_btn = gtk4::Button::new();
    let sbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    sbox.append(&gtk4::Image::from_icon_name("speedometer-symbolic"));
    sbox.append(&gtk4::Label::new(Some(tr!("Speed"))));
    sort_speed_btn.set_child(Some(&sbox));
    sort_speed_btn.set_sensitive(false);
    mirror_toolbar.append(&sort_speed_btn);

    let sort_country_btn = gtk4::Button::new();
    let cbox = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    cbox.append(&gtk4::Image::from_icon_name("mark-location-symbolic"));
    cbox.append(&gtk4::Label::new(Some(tr!("Country"))));
    sort_country_btn.set_child(Some(&cbox));
    sort_country_btn.set_sensitive(false);
    mirror_toolbar.append(&sort_country_btn);

    let sort_age_btn = gtk4::Button::new();
    let abox = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    abox.append(&gtk4::Image::from_icon_name("document-open-recent-symbolic"));
    abox.append(&gtk4::Label::new(Some(tr!("Age"))));
    sort_age_btn.set_child(Some(&abox));
    sort_age_btn.set_sensitive(false);
    mirror_toolbar.append(&sort_age_btn);

    let avail_btn = gtk4::Button::new();
    let abox = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    abox.append(&gtk4::Image::from_icon_name("emblem-ok-symbolic"));
    abox.append(&gtk4::Label::new(Some(tr!("Availability"))));
    avail_btn.set_child(Some(&abox));
    avail_btn.set_tooltip_text(Some(tr!("Check mirror availability via HEAD request")));
    mirror_toolbar.append(&avail_btn);

    let iran_btn = gtk4::Button::new();
    let ibox = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
    ibox.append(&gtk4::Image::from_icon_name("network-server-symbolic"));
    ibox.append(&gtk4::Label::new(Some(tr!("Iran Blackout"))));
    iran_btn.set_child(Some(&ibox));
    iran_btn.set_tooltip_text(Some(tr!("Add Iranian mirrors")));
    mirror_toolbar.append(&iran_btn);

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

    // ── Helper: update mirror list widget ──
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
            let parts = vec![
                format!("{} {}", country_flag(&m.country_code), m.country),
                format!("🔗 {}", m.protocol.to_uppercase()),
                ip_display,
                match m.speed {
                    Some(s) if s < 100.0 => format!("🟢 {:.0}ms", s),
                    Some(s) if s < 300.0 => format!("🟡 {:.0}ms", s),
                    Some(s) => format!("🔴 {:.0}ms", s),
                    None => format!("{} {}", "⚪", tr!("Not tested")),
                },
                format!("🕒 {}",
                    m.last_sync.as_ref().and_then(|s| s.split('T').next()).unwrap_or(tr!("Unknown"))),
            ];
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
    let l_sort_speed = sort_speed_btn.clone();
    let l_sort_country = sort_country_btn.clone();
    let l_sort_age = sort_age_btn.clone();
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
                        l_sort_speed.set_sensitive(has_mirrors);
                        l_sort_country.set_sensitive(has_mirrors);
                        l_sort_age.set_sensitive(has_mirrors);
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
        iran_btn.connect_clicked(move |_| {
            let mut mgr = mm.lock().unwrap();
            mgr.add_iran_mirrors();
            let count = mgr.mirrors.len();
            mirror_scroll.set_child(Some(&mirror_list));
            refresh_list_ui(&mirror_list, &mgr.mirrors);
            inform(&win, tr!("Iran Blackout Added"),
                &format!("{}\n\n{} {}", tr!("Added 5 Iranian mirrors."), tr!("Total mirrors:"), count));
        });
    }

    // ── Enable / Disable / Selection ──
    let sel_idx = Arc::new(std::sync::Mutex::new(None::<usize>));
    {
        let mm = mm.clone();
        let list = mirror_list.clone();
        let sel = sel_idx.clone();
        enable_btn.connect_clicked(move |_| {
            if let Some(idx) = *sel.lock().unwrap() {
                let mut mgr = mm.lock().unwrap();
                if idx < mgr.mirrors.len() {
                    mgr.mirrors[idx].enabled = true;
                    refresh_list_ui(&list, &mgr.mirrors);
                }
            }
        });
    }
    {
        let mm = mm.clone();
        let list = mirror_list.clone();
        let sel = sel_idx.clone();
        disable_btn.connect_clicked(move |_| {
            if let Some(idx) = *sel.lock().unwrap() {
                let mut mgr = mm.lock().unwrap();
                if idx < mgr.mirrors.len() {
                    mgr.mirrors[idx].enabled = false;
                    refresh_list_ui(&list, &mgr.mirrors);
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

    // ── Sort buttons ──
    {
        let mm = mm.clone();
        let list = mirror_list.clone();
        sort_speed_btn.connect_clicked(move |_| {
            let mut mgr = mm.lock().unwrap();
            mgr.sort_by_speed();
            refresh_list_ui(&list, &mgr.mirrors);
        });
    }
    {
        let mm = mm.clone();
        let list = mirror_list.clone();
        sort_country_btn.connect_clicked(move |_| {
            let mut mgr = mm.lock().unwrap();
            mgr.sort_by_country();
            refresh_list_ui(&list, &mgr.mirrors);
        });
    }
    {
        let mm = mm.clone();
        let list = mirror_list.clone();
        sort_age_btn.connect_clicked(move |_| {
            let mut mgr = mm.lock().unwrap();
            mgr.sort_by_age();
            refresh_list_ui(&list, &mgr.mirrors);
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
            dialog.present(Some(&win));

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
            content.append(&name_entry);
            content.append(&url_entry);
            dialog.set_extra_child(Some(&content));

            dialog.add_response("cancel", tr!("Cancel"));
            dialog.add_response("add", tr!("Add"));
            dialog.set_response_appearance("add", ResponseAppearance::Suggested);

            let name_entry = name_entry.clone();
            let url_entry = url_entry.clone();
            let rc = rc.clone();
            let repo_list = repo_list.clone();
            let win_resp = win.clone();
            dialog.connect_response(None, move |_, resp| {
                if resp != "add" {
                    return;
                }
                let name = name_entry.text().to_string();
                let url = url_entry.text().to_string();
                match rc.lock().unwrap().add_repository(&name, &url) {
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

    // ── Settings ──
    {
        let win = window.clone();
        settings_btn.connect_clicked(move |_| {
            pacman_settings::show_settings_window(&win);
        });
    }

    // ── About ──
    {
        let win = window.clone();
        about_btn.connect_clicked(move |_| {
            let a = adw::AboutWindow::new();
            a.set_transient_for(Some(&win));
            a.set_application_name(tr!("Parch Repository Manager"));
            a.set_application_icon("com.parchlinux.mirrorman");
            a.set_version("0.3");
            a.set_developer_name(tr!("Parch GNU/Linux Team"));
            a.set_website("https://parchlinux.com");
            a.set_copyright(tr!("Copyright 2026 Parch GNU/Linux Team"));
            a.set_license_type(gtk4::License::Gpl30);
            a.set_release_notes(tr!(
"<p>Version 0.3 (2026)</p>
<ul>
<li>Full Rust rewrite of Python mirrorman GUI</li>
<li>Country flag emoji display</li>
<li>Mirror availability check via HEAD requests</li>
<li>Concurrent speed testing (50 workers)</li>
<li>Pacman.conf settings editor</li>
<li>Iranian mirror support (Iran Blackout)</li>
<li>Polkit integration with policy file</li>
<li>Mirrorlist backup and safety checks</li>
<li>Third-party repository key imports</li>
<li>Desktop file, app icon, and PKGBUILD</li>
<li>i18n/gettext translation support</li>
<li>Custom repository addition dialog</li>
<li>Package cache cleaning</li>
</ul>"));
            a.present();
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
            let name = row.title().to_string();
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
                    if let Err(e) = cfg.toggle_repo_in_config(&name, active, is_third) {
                        let _ = tx.send(format!("err:{e}"));
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

fn main() {
    gtk4::init().expect("Failed to initialize GTK.");
    i18n::init();

    let app = adw::Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);

    app.run();
}
