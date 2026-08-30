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
  <img alt="Status" src="https://img.shields.io/badge/status-v0.1%20llama--cli%20%2B%20BYOK-89dceb?style=flat-square" />
</p>

<p align="center">
  <a href="#quickstart">Quickstart</a> ·
  <a href="#what-works-today">What works today</a> ·
  <a href="#roadmap">Roadmap</a> ·
  <a href="#why-not-just-ollama">Why Reco</a>
</p>

---

## Demo

`reco ai` reads your hardware, indexes GGUF repos from Hugging Face, ranks a quant that fits (40/20/20/20), and opens a Ratatui catalog. Enter downloads the file with `reco run`.

<p align="center">
  <img src="docs/assets/demo-reco-ai.gif" alt="reco ai detecting hardware" width="820" />
</p>

`reco run` resolves the repo, picks the quant that fits, and shows the Hugging Face URL (add `--dry-run` to skip the download):

<p align="center">
  <img src="docs/assets/demo-reco-run.gif" alt="reco run dry-run" width="820" />
</p>

Raw hardware profile:

<p align="center">
  <img src="docs/assets/demo-reco-hw.gif" alt="reco hw --json" width="820" />
</p>

---

## Coming soon

These are product previews, not shipping UI yet.

<table>
  <tr>
    <td align="center" width="33%">
      <img src="docs/assets/preview-tui.gif" alt="TUI catalog" /><br />
      <sub><b>TUI catalog</b> — Ratatui, flechas + <code>/</code> buscar · <i>listo</i></sub>
    </td>
    <td align="center" width="33%">
      <img src="docs/assets/preview-prueba.gif" alt="Prueba chat" /><br />
      <sub><b>Prueba</b> — chat TUI + SQLite · Tauri window en <code>crates/reco-desktop</code></sub>
    </td>
    <td align="center" width="33%">
      <img src="docs/assets/preview-serve.gif" alt="reco serve" /><br />
      <sub><b>reco serve</b> — API local + <code>sk-reco-...</code> · llama-cli / BYOK / echo</sub>
    </td>
  </tr>
</table>

---

## Features

| | |
| --- | --- |
| **Hardware-aware picks** | CPU, RAM, NVIDIA (NVML / `nvidia-smi`), Linux DRM, Apple Metal. Score: 40% fit, 20% speed, 20% quality, 20% popularity. |
| **GGUF catalog** | Live Hugging Face index (`filter=gguf`), 12h cache, offline seed. |
| **One command** | `reco run` downloads the GGUF and opens **Prueba**. |
| **Prueba** | Terminal chat + SQLite history. Optional Tauri window (`reco-desktop`) over the same store. |
| **`reco serve`** | Local OpenAI-style API + generated `sk-reco-...` key. |
| **llama.cpp** | Real tokens via `llama-cli` on PATH (or `reco config set llama-cli`). |
| **BYOK** | Your OpenAI / Anthropic keys next to local GGUF (`reco config`). |

---

## Why not just Ollama?

| | Reco AI | Ollama | Hugging Face |
| --- | --- | --- | --- |
| Run a model locally | download + `llama-cli` (or BYOK) | excellent | DIY |
| Catalog size | HF GGUF index (top download slice) | curated subset | huge |
| “Will this fit my 8 GB 4060?” | measured hardware + 40/20/20/20 | you guess the tag | you guess the quant |
| Native chat + history | Prueba TUI + SQLite (+ Tauri) | separate apps | browser / spaces |
| Local OpenAI-style API | `reco serve` | yes | no |

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
reco ai                      # TUI: ↑↓, enter descarga, / busca
reco ai --list               # same ranking, plain text
reco ai --json
reco run Llama-3.1-8B        # descarga GGUF + Prueba (llama-cli / BYOK / echo)
reco run Llama-3.1-8B --demo # Prueba con EchoEngine
reco run org/repo --dry-run
reco chat Llama-3.1-8B --offline --fixture rtx4060
reco serve Llama-3.1-8B --demo --offline --fixture rtx4060
reco config set openai-key sk-...
reco config show
reco hw --json
```

Put `llama-cli` on `PATH` (or `reco config set llama-cli /ruta`) after installing [llama.cpp](https://github.com/ggml-org/llama.cpp). Keys can also come from `OPENAI_API_KEY` / `ANTHROPIC_API_KEY`.

User-facing CLI text is in Spanish; crate names and this README are in English.

---

## What works today

- [x] Cargo workspace (`reco-core`, `reco-catalog`, `reco-cli`)
- [x] `reco ai` / `reco hw` — CPU, RAM, OS, GPU/VRAM best-effort
- [x] Hugging Face GGUF index + local cache + offline seed
- [x] Weighted recommendations (40 / 20 / 20 / 20)
- [x] Ratatui catalog (`reco ai`, `--list` to skip)
- [x] `reco run` — resolve spec + download GGUF to the local cache
- [x] **Prueba** TUI chat + SQLite history (`reco chat` to reopen)
- [x] `reco serve` — `/v1/chat/completions` + `sk-reco-...`
- [x] llama.cpp tokens via `llama-cli` (`InferEngine`, `--provider llama|auto`)
- [x] Tauri desktop window for Prueba (`crates/reco-desktop`, optional)
- [x] BYOK OpenAI / Anthropic (`reco config`, `--provider openai|anthropic`)
- [x] AppImage / `.deb` / `.rpm` / AUR metadata (`packaging/`)

GPU detection never requires the CUDA toolkit. NVIDIA uses NVML at runtime or `nvidia-smi`. No GPU → CPU backend, no crash.

---

## Roadmap

1. ~~Catalog, recommender, TUI, download, Prueba TUI, serve demo~~  
2. ~~llama.cpp via `llama-cli`, BYOK, Tauri Prueba, Linux packaging recipes~~  
3. In-process llama.cpp (optional feature), CUDA/Metal GPU offload tuning  
4. Signed macOS / Windows installers  

---

## Crates

```
crates/reco-core      hardware, GGUF types, scoring, chat store, InferEngine
crates/reco-catalog   Hugging Face client + cache + seed
crates/reco-cli       `reco` binary (clap + Ratatui)
crates/reco-desktop   Tauri app “Prueba” (not a workspace member; needs WebKitGTK)

Linux packages: see [packaging/README.md](packaging/README.md).
```

Regenerate terminal GIFs with [VHS](https://github.com/charmbracelet/vhs) (`docs/vhs/*.tape`) or:

```bash
python3 docs/vhs/render_gifs.py
```

---

## License

[MIT](LICENSE)
