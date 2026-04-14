import subprocess
import re
import os
import gettext

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
    """Detect and return the command to open a terminal based on desktop environment."""
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
    """Open a terminal with the given command."""
    escaped_cmd = command.replace("'", "'\"'\"'")
    terminal_cmd = get_suitable_terminal()
    if terminal_cmd is None:
        if parent_window:
            parent_window.show_error_dialog(
                _("Terminal Error"), _("No suitable terminal emulator found.")
            )
        else:
            print("Error: No suitable terminal emulator found.")
        return
    full_cmd = f"{terminal_cmd} bash -c '{escaped_cmd}; echo; echo Press ENTER to close...; read'"
    try:
        subprocess.Popen(full_cmd, shell=True)
    except Exception as e:
        if parent_window:
            parent_window.show_error_dialog(_("Terminal Error"), str(e))
        else:
            print(f"Error opening terminal: {e}")


class RepoConfig:
    def __init__(self):
        self.pacman_conf = "/etc/pacman.conf"
        self.standard_repos = ["core", "extra", "multilib"]
        self.third_party_repos = ["chaotic-aur", "blackarch", "archlinuxcn"]
        self.repositories = {
            repo: False for repo in self.standard_repos + self.third_party_repos
        }
        self.load_pacman_conf()

    def load_pacman_conf(self):
        """Load and parse pacman.conf to determine enabled repositories."""
        try:
            with open(self.pacman_conf, "r") as f:
                lines = f.readlines()
            repo_pattern = re.compile(r"^\s*(#?)\s*\[([^]]+)\]\s*$")
            current_repo = None
            for line in lines:
                match = repo_pattern.match(line)
                if match:
                    comment, repo_name = match.groups()
                    if repo_name == "options":
                        continue
                    current_repo = repo_name
                    enabled = not bool(comment)
                    if repo_name in self.repositories:
                        self.repositories[repo_name] = enabled
        except Exception as e:
            print(f"Error loading pacman.conf: {e}")

    def get_repositories(self):
        """Return the dictionary of repositories and their enabled status."""
        return self.repositories

    def set_repository_enabled(self, repo_name, enabled, parent_window=None):
        """Enable or disable a repository."""
        if repo_name not in self.repositories:
            print(f"Repository {repo_name} not supported.")
            return
        current_enabled = self.repositories[repo_name]
        if current_enabled == enabled:
            return  # Already in desired state
        if repo_name in self.standard_repos:
            if enabled:
                sed_cmd = f"sudo sed -i '/^#\\[{repo_name}\\]/,/^\\[/s/^#//g' {self.pacman_conf}"
            else:
                sed_cmd = f"sudo sed -i '/^\\[{repo_name}\\]/,/^\\[/s/^/#/' {self.pacman_conf}"
            commands = [sed_cmd, "sudo pacman -Syy"]
            cmd_str = " && ".join(commands)
            open_terminal_with_repo_command(cmd_str, parent_window)
        elif repo_name in self.third_party_repos:
            func_name = repo_name.replace("-", "_")
            if enabled:
                getattr(self, f"enable_{func_name}")(parent_window)
            else:
                getattr(self, f"disable_{func_name}")(parent_window)

    def add_repository(self, repo_name, repo_url, parent_window=None):
        """Add a new repository to pacman.conf."""
        if repo_name in self.repositories:
            print(f"Repository {repo_name} already exists or is reserved.")
            return
        commands = [
            "echo '' | sudo tee -a /etc/pacman.conf",
            f"echo '[{repo_name}]' | sudo tee -a /etc/pacman.conf",
            f"echo 'Server = {repo_url}' | sudo tee -a /etc/pacman.conf",
            "sudo pacman -Syy",
        ]
        cmd_str = " && ".join(commands)
        open_terminal_with_repo_command(cmd_str, parent_window)
        self.repositories[repo_name] = True

    def enable_chaotic_aur(self, parent_window=None):
        """Enable Chaotic-AUR repository."""
        commands = [
            "sudo pacman-key --recv-key 3056513887B78AEB --keyserver keyserver.ubuntu.com",
            "sudo pacman-key --lsign-key 3056513887B78AEB",
            "sudo pacman -U 'https://cdn-mirror.chaotic.cx/chaotic-aur/chaotic-keyring.pkg.tar.zst' --noconfirm",
            "sudo pacman -U 'https://cdn-mirror.chaotic.cx/chaotic-aur/chaotic-mirrorlist.pkg.tar.zst' --noconfirm",
            "echo '' | sudo tee -a /etc/pacman.conf",
            "echo '[chaotic-aur]' | sudo tee -a /etc/pacman.conf",
            "echo 'Include = /etc/pacman.d/chaotic-mirrorlist' | sudo tee -a /etc/pacman.conf",
            "sudo pacman -Syy",
        ]
        cmd_str = " && ".join(commands)
        open_terminal_with_repo_command(cmd_str, parent_window)

    def disable_chaotic_aur(self, parent_window=None):
        """Disable Chaotic-AUR repository."""
        commands = [
            # Remove the entire [chaotic-aur] section until the next section or end of file
            r"sudo sed -i '/\[chaotic-aur\]/,/^\[/{/^\[/!d; /^\[chaotic-aur\]/d}' /etc/pacman.conf",
            "sudo pacman -Syy",
        ]
        cmd_str = " && ".join(commands)
        open_terminal_with_repo_command(cmd_str, parent_window)

    def enable_blackarch(self, parent_window=None):
        """Enable BlackArch repository."""
        commands = [
            "cd /tmp",
            "curl -O https://blackarch.org/strap.sh",
            "echo '26849980b35a42e6e192c6d9ed8c46f0d6d06047 strap.sh' | sha1sum -c",
            "if [ $? -eq 0 ]; then chmod +x strap.sh && sudo ./strap.sh; else echo 'SHA1 verification failed!'; exit 1; fi",
            "rm -f strap.sh",
            "sudo pacman -Syy",
        ]
        cmd_str = " && ".join(commands)
        open_terminal_with_repo_command(cmd_str, parent_window)

    def disable_blackarch(self, parent_window=None):
        """Disable BlackArch repository."""
        commands = [
            # Remove the entire [blackarch] section until the next section or end of file
            r"sudo sed -i '/\[blackarch\]/,/^\[/{/^\[/!d; /^\[blackarch\]/d}' /etc/pacman.conf",
            "sudo pacman -Syy",
        ]
        cmd_str = " && ".join(commands)
        open_terminal_with_repo_command(cmd_str, parent_window)

    def enable_archlinuxcn(self, parent_window=None):
        """Enable ArchLinuxCN repository."""
        commands = [
            "echo '' | sudo tee -a /etc/pacman.conf",
            "echo '[archlinuxcn]' | sudo tee -a /etc/pacman.conf",
            "echo 'Server = https://repo.archlinuxcn.org/$arch' | sudo tee -a /etc/pacman.conf",
            "sudo pacman -Syy",
            "sudo pacman -S archlinuxcn-keyring --noconfirm",
        ]
        cmd_str = " && ".join(commands)
        open_terminal_with_repo_command(cmd_str, parent_window)

    def disable_archlinuxcn(self, parent_window=None):
        """Disable ArchLinuxCN repository."""
        commands = [
            # Remove the entire [archlinuxcn] section until the next section or end of file
            r"sudo sed -i '/\[archlinuxcn\]/,/^\[/{/^\[/!d; /^\[archlinuxcn\]/d}' /etc/pacman.conf",
            "sudo pacman -Syy",
        ]
        cmd_str = " && ".join(commands)
        open_terminal_with_repo_command(cmd_str, parent_window)
