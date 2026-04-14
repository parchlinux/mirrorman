import configparser
import os
import gi
import gettext

gi.require_version("Gtk", "4.0")
gi.require_version("Adw", "1")
from gi.repository import Gtk, Adw, Gio, GLib
import tempfile
import subprocess

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


class PacmanOptionsWindow(Adw.Window):
    __gtype_name__ = "PacmanOptionsWindow"

    def __init__(self, parent=None):
        super().__init__(transient_for=parent, modal=True)
        self.set_title(_("Pacman Options"))
        self.set_default_size(600, 600)

        self.config = configparser.ConfigParser(allow_no_value=True)
        self.pacman_conf = "/etc/pacman.conf"
        self.load_config()

        toolbar_view = Adw.ToolbarView()
        self.set_content(toolbar_view)

        header = Adw.HeaderBar()
        toolbar_view.add_top_bar(header)

        save_btn = Gtk.Button(label=_("Save"))
        save_btn.add_css_class("suggested-action")
        save_btn.connect("clicked", self.on_save)
        header.pack_end(save_btn)

        cancel_btn = Gtk.Button(label=_("Cancel"))
        cancel_btn.connect("clicked", lambda btn: self.destroy())
        header.pack_start(cancel_btn)

        scroll = Gtk.ScrolledWindow()
        scroll.set_policy(Gtk.PolicyType.NEVER, Gtk.PolicyType.AUTOMATIC)
        scroll.set_vexpand(True)
        toolbar_view.set_content(scroll)

        box = Gtk.Box(orientation=Gtk.Orientation.VERTICAL, spacing=12)
        box.set_margin_top(12)
        box.set_margin_bottom(12)
        box.set_margin_start(12)
        box.set_margin_end(12)
        scroll.set_child(box)

        group = Adw.PreferencesGroup(title=_("Package Management"))
        box.append(group)

        ignore_row = Adw.ActionRow(title=_("IgnorePkg"))
        ignore_row.set_subtitle(_("Space-separated packages to ignore during upgrades"))
        self.ignore_entry = Gtk.Entry(
            text=" ".join(self.get_list("options", "IgnorePkg"))
        )
        ignore_row.add_suffix(self.ignore_entry)
        group.add(ignore_row)

        hold_row = Adw.ActionRow(title=_("HoldPkg"))
        hold_row.set_subtitle(_("Space-separated packages to hold during upgrades"))
        self.hold_entry = Gtk.Entry(text=" ".join(self.get_list("options", "HoldPkg")))
        hold_row.add_suffix(self.hold_entry)
        group.add(hold_row)

        noupgrade_row = Adw.ActionRow(title=_("NoUpgrade"))
        noupgrade_row.set_subtitle(_("Space-separated files to protect from upgrade"))
        self.noupgrade_entry = Gtk.Entry(
            text=" ".join(self.get_list("options", "NoUpgrade"))
        )
        noupgrade_row.add_suffix(self.noupgrade_entry)
        group.add(noupgrade_row)

        noextract_row = Adw.ActionRow(title=_("NoExtract"))
        noextract_row.set_subtitle(_("Space-separated files to skip during extraction"))
        self.noextract_entry = Gtk.Entry(
            text=" ".join(self.get_list("options", "NoExtract"))
        )
        noextract_row.add_suffix(self.noextract_entry)
        group.add(noextract_row)

        syncfirst_row = Adw.ActionRow(title=_("SyncFirst"))
        syncfirst_row.set_subtitle(_("Space-separated packages to sync first"))
        self.syncfirst_entry = Gtk.Entry(
            text=" ".join(self.get_list("options", "SyncFirst"))
        )
        syncfirst_row.add_suffix(self.syncfirst_entry)
        group.add(syncfirst_row)

        checkspace_row = Adw.ActionRow(title=_("CheckSpace"))
        checkspace_row.set_subtitle(
            _("Check for sufficient disk space before installing")
        )
        self.checkspace_switch = Gtk.Switch()
        self.checkspace_switch.set_valign(Gtk.Align.CENTER)
        self.checkspace_switch.set_active(self.get_bool("options", "CheckSpace"))
        checkspace_row.add_suffix(self.checkspace_switch)
        checkspace_row.set_activatable_widget(self.checkspace_switch)
        group.add(checkspace_row)

        candy_row = Adw.ActionRow(title=_("ILoveCandy"))
        candy_row.set_subtitle(_("Display a candy cane progress bar"))
        self.candy_switch = Gtk.Switch()
        self.candy_switch.set_valign(Gtk.Align.CENTER)
        self.candy_switch.set_active(self.get_bool("options", "ILoveCandy"))
        candy_row.add_suffix(self.candy_switch)
        candy_row.set_activatable_widget(self.candy_switch)
        group.add(candy_row)

        parallel_row = Adw.ActionRow(title=_("ParallelDownloads"))
        parallel_row.set_subtitle(_("Number of parallel downloads (1-100)"))
        self.parallel_spin = Gtk.SpinButton()
        self.parallel_spin.set_adjustment(
            Gtk.Adjustment(lower=1, upper=100, step_increment=1)
        )
        self.parallel_spin.set_value(self.get_int("options", "ParallelDownloads", 5))
        parallel_row.add_suffix(self.parallel_spin)
        group.add(parallel_row)

        cleanmethod_row = Adw.ComboRow(title=_("CleanMethod"))
        cleanmethod_row.set_subtitle(_("Cache cleaning method"))
        cleanmethod_store = Gtk.StringList()
        cleanmethod_store.append(_("KeepInstalled"))
        cleanmethod_store.append(_("KeepCurrent"))
        cleanmethod_row.set_model(cleanmethod_store)
        cleanmethod_value = self.get_string("options", "CleanMethod", "KeepInstalled")
        cleanmethod_row.set_selected(0 if cleanmethod_value == "KeepInstalled" else 1)
        group.add(cleanmethod_row)
        self.cleanmethod_row = cleanmethod_row

        arch_row = Adw.ComboRow(title=_("Architecture"))
        arch_row.set_subtitle(_("System architecture"))
        arch_store = Gtk.StringList()
        arch_store.append(_("auto"))
        arch_store.append(_("x86_64"))
        arch_store.append(_("x86_64_v3"))
        arch_row.set_model(arch_store)
        arch_value = self.get_string("options", "Architecture", "auto")
        if arch_value == "auto":
            arch_row.set_selected(0)
        elif arch_value == "x86_64":
            arch_row.set_selected(1)
        elif arch_value == "x86_64_v3":
            arch_row.set_selected(2)
        else:
            arch_row.set_selected(0)
        group.add(arch_row)
        self.arch_row = arch_row

    def load_config(self):
        if os.path.exists(self.pacman_conf):
            try:
                self.config.read(self.pacman_conf)
            except configparser.Error as e:
                dialog = Adw.AlertDialog(
                    heading=_("Error"),
                    body=f"{_('Failed to parse pacman.conf')}: {str(e)}",
                )
                dialog.add_response("ok", _("OK"))
                dialog.present(self)

    def get_list(self, section, option):
        if self.config.has_option(section, option):
            return self.config.get(section, option).split()
        return []

    def get_bool(self, section, option):
        if self.config.has_option(section, option):
            try:
                value = self.config.get(section, option)
                if value is None or value == "":
                    return True
                return self.config.getboolean(section, option)
            except (ValueError, configparser.Error):
                return False
        return False

    def get_int(self, section, option, default=0):
        if self.config.has_option(section, option):
            try:
                return self.config.getint(section, option)
            except ValueError:
                return default
        return default

    def get_string(self, section, option, default=""):
        if self.config.has_option(section, option):
            return self.config.get(section, option)
        return default

    def get_key_from_line(self, line):
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            return None
        if "=" in stripped:
            return stripped.split("=")[0].strip()
        else:
            return stripped

    def on_save(self, button):
        updates = {}

        # IgnorePkg
        ignore_list = [x for x in self.ignore_entry.get_text().split() if x]
        if ignore_list:
            updates["IgnorePkg"] = f"IgnorePkg = {' '.join(ignore_list)}\n"
        else:
            updates["IgnorePkg"] = None

        # HoldPkg
        hold_list = [x for x in self.hold_entry.get_text().split() if x]
        if hold_list:
            updates["HoldPkg"] = f"HoldPkg = {' '.join(hold_list)}\n"
        else:
            updates["HoldPkg"] = None

        # NoUpgrade
        noupgrade_list = [x for x in self.noupgrade_entry.get_text().split() if x]
        if noupgrade_list:
            updates["NoUpgrade"] = f"NoUpgrade = {' '.join(noupgrade_list)}\n"
        else:
            updates["NoUpgrade"] = None

        # NoExtract
        noextract_list = [x for x in self.noextract_entry.get_text().split() if x]
        if noextract_list:
            updates["NoExtract"] = f"NoExtract = {' '.join(noextract_list)}\n"
        else:
            updates["NoExtract"] = None

        # SyncFirst
        syncfirst_list = [x for x in self.syncfirst_entry.get_text().split() if x]
        if syncfirst_list:
            updates["SyncFirst"] = f"SyncFirst = {' '.join(syncfirst_list)}\n"
        else:
            updates["SyncFirst"] = None

        # CheckSpace
        if self.checkspace_switch.get_active():
            updates["CheckSpace"] = "CheckSpace\n"
        else:
            updates["CheckSpace"] = None

        # ILoveCandy
        if self.candy_switch.get_active():
            updates["ILoveCandy"] = "ILoveCandy\n"
        else:
            updates["ILoveCandy"] = None

        # ParallelDownloads
        parallel_val = int(self.parallel_spin.get_value())
        updates["ParallelDownloads"] = f"ParallelDownloads = {parallel_val}\n"

        # CleanMethod
        cleanmethod_value = (
            "KeepInstalled"
            if self.cleanmethod_row.get_selected() == 0
            else "KeepCurrent"
        )
        updates["CleanMethod"] = f"CleanMethod = {cleanmethod_value}\n"

        # Architecture
        arch_selected = self.arch_row.get_selected()
        arch_value = (
            "auto"
            if arch_selected == 0
            else "x86_64"
            if arch_selected == 1
            else "x86_64_v3"
        )
        updates["Architecture"] = f"Architecture = {arch_value}\n"

        temp_path = None
        try:
            with open(self.pacman_conf, "r") as f:
                lines = f.readlines()

            new_lines = []
            in_options = False
            added = set()

            for line in lines:
                added_this = False
                if line.strip() == "[options]":
                    in_options = True
                elif (
                    line.strip().startswith("[")
                    and line.strip().endswith("]")
                    and line.strip() != "[options]"
                ):
                    # Add pending updates before leaving section
                    for key, val in updates.items():
                        if val and key not in added:
                            new_lines.append(val)
                            added.add(key)
                    in_options = False

                if in_options:
                    key = self.get_key_from_line(line)
                    if key in updates:
                        if updates[key]:
                            new_lines.append(updates[key])
                            added.add(key)
                            added_this = True
                        # else skip to remove
                if not (in_options and added_this):
                    new_lines.append(line)

            # If file ended while still in options section, add remaining
            if in_options:
                for key, val in updates.items():
                    if val and key not in added:
                        new_lines.append(val)
                        added.add(key)

            # Write to temp file
            with tempfile.NamedTemporaryFile(mode="w", delete=False) as temp:
                for line in new_lines:
                    temp.write(line)
                temp_path = temp.name

            # Use pkexec to copy
            subprocess.check_call(["pkexec", "cp", temp_path, self.pacman_conf])

            dialog = Adw.AlertDialog(
                heading=_("Success"), body=_("Settings saved successfully.")
            )
            dialog.add_response("ok", _("OK"))
            dialog.present(self)
        except Exception as e:
            dialog = Adw.AlertDialog(heading=_("Error"), body=str(e))
            dialog.add_response("ok", _("OK"))
            dialog.present(self)
        finally:
            if temp_path:
                os.unlink(temp_path)

        self.destroy()
