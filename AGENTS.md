# Iron – Rust‑based Chromium‑GTK Browser  
**Agent Instructions**

---

## 📖 Overview  

**Iron** is a lightweight web browser built in Rust that embeds the Chromium Embedded Framework (CEF) inside a GTK4 UI. It provides:

- Full‑screen and windowed browsing with CEF rendering.  
- Smart‑card (CAC) support via PKCS#11.  
- Persistent session handling (cookies, cache, history).  
- Search‑engine registry, fuzzy command palette, hint‑mode navigation, and download manager.  
- Customizable key‑bindings and theming (via `noctalia`).  

All heavy lifting (CEF integration, GTK4 UI) is handled in Rust; the binary is distributed via a Homebrew tap (`Akinus21/homebrew-tap`).

---

## 🏗️ Build System  

| Item | Value |
|------|-------|
| **Language** | Rust |
| **Build command** | `cargo build --release` |
| **Resulting binary** | `target/release/iron` |
| **Version source** | `Cargo.toml` |
| **Homebrew tap** | `Akinus21/homebrew-tap` |
| **CI** | GitHub Actions (no local Rust install required) |

**Important:** Do **not** install Rust locally. Let the CI pipeline compile the binary and report any errors. If you need a quick syntax check, just run `cargo check` locally (it only needs the Rust toolchain, not the full CEF build).

---

## 🔐 Authentication & Secrets  

| Secret | Location / How to set |
|--------|----------------------|
| **SSH key for Git** | `/home/akinus/.ssh/github` (used via `GIT_SSH_COMMAND`) |
| **Project secrets** | `/home/akinus/dockge-stacks/dev-stack/.secrets` |
| **GitHub webhook secret** | Set with `gh secret set WEBHOOK_SECRET --body "<value>"` (currently **NOT FOUND**) |
| **Webhook URL** | `https://webhook.akinus21.com/webhook/iron-build` (set with `gh secret set WEBHOOK_URL`) |
| **Webhook endpoint (runtime)** | `https://webhook.akinus21.com/webhook/iron-build` |

> **Note:** The CI workflow reads the above secrets automatically. Ensure they exist before triggering a build.

---

## 📦 Release & Homebrew  

When a new version is tagged (e.g., `v1.2.3`), the CI will:

1. Build `iron` with `cargo build --release`.  
2. Upload the binary to the Homebrew tap (`Akinus21/homebrew-tap`).  
3. Publish a GitHub Release containing the checksum and release notes.

**Manual Homebrew update (if needed):**

```bash
brew tap Akinus21/homebrew-tap
brew install iron
brew upgrade iron   # after a new release is published
```

---

## 🔄 Git Push Workflow  

Because the `gh` CLI is not authenticated on the runner, push via SSH directly:

```bash
cd /home/akinus/dockge-stacks/dev-stack/projects/Iron
git add -A
git commit -m "<description>"
GIT_SSH_COMMAND="ssh -i /home/akinus/.ssh/github -o StrictHostKeyChecking=no" \
    git push origin main
```

> **Always** push after making changes; the CI will automatically build and report status.

---

## 📂 Project Structure  

```
Iron/
├── Cargo.toml                 # Crate metadata, version, dependencies
├── build.rs                   # Build script – sets up CEF library paths, copies resources
├── AGENTS.md                  # ← This file
├── src/
│   ├── main.rs                # Entry point – wires modules together
│   ├── cac.rs                 # CAC / smart‑card status helper (PKCS#11)
│   ├── cef_browser.rs        # GTK4 widget wrapper around CEF
│   ├── cef_init.rs            # Global CEF lifecycle & config
│   ├── command.rs             # `Command` enum – all user‑triggered actions
│   ├── config.rs              # Serializable config (key bindings, modes, etc.)
│   ├── download.rs            # Download manager & notification handling
│   ├── find.rs                # UI overlay for “find in page”
│   ├── fuzzy.rs               # Lightweight fuzzy matching for command/history
│   ├── hints.rs               # JavaScript hint overlay injected into pages
│   ├── history.rs             # SQLite‑backed browsing history manager
│   ├── noctalia.rs            # Theme manager (hex → rgba conversion, CSS handling)
│   ├── search.rs              # Search‑engine registry & URL builder
│   ├── session.rs             # Persistent session (cache, cookies, incognito)
│   ├── settings.rs            # Settings overlay UI
│   └── ... (additional modules) 
├── resources/                 # CEF binaries & assets (populated by CI)
└── .github/
    └── workflows/ci.yml       # GitHub Actions CI definition
```

### Key Files Explained  

| File | Purpose |
|------|---------|
| **build.rs** | Runs before compilation; reads `CEF_TRACK` & `CEF_DIR` env vars, configures linker flags, copies CEF resources into the output directory. |
| **src/cef_init.rs** | Holds global `CEF_INITIALIZED` flag, reference counting, and `CefConfig` struct (track, cache path, log level, etc.). |
| **src/cef_browser.rs** | Provides `CefBrowserWrapper` – a GTK4 `Widget` that embeds the CEF browser, exposing URL, title, and loading state. |
| **src/config.rs** | Defines `Config`, `KeyBinding`, `Mode`, and `CefTrack` (stable/nightly). Serialized to/from `~/.config/iron/config.toml`. |
| **src/command.rs** | Central enum for all commands the UI can invoke (open URL, navigation, settings, CAC status, search engine management, etc.). |
| **src/search.rs** | `SearchEngine` struct + registry handling; builds final URLs from query strings. |
| **src/hints.rs** | JavaScript module injected into pages to render hint overlays for keyboard navigation. |
| **src/fuzzy.rs** | Scoring algorithm used by the command palette and history filter. |
| **src/history.rs** | SQLite manager storing `HistoryItem`s under `~/.local/share/iron/history.sqlite`. |
| **src/session.rs** | Manages per‑profile data directories, incognito mode, and site‑data clearing. |
| **src/noctalia.rs** | Theme utilities – hex‑to‑rgba conversion, CSS provider setup, and `ThemeManager` struct. |
| **src/settings.rs** | Builds the full‑window settings overlay (key‑binding editor, theme picker, etc.). |
| **src/find.rs** | UI overlay for “find in page” functionality, with entry widget and match counter. |
| **src/download.rs** | Simple download manager exposing progress, notifications, and error handling. |

---

## 🛠️ Development Conventions  

| Area | Convention |
|------|------------|
| **Code style** | Follow `rustfmt` defaults. Use `cargo clippy` for linting. |
| **Error handling** | Propagate errors with `Result<T, anyhow::Error>` where appropriate; UI‑level errors should surface as GTK notifications. |
| **Logging** | Use `log` crate (`env_logger` in CI). Respect `CefConfig.log_level`. |
| **Configuration** | Store user‑editable settings in `~/.config/iron/config.toml`. Keep defaults in `Config::default()`. |
| **Secrets** | Never commit `.secrets` or any private key. Access them via environment variables injected by CI. |
| **Testing** | Unit tests live in `src/*_test.rs` modules; run with `cargo test`. UI integration tests are out‑of‑scope for CI. |
| **Documentation** | Public structs/enums should have `///` doc comments. Keep `README.md` up‑to‑date with usage examples and install instructions. |
| **Version bump** | Update `Cargo.toml` version **before** tagging a release. CI will read this version for Homebrew packaging. |
| **Branch policy** | All work happens on feature branches; merge to `main` via PRs. CI must pass before merge. |
| **Commit messages** | Use conventional commits (`feat:`, `fix:`, `docs:`, `chore:`). |

---

## 📡 Webhook Integration  

The project is wired to a custom webhook endpoint that triggers a remote build pipeline:

- **Endpoint:** `https://webhook.akinus21.com/webhook/iron-build`  
- **Secret:** `WEBHOOK_SECRET` (must be set in the repo’s GitHub secrets).  

When a push to `main` occurs, GitHub sends a POST to the above URL. The remote service pulls the repo, runs the CI build, and reports status back to the GitHub Checks API.

---

## 📦 Release Checklist  

1. **Update version** in `Cargo.toml`.  
2. **Run local tests:** `cargo test`.  
3. **Commit & push** (use SSH workflow).  
4. Verify **GitHub Actions** succeeded.  
5. **Tag** the commit: `git tag -a vX.Y.Z -m "Release vX.Y.Z"` then push tags.  
6. CI will automatically publish the Homebrew formula update.  
7. **Update README** with any new command‑line flags or UI changes.  

---

## 🙋‍♀️ Support & Contributions  

- **Issues:** Open on the GitHub repo (`Akinus21/Iron`).  
- **Pull Requests:** Follow the branch policy and include a short description of the change.  
- **Contact:** For secret‑related problems, reach out to the repository owner (Akinus21) via the internal Slack channel `#iron-dev`.  

--- 

*End of AGENTS.md*