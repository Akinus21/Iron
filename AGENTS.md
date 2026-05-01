## Project: Iron Browser
Iron is a custom GTK4 web browser being built for the BlueAK Linux distribution, written in Rust. The name is a deliberate play on the metal browser naming tradition (Chrome, Titanium) and a nod to Rust — iron rusts.
Core tech stack:

## Language: Rust
GUI framework: GTK4 via gtk4 (gtk-rs) + relm4
Rendering engine: WebKit via webkit-rs (GTK4 flavor)
Reference/upstream: antoyo/titanium on GitHub (GTK3-era, needs significant modernization — treat as architectural reference, not a direct fork)

## Base Project Start:
This project starts by cloning titanium browser from:
https://github.com/antoyo/titanium.git
The dev repo is https://github.com/Akinus21/Iron.git

## Theming:

BlueAK uses the Noctalia shell/theme system
Noctalia targets GTK4 with adw-gtk-theme (libadwaita-compatible stylesheet) as its GTK layer
All custom UI widgets must use standard GTK4 widget classes and adwaita CSS conventions so Noctalia's color scheme cascades automatically
No hardcoded colors anywhere — use GTK theme variables throughout
Noctalia exposes a colors.json token set that regenerates on theme change; Iron should consume these tokens natively for seamless sync

## Context:

Gabriel is a Rust developer with an active homelab running Ubuntu on a Hetzner Server. He utilizes OpenCode inside of a Docker Container on this server to do much of his developing.
His devices (Surface laptop, Desktop PC, Handheld Gaming Console, and any future devices) run BlueAK which has a base image of a Fedora/atomic desktop from the Silverblue project.
Most other BlueAK projects are already in Rust, so consistency is a priority
Keyboard-driven UX (qutebrowser/Vimperator-style) is a desired design direction inherited from Titanium
The browser needs to feel native to the BlueAK/Noctalia desktop — not a port, not an afterthought

## Building

**IMPORTANT: Do NOT install Rust locally.** GitHub Actions handles all building automatically. The CI workflow builds the binary and reports any errors back to you.

If you need to verify code changes without building:
1. Review the code logic manually
2. Check for syntax errors by reading the files
3. Push to GitHub and wait for the build results

## Git Push Workflow

Since gh CLI is not authenticated, use SSH directly:

```bash
cd /home/opencode/projects/Iron
git add -A
git commit -m "<description>"
GIT_SSH_COMMAND="ssh -i /config/.ssh/github -o StrictHostKeyChecking=no" git push origin main
```

**IMPORTANT: Always push to GitHub after making and verifying changes.**

## Documentation Updates

**IMPORTANT: Update README.md when adding new features or changing existing features.**

The README should reflect:
- New commands added
- Changed command behavior
- Updated installation instructions
- New use cases or examples

## Phase 1 — Audit & Strip
Before writing new code, understand what Titanium gives you and what has to go.
Keep (as architectural reference):

- Keyboard-driven command/hint system design
- URL bar + mode switching concepts
- Configuration file approach

Rip out or replace entirely:

- GTK3 → GTK4 (widgets, signals, event model all changed)
- Any reliance on glib/gtk crates pre-gtk4-rs
- The WebKit bindings — titanium uses an old webkit2gtk that predates the GTK4 flavor
- Any hardcoded colors, themes, or CSS that isn't adwaita-compatible

## Phase 2 — New Foundation
Rebuild the skeleton with the modern stack before porting any features.
Cargo.toml targets:

- gtk4 (gtk-rs ecosystem)
- relm4 + relm4-components for the app architecture
- webkit6 (the GTK4 WebKit crate — formerly webkit2gtk-5.0)
- serde + toml or ron for config
- adw (libadwaita bindings) for window chrome

Structural goals:

- ApplicationWindow wrapping an adw::ToolbarView or adw::NavigationView
- A proper relm4 Component for the browser tab/webview
- A relm4 Component for the command bar (the vim-style input layer)
- Overlays (command bar, settings, find-in-page) live inside gtk4::Overlay, not modal windows

## Phase 3 — Noctalia Integration ✅ (Complete)
This is what makes Iron feel native rather than just functional.

### Token source
Noctalia stores theme files at:
`~/.config/noctalia/colorschemes/<Name>/<Name>.json`

Each file has `dark` and `light` variants with these keys:
`mPrimary`, `mOnPrimary`, `mSecondary`, `mOnSecondary`, `mTertiary`, `mOnTertiary`,
`mError`, `mOnError`, `mSurface`, `mOnSurface`, `mHover`, `mOnHover`,
`mSurfaceVariant`, `mOnSurfaceVariant`, `mOutline`, `mShadow`

### Architecture
- `src/noctalia.rs` → `ThemeManager` struct
  - `load()` — scans `~/.config/noctalia/colorschemes/`, picks first theme, parses JSON, generates CSS
  - `apply_webkit_css()` — injects a `UserStyleSheet` into the WebView's `UserContentManager` for cohesive page rendering
  - `start_watch()` — uses `gio::FileMonitor` on the theme directory; on `ChangesDoneHint` event, reloads the theme and re-applies the WebKit stylesheet (no polling)
- GTK chrome theming is handled automatically by Noctalia's `adw-gtk-theme` stylesheet — Iron just uses standard adwaita widget classes
- WebKit user stylesheet: `body, .content, main` get `background-color`/`color` from surface tokens; `code, pre, textarea, input, select` get surface_variant backgrounds

### Key decisions
- **No `notify` crate** — uses `gio::FileMonitor` (GLib-native, no threads, callback on main context)
- **GTK CSS provider dropped** — Noctalia's GTK template handles the GTK chrome cascade; Iron only needs the WebKit user stylesheet for web page cohesion
- **Dark mode** — detected via `GTK_THEME` env var containing "dark"; selects the appropriate variant from the theme JSON

## Phase 4 — Core Browser Features
Port and modernize Titanium's killer features in priority order:

- Hint/link navigation — the f key overlay that labels links for keyboard selection
- Command mode — vim-style :open, :tabopen, :back, :forward, etc.
- Keybinding layer — fully configurable, loaded from a config file at startup
- Tab management — adw::TabBar + adw::TabView give you this almost for free
- Download manager — WebKit's download signals wired to a sidebar or notification
- History & bookmarks — SQLite via rusqlite, keep it simple

## Phase 5 — BlueAK Polish
The things that make it feel like a first-class BlueAK citizen:

- Ship a .desktop file referencing the correct icon theme name
- Follow the BlueAK/Silverblue packaging conventions (likely a Flatpak or rpm-ostree layer)
- Register as a default browser handler via xdg-mime
- Respect the system gtk-application-prefer-dark-theme setting automatically (libadwaita handles this if you use adw::Application)