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
  <img alt="Status" src="https://img.shields.io/badge/status-v0.1%20catalog%20%2B%20recs-89dceb?style=flat-square" />
</p>

<p align="center">
  <a href="#quickstart">Quickstart</a> ·
  <a href="#what-works-today">What works today</a> ·
  <a href="#roadmap">Roadmap</a> ·
  <a href="#why-not-just-ollama">Why Reco</a>
</p>

---

## Demo

`reco ai` reads your CPU, RAM, and GPU, indexes GGUF repos from Hugging Face, and ranks a quant that actually fits (40% compatibility, 20% speed, 20% quality, 20% popularity).

<p align="center">
  <img src="docs/assets/demo-reco-ai.gif" alt="reco ai detecting hardware" width="820" />
</p>

Raw profile for scripts and later scoring:

<p align="center">
  <img src="docs/assets/demo-reco-hw.gif" alt="reco hw --json" width="820" />
</p>

---

## Coming soon

These are product previews, not shipping UI yet.

<table>
  <tr>
    <td align="center" width="33%">
      <img src="docs/assets/preview-tui.gif" alt="TUI catalog preview" /><br />
      <sub><b>TUI catalog</b> — Ratatui, arrows or click · <i>próximamente</i></sub>
    </td>
    <td align="center" width="33%">
      <img src="docs/assets/preview-prueba.gif" alt="Prueba chat preview" /><br />
      <sub><b>Prueba</b> — native Tauri chat + SQLite history · <i>próximamente</i></sub>
    </td>
    <td align="center" width="33%">
      <img src="docs/assets/preview-serve.gif" alt="reco serve preview" /><br />
      <sub><b>reco serve</b> — local API + <code>sk-...</code> key · <i>próximamente</i></sub>
    </td>
  </tr>
</table>

---

## Features

| | |
| --- | --- |
| **Hardware-aware picks** | CPU, RAM, NVIDIA (NVML / `nvidia-smi`), Linux DRM, Apple Metal. Score: 40% fit, 20% speed, 20% quality, 20% popularity. |
| **GGUF catalog** | Live Hugging Face index (`filter=gguf`), 12h cache, offline seed. |
| **One command** | `reco run <model>` downloads the right quant and opens **Prueba**. *Next.* |
| **Prueba** | Native desktop chat (Tauri, not Electron). History in SQLite. *Next.* |
| **BYOK** | Your OpenAI / Anthropic / … keys next to local GGUF. *Next.* |
| **`reco serve`** | Turn this PC into a local OpenAI-style server with a generated `sk-...` key. *Next.* |

---

## Why not just Ollama?

| | Reco AI | Ollama | Hugging Face |
| --- | --- | --- | --- |
| Run a model locally | yes (llama.cpp, *next*) | excellent | DIY |
| Catalog size | full GGUF index (*next*) | curated subset | huge |
| “Will this fit my 8 GB 4060?” | measured hardware + 40/20/20/20 | you guess the tag | you guess the quant |
| Native chat + history | Prueba (Tauri) (*next*) | separate apps | browser / spaces |
| Local OpenAI-style API | `reco serve` (*next*) | yes | no |

Reco is the missing middle: **HF’s catalog, Ollama’s “just run it”, plus a real hardware check**.

---

## Quickstart

Requires a [Rust](https://rustup.rs/) toolchain (1.83+).

```bash
git clone https://github.com/DarioDGR12/Reco-AI.git
cd Reco-AI
cargo install --path crates/reco-cli
```

```bash
reco ai                 # detect hardware + rank GGUF that fit
reco ai --json          # same, machine-readable
reco ai --refresh       # ignore cache, fetch Hugging Face again
reco ai --offline       # cache or bundled seed, no network
reco hw --json          # hardware profile only
reco run <modelo>       # download + open Prueba          (not yet)
reco serve <modelo>     # local server + API key          (not yet)
```

User-facing CLI text is in Spanish; crate names and this README are in English.

---

## What works today

- [x] Cargo workspace (`reco-core`, `reco-catalog`, `reco-cli`)
- [x] `reco ai` / `reco hw` — CPU, RAM, OS, GPU/VRAM best-effort
- [x] Hugging Face GGUF index + local cache + offline seed
- [x] Weighted recommendations (40 / 20 / 20 / 20)
- [ ] Ratatui catalog
- [ ] `reco run` + llama.cpp + Prueba
- [ ] Chat history (SQLite)
- [ ] BYOK
- [ ] `reco serve`
- [ ] AppImage / `.deb` / `.rpm` / AUR

GPU detection never requires the CUDA toolkit. NVIDIA uses NVML at runtime or `nvidia-smi`. No GPU → CPU backend, no crash.

---

## Roadmap

1. ~~Catalog~~ and ~~recommender~~ — in this tree  
2. **TUI** — Ratatui inside `reco-cli`  
3. **`reco run`** — download, pick quant, llama.cpp, launch Prueba  
4. **`reco serve`** — local HTTP API + `sk-...`  

---

## Crates

```
crates/reco-core      hardware, GGUF types, scoring
crates/reco-catalog   Hugging Face client + cache + seed
crates/reco-cli       `reco` binary (clap; Ratatui later)

later:
crates/reco-desktop   Tauri app “Prueba”
```

Regenerate terminal GIFs with [VHS](https://github.com/charmbracelet/vhs) (`docs/vhs/*.tape`) or:

```bash
python3 docs/vhs/render_gifs.py
```

---

## License

[MIT](LICENSE)
