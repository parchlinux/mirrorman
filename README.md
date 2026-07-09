<picture>
  <source media="(prefers-color-scheme: dark)" srcset="data/com.parchlinux.mirrorman.svg">
  <img alt="MirrorMan Logo" src="data/com.parchlinux.mirrorman.svg" width="128" height="128">
</picture>

# MirrorMan

**Pacman mirror and repository manager** for Parch Linux, built with GTK4 and Libadwaita in Rust.

MirrorMan fetches the latest Arch Linux mirrors, tests their speed, and generates an optimized mirrorlist. It also manages pacman repositories, edits pacman.conf, syncs packages, and cleans the package cache, all through a modern graphical interface.

## Features

- Fetch and filter mirrors by country, protocol, and IP version
- Test mirror speed concurrently with 50 worker threads
- Enable or disable individual mirrors and save the mirrorlist to `/etc/pacman.d/mirrorlist`
- Toggle standard repositories (core, extra, multilib) and third-party repos (Chaotic-AUR, BlackArch, ArchLinuxCN)
- Add custom repositories with name and server URL
- Edit pacman.conf options (ParallelDownloads, ILoveCandy, IgnorePkg, and more)
- Run system updates and clean package cache with PolicyKit privilege escalation
- Iran Blackout feature for quick access to domestic mirrors
- Persian (Farsi) translation support

## Installation

**Parch Linux** (from the world repo):

```
sudo pacman -S mirrorman
```

**Build from source:**

```bash
git clone https://github.com/parchlinux/mirrorman
cd mirrorman
makepkg -sic
```

## License

GNU General Public License v3.0
