use std::cell::RefCell;
use std::rc::Rc;

use adw::prelude::*;
use gtk4::glib;
use mirrorman_core::dev::models::{DevEcosystemKind, DevMirror, EcosystemStatus};
use mirrorman_core::dev::{
    reset_ecosystem_mirror, scan_ecosystems, set_ecosystem_mirror, test_ecosystem_mirrors,
};

static APP_ID: &str = "com.parchlinux.mirrorman.dev";

struct AppState {
    statuses: Vec<EcosystemStatus>,
    benchmarked_mirrors: std::collections::HashMap<DevEcosystemKind, Vec<DevMirror>>,
}

type SharedState = Rc<RefCell<AppState>>;

fn main() -> glib::ExitCode {
    let app = adw::Application::builder()
        .application_id(APP_ID)
        .build();

    app.connect_activate(build_ui);
    app.run()
}

fn build_ui(app: &adw::Application) {
    let window = adw::ApplicationWindow::new(app);
    window.set_title(Some("Developer Mirrors"));
    window.set_icon_name(Some("com.parchlinux.mirrorman.dev"));
    window.set_default_size(780, 580);
    window.set_size_request(420, 380);

    // Restore window dimensions if saved
    let config_dir = glib::user_config_dir().join("mirrorman");
    let state_path = config_dir.join("dev-window-state");
    if let Ok(content) = std::fs::read_to_string(&state_path) {
        let parts: Vec<&str> = content.trim().split(',').collect();
        if parts.len() == 2 {
            if let (Ok(w), Ok(h)) = (parts[0].parse::<i32>(), parts[1].parse::<i32>()) {
                window.set_default_size(w, h);
            }
        }
    }

    // Save window size on close
    let win_clone = window.clone();
    window.connect_close_request(move |_| {
        let (w, h) = win_clone.default_size();
        let _ = std::fs::create_dir_all(&config_dir);
        let _ = std::fs::write(&state_path, format!("{w},{h}"));
        glib::Propagation::Proceed
    });

    let state: SharedState = Rc::new(RefCell::new(AppState {
        statuses: scan_ecosystems(),
        benchmarked_mirrors: std::collections::HashMap::new(),
    }));

    let toast_overlay = adw::ToastOverlay::new();
    let nav_view = adw::NavigationView::new();
    toast_overlay.set_child(Some(&nav_view));
    window.set_content(Some(&toast_overlay));

    // Build the overview page
    build_overview_page(&nav_view, &state, &toast_overlay, &window);

    window.present();
}

fn build_overview_page(
    nav_view: &adw::NavigationView,
    state: &SharedState,
    toast_overlay: &adw::ToastOverlay,
    window: &adw::ApplicationWindow,
) {
    let toolbar_view = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    toolbar_view.add_top_bar(&header);

    let title = adw::WindowTitle::new("Developer Mirrors", "Language Package Manager Mirrors");
    header.set_title_widget(Some(&title));

    // Benchmark all button
    let benchmark_all_btn = gtk4::Button::builder()
        .icon_name("view-refresh-symbolic")
        .tooltip_text("Benchmark All Installed Tools")
        .build();
    header.pack_start(&benchmark_all_btn);

    // Primary menu
    let menu_btn = gtk4::MenuButton::builder()
        .icon_name("open-menu-symbolic")
        .tooltip_text("Main Menu")
        .build();
    let menu = gtk4::gio::Menu::new();
    menu.append(Some("About Developer Mirrors"), Some("app.about"));
    menu_btn.set_menu_model(Some(&menu));
    header.pack_end(&menu_btn);

    // Set up About action
    let win_weak = window.downgrade();
    if let Some(app) = window.application() {
        let about_action = gtk4::gio::SimpleAction::new("about", None);
        about_action.connect_activate(move |_, _| {
            if let Some(win) = win_weak.upgrade() {
                show_about_dialog(&win);
            }
        });
        app.add_action(&about_action);
    }

    // Populate content
    refresh_overview_list(&toolbar_view, nav_view, state, toast_overlay, window);

    // Connect benchmark all
    let toolbar_view_clone = toolbar_view.clone();
    let nav_clone = nav_view.clone();
    let state_clone = state.clone();
    let toast_clone = toast_overlay.clone();
    let win_clone = window.clone();
    let btn_clone = benchmark_all_btn.clone();

    benchmark_all_btn.connect_clicked(move |_| {
        btn_clone.set_sensitive(false);
        let toast = adw::Toast::new("Testing candidate mirrors for all installed tools...");
        toast_clone.add_toast(toast);

        let installed_kinds: Vec<DevEcosystemKind> = state_clone
            .borrow()
            .statuses
            .iter()
            .filter(|s| s.is_installed)
            .map(|s| s.kind)
            .collect();

        let state_thread = state_clone.clone();
        let toolbar_thread = toolbar_view_clone.clone();
        let nav_thread = nav_clone.clone();
        let toast_thread = toast_clone.clone();
        let win_thread = win_clone.clone();
        let btn_thread = btn_clone.clone();

        glib::spawn_future_local(async move {
            let results = gio_async_benchmark_all(installed_kinds).await;
            {
                let mut b = state_thread.borrow_mut();
                for (kind, mirrors) in results {
                    b.benchmarked_mirrors.insert(kind, mirrors);
                }
            }
            refresh_overview_list(
                &toolbar_thread,
                &nav_thread,
                &state_thread,
                &toast_thread,
                &win_thread,
            );
            btn_thread.set_sensitive(true);
            let done_toast = adw::Toast::new("Benchmarking completed!");
            toast_thread.add_toast(done_toast);
        });
    });

    let overview_nav_page = adw::NavigationPage::new(&toolbar_view, "Overview");
    nav_view.push(&overview_nav_page);
}

async fn gio_async_benchmark_all(
    kinds: Vec<DevEcosystemKind>,
) -> Vec<(DevEcosystemKind, Vec<DevMirror>)> {
    let (tx, rx) = async_channel::bounded(1);
    std::thread::spawn(move || {
        let mut res = Vec::new();
        for kind in kinds {
            let tested = test_ecosystem_mirrors(kind, 10);
            res.push((kind, tested));
        }
        let _ = tx.send_blocking(res);
    });
    rx.recv().await.unwrap_or_default()
}

fn refresh_overview_list(
    toolbar_view: &adw::ToolbarView,
    nav_view: &adw::NavigationView,
    state: &SharedState,
    toast_overlay: &adw::ToastOverlay,
    window: &adw::ApplicationWindow,
) {
    let page = adw::PreferencesPage::new();

    let statuses = state.borrow().statuses.clone();
    let installed: Vec<&EcosystemStatus> = statuses.iter().filter(|s| s.is_installed).collect();
    let uninstalled: Vec<&EcosystemStatus> = statuses.iter().filter(|s| !s.is_installed).collect();

    if installed.is_empty() {
        let status_page = adw::StatusPage::builder()
            .icon_name("application-x-executable-symbolic")
            .title("No Developer Tools Detected")
            .description("Install pip, npm, cargo, go, gem, or composer to manage their package mirrors.")
            .build();
        let group = adw::PreferencesGroup::new();
        group.add(&status_page);
        page.add(&group);
        toolbar_view.set_content(Some(&page));
        return;
    }

    let installed_group = adw::PreferencesGroup::builder()
        .title("Installed Package Managers")
        .description("Select an ecosystem to test and switch mirrors")
        .build();

    for status in installed {
        let row = adw::ActionRow::builder()
            .title(&status.name)
            .subtitle(status.current_mirror.as_deref().unwrap_or("Default"))
            .activatable(true)
            .build();

        row.add_prefix(&gtk4::Image::from_icon_name("application-x-executable-symbolic"));

        // If benchmarked, display latency pill
        if let Some(tested) = state.borrow().benchmarked_mirrors.get(&status.kind) {
            if let Some(curr_mirror) = &status.current_mirror {
                if let Some(m) = tested.iter().find(|m| m.url.trim_end_matches('/') == curr_mirror.trim_end_matches('/')) {
                    if let Some(ms) = m.speed {
                        let badge = make_latency_badge(ms);
                        row.add_suffix(&badge);
                    }
                }
            }
        }

        row.add_suffix(&gtk4::Image::from_icon_name("go-next-symbolic"));

        let kind = status.kind;
        let nav_clone = nav_view.clone();
        let state_clone = state.clone();
        let toast_clone = toast_overlay.clone();
        let win_clone = window.clone();
        let toolbar_clone = toolbar_view.clone();

        row.connect_activated(move |_| {
            open_detail_page(
                kind,
                &nav_clone,
                &state_clone,
                &toast_clone,
                &win_clone,
                &toolbar_clone,
            );
        });

        installed_group.add(&row);
    }
    page.add(&installed_group);

    if !uninstalled.is_empty() {
        let uninstalled_group = adw::PreferencesGroup::builder()
            .title("Other Supported Tools")
            .description("Not currently installed on this system")
            .build();

        for status in uninstalled {
            let row = adw::ActionRow::builder()
                .title(&status.name)
                .subtitle(format!("Command '{}' not found in PATH", status.command))
                .sensitive(false)
                .build();
            row.add_prefix(&gtk4::Image::from_icon_name("dialog-information-symbolic"));
            uninstalled_group.add(&row);
        }
        page.add(&uninstalled_group);
    }

    toolbar_view.set_content(Some(&page));
}

fn open_detail_page(
    kind: DevEcosystemKind,
    nav_view: &adw::NavigationView,
    state: &SharedState,
    toast_overlay: &adw::ToastOverlay,
    window: &adw::ApplicationWindow,
    overview_toolbar: &adw::ToolbarView,
) {
    let toolbar_view = adw::ToolbarView::new();
    let header = adw::HeaderBar::new();
    toolbar_view.add_top_bar(&header);

    let title = adw::WindowTitle::new(kind.display_name(), "Manage Package Mirror");
    header.set_title_widget(Some(&title));

    let icon_name = test_mirror_icon_name();
    let test_btn = gtk4::Button::builder()
        .icon_name(icon_name)
        .tooltip_text("Test Candidate Mirrors")
        .build();
    header.pack_end(&test_btn);

    // Build the detail content
    rebuild_detail_content(
        kind,
        &toolbar_view,
        nav_view,
        state,
        toast_overlay,
        window,
        overview_toolbar,
    );

    // Test button handler
    let toolbar_clone = toolbar_view.clone();
    let nav_clone = nav_view.clone();
    let state_clone = state.clone();
    let toast_clone = toast_overlay.clone();
    let win_clone = window.clone();
    let overview_clone = overview_toolbar.clone();
    let btn_clone = test_btn.clone();

    test_btn.connect_clicked(move |_| {
        btn_clone.set_sensitive(false);
        let toast = adw::Toast::new(&format!("Testing mirrors for {}...", kind.display_name()));
        toast_clone.add_toast(toast);

        let state_thread = state_clone.clone();
        let toolbar_thread = toolbar_clone.clone();
        let nav_thread = nav_clone.clone();
        let toast_thread = toast_clone.clone();
        let win_thread = win_clone.clone();
        let overview_thread = overview_clone.clone();
        let btn_thread = btn_clone.clone();

        glib::spawn_future_local(async move {
            let (tx, rx) = async_channel::bounded(1);
            std::thread::spawn(move || {
                let tested = test_ecosystem_mirrors(kind, 10);
                let _ = tx.send_blocking(tested);
            });

            let tested = rx.recv().await.unwrap_or_default();
            state_thread
                .borrow_mut()
                .benchmarked_mirrors
                .insert(kind, tested);

            rebuild_detail_content(
                kind,
                &toolbar_thread,
                &nav_thread,
                &state_thread,
                &toast_thread,
                &win_thread,
                &overview_thread,
            );
            btn_thread.set_sensitive(true);
            let done_toast = adw::Toast::new("Mirror speed test complete!");
            toast_thread.add_toast(done_toast);
        });
    });

    let nav_page = adw::NavigationPage::new(&toolbar_view, kind.display_name());
    nav_view.push(&nav_page);
}

fn rebuild_detail_content(
    kind: DevEcosystemKind,
    toolbar_view: &adw::ToolbarView,
    nav_view: &adw::NavigationView,
    state: &SharedState,
    toast_overlay: &adw::ToastOverlay,
    window: &adw::ApplicationWindow,
    overview_toolbar: &adw::ToolbarView,
) {
    let page = adw::PreferencesPage::new();

    // Refresh status from filesystem
    let current_status = {
        let detector = mirrorman_core::dev::get_detector(kind);
        let mirrors = mirrorman_core::dev::mirrors::get_mirrors_for_ecosystem(kind);
        detector.get_status(mirrors)
    };

    // Update state
    {
        let mut b = state.borrow_mut();
        if let Some(s) = b.statuses.iter_mut().find(|s| s.kind == kind) {
            *s = current_status.clone();
        }
    }

    // Group 1: Current Configuration
    let config_group = adw::PreferencesGroup::builder()
        .title("Configuration")
        .build();

    let mirror_row = adw::ActionRow::builder()
        .title("Current Mirror")
        .subtitle(current_status.current_mirror.as_deref().unwrap_or("Default"))
        .build();
    mirror_row.add_prefix(&gtk4::Image::from_icon_name("network-server-symbolic"));
    config_group.add(&mirror_row);

    if let Some(ref path) = current_status.config_path {
        let path_row = adw::ActionRow::builder()
            .title("Configuration File")
            .subtitle(path)
            .build();
        path_row.add_prefix(&gtk4::Image::from_icon_name("text-x-generic-symbolic"));
        config_group.add(&path_row);
    }

    // Reset button
    let reset_row = adw::ActionRow::builder()
        .title("Restore Official Upstream")
        .subtitle("Revert mirror settings to default repository")
        .activatable(true)
        .build();
    reset_row.add_prefix(&gtk4::Image::from_icon_name("edit-undo-symbolic"));

    let reset_btn = gtk4::Button::builder()
        .label("Restore")
        .valign(gtk4::Align::Center)
        .build();
    reset_row.add_suffix(&reset_btn);

    let toolbar_clone = toolbar_view.clone();
    let nav_clone = nav_view.clone();
    let state_clone = state.clone();
    let toast_clone = toast_overlay.clone();
    let win_clone = window.clone();
    let overview_clone = overview_toolbar.clone();

    let do_reset = move || {
        if let Err(e) = reset_ecosystem_mirror(kind) {
            let toast = adw::Toast::new(&format!("Failed to reset: {e}"));
            toast_clone.add_toast(toast);
        } else {
            let toast = adw::Toast::new(&format!("Reset {} to default mirror", kind.display_name()));
            toast_clone.add_toast(toast);
            rebuild_detail_content(
                kind,
                &toolbar_clone,
                &nav_clone,
                &state_clone,
                &toast_clone,
                &win_clone,
                &overview_clone,
            );
            refresh_overview_list(
                &overview_clone,
                &nav_clone,
                &state_clone,
                &toast_clone,
                &win_clone,
            );
        }
    };

    let do_reset_rc = Rc::new(do_reset);
    let do_reset_1 = do_reset_rc.clone();
    reset_row.connect_activated(move |_| do_reset_1());
    let do_reset_2 = do_reset_rc;
    reset_btn.connect_clicked(move |_| do_reset_2());

    config_group.add(&reset_row);
    page.add(&config_group);

    // Group 2: Available Mirrors
    let mirrors_group = adw::PreferencesGroup::builder()
        .title("Candidate Mirrors")
        .description("Choose a mirror from Mirava or global registries")
        .build();

    let candidate_mirrors = state
        .borrow()
        .benchmarked_mirrors
        .get(&kind)
        .cloned()
        .unwrap_or_else(|| current_status.mirrors.clone());

    for mirror in candidate_mirrors {
        let is_active = current_status
            .current_mirror
            .as_deref()
            .map(|curr| curr.trim_end_matches('/') == mirror.url.trim_end_matches('/'))
            .unwrap_or(false);

        let flag = mirrorman_core::mirror_manager::country_flag(&mirror.country_code);
        let flag_prefix = if !flag.is_empty() {
            format!("{flag} ")
        } else {
            String::new()
        };

        let row = adw::ActionRow::builder()
            .title(format!("{flag_prefix}{}", mirror.name))
            .subtitle(&mirror.url)
            .activatable(!is_active)
            .build();

        // Speed badge if tested
        if let Some(ms) = mirror.speed {
            let badge = make_latency_badge(ms);
            row.add_suffix(&badge);
        }

        if is_active {
            let active_badge = gtk4::Label::builder()
                .label("Active")
                .css_classes(["success", "heading"])
                .valign(gtk4::Align::Center)
                .build();
            row.add_suffix(&active_badge);
            row.add_suffix(&gtk4::Image::from_icon_name("emblem-ok-symbolic"));
        } else {
            let apply_btn = gtk4::Button::builder()
                .label("Use")
                .css_classes(["suggested-action"])
                .valign(gtk4::Align::Center)
                .build();
            row.add_suffix(&apply_btn);

            let mirror_clone = mirror.clone();
            let toolbar_clone = toolbar_view.clone();
            let nav_clone = nav_view.clone();
            let state_clone = state.clone();
            let toast_clone = toast_overlay.clone();
            let win_clone = window.clone();
            let overview_clone = overview_toolbar.clone();

            let do_apply = move || {
                match set_ecosystem_mirror(kind, &mirror_clone.name) {
                    Ok(_) => {
                        let toast = adw::Toast::new(&format!(
                            "Switched {} to {}",
                            kind.display_name(),
                            mirror_clone.name
                        ));
                        toast_clone.add_toast(toast);
                        rebuild_detail_content(
                            kind,
                            &toolbar_clone,
                            &nav_clone,
                            &state_clone,
                            &toast_clone,
                            &win_clone,
                            &overview_clone,
                        );
                        refresh_overview_list(
                            &overview_clone,
                            &nav_clone,
                            &state_clone,
                            &toast_clone,
                            &win_clone,
                        );
                    }
                    Err(e) => {
                        let toast = adw::Toast::new(&format!("Error: {e}"));
                        toast_clone.add_toast(toast);
                    }
                }
            };

            let do_apply_rc = Rc::new(do_apply);
            let do_apply_1 = do_apply_rc.clone();
            row.connect_activated(move |_| do_apply_1());
            let do_apply_2 = do_apply_rc;
            apply_btn.connect_clicked(move |_| do_apply_2());
        }

        mirrors_group.add(&row);
    }

    page.add(&mirrors_group);
    toolbar_view.set_content(Some(&page));
}

fn make_latency_badge(ms: f64) -> gtk4::Label {
    let text = format!("{:.0} ms", ms);
    let class = if ms < 120.0 {
        "success"
    } else if ms < 300.0 {
        "warning"
    } else {
        "error"
    };

    gtk4::Label::builder()
        .label(&text)
        .css_classes([class, "caption"])
        .valign(gtk4::Align::Center)
        .build()
}

fn show_about_dialog(parent: &adw::ApplicationWindow) {
    let dialog = adw::AboutDialog::builder()
        .application_name("Developer Mirrors")
        .application_icon("com.parchlinux.mirrorman.dev")
        .developer_name("Parch GNU/Linux Team")
        .version(env!("CARGO_PKG_VERSION"))
        .comments("Developer package manager mirror manager for Parch Linux")
        .website("https://parchlinux.com")
        .license_type(gtk4::License::Gpl30)
        .build();

    dialog.present(Some(parent));
}

fn test_mirror_icon_name() -> &'static str {
    if let Some(disp) = gtk4::gdk::Display::default() {
        let theme = gtk4::IconTheme::for_display(&disp);
        if theme.has_icon("speedometer-symbolic") {
            return "speedometer-symbolic";
        }
    }
    "view-refresh-symbolic"
}
