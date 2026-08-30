#!/usr/bin/env bash
# Install the reco CLI with Cargo.
set -euo pipefail

if ! command -v cargo >/dev/null 2>&1; then
  echo "Reco AI necesita Rust. Instálalo en https://rustup.rs/" >&2
  exit 1
fi

echo "Instalando reco…"
cargo install --git https://github.com/DarioDGR12/Reco-AI --path crates/reco-cli --locked || \
  cargo install --git https://github.com/DarioDGR12/Reco-AI --path crates/reco-cli

echo
echo "Listo. Prueba:"
echo "  reco"
echo "  reco doctor"
echo "  reco desktop    # ventana (si compilaste scripts/build-desktop.sh)"
echo "  reco ai"
