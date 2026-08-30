# Packaging Reco AI

Linux packages wrap the `reco` CLI (`cargo build --release -p reco-cli`).

| Format | How |
| --- | --- |
| **`.deb`** | `cargo install cargo-deb && cargo deb -p reco-cli` |
| **`.rpm`** | `cargo install cargo-generate-rpm && cargo build --release -p reco-cli && cargo generate-rpm -p reco-cli` |
| **AppImage** | `packaging/appimage/build.sh` (needs a release `reco` binary) |
| **AUR** | `packaging/aur/PKGBUILD` |

The Tauri window (`crates/reco-desktop`) can also emit `.deb` / `.rpm` / AppImage via `npm run tauri build` when WebKitGTK is installed.

macOS / Windows installers are `cargo install --path crates/reco-cli` today; Tauri bundle targets can be added later.
