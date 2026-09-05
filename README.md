<p align="center">
  <img src="docs/assets/banner.png" alt="Reco AI" width="920" />
</p>

<p align="center">
  <strong>Pick a local model that actually fits your machine — then chat with it.</strong><br />
  Hugging Face GGUF catalog + real hardware detection + one command.
</p>

<p align="center">
  <img alt="Rust" src="https://img.shields.io/badge/Rust-1.83+-DEA584?style=flat-square&logo=rust&logoColor=white" />
  <img alt="License MIT" src="https://img.shields.io/badge/license-MIT-a78bfa?style=flat-square" />
  <img alt="Windows" src="https://img.shields.io/badge/Windows-0078D6?style=flat-square&logo=windows&logoColor=white" />
  <img alt="macOS" src="https://img.shields.io/badge/macOS-000000?style=flat-square&logo=apple&logoColor=white" />
  <img alt="Linux" src="https://img.shields.io/badge/Linux-FCC624?style=flat-square&logo=linux&logoColor=black" />
  <img alt="Status" src="https://img.shields.io/badge/version-0.3-89dceb?style=flat-square" />
</p>

<p align="center">
  <a href="#quickstart">Quickstart</a> ·
  <a href="#commands">Commands</a> ·
  <a href="#why-not-just-ollama">Why Reco</a>
</p>

The CLI is Spanish. This README is English.

---

## Quickstart

One command. The installer installs Rust if needed, then `reco` and `llama-cli`. The Tauri window is opt-in (`--desktop`) so the first `curl | bash` stays short.

**macOS / Linux**

```bash
curl -fsSL https://raw.githubusercontent.com/DarioDGR12/Reco-AI/main/scripts/install.sh | bash
```

**Any new terminal** (bash or zsh) — paste this and `reco` is on PATH:

```bash
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"
[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"
reco
```

One line:

```bash
export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"; [ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"; reco
```

Fish:

```fish
set -gx PATH $HOME/.local/bin $HOME/.cargo/bin $PATH
reco
```

The installer also writes that PATH into `~/.bashrc`, `~/.profile`, and `~/.zshrc`. If a new terminal still says `command not found: reco`, paste the block above.

Flags (pass after `bash -s --`):

| Flag | Effect |
| --- | --- |
| `--desktop` | Also build the Tauri window (slow) |
| `--no-llama` | Skip `llama-cli` |
| `--cli` | CLI only (no llama-cli, no window) |

```bash
curl -fsSL https://raw.githubusercontent.com/DarioDGR12/Reco-AI/main/scripts/install.sh | bash -s -- --desktop
```

**Windows** (PowerShell)

```powershell
irm https://raw.githubusercontent.com/DarioDGR12/Reco-AI/main/scripts/install.ps1 | iex
```

Any new PowerShell:

```powershell
$env:Path = "$env:USERPROFILE\.local\bin;$env:USERPROFILE\.cargo\bin;$env:LOCALAPPDATA\Reco\bin;$env:Path"
reco
```

Then:

```bash
reco                         # menú · flechas + enter (reco ai, reco models, …)
reco --list                  # estado sin menú
reco setup                   # first-run: llama.cpp, window, keys
reco doctor                  # llama.cpp, keys, cache, window
reco desktop                 # Prueba: catalog + chat
reco ai                      # ranked catalog · enter opens the window
reco run Qwen2.5-7B          # download + Prueba
reco api create Qwen2.5-7B --name mi-app
reco api start               # this machine serves the APIs
reco api code mi-app --client python
```

Chat opens the **Prueba** window when it is installed. `--tui` stays in the terminal.

Distro packages (`.deb`, `.rpm`, AppImage, AUR): [packaging/README.md](packaging/README.md). From source: `cargo install --git https://github.com/DarioDGR12/Reco-AI --branch main reco-cli`.

---

## Demo

`reco ai` reads your hardware, indexes GGUF repos from Hugging Face, ranks a quant that fits (40/20/20/20), and opens a catalog. Enter downloads the GGUF and opens **Prueba**. `--tui` stays in the terminal.

<p align="center">
  <img src="docs/assets/demo-reco-ai.gif" alt="reco ai detecting hardware" width="820" />
</p>

`reco run` resolves the repo, picks the quant that fits, and shows the Hugging Face URL (`--dry-run` skips the download):

<p align="center">
  <img src="docs/assets/demo-reco-run.gif" alt="reco run dry-run" width="820" />
</p>

Raw hardware profile:

<p align="center">
  <img src="docs/assets/demo-reco-hw.gif" alt="reco hw --json" width="820" />
</p>

---

## Product

<table>
  <tr>
    <td align="center" width="33%">
      <img src="docs/assets/preview-tui.gif" alt="TUI catalog" /><br />
      <sub><b>Catálogo</b> — Ratatui, flechas + <code>/</code> buscar + panel de scores</sub>
    </td>
    <td align="center" width="33%">
      <img src="docs/assets/preview-prueba.gif" alt="Prueba chat" /><br />
      <sub><b>Prueba</b> — ventana Tauri (catálogo + chat) · historial SQLite · TUI con <code>--tui</code></sub>
    </td>
    <td align="center" width="33%">
      <img src="docs/assets/preview-serve.gif" alt="reco serve" /><br />
      <sub><b>reco serve</b> — API OpenAI-compat, CORS, <code>stream</code>, clave persistente</sub>
    </td>
  </tr>
</table>

---

## Features

| | |
| --- | --- |
| **Hardware-aware picks** | CPU, RAM, NVIDIA (NVML / `nvidia-smi`), Linux DRM, Apple Metal. Score: 40% fit, 20% speed, 20% quality, 20% popularity. |
| **GGUF catalog** | Live Hugging Face index (`filter=gguf`), 12h cache, offline seed. |
| **Download + chat** | `reco run` downloads the GGUF that fits and opens **Prueba**. |
| **Prueba** | Default chat is the Tauri window (same SQLite as the CLI). Catalog + model switcher via `reco desktop`. `--tui` keeps the Ratatui chat. |
| **`reco serve`** | Local OpenAI-style API + generated `sk-reco-...` key. |
| **llama.cpp** | Real tokens via `llama-cli` on PATH (or `reco config set llama-cli`). |
| **BYOK** | OpenAI / Anthropic keys next to local GGUF (`reco config`). |

---

## Why not just Ollama?

| | Reco AI | Ollama | Hugging Face |
| --- | --- | --- | --- |
| Run a model locally | download + `llama-cli` (or BYOK) | excellent | DIY |
| Catalog size | HF GGUF index (top download slice) | curated subset | huge |
| “Will this fit my 8 GB 4060?” | measured hardware + 40/20/20/20 | you guess the tag | you guess the quant |
| Native chat + history | Prueba window + SQLite (TUI fallback) | separate apps | browser / spaces |
| Local OpenAI-style API | `reco serve` | yes | no |

Reco is the missing middle: **HF’s catalog, Ollama’s “just run it”, plus a real hardware check**.

---

## Your machine is the server

`reco api create` generates a **named API**: unique key, OpenAI-compatible URL, and a client kit (Python, JS, curl, Continue, Cursor, Open WebUI, LangChain, OpenAPI, `.env`).

```bash
reco api create Llama-3.1-8B --name continue --lan
reco api start                 # hub: every key on one port
reco api code continue --client continue
```

Another app only needs:

- **Base URL** `http://<this-machine>:11434/v1`
- **API key** `sk-reco-<name>-…` (scoped to that model)
- **Model** the Hugging Face repo id (or the API slug)

`--lan` binds `0.0.0.0` and prints the LAN IP so a phone or another PC can use it. Each key only unlocks its model. `reco api rotate <name>` issues a new key.

Kits land in `~/.config/reco/clients/<slug>/`.

Cloud keys also read `OPENAI_API_KEY` / `ANTHROPIC_API_KEY`.

---

## Commands

| | |
| --- | --- |
| `reco` | Arrow menu: `reco ai`, `reco models`, `reco desktop`, … (`--list` = text home) |
| `reco setup` | First-run: llama.cpp, desktop window, keys |
| `reco ai` | Ranked catalog (TUI). Enter opens Prueba |
| `reco desktop [modelo]` | Tauri window: catalog, or chat if you pass a model |
| `reco run <modelo>` | Download the GGUF that fits, then open Prueba |
| `reco chat <modelo>` | Reopen the last conversation (`--tui` = terminal) |
| `reco api` | Generate / list / start / code custom APIs for other apps |
| `reco serve` | Hub of all APIs, or `reco serve <modelo>` for one |
| `reco models` | List (or `rm`) cached GGUFs |
| `reco doctor` | llama-cli, BYOK, catalog cache |
| `reco config` | `show` / `set` / `get` / `unset` |
| `reco completions bash` | Shell completions (`zsh`, `fish`) |
| `reco hw` | Hardware profile |

`--provider auto|llama|openai|anthropic|echo` on run / chat / serve. Downloads resume if interrupted.

GPU detection never needs the CUDA toolkit. NVIDIA uses NVML or `nvidia-smi`. No GPU → CPU, no crash.

---

## Next

Signed macOS / Windows installers. In-process llama.cpp. Linux packages: [packaging/README.md](packaging/README.md).

---

## Crates

```
crates/reco-core      hardware, GGUF types, scoring, chat store, InferEngine
crates/reco-catalog   Hugging Face client + cache + seed
crates/reco-cli       `reco` binary (clap + Ratatui)
crates/reco-desktop   Tauri app “Prueba” (not a workspace member; needs WebKitGTK)
                      build: scripts/build-desktop.sh
```

Regenerate terminal GIFs with [VHS](https://github.com/charmbracelet/vhs) (`docs/vhs/*.tape`) or:

```bash
python3 docs/vhs/render_gifs.py
```

---

## License

[MIT](LICENSE)
