# Roadmap

Planned work for MirrorMan. Ordering is indicative, not a commitment.

## v0.5.x — Stabilization

- [x] Hardened `mirrorman-helper` (polkit per-method authorization, strict allow-lists, dedicated BlackArch strap operation)
- [x] Multi-country mirror selection + country filter fixes
- [x] Functional fixes for repository toggle threading
- [ ] Automated D-Bus integration tests against a real system bus
- [ ] Optional `pkexec`-free fallback removal once the helper ships everywhere

## v0.6 — Power features

- [ ] Mirror ranking by historical latency/score trends (not just one-shot measurement)
- [ ] Schedule-based auto refresh (`mirrorman-refresh.timer` presets in the GUI)
- [ ] Import/export of full mirrorlist profiles as shareable files
- [ ] Per-repo SigLevel and `Include` editing in the custom repository dialog
- [ ] Dark/light accent theming for generated mirrorlists
- [ ] Notifications on failed sync with actionable retry

## v0.7 — Repository management

- [ ] In-app custom repository manager (add/remove/edit arbitrary repos)
- [ ] BlackArch/Chaotic-AUR/ArchLinuxCN package search without leaving the app
- [ ] Partial mirror support with `--ignore` lists
- [ ] Rollback snapshots of `pacman.conf` before destructive edits

## Backlog — Nice-to-haves

- [ ] Wayland screencast/portal integration for sharing config
- [ ] Flatpak build with `--socket=system-bus` policy
- [ ] More translations (ar, ru, tr)
- [ ] CLI: `mirrorman-cli status` for cron-friendly monitoring
