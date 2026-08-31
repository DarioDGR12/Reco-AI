# Changelog

## 0.3.1

- **`reco` is a command menu** — arrows move, enter runs `reco ai`, `reco models`, `reco desktop`, `reco chat`, `reco run`, `reco serve`, `reco doctor`, `reco setup`, `reco hw`, `reco config`. `q` quits. `--list` prints the old text home. `reco menu` is the same screen.
- **Install fix** — `curl | bash` no longer concatenates log lines into the cargo `--path` (that broke `reco-cli` on Pop!_OS). CLI installs with `cargo install --git`.

## 0.3.0

- **One-command install** — `scripts/install.sh` (and `scripts/install.ps1` on Windows) installs Rust if needed, then `reco`, `llama-cli`, and the Tauri window when possible. Flags: `--no-desktop`, `--no-llama`, `--cli`.
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
