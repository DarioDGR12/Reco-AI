# Changelog

## 0.3.1

- **`reco` is a command menu** — arrows move, enter runs `reco ai`, `reco models`, `reco desktop`, `reco chat`, `reco run`, `reco serve`, `reco doctor`, `reco setup`, `reco hw`, `reco config`. `q` quits. `--list` prints the old text home. `reco menu` is the same screen.
- **Installer rewritten** — linear script, always `cargo install --git … reco-cli` (package name, not `--path`; cargo 1.98 rejects `--git` + `--path`). Default is `reco` + `llama-cli`; `--desktop` is opt-in.
- **Tauri window actually installs** — `--desktop` installs WebKit/Node via apt when missing, builds with `--no-bundle` (AppImage/deb used to fail and skip copying `reco-desktop`), and `reco desktop` sets WebKit env vars so the window appears on NVIDIA + Wayland.
- **New-terminal snippet** — README documents the PATH block to paste in any new shell (`export PATH=…` + `source ~/.cargo/env` + `reco`). The installer writes it to `.bashrc` / `.profile` / `.zshrc` (and Fish when present).
- **llama-cli actually installs** — llama.cpp’s `/releases/latest` is `v0.4.0` with no binaries (the real builds are prerelease `bXXXX`). The installer now picks `llama-b*-bin-ubuntu-x64` from the release list, writes `~/.local/share/reco/llama/llama-cli` + a PATH wrapper, and `reco config set llama-cli`. Without this, chat stayed in demo mode.

## 0.3.0

- **One-command install** — `scripts/install.sh` (and `scripts/install.ps1` on Windows) installs Rust if needed, then `reco` and `llama-cli`. The Tauri window is `--desktop`.
- **`reco setup`** — first-run: llama.cpp, desktop window, keys.
- **Desktop is the default chat** — `reco run` / `reco chat` / `reco ai` open Prueba; `--tui` stays in the terminal.
- Product polish: one-command install as the documented path; packaging reserved for distro packages.

## 0.2.1

- **`reco api`** — generate named APIs (unique key, port, client kit) so other apps use models on this machine.
- Hub: one server, many keys, each key scoped to its model. `--lan` binds `0.0.0.0`.
- Clients: curl, Python, JS, Continue, Cursor, Open WebUI, LangChain, `.env`, OpenAPI.
- `reco serve` without a model starts the hub.

## 0.2.0

Product pass over the 0.1 CLI.

- `reco` with no args is a home dashboard (hardware, engine, disk, recent chats).
- `reco doctor`, `reco models` / `models rm`, `reco config get|unset`, shell completions.
- Catalog TUI: score panel, downloaded badge, `d` filter, `?` help.
- Prueba: scroll, new conversation (`Ctrl+N`), help overlay.
- `reco serve`: CORS, HTML docs, `stream: true`, persistent `sk-reco-…` key, `usage`.
- Downloads resume via HTTP Range; progress bar with speed.
- Failed model lookups suggest nearby repos.
- Desktop Prueba: sidebar of conversations.

## 0.1.0

Hardware detection, Hugging Face GGUF catalog, scoring, TUI, download, Prueba, serve, llama-cli / BYOK, packaging recipes.
