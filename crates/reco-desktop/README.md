# reco-desktop

Tauri 2 window for **Prueba**: hardware-aware GGUF catalog, download, and chat.

Same SQLite history (`~/.cache/reco/reco.db`) and the same `InferEngine` as `reco chat`.

This crate is **not** a workspace member: building it needs WebKitGTK (`libwebkit2gtk-4.1-dev` on Debian/Ubuntu). The CLI opens the Ratatui Prueba TUI when this binary is missing, or when you pass `--tui`.

```bash
# system deps (Debian/Ubuntu)
sudo apt install -y nodejs npm libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf pkg-config libgtk-3-dev

# from the repo root (puts the binary in target/release/ next to reco)
scripts/build-desktop.sh

# or
cd crates/reco-desktop
npm install
npm run tauri build
```

`reco desktop` opens the catalog. `reco ai` / `reco run` / `reco chat` spawn `reco-desktop` (or `reco-prueba`) from `RECO_DESKTOP`, next to the `reco` binary, `~/.cargo/bin`, or `PATH`:

```bash
reco desktop
reco desktop Qwen2.5-7B --demo
reco-desktop --repo Qwen/Qwen2.5-7B-Instruct-GGUF --file qwen2.5-7b-instruct-q4_k_m.gguf
reco-desktop --demo
```
