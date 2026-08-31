#!/usr/bin/env bash
# Build the Reco AI Tauri window and place it next to `reco`.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/crates/reco-desktop"

if ! command -v npm >/dev/null 2>&1; then
  echo "Reco desktop necesita Node/npm." >&2
  exit 1
fi

if [[ "$(uname -s)" == "Linux" ]] && ! pkg-config --exists webkit2gtk-4.1; then
  echo "Falta WebKitGTK. En Debian/Ubuntu:" >&2
  echo "  sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf" >&2
  exit 1
fi

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"

echo "Instalando CLI de Tauri…"
npm install

echo "Compilando reco-desktop…"
npm run tauri -- build

BIN="$CARGO_TARGET_DIR/release/reco-desktop"
if [[ ! -x "$BIN" ]]; then
  BIN="$ROOT/crates/reco-desktop/src-tauri/target/release/reco-desktop"
fi

echo
if [[ -x "$BIN" ]]; then
  mkdir -p "$HOME/.local/bin" "$HOME/.cargo/bin"
  cp -f "$BIN" "$HOME/.local/bin/reco-desktop"
  cp -f "$BIN" "$HOME/.cargo/bin/reco-desktop"
  chmod +x "$HOME/.local/bin/reco-desktop" "$HOME/.cargo/bin/reco-desktop"
  echo "Listo: $BIN"
  echo "Instalado en ~/.local/bin y ~/.cargo/bin"
  echo "  reco ai / reco run / reco desktop"
  echo "abren la ventana."
else
  echo "La compilación terminó; busca reco-desktop en target/release/." >&2
fi
