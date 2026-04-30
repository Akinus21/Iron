# Iron Browser

A GTK4 keyboard-driven web browser for the [BlueAK Linux](https://github.com/blueaklinux) distribution, written in Rust.

[![Build status](https://github.com/Akinus21/Iron/actions/workflows/build.yml/badge.svg?branch=main)](https://github.com/Akinus21/Iron/actions/workflows/build.yml)

Iron is a spiritual successor to the [Titanium](https://github.com/antoyo/titanium) browser, rebuilt from scratch on the modern GTK4 stack (GTK4, libadwaita, and WebKit 6.0). It is designed to feel native to the BlueAK/Noctalia desktop — not a port, not an afterthought.

## Installation

```bash
brew install Akinus21/homebrew-core/iron
```

## What's different from Titanium?

| Titanium (GTK3) | Iron (GTK4) |
|---|---|
| gtk 0.16, gdk, relm 0.24 | gtk4 0.11, libadwaita 0.9, relm4 (planned) |
| webkit2gtk 1.0 (GTK3 WebKit) | webkit6 0.6 (GTK4 WebKit) |
| mg/minigui for UI chrome | adw::ApplicationWindow + adw::ToolbarView |
| Hardcoded colors | Noctalia theme tokens (no hardcoded hex values) |

## Current status (Phase 2 — New Foundation)

- [x] `adw::ApplicationWindow` with `webkit6::WebView` rendering a live page
- [x] Noctalia `colors.json` token loading stub (no hardcoded colors from day one)
- [x] GitHub Actions CI with auto-release, homebrew tap, and build webhooks
- [ ] relm4 app architecture
- [ ] Tab management (`adw::TabBar` + `adw::TabView`)
- [ ] Command bar / vim-style input layer

## Planned features (inherited from Titanium)

- vim-like keybindings
- hint/link navigation (`f` key overlay)
- command mode (`:open`, `:tabopen`, `:back`, `:forward`, etc.)
- pop-up blocker with blacklist and whitelist
- user scripts and user stylesheets
- page search
- search engines
- download manager
- bookmarks (with completions)
- ad blocker

## Theming

Iron targets [Noctalia](https://github.com/Akinus21/noctalia), BlueAK's GTK4/adwaita theme system. All UI widgets use standard GTK4 widget classes and adwaita CSS conventions so Noctalia's color scheme cascades automatically. **No hardcoded hex values anywhere.**

On startup, Iron loads `~/.config/noctalia/colors.json`, maps the tokens to GTK CSS variables, and injects a CSS provider. A file watcher (inotify) picks up theme changes live.

## Building

GitHub Actions handles all building automatically. The CI workflow builds the binary on `ubuntu-latest`, creates versioned GitHub releases, and updates the homebrew tap.

If you need to verify code changes locally without building:
1. Review the code logic manually
2. Check for syntax errors by reading the files
3. Push to GitHub and wait for the CI results

## Inspiration

Iron is inspired by [Titanium](https://github.com/antoyo/titanium), [qutebrowser](https://www.qutebrowser.org/), and Vimperator.

## License

[MIT](LICENSE)
