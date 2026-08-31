# Packaging Reco AI

Humans should install with [`scripts/install.sh`](../scripts/install.sh) (see the [Quickstart](../README.md#quickstart)); this directory is for distro packages.

Linux packages wrap the `reco` CLI (`cargo build --release -p reco-cli`).

| Format | How |
| --- | --- |
| **`.deb`** | `cargo install cargo-deb && cargo deb -p reco-cli` |
| **`.rpm`** | `cargo install cargo-generate-rpm && cargo build --release -p reco-cli && cargo generate-rpm -p reco-cli` |
| **AppImage** | `packaging/appimage/build.sh` (needs a release `reco` binary) |
| **AUR** | `packaging/aur/PKGBUILD` |

The Tauri window (`crates/reco-desktop`) can also emit `.deb` / `.rpm` / AppImage via `npm run tauri build` when WebKitGTK is installed.

macOS / Windows: use the [Quickstart](../README.md#quickstart) installer. Tauri bundle targets can be added later.
