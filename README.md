# MirrorMan

<p align="center">
  <img src="assets/icon.png" alt="MirrorMan Logo" width="128" height="128"/>
  <br/><i>(icon needed in assets/icon.png)</i>
</p>

<p align="center">
  <b>A modern GTK4/Libadwaita repository manager for Parch Linux</b>
</p>

---

MirrorMan is a modern GTK4/Libadwaita repository manager designed specifically for Parch Linux. It provides an intuitive graphical interface for managing Pacman mirrors, testing connection speeds, sorting mirrors by performance, and configuring system repositories. The application fetches mirror data directly from the Arch Linux mirror status API, ensuring you always have access to the most current mirror information available.

The application focuses on simplicity and usability while providing all the necessary tools for effective mirror management. Whether you need to quickly find the fastest mirror for your location or configure multiple third-party repositories, MirrorMan handles everything through a clean, native GTK4 interface that follows Libadwaita design guidelines.

---

## Features

**Mirror Management**
- Filter mirrors by country, protocol (HTTP/HTTPS), and IP version (IPv4/IPv6)
- Test mirror response times and auto-rank by speed
- Enable/disable mirrors individually with toggle switches
- Sort mirrors by speed, country, or last sync time
- Iran Blackout quick-add for common regional mirrors

**Repository Configuration**
- Toggle standard Arch repositories
- Manage third-party repositories (Chaotic-AUR, BlackArch, ArchLinuxCN)
- Sync package databases with a single click

**Pacman Settings**
- Configure parallel downloads and download timeout
- Toggle verbose package lists and color output
- Manage package cache settings

---

## Requirements

| Dependency | Version |
|------------|---------|
| Python     | 3.8+    |
| GTK        | 4.0     |
| Libadwaita | 1.0+    |
| PyGObject  | Latest  |
| pkexec     | Latest  |

---

## Installation

```bash
# Clone the repository
git clone https://github.com/parchlinux/mirrorman.git
cd mirrorman

# Install dependencies
sudo pacman -S python-gobject gtk4 libadwaita

# Run the application
python src/main.py

# Or build with makepkg
makepkg -sic
```

---

## Usage

1. **Configure Filters** - Select country, protocols, and IP versions in the sidebar
2. **Fetch Mirrors** - Click **Fetch** to retrieve available mirrors from the Arch API
3. **Test Speed** - Use **Test & Rank** to measure response times and sort by performance
4. **Enable Mirrors** - Toggle your preferred mirrors on or off
5. **Sync** - Save configuration and sync package databases

All privileged operations use `pkexec` for secure authentication.

---

## Architecture

```
mirrorman/
├── src/
│   ├── main.py          # GTK4 interface & application logic
│   ├── mirror_manager.py # Mirror fetching, testing, persistence
│   ├── repo_config.py   # Repository configuration management
│   ├── sync_manager.py  # Package database synchronization
│   ├── pacman_util.py   # Pacman settings interface
│   └── utils.py         # Helper utilities
├── po/                  # Gettext translations
├── locale/              # Compiled translation files
├── assets/             # Icons and resources
├── PKGBUILD            # Arch package build script
└── README.md           # This file
```

---

## Roadmap

See [ROADMAP.md](ROADMAP.md) for the detailed feature roadmap and version planning.

---

## License

GPL-3 - See [LICENSE](LICENSE) for details.

---

## Support

Report bugs and request features on the [GitHub Issues](https://github.com/parchlinux/mirrorman/issues) page or post on the Parch Linux Forum.