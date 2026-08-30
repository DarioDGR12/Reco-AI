# reco-desktop

Tauri 2 window for **Prueba**. Same SQLite history (`~/.cache/reco/reco.db`) and the same `InferEngine` as `reco chat`.

This crate is **not** a workspace member: building it needs WebKitGTK (`libwebkit2gtk-4.1-dev` on Debian/Ubuntu). The CLI still opens the Ratatui Prueba TUI when this binary is missing.

```bash
# system deps (Debian/Ubuntu)
sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev

cd crates/reco-desktop
npm install
npm run tauri build
```

`reco run` / `reco chat` spawn `reco-desktop` (or `reco-prueba`) from `PATH` or next to the `reco` binary when present:

```bash
reco-desktop --repo Qwen/Qwen2.5-7B-Instruct-GGUF --file qwen2.5-7b-instruct-q4_k_m.gguf
reco-desktop --demo
```
