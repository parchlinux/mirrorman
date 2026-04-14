# Changelog

All notable changes to MirrorMan will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [0.2.1] - 2026-04-15

### Added
- Full Persian (Farsi) translation support
- Gettext localization system
- Roadmap document for future features

### Changed
- README completely rewritten with better formatting
- Application ID changed to com.parchlinux.mirrorman
- PKGBUILD now uses git source instead of tarball
- Improved Persian translations for pacman options
- Iran Blackout feature translated to "اینترنت داخلی"

### Fixed
- Duplicate "Country" entry in po file causing msgfmt failure
- Multiple untranslated UI strings now properly wrapped in _()

## [0.2] - 2026-03-15

### Added
- Iran Blackout button for quick access to common Iranian mirror URLs
- Concurrent mirror testing for faster ranking
- Repository toggles for Chaotic-AUR, BlackArch, and ArchLinuxCN
- Pacman settings configuration window
- About dialog with changelog

### Changed
- Updated UI to follow Libadwaita design guidelines
- Improved error handling and user feedback

### Fixed
- Mirror parsing issues
- Configuration file handling

## [0.1] - 2026-01-15

### Added
- Initial release
- Mirror fetching from Arch Linux mirror status API
- Mirror filtering by country, protocol, and IP version
- Mirror testing and speed ranking
- Repository management (core, extra, multilib)
- System update functionality
- Mirrorlist saving with pkexec privilege escalation

---

## Translation Status

| Language | Status | Coverage |
|----------|--------|----------|
| English | Complete | 100% |
| Persian (fa) | Complete | ~95% |

To contribute translations, edit the `.po` files in the `po/` directory and compile them with `msgfmt`.

## Installation from Source

```bash
# Clone the repository
git clone https://git.xerocloud.ir/sohrab/mirrorman.git
cd mirrorman

# Compile translations
msgfmt -o locale/fa/LC_MESSAGES/mirrorman.mo po/fa.po

# Run the application
python src/main.py

# Or build package
makepkg -sic
```

## Reporting Issues

Please report bugs and feature requests on the GitHub issues page or post on the Parch Linux Forum.