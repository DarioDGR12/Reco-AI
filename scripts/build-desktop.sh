#!/usr/bin/env bash
# Build the Reco AI Tauri window and put reco-desktop on PATH.
# Does not build .deb/.rpm/AppImage — those often fail and used to skip the copy.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT/crates/reco-desktop"

if ! command -v npm >/dev/null 2>&1; then
  echo "Reco desktop necesita Node/npm." >&2
  echo "  sudo apt install -y nodejs npm" >&2
  exit 1
fi

if [[ "$(uname -s)" == "Linux" ]] && ! pkg-config --exists webkit2gtk-4.1; then
  echo "Falta WebKitGTK. En Pop!_OS / Ubuntu:" >&2
  echo "  sudo apt install -y libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf pkg-config libgtk-3-dev" >&2
  exit 1
fi

export CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$ROOT/target}"

echo "Instalando CLI de Tauri…"
npm install

echo "Compilando reco-desktop (sin empaquetar)…"
npm run tauri -- build --no-bundle --ci

BIN=""
for candidate in \
  "$CARGO_TARGET_DIR/release/reco-desktop" \
  "$ROOT/crates/reco-desktop/src-tauri/target/release/reco-desktop" \
  "$ROOT/target/release/reco-desktop"
do
  if [[ -x "$candidate" ]]; then
    BIN="$candidate"
    break
  fi
done

if [[ -z "$BIN" ]]; then
  echo "La compilación no dejó reco-desktop. Busca en target/release/." >&2
  exit 1
fi

mkdir -p "$HOME/.local/bin" "$HOME/.cargo/bin"
cp -f "$BIN" "$HOME/.local/bin/reco-desktop"
cp -f "$BIN" "$HOME/.cargo/bin/reco-desktop"
chmod +x "$HOME/.local/bin/reco-desktop" "$HOME/.cargo/bin/reco-desktop"

echo
echo "Listo: $BIN"
echo "Instalado en ~/.local/bin y ~/.cargo/bin"
echo "  reco desktop"
echo "abre la ventana."
