import urllib.request
import os
import json
import time
import ssl
import socket
import concurrent.futures
from urllib.error import URLError, HTTPError
from datetime import datetime, timezone

IRANIAN_MIRRORS = [
    "https://mirror.0-1.cloud/archlinux/$repo/os/$arch",
    "https://mirror.kernel.ir/archlinux/$repo/os/$arch",
    "https://mirror.mobinhost.com/archlinux/$repo/os/$arch",
    "http://repo.iut.ac.ir/repo/archlinux/$repo/os/$arch",
    "https://mirror.arvancloud.ir/archlinux/$repo/os/$arch",
]


class MirrorManager:
    class Mirror:
        def __init__(
            self,
            url,
            country,
            protocol,
            speed=None,
            last_sync=None,
            enabled=True,
            ipv4=True,
            ipv6=False,
        ):
            self.url = url
            self.country = country
            self.protocol = protocol
            self.speed = speed
            self.last_sync = last_sync
            self.enabled = enabled
            self.ipv4 = ipv4
            self.ipv6 = ipv6

    def __init__(self):
        self.mirrors = []
        # ONLY touch mirrorlist, NEVER pacman.conf
        self.mirrorlist_file = "/etc/pacman.d/mirrorlist"
        self.mirrorlist_backup = "/etc/pacman.d/mirrorlist.backup"
        self.countries = set()
        # Create SSL context that handles modern SSL properly
        self.ssl_context = ssl.create_default_context()
        # For testing/development, you can uncomment this line:
        # self.ssl_context.check_hostname = False
        # self.ssl_context.verify_mode = ssl.CERT_NONE

    def fetch_countries_only(self):
        """Fetch just the list of countries without loading all mirrors"""
        try:
            request = urllib.request.Request(
                "https://archlinux.org/mirrors/status/json/",
                headers={"User-Agent": "Arch-Repository-Manager/1.0"},
            )

            with urllib.request.urlopen(
                request, timeout=10, context=self.ssl_context
            ) as response:
                data = json.loads(response.read().decode())

                countries = set()
                for mirror in data.get("urls", []):
                    country = mirror.get("country", "Unknown")
                    if country and country != "Unknown":
                        countries.add(country)

                self.countries = countries
                return countries

        except Exception as e:
            # Return empty set on error, don't crash
            return set()

    def fetch_mirrors(
        self, country=None, protocols=None, ip_versions=None, use_status=False
    ):
        """Fetch mirror list from Arch Linux mirror status API"""
        try:
            # Use proper SSL context
            request = urllib.request.Request(
                "https://archlinux.org/mirrors/status/json/",
                headers={"User-Agent": "Arch-Repository-Manager/1.0"},
            )

            with urllib.request.urlopen(
                request, timeout=10, context=self.ssl_context
            ) as response:
                data = json.loads(response.read().decode())

                self.mirrors = []
                self.countries = set()

                for mirror in data.get("urls", []):
                    mirror_country = mirror.get("country", "Unknown")
                    self.countries.add(mirror_country)

                    # Apply country filter
                    if country and country != mirror_country:
                        continue

                    # Apply protocol filter
                    protocol = mirror.get("protocol", "unknown")
                    if protocols and protocol.lower() not in protocols:
                        continue

                    # Get mirror details
                    url = mirror.get("url")
                    if not url:
                        continue

                    # Apply IP version
                    ipv4 = "4" in ip_versions if ip_versions else True
                    ipv6 = "6" in ip_versions if ip_versions else False

                    last_sync = mirror.get("last_sync", None)

                    # Apply status filter
                    if use_status and not self.is_mirror_up_to_date(mirror):
                        continue

                    # Create mirror object
                    self.mirrors.append(
                        self.Mirror(
                            url=url,
                            country=mirror_country,
                            protocol=protocol,
                            last_sync=last_sync,
                            ipv4=ipv4,
                            ipv6=ipv6,
                        )
                    )

                self.countries.add("Worldwide")
                return True

        except HTTPError as e:
            raise Exception(f"HTTP Error {e.code}: {e.reason}")
        except URLError as e:
            if hasattr(e.reason, "errno"):
                raise Exception(f"Network error: {e.reason}")
            else:
                raise Exception(f"Failed to connect: {e.reason}")
        except ssl.SSLError as e:
            raise Exception(
                f"SSL Error: {str(e)}\n\nTry updating your system certificates:\nsudo pacman -S ca-certificates"
            )
        except json.JSONDecodeError as e:
            raise Exception(f"Invalid response from mirror status API: {str(e)}")
        except Exception as e:
            raise Exception(f"Unexpected error: {str(e)}")

    def is_mirror_up_to_date(self, mirror):
        """Check if mirror was synced within the last 24 hours"""
        last_sync = mirror.get("last_sync")
        if not last_sync:
            return False

        try:
            # Parse ISO format timestamp
            sync_time = datetime.fromisoformat(last_sync.replace("Z", "+00:00"))
            now = datetime.now(timezone.utc)
            hours_old = (now - sync_time).total_seconds() / 3600

            # Consider up-to-date if synced within 24 hours
            return hours_old < 24

        except (ValueError, AttributeError):
            return False

    def test_mirror_speed(self, mirror):
        """Test mirror response time"""
        if not mirror.url:
            mirror.speed = None
            return

        # Only test HTTP/HTTPS mirrors
        if not mirror.url.startswith(("http://", "https://")):
            mirror.speed = None
            return

        start_time = time.time()
        try:
            # Test with a small core database file
            test_url = mirror.url.rstrip("/") + "/core/os/x86_64/core.db"

            request = urllib.request.Request(
                test_url, headers={"User-Agent": "Arch-Repository-Manager/1.0"}
            )

            with urllib.request.urlopen(
                request, timeout=5, context=self.ssl_context
            ) as response:
                # Just read headers, don't download entire file
                response.read(1024)
                elapsed = (time.time() - start_time) * 1000  # Convert to ms
                mirror.speed = elapsed

        except (URLError, HTTPError, ssl.SSLError, TimeoutError):
            # Mirror unreachable or slow
            mirror.speed = None
        except Exception:
            # Any other error
            mirror.speed = None

    def refresh_mirrors(
        self, country=None, protocols=None, ip_versions=None, use_status=False
    ):
        """Refresh mirror list and test speeds - DOES NOT SAVE"""
        if self.fetch_mirrors(country, protocols, ip_versions, use_status):
            # Test mirror speeds (this can take a while)
            for i, mirror in enumerate(self.mirrors):
                self.test_mirror_speed(mirror)
                # Optional: Add progress callback here

    def enable_mirror(self, mirror):
        """Enable a specific mirror - DOES NOT SAVE"""
        mirror.enabled = True

    def disable_mirror(self, mirror):
        """Disable a specific mirror - DOES NOT SAVE"""
        mirror.enabled = False

    def rank_mirrors(self):
        """Sort mirrors by speed - DOES NOT SAVE"""
        self.sort_by_speed()

    def sort_by_speed(self):
        """Sort mirrors by response time (fastest first)"""
        # Put mirrors with no speed at the end
        self.mirrors.sort(
            key=lambda m: (m.speed is None, m.speed if m.speed else float("inf"))
        )

    def sort_by_country(self):
        """Sort mirrors alphabetically by country"""
        self.mirrors.sort(key=lambda m: m.country)

    def sort_by_age(self):
        """Sort mirrors by last sync time (newest first)"""

        def sort_key(m):
            if not m.last_sync:
                return "9999-12-31"  # Put unknowns at the end
            return m.last_sync

        self.mirrors.sort(key=sort_key, reverse=True)

    def get_countries(self):
        """Get sorted list of available countries"""
        countries = self.countries - {"Worldwide"}
        return sorted(countries)

    def add_iran_mirrors(self):
        """Add Iranian mirrors to the mirror list"""
        for mirror_url in IRANIAN_MIRRORS:
            url = mirror_url.replace("$repo/os/$arch", "")
            protocol = "https" if mirror_url.startswith("https") else "http"
            self.mirrors.append(
                self.Mirror(
                    url=url,
                    country="IRAN",
                    protocol=protocol,
                    last_sync=None,
                    ipv4=True,
                    ipv6=False,
                )
            )

    def test_mirror_speed_concurrent(self, mirrors, max_workers=10, timeout=5):
        """Test mirror speeds concurrently using ThreadPoolExecutor"""

        def test_single_mirror(mirror):
            if not mirror.url or not mirror.url.startswith(("http://", "https://")):
                mirror.speed = None
                return mirror
            start_time = time.time()
            try:
                test_url = mirror.url.rstrip("/") + "/core/os/x86_64/core.db"
                request = urllib.request.Request(
                    test_url, headers={"User-Agent": "Arch-Repository-Manager/1.0"}
                )
                socket.setdefaulttimeout(timeout)
                with urllib.request.urlopen(
                    request, timeout=timeout, context=self.ssl_context
                ) as response:
                    response.read(1024)
                    elapsed = (time.time() - start_time) * 1000
                    mirror.speed = elapsed
            except Exception:
                mirror.speed = None
            return mirror

        with concurrent.futures.ThreadPoolExecutor(max_workers=max_workers) as executor:
            futures = {executor.submit(test_single_mirror, m): m for m in mirrors}
            for future in concurrent.futures.as_completed(futures):
                pass

        return mirrors

    def save_mirrorlist(self):
        """Save enabled mirrors to /etc/pacman.d/mirrorlist - NEVER TOUCHES pacman.conf"""
        try:
            # SAFETY CHECK - verify we're writing to the correct file
            if self.mirrorlist_file != "/etc/pacman.d/mirrorlist":
                raise Exception("SAFETY ERROR: Attempting to write to wrong file!")

            # Create backup of existing mirrorlist
            import shutil

            if os.path.exists(self.mirrorlist_file):
                shutil.copy2(self.mirrorlist_file, self.mirrorlist_backup)

            # Write new mirrorlist
            with open(self.mirrorlist_file, "w") as f:
                f.write("##\n")
                f.write("## Parch Linux repository mirrorlist\n")
                f.write(
                    f"## Generated by Parch Repository Manager on {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}\n"
                )
                f.write("##\n\n")

                enabled_count = sum(1 for m in self.mirrors if m.enabled)
                f.write(f"## {enabled_count} enabled mirror(s)\n\n")

                for mirror in self.mirrors:
                    if mirror.enabled:
                        url = mirror.url.rstrip("/") + "/$repo/os/$arch"
                        f.write(f"Server = {url}\n")

        except PermissionError:
            raise Exception("Permission denied. Ensure you are running as root.")
        except IOError as e:
            raise Exception(f"Failed to write mirrorlist: {str(e)}")
