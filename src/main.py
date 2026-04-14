import gi
import gettext

gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Gtk, Adw, Gio, GLib
import sys
import os
import threading
from mirror_manager import MirrorManager
from repo_config import RepoConfig
from sync_manager import SyncManager
import subprocess
from pacman_util import PacmanOptionsWindow
import tempfile
import urllib.request

script_dir = os.path.dirname(os.path.abspath(__file__))
localedir = os.path.join(os.path.dirname(script_dir), "locale")
if os.path.exists(localedir):
    try:
        t = gettext.translation("mirrorman", localedir=localedir, fallback=True)
        _ = t.gettext
    except Exception:

        def _(s):
            return s
else:
    try:
        t = gettext.translation(
            "mirrorman", localedir="/usr/share/locale", fallback=True
        )
        _ = t.gettext
    except Exception:

        def _(s):
            return s


def get_suitable_terminal():
    desktop = os.environ.get("XDG_CURRENT_DESKTOP", "").upper()
    if "GNOME" in desktop:
        if subprocess.run(["which", "ptyxis"], capture_output=True).returncode == 0:
            return "ptyxis -x "
        elif (
            subprocess.run(["which", "gnome-terminal"], capture_output=True).returncode
            == 0
        ):
            return "gnome-terminal -- "
    elif "KDE" in desktop or "PLASMA" in desktop:
        if subprocess.run(["which", "konsole"], capture_output=True).returncode == 0:
            return "konsole -e "
    elif "XFCE" in desktop:
        if (
            subprocess.run(["which", "xfce4-terminal"], capture_output=True).returncode
            == 0
        ):
            return "xfce4-terminal -e "
    for term in ["alacritty", "kitty", "xterm"]:
        if subprocess.run(["which", term], capture_output=True).returncode == 0:
            return f"{term} -e "
    return None


def open_terminal_with_repo_command(command, parent_window=None):
    escaped_cmd = command.replace("'", "'\"'\"'")
    terminal_cmd = get_suitable_terminal()
    if terminal_cmd is None:
        if parent_window:
            parent_window.show_error_dialog(
                "Terminal Error", "No suitable terminal emulator found."
            )
        else:
            print("Error: No suitable terminal emulator found.")
        return
    full_cmd = f"{terminal_cmd} bash -c '{escaped_cmd}; echo; echo Press ENTER to close...; read'"
    try:
        subprocess.Popen(full_cmd, shell=True)
    except Exception as e:
        if parent_window:
            parent_window.show_error_dialog("Terminal Error", str(e))
        else:
            print(f"Error opening terminal: {e}")


class MainWindow(Adw.ApplicationWindow):
    def __init__(self, app):
        super().__init__(application=app)
        self.set_title(_("Parch Repository Manager"))
        self.set_default_size(1200, 800)
        self.mirror_manager = MirrorManager()
        self.repo_config = RepoConfig()
        self.sync_manager = SyncManager()
        self.selected_mirror = None
        self.is_loading = False
        self.is_syncing = False
        toolbar_view = Adw.ToolbarView()
        self.set_content(toolbar_view)
        header = Adw.HeaderBar()
        toolbar_view.add_top_bar(header)
        self.header_refresh_btn = Gtk.Button()
        self.header_refresh_btn.set_icon_name("view-refresh-symbolic")
        self.header_refresh_btn.set_tooltip_text(_("Refresh Mirrors"))
        self.header_refresh_btn.connect("clicked", self.on_refresh_mirrors)
        header.pack_start(self.header_refresh_btn)
        self.settings_btn = Gtk.Button()
        self.settings_btn.set_icon_name("preferences-system-symbolic")
        self.settings_btn.set_tooltip_text(_("Pacman Settings"))
        self.settings_btn.connect("clicked", self.on_settings_clicked)
        header.pack_end(self.settings_btn)
        self.about_btn = Gtk.Button()
        self.about_btn.set_icon_name("help-about-symbolic")
        self.about_btn.set_tooltip_text(_("About"))
        self.about_btn.connect("clicked", self.on_about_clicked)
        header.pack_end(self.about_btn)
        paned = Gtk.Paned(orientation=Gtk.Orientation.HORIZONTAL)
        paned.set_position(320)
        paned.set_shrink_start_child(False)
        paned.set_shrink_end_child(False)
        toolbar_view.set_content(paned)
        left_sidebar = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        left_sidebar.add_css_class("sidebar")
        paned.set_start_child(left_sidebar)
        sidebar_scroll = Gtk.ScrolledWindow()
        sidebar_scroll.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        sidebar_scroll.set_vexpand(True)
        left_sidebar.append(sidebar_scroll)
        sidebar_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=18)
        sidebar_box.set_margin_top(18)
        sidebar_box.set_margin_bottom(18)
        sidebar_box.set_margin_start(12)
        sidebar_box.set_margin_end(12)
        sidebar_scroll.set_child(sidebar_box)
        filter_clamp = Adw.Clamp()
        filter_clamp.set_maximum_size(400)
        sidebar_box.append(filter_clamp)
        filter_group = Adw.PreferencesGroup()
        filter_group.set_title(_("Mirror Filters"))
        filter_group.set_description(_("Configure mirror selection criteria"))
        filter_clamp.set_child(filter_group)
        self.country_row = Adw.ComboRow()
        self.country_row.set_title(_("Country"))
        self.country_store = Gtk.StringList()
        self.country_store.append(_("Worldwide"))
        self.country_row.set_model(self.country_store)
        self.country_row.set_selected(0)
        filter_group.add(self.country_row)
        protocol_row = Adw.ActionRow()
        protocol_row.set_title(_("Protocol"))
        protocol_row.set_icon_name("network-wired-symbolic")
        protocol_box = Gtk.Box(spacing=12)
        protocol_box.set_margin_top(6)
        protocol_box.set_margin_bottom(6)
        self.http_check = Gtk.CheckButton(label="HTTP")
        self.http_check.set_active(True)
        self.https_check = Gtk.CheckButton(label="HTTPS")
        self.https_check.set_active(True)
        protocol_box.append(self.http_check)
        protocol_box.append(self.https_check)
        protocol_row.add_suffix(protocol_box)
        filter_group.add(protocol_row)
        ip_row = Adw.ActionRow()
        ip_row.set_title(_("IP Version"))
        ip_row.set_icon_name("network-transmit-receive-symbolic")
        ip_box = Gtk.Box(spacing=12)
        ip_box.set_margin_top(6)
        ip_box.set_margin_bottom(6)
        self.ipv4_check = Gtk.CheckButton(label="IPv4")
        self.ipv4_check.set_active(True)
        self.ipv6_check = Gtk.CheckButton(label="IPv6")
        ip_box.append(self.ipv4_check)
        ip_box.append(self.ipv6_check)
        ip_row.add_suffix(ip_box)
        filter_group.add(ip_row)
        status_row = Adw.ActionRow()
        status_row.set_title(_("Up-to-date only"))
        status_row.set_subtitle(_("Show only synchronized mirrors"))
        status_row.set_icon_name("emblem-synchronizing-symbolic")
        self.status_check = Gtk.Switch()
        self.status_check.set_valign(Gtk.Align.CENTER)
        self.status_check.set_active(True)
        status_row.add_suffix(self.status_check)
        status_row.set_activatable_widget(self.status_check)
        filter_group.add(status_row)
        btn_box = Gtk.Box(spacing=8)
        btn_box.set_margin_top(12)
        btn_box.set_homogeneous(True)
        sidebar_box.append(btn_box)
        self.refresh_btn = Gtk.Button()
        refresh_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        refresh_box.set_halign(Gtk.Align.CENTER)
        refresh_icon = Gtk.Image.new_from_icon_name("view-refresh-symbolic")
        refresh_label = Gtk.Label(label=_("Fetch"))
        refresh_box.append(refresh_icon)
        refresh_box.append(refresh_label)
        self.refresh_btn.set_child(refresh_box)
        self.refresh_btn.add_css_class("suggested-action")
        self.refresh_btn.connect("clicked", self.on_refresh_mirrors)
        btn_box.append(self.refresh_btn)
        self.rank_btn = Gtk.Button()
        rank_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        rank_box.set_halign(Gtk.Align.CENTER)
        rank_icon = Gtk.Image.new_from_icon_name("emblem-default-symbolic")
        rank_label = Gtk.Label(label=_("Test & Rank"))
        rank_box.append(rank_icon)
        rank_box.append(rank_label)
        self.rank_btn.set_child(rank_box)
        self.rank_btn.connect("clicked", self.on_rank_mirrors)
        self.rank_btn.set_sensitive(False)
        btn_box.append(self.rank_btn)
        loading_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=10)
        loading_box.set_margin_top(8)
        loading_box.set_halign(Gtk.Align.CENTER)
        self.loading_spinner = Gtk.Spinner()
        self.loading_label = Gtk.Label(label="")
        self.loading_label.add_css_class("dim-label")
        self.loading_label.add_css_class("caption")
        loading_box.append(self.loading_spinner)
        loading_box.append(self.loading_label)
        sidebar_box.append(loading_box)
        separator1 = Gtk.Separator()
        separator1.set_margin_top(6)
        separator1.set_margin_bottom(6)
        sidebar_box.append(separator1)
        repo_clamp = Adw.Clamp()
        repo_clamp.set_maximum_size(400)
        sidebar_box.append(repo_clamp)
        repo_group = Adw.PreferencesGroup()
        repo_group.set_title(_("Repositories"))
        repo_group.set_description(_("Enable or disable repositories"))
        repo_clamp.set_child(repo_group)
        self.repo_list = Gtk.ListBox()
        self.repo_list.set_selection_mode(Gtk.SelectionMode.NONE)
        self.repo_list.add_css_class("boxed-list")
        repo_group.add(self.repo_list)
        self.update_repo_list()
        separator2 = Gtk.Separator()
        separator2.set_margin_top(6)
        separator2.set_margin_bottom(6)
        sidebar_box.append(separator2)
        third_clamp = Adw.Clamp()
        third_clamp.set_maximum_size(400)
        sidebar_box.append(third_clamp)
        third_group = Adw.PreferencesGroup()
        third_group.set_title(_("Third-Party Repositories"))
        third_group.set_description(_("Enable or disable additional repositories"))
        third_clamp.set_child(third_group)
        self.third_list = Gtk.ListBox()
        self.third_list.set_selection_mode(Gtk.SelectionMode.NONE)
        self.third_list.add_css_class("boxed-list")
        third_group.add(self.third_list)
        self.third_party_repos = ["chaotic-aur", "blackarch", "archlinuxcn"]
        self.third_party_configs = {
            "chaotic-aur": {
                "key": "3056513887B78AEB",
                "keyring_url": "https://cdn-mirror.chaotic.cx/chaotic-aur/chaotic-keyring.pkg.tar.zst",
                "mirrorlist_url": "https://cdn-mirror.chaotic.cx/chaotic-aur/chaotic-mirrorlist.pkg.tar.zst",
                "section": "[chaotic-aur]\nInclude = /etc/pacman.d/chaotic-mirrorlist\n",
            },
            "blackarch": {
                "strap_url": "https://blackarch.org/strap.sh",
                "section": "[blackarch]\nServer = https://blackarch.org/blackarch/$repo/os/$arch\n",
            },
            "archlinuxcn": {
                "key": "4D41FD3D9E72E7966A573093E8CA6AEB220E236C",
                "section": "[archlinuxcn]\nServer = https://repo.archlinuxcn.org/$arch\n",
            },
        }
        display_names = ["Chaotic-AUR", "BlackArch", "ArchLinuxCN"]
        for i, repo_name in enumerate(self.third_party_repos):
            row = Adw.ActionRow()
            row.set_title(display_names[i])
            row.set_icon_name("folder-symbolic")
            switch = Gtk.Switch()
            switch.set_valign(Gtk.Align.CENTER)
            switch.set_active(self.repo_config.repositories[repo_name])
            switch.connect("state-set", self.on_third_party_toggle, repo_name)
            row.add_suffix(switch)
            row.set_activatable_widget(switch)
            self.third_list.append(row)
        sys_box = Gtk.Box(spacing=8)
        sys_box.set_margin_top(12)
        sys_box.set_homogeneous(True)
        sidebar_box.append(sys_box)
        sync_btn = Gtk.Button()
        sync_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        sync_box.set_halign(Gtk.Align.CENTER)
        sync_icon = Gtk.Image.new_from_icon_name("emblem-synchronizing-symbolic")
        sync_label = Gtk.Label(label=_("Sync"))
        sync_box.append(sync_icon)
        sync_box.append(sync_label)
        sync_btn.set_child(sync_box)
        sync_btn.set_tooltip_text(_("Save mirrorlist and sync repositories"))
        sync_btn.connect("clicked", self.on_sync_repos)
        sys_box.append(sync_btn)
        update_btn = Gtk.Button()
        update_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=8)
        update_box.set_halign(Gtk.Align.CENTER)
        update_icon = Gtk.Image.new_from_icon_name("system-software-update-symbolic")
        update_label = Gtk.Label(label=_("Update"))
        update_box.append(update_icon)
        update_box.append(update_label)
        update_btn.set_child(update_box)
        update_btn.add_css_class("destructive-action")
        update_btn.set_tooltip_text(_("Update all system packages"))
        update_btn.connect("clicked", self.on_update_system)
        sys_box.append(update_btn)
        right_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL)
        right_box.add_css_class("view")
        paned.set_end_child(right_box)
        mirror_toolbar = Gtk.Box(spacing=6)
        mirror_toolbar.add_css_class("toolbar")
        mirror_toolbar.set_margin_top(12)
        mirror_toolbar.set_margin_bottom(12)
        mirror_toolbar.set_margin_start(12)
        mirror_toolbar.set_margin_end(12)
        right_box.append(mirror_toolbar)
        left_controls = Gtk.Box(spacing=6)
        mirror_toolbar.append(left_controls)
        self.enable_btn = Gtk.Button()
        enable_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        enable_icon = Gtk.Image.new_from_icon_name("emblem-ok-symbolic")
        enable_label = Gtk.Label(label=_("Enable"))
        enable_box.append(enable_icon)
        enable_box.append(enable_label)
        self.enable_btn.set_child(enable_box)
        self.enable_btn.add_css_class("suggested-action")
        self.enable_btn.connect("clicked", self.on_enable_mirror)
        self.enable_btn.set_sensitive(False)
        self.enable_btn.set_tooltip_text(_("Enable selected mirror"))
        left_controls.append(self.enable_btn)
        self.disable_btn = Gtk.Button()
        disable_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        disable_icon = Gtk.Image.new_from_icon_name("process-stop-symbolic")
        disable_label = Gtk.Label(label=_("Disable"))
        disable_box.append(disable_icon)
        disable_box.append(disable_label)
        self.disable_btn.set_child(disable_box)
        self.disable_btn.connect("clicked", self.on_disable_mirror)
        self.disable_btn.set_sensitive(False)
        self.disable_btn.set_tooltip_text(_("Disable selected mirror"))
        left_controls.append(self.disable_btn)
        spacer = Gtk.Box()
        spacer.set_hexpand(True)
        mirror_toolbar.append(spacer)
        sort_label = Gtk.Label(label=_("Sort by:"))
        sort_label.add_css_class("dim-label")
        mirror_toolbar.append(sort_label)
        self.sort_speed_btn = Gtk.Button()
        speed_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        speed_icon = Gtk.Image.new_from_icon_name("speedometer-symbolic")
        speed_label = Gtk.Label(label=_("Speed"))
        speed_box.append(speed_icon)
        speed_box.append(speed_label)
        self.sort_speed_btn.set_child(speed_box)
        self.sort_speed_btn.connect("clicked", self.on_sort_speed)
        self.sort_speed_btn.set_sensitive(False)
        self.sort_speed_btn.set_tooltip_text(_("Sort mirrors by response time"))
        mirror_toolbar.append(self.sort_speed_btn)
        self.sort_country_btn = Gtk.Button()
        country_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        country_icon = Gtk.Image.new_from_icon_name("mark-location-symbolic")
        country_label = Gtk.Label(label=_("Country"))
        country_box.append(country_icon)
        country_box.append(country_label)
        self.sort_country_btn.set_child(country_box)
        self.sort_country_btn.connect("clicked", self.on_sort_country)
        self.sort_country_btn.set_sensitive(False)
        self.sort_country_btn.set_tooltip_text(_("Sort mirrors by country"))
        mirror_toolbar.append(self.sort_country_btn)
        self.sort_age_btn = Gtk.Button()
        age_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        age_icon = Gtk.Image.new_from_icon_name("document-open-recent-symbolic")
        age_label = Gtk.Label(label=_("Age"))
        age_box.append(age_icon)
        age_box.append(age_label)
        self.sort_age_btn.set_child(age_box)
        self.sort_age_btn.connect("clicked", self.on_sort_age)
        self.sort_age_btn.set_sensitive(False)
        self.sort_age_btn.set_tooltip_text(_("Sort mirrors by last sync time"))
        mirror_toolbar.append(self.sort_age_btn)
        self.iran_blackout_btn = Gtk.Button()
        iran_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
        iran_icon = Gtk.Image.new_from_icon_name("network-server-symbolic")
        iran_label = Gtk.Label(label=_("Iran Blackout"))
        iran_box.append(iran_icon)
        iran_box.append(iran_label)
        self.iran_blackout_btn.set_child(iran_box)
        self.iran_blackout_btn.connect("clicked", self.on_iran_blackout)
        self.iran_blackout_btn.set_tooltip_text(_("Add Iranian mirrors"))
        mirror_toolbar.append(self.iran_blackout_btn)
        self.mirror_scroll = Gtk.ScrolledWindow()
        self.mirror_scroll.set_vexpand(True)
        self.mirror_scroll.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        right_box.append(self.mirror_scroll)
        self.status_bar = Adw.StatusPage()
        self.status_bar.set_title(_("No Mirrors Loaded"))
        self.status_bar.set_description(
            _("Configure your filters and click 'Fetch' to load available mirrors")
        )
        self.status_bar.set_icon_name("network-server-symbolic")
        self.mirror_scroll.set_child(self.status_bar)
        self.mirror_list = Gtk.ListBox()
        self.mirror_list.set_selection_mode(Gtk.SelectionMode.SINGLE)
        self.mirror_list.add_css_class("boxed-list")
        self.mirror_list.set_margin_top(6)
        self.mirror_list.set_margin_bottom(12)
        self.mirror_list.set_margin_start(12)
        self.mirror_list.set_margin_end(12)
        self.mirror_list.connect("row-selected", self.on_mirror_selected)
        GLib.idle_add(self.load_country_list)

    def toggle_repo_config(self, repo_name, enable, is_third_party=False):
        try:
            with open("/etc/pacman.conf", "r") as f:
                config_text = f.read()
            section_snippet = None
            if is_third_party:
                config = self.third_party_configs.get(repo_name, {})
                section_snippet = config.get("section")
            modified = self.toggle_repo(config_text, repo_name, enable, section_snippet)
            temp_path = None
            with tempfile.NamedTemporaryFile(mode="w", delete=False) as temp:
                temp.write(modified)
                temp_path = temp.name
            subprocess.check_call(["pkexec", "cp", temp_path, "/etc/pacman.conf"])
            return True
        except Exception as e:
            self.show_error_dialog(_("Config Update Failed"), str(e))
            return False
        finally:
            if temp_path:
                os.unlink(temp_path)

    def toggle_repo(self, config_text, repo_name, enable, section_snippet=None):
        lines = config_text.splitlines()
        new_lines = []
        found_section = False
        in_section = False
        for line in lines:
            stripped = line.strip()
            if stripped == f"[{repo_name}]":
                found_section = True
                in_section = True
                if enable:
                    new_lines.append(line.lstrip("#"))
                else:
                    new_lines.append("#" + line if not line.startswith("#") else line)
                continue
            if in_section:
                if stripped.startswith("[") and not stripped == f"[{repo_name}]":
                    in_section = False
                    new_lines.append(line)
                    continue
                if stripped.startswith("Include =") or stripped.startswith("Server ="):
                    if enable:
                        new_lines.append(line.lstrip("#"))
                    else:
                        new_lines.append(
                            "#" + line if not line.startswith("#") else line
                        )
                else:
                    new_lines.append(line)
            else:
                new_lines.append(line)
        if enable and not found_section and section_snippet:
            new_lines.append("")
            new_lines += section_snippet.splitlines()
        return "\n".join(new_lines) + "\n"

    def load_country_list(self):
        def fetch_countries():
            try:
                countries = self.mirror_manager.fetch_countries_only()
                GLib.idle_add(self.update_country_list, countries)
            except:
                pass

        thread = threading.Thread(target=fetch_countries, daemon=True)
        thread.start()
        return False

    def update_country_list(self, countries):
        while self.country_store.get_n_items() > 1:
            self.country_store.remove(1)

        for country in sorted(countries):
            self.country_store.append(country)

        return False

    def set_loading_state(self, loading, message=""):
        self.is_loading = loading
        self.loading_spinner.set_spinning(loading)
        self.loading_label.set_text(message)
        self.refresh_btn.set_sensitive(not loading)
        self.header_refresh_btn.set_sensitive(not loading)
        self.rank_btn.set_sensitive(
            not loading and len(self.mirror_manager.mirrors) > 0
        )
        self.http_check.set_sensitive(not loading)
        self.https_check.set_sensitive(not loading)
        self.ipv4_check.set_sensitive(not loading)
        self.ipv6_check.set_sensitive(not loading)
        self.status_check.set_sensitive(not loading)
        self.country_row.set_sensitive(not loading)

    def show_error_dialog(self, title, message):
        dialog = Adw.AlertDialog(heading=title, body=message)
        dialog.add_response("ok", _("OK"))
        dialog.present(self)
        return False

    def show_info_dialog(self, title, message):
        dialog = Adw.AlertDialog(heading=title, body=message)
        dialog.add_response("ok", _("OK"))
        dialog.set_response_appearance("ok", Adw.ResponseAppearance.SUGGESTED)
        dialog.present(self)
        return False

    def on_refresh_mirrors(self, button):
        if self.is_loading:
            return

        protocols = []
        if self.http_check.get_active():
            protocols.append("http")
        if self.https_check.get_active():
            protocols.append("https")

        if not protocols:
            self.show_error_dialog(_("No Protocols"), _("Select at least one protocol"))
            return

        ip_versions = []
        if self.ipv4_check.get_active():
            ip_versions.append("4")
        if self.ipv6_check.get_active():
            ip_versions.append("6")

        if not ip_versions:
            self.show_error_dialog(
                _("No IP Versions"), _("Select at least one IP version")
            )
            return

        selected = self.country_row.get_selected()
        country = (
            self.country_store.get_string(selected)
            if selected < self.country_store.get_n_items()
            else None
        )
        if country == "Worldwide":
            country = None

        use_status = self.status_check.get_active()

        def fetch_in_background():
            try:
                GLib.idle_add(self.set_loading_state, True, _("Fetching mirrors..."))
                self.mirror_manager.fetch_mirrors(
                    country, protocols, ip_versions, use_status
                )
                GLib.idle_add(self.on_fetch_complete)
            except Exception as e:
                GLib.idle_add(self.on_fetch_error, str(e))

        thread = threading.Thread(target=fetch_in_background, daemon=True)
        thread.start()

    def on_fetch_complete(self):
        self.set_loading_state(False)
        self.update_mirror_list()
        self.enable_mirror_controls()
        self.mirror_scroll.set_child(self.mirror_list)

        count = len(self.mirror_manager.mirrors)
        self.show_info_dialog(_("Success"), f"Loaded {count} mirror(s)")
        return False

    def on_fetch_error(self, error_message):
        self.set_loading_state(False)
        self.show_error_dialog(_("Fetch Failed"), error_message)
        return False

    def on_rank_mirrors(self, button):
        if self.is_loading or not self.mirror_manager.mirrors:
            return

        def rank_in_background():
            try:
                total = len(self.mirror_manager.mirrors)
                for i, mirror in enumerate(self.mirror_manager.mirrors):
                    GLib.idle_add(
                        self.set_loading_state,
                        True,
                        f"Testing mirror {i + 1}/{total}...",
                    )
                    self.mirror_manager.test_mirror_speed(mirror)
                    GLib.idle_add(self.update_mirror_list)

                self.mirror_manager.sort_by_speed()
                GLib.idle_add(self.on_rank_complete)
            except Exception as e:
                GLib.idle_add(self.on_rank_error, str(e))

        thread = threading.Thread(target=rank_in_background, daemon=True)
        thread.start()

    def on_rank_complete(self):
        self.set_loading_state(False)
        self.update_mirror_list()
        self.show_info_dialog(
            _("Ranking Complete"),
            _("Mirrors sorted by speed.")
            + "\n\n"
            + _("Enable/disable mirrors, then use 'Sync' to save changes."),
        )
        return False

    def on_rank_error(self, error_message):
        self.set_loading_state(False)
        self.show_error_dialog(_("Ranking Error"), error_message)
        return False

    def enable_mirror_controls(self):
        has_mirrors = len(self.mirror_manager.mirrors) > 0
        self.rank_btn.set_sensitive(has_mirrors)
        self.sort_speed_btn.set_sensitive(has_mirrors)
        self.sort_country_btn.set_sensitive(has_mirrors)
        self.sort_age_btn.set_sensitive(has_mirrors)

    def on_mirror_selected(self, listbox, row):
        self.selected_mirror = row.mirror if row else None
        has_selection = self.selected_mirror is not None
        self.enable_btn.set_sensitive(has_selection)
        self.disable_btn.set_sensitive(has_selection)

    def on_enable_mirror(self, button):
        if self.selected_mirror:
            self.selected_mirror.enabled = True
            self.update_mirror_list()

    def on_disable_mirror(self, button):
        if self.selected_mirror:
            self.selected_mirror.enabled = False
            self.update_mirror_list()

    def on_sort_speed(self, button):
        self.mirror_manager.sort_by_speed()
        self.update_mirror_list()

    def on_sort_country(self, button):
        self.mirror_manager.sort_by_country()
        self.update_mirror_list()

    def on_sort_age(self, button):
        self.mirror_manager.sort_by_age()
        self.update_mirror_list()

    def update_mirror_list(self):
        while self.mirror_list.get_first_child():
            self.mirror_list.remove(self.mirror_list.get_first_child())

        for mirror in self.mirror_manager.mirrors:
            row = Adw.ActionRow()
            row.set_title(mirror.url)
            subtitle_parts = []
            country_display = f"📍 {mirror.country}"
            subtitle_parts.append(country_display)
            protocol_display = f"🔗 {mirror.protocol.upper()}"
            subtitle_parts.append(protocol_display)
            if mirror.speed is not None:
                speed_text = f"{mirror.speed:.0f}ms"
                if mirror.speed < 100:
                    speed_display = f"🟢 {speed_text}"
                elif mirror.speed < 300:
                    speed_display = f"🟡 {speed_text}"
                else:
                    speed_display = f"🔴 {speed_text}"
            else:
                speed_display = "⚪ Not tested"
            subtitle_parts.append(speed_display)
            sync_text = (
                mirror.last_sync.split("T")[0] if mirror.last_sync else "Unknown"
            )
            subtitle_parts.append(f"🕒 {sync_text}")
            row.set_subtitle(" • ".join(subtitle_parts))
            status_box = Gtk.Box(orientation=Gtk.Orientation.HORIZONTAL, spacing=6)
            if mirror.enabled:
                status_icon = Gtk.Image.new_from_icon_name("emblem-ok-symbolic")
                status_icon.add_css_class("success")
                status_label = Gtk.Label(label=_("Enabled"))
                status_label.add_css_class("success")
            else:
                status_icon = Gtk.Image.new_from_icon_name("window-close-symbolic")
                status_icon.add_css_class("error")
                status_label = Gtk.Label(label=_("Disabled"))
                status_label.add_css_class("dim-label")
            status_box.append(status_icon)
            status_box.append(status_label)
            row.add_suffix(status_box)
            row.mirror = mirror
            self.mirror_list.append(row)

        return False

    def on_sync_repos(self, button):
        mirrorlist_content = (
            "## Parch Mirrorlist\n\n"
            + "\n".join(
                f"Server = {m.url}$repo/os/$arch"
                for m in self.mirror_manager.mirrors
                if m.enabled
            )
            + "\n"
        )
        temp_path = None
        try:
            with tempfile.NamedTemporaryFile(mode="w", delete=False) as temp:
                temp.write(mirrorlist_content)
                temp_path = temp.name
            subprocess.check_call(
                ["pkexec", "cp", temp_path, "/etc/pacman.d/mirrorlist"]
            )
            self.show_sync_progress()
        except Exception as e:
            self.show_error_dialog(_("Save Failed"), str(e))
        finally:
            if temp_path:
                os.unlink(temp_path)

    def show_sync_progress(self):
        dialog = Adw.AlertDialog(heading=_("Syncing Repositories"))
        progress = Gtk.ProgressBar()
        progress.set_show_text(True)
        progress.set_text(_("Syncing..."))
        content_box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        content_box.append(progress)
        dialog.set_extra_child(content_box)
        dialog.add_response("ok", _("OK"))
        dialog.set_response_appearance("ok", Adw.ResponseAppearance.SUGGESTED)
        dialog.present(self)

        def pulse():
            progress.pulse()
            return self.is_syncing

        self.is_syncing = True
        timeout_id = GLib.timeout_add(100, pulse)

        def sync_thread():
            try:
                subprocess.check_call(["pkexec", "pacman", "-Sy"])
                GLib.idle_add(self.on_sync_complete, dialog, timeout_id)
            except Exception as e:
                GLib.idle_add(self.on_sync_error, dialog, timeout_id, str(e))

        thread = threading.Thread(target=sync_thread, daemon=True)
        thread.start()

    def on_sync_complete(self, dialog, timeout_id):
        GLib.source_remove(timeout_id)
        self.is_syncing = False
        dialog.close()
        self.show_info_dialog(
            _("Success"), _("Mirrorlist saved and repositories synced successfully")
        )
        return False

    def on_sync_error(self, dialog, timeout_id, error):
        GLib.source_remove(timeout_id)
        self.is_syncing = False
        dialog.close()
        self.show_error_dialog(_("Sync Failed"), error)
        return False

    def on_update_system(self, button):
        dialog = Adw.AlertDialog(
            heading=_("Update System?"),
            body=_(
                "This will update all system packages. This operation may take some time. Continue?"
            ),
        )
        dialog.add_response("cancel", _("Cancel"))
        dialog.add_response("update", _("Update"))
        dialog.set_response_appearance("update", Adw.ResponseAppearance.DESTRUCTIVE)
        dialog.connect("response", self.on_update_confirmed)
        dialog.present(self)

    def on_update_confirmed(self, dialog, response):
        if response == "update":
            open_terminal_with_repo_command("pkexec pacman -Syu", self)

    def update_repo_list(self):
        while self.repo_list.get_first_child():
            self.repo_list.remove(self.repo_list.get_first_child())

        repositories = self.repo_config.get_repositories()
        for repo_name in self.repo_config.standard_repos:
            enabled = repositories.get(repo_name, False)
            row = Adw.ActionRow()
            row.set_title(repo_name)
            row.set_icon_name("folder-symbolic")
            switch = Gtk.Switch()
            switch.set_active(enabled)
            switch.set_valign(Gtk.Align.CENTER)
            switch.connect("state-set", self.on_repo_toggle, repo_name)
            row.add_suffix(switch)
            row.set_activatable_widget(switch)
            self.repo_list.append(row)

    def update_third_list(self):
        row = self.third_list.get_first_child()
        i = 0
        while row:
            repo_name = self.third_party_repos[i]
            switch = row.get_activatable_widget()
            switch.set_active(self.repo_config.repositories[repo_name])
            row = row.get_next_sibling()
            i += 1

    def on_repo_toggle(self, switch, state, repo_name):
        success = self.toggle_repo_config(repo_name, state)
        if success:
            self.repo_config.repositories[repo_name] = state
            self.update_repo_list()
        else:
            switch.set_state(not state)
        return False

    def on_third_party_toggle(self, switch, state, repo_name):
        if state:
            try:
                config = self.third_party_configs[repo_name]
                if "strap_url" in config:
                    strap_path = f"/tmp/strap_{repo_name}.sh"
                    urllib.request.urlretrieve(config["strap_url"], strap_path)
                    os.chmod(strap_path, 0o755)
                    subprocess.check_call(["pkexec", "bash", strap_path])
                    os.unlink(strap_path)
                else:
                    if "key" in config:
                        subprocess.check_call(
                            [
                                "pkexec",
                                "pacman-key",
                                "--recv-key",
                                config["key"],
                                "--keyserver",
                                "keyserver.ubuntu.com",
                            ]
                        )
                        subprocess.check_call(
                            ["pkexec", "pacman-key", "--lsign-key", config["key"]]
                        )
                    if "keyring_url" in config:
                        subprocess.check_call(
                            [
                                "pkexec",
                                "pacman",
                                "-U",
                                "--noconfirm",
                                config["keyring_url"],
                                config["mirrorlist_url"],
                            ]
                        )
            except Exception as e:
                self.show_error_dialog("Enable Failed", str(e))
                switch.set_state(False)
                return False
        success = self.toggle_repo_config(repo_name, state, is_third_party=True)
        if success:
            self.repo_config.repositories[repo_name] = state
            self.update_third_list()
        else:
            switch.set_state(not state)
        return False

    def on_settings_clicked(self, button):
        options_window = PacmanOptionsWindow(parent=self)
        options_window.present()

    def on_iran_blackout(self, button):
        self.mirror_manager.add_iran_mirrors()
        self.update_mirror_list()
        self.enable_mirror_controls()
        self.mirror_scroll.set_child(self.mirror_list)
        count = len(self.mirror_manager.mirrors)
        self.show_info_dialog(
            _("Iran Blackout Added"),
            f"{_('Added')} 5 {_('Iranian mirrors')}.\n\n{_('Total mirrors')}: {count}",
        )

    def on_about_clicked(self, button):
        changelog = f"""<p>{_("Version")} 0.2 (2026)</p>
<ul>
<li>{_("Iran Blackout")}</li>
<li>{_("Concurrent testing")}</li>
<li>{_("Fixed parsing")}</li>
</ul>"""
        about = Adw.AboutWindow(
            transient_for=self,
            application_name=_("Parch Repository Manager"),
            application_icon="system-software-manager",
            version="0.2",
            developer_name="2026 Parch GNU/Linux Team",
            website="https://parchlinux.com",
            copyright="Copyright 2026 Parch GNU/Linux Team",
            license_type=Gtk.License.GPL_3_0,
            release_notes=changelog,
            release_notes_version="0.2",
        )
        about.present()


class ParchRepoManagerApp(Adw.Application):
    def __init__(self):
        super().__init__(application_id="com.parchlinux.mirrorman")
        self.connect("activate", self.on_activate)

    def on_activate(self, app):
        win = MainWindow(self)
        win.present()


def main():
    if not os.environ.get("DISPLAY") and not os.environ.get("WAYLAND_DISPLAY"):
        print("Error: No display environment detected.")
        print("Ensure you are running with 'sudo -E'.")
        sys.exit(1)

    if not Gtk.init_check():
        print("Error: Failed to initialize GTK.")
        sys.exit(1)

    Adw.StyleManager.get_default().set_color_scheme(Adw.ColorScheme.DEFAULT)

    app = ParchRepoManagerApp()
    app.run(sys.argv)


if __name__ == "__main__":
    main()
