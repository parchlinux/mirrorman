import subprocess
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


class SyncManager:
    """Manages pacman repository sync and system updates"""

    def __init__(self):
        self.pacman_bin = "/usr/bin/pacman"
        self._verify_pacman()

    def _verify_pacman(self):
        """Verify pacman is available"""
        if not os.path.exists(self.pacman_bin):
            raise Exception(_("Pacman not found. Is this an Arch-based system?"))

    def sync_repositories(self):
        """
        Sync package databases (pacman -Syy)
        Forces refresh of all package databases
        """
        try:
            result = subprocess.run(
                [self.pacman_bin, "-Syy", "--noconfirm"],
                capture_output=True,
                text=True,
                check=True,
            )
            return result.stdout

        except subprocess.CalledProcessError as e:
            error_msg = e.stderr if e.stderr else str(e)
            raise Exception(f"{_('Failed to sync repositories')}:\n{error_msg}")
        except FileNotFoundError:
            raise Exception(_("Pacman executable not found"))
        except Exception as e:
            raise Exception(f"{_('Unexpected error during sync')}: {str(e)}")

    def update_system(self):
        """
        Update system packages (pacman -Syu)
        Syncs databases and upgrades packages
        """
        try:
            result = subprocess.run(
                [self.pacman_bin, "-Syu", "--noconfirm"],
                capture_output=True,
                text=True,
                check=True,
            )
            return result.stdout

        except subprocess.CalledProcessError as e:
            error_msg = e.stderr if e.stderr else str(e)
            raise Exception(f"{_('Failed to update system')}:\n{error_msg}")
        except FileNotFoundError:
            raise Exception(_("Pacman executable not found"))
        except Exception as e:
            raise Exception(f"{_('Unexpected error during update')}: {str(e)}")

    def check_updates(self):
        """
        Check for available updates without installing them
        Returns list of packages that can be updated
        """
        try:
            result = subprocess.run(
                [self.pacman_bin, "-Qu"],
                capture_output=True,
                text=True,
                check=False,  # Non-zero exit is normal when no updates
            )

            if result.returncode == 0:
                # Parse output to get list of updateable packages
                updates = []
                for line in result.stdout.strip().split("\n"):
                    if line:
                        updates.append(line)
                return updates
            else:
                return []

        except Exception as e:
            raise Exception(f"{_('Failed to check for updates')}: {str(e)}")

    def clean_cache(self):
        """
        Clean package cache (pacman -Sc)
        Removes old package files from cache
        """
        try:
            result = subprocess.run(
                [self.pacman_bin, "-Sc", "--noconfirm"],
                capture_output=True,
                text=True,
                check=True,
            )
            return result.stdout

        except subprocess.CalledProcessError as e:
            error_msg = e.stderr if e.stderr else str(e)
            raise Exception(f"{_('Failed to clean cache')}:\n{error_msg}")
        except Exception as e:
            raise Exception(f"{_('Unexpected error during cache clean')}: {str(e)}")
