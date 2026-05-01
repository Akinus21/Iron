# Iron Browser

A GTK4 keyboard-driven web browser for the [BlueAK Linux](https://github.com/blueaklinux) distribution, written in Rust.

[![Build status](https://github.com/Akinus21/Iron/actions/workflows/build.yml/badge.svg?branch=main)](https://github.com/Akinus21/Iron/actions/workflows/build.yml)

Iron is a spiritual successor to the [Titanium](https://github.com/antoyo/titanium) browser, rebuilt from scratch on the modern GTK4 stack (GTK4, libadwaita, and WebKit 6.0). It is designed to feel native to the BlueAK/Noctalia desktop — not a port, not an afterthought.

## Installation

```bash
brew install Akinus21/homebrew-tap/iron
```

### BlueAK / Fedora SilverBlue (atomic)

1. Install the binary (e.g. via Homebrew or from the GitHub release)
2. Copy the `.desktop` file into your writable app directory:
   ```bash
   mkdir -p ~/.local/share/applications
   cp /path/to/org.blueak.iron.desktop ~/.local/share/applications/
   ```
3. Register Iron as the system default browser:
   ```bash
   xdg-settings set default-url-scheme-handler https org.blueak.iron.desktop
   ```
   (If you get "exited with code 2", the `.desktop` file isn't visible to xdg — make step 2 was done and run `update-desktop-database ~/.local/share/applications/`)
4. You can now open links with `iron https://example.com` or via xdg-open

## What's different from Titanium?

| Titanium (GTK3) | Iron (GTK4) |
|---|---|
| gtk 0.16, gdk, relm 0.24 | gtk4 0.11, libadwaita 0.9, relm4 (planned) |
| webkit2gtk 1.0 (GTK3 WebKit) | webkit6 0.6 (GTK4 WebKit) |
| mg/minigui for UI chrome | adw::ApplicationWindow + adw::ToolbarView |
| Hardcoded colors | Noctalia theme tokens (no hardcoded hex values) |

## Current status

### Done
- [x] `adw::ApplicationWindow` with `webkit6::WebView` rendering live pages
- [x] Noctalia theme integration (token loading, CSS generation, file-watch live reload)
- [x] WebKit CSS injection (form controls themed, dark/light `color-scheme` hint)
- [x] Keyboard-driven hint mode (`f` key, qutebrowser-style link navigation)
- [x] Full-window command overlay (lists keybindings, commands, themed)
- [x] Keybinding config layer (TOML file at `~/.config/iron/config.toml`)
- [x] Settings window with keybinding editor (add/remove bindings, protected defaults)
- [x] New-window-open command (`:new-window-open URL` / `:nwo URL`)
- [x] xdg-open / default browser handling
- [x] CAC / smart-card access — status check command (`:cac-status`), PKCS#11 readiness detection
- [x] Search engines (`:search`, `:search-add`, `:search-del`, default engine in config)
- [x] Find-in-page (`/` or `:find QUERY` via WebKit `FindController`)
- [x] Duplicate window (`:duplicate` / `Ctrl+d`) — clones current window with same page
- [x] Copy address to clipboard (`:copy-address` / `:cpa`) — copies current URL
- [x] GitHub Actions CI with auto-release, homebrew tap, and build webhooks

### ToDo
- [x] Download manager — intercept WebKit downloads, auto-save to XDG_DOWNLOAD_DIR, uniquify filenames, Gio desktop notification, `xdg-open` folder action
- [x] Session management — persistent cookies (SQLite `cookies.sqlite`), third-party cookie blocking, WebKit-native credential storage via libsecret/keyring, `:clear-site-data` / `:csd`, `:clear-cookies` / `:cc`, incognito-ready `NetworkSession`
- [ ] History (SQLite via `rusqlite`, `:history` command)
- [ ] Bookmarks (SQLite, `:bookmark` command, completions in command overlay)
- [ ] Ad blocker (content-blocking rules)
- [ ] User scripts & user stylesheets
- [ ] Pop-up blocker (blacklist/whitelist)
- [ ] relm4 app architecture
- [ ] Zoom controls (`Ctrl+`/`Ctrl-`/`Ctrl0`)
- [ ] Fullscreen mode (`F11`)
- [ ] Private browsing / incognito mode
- [ ] Flatpak packaging

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
