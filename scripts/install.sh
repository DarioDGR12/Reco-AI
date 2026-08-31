#!/usr/bin/env bash
# Reco AI — un comando. Copia y pega:
#
#   curl -fsSL https://raw.githubusercontent.com/DarioDGR12/Reco-AI/main/scripts/install.sh | bash
#
# Instala Rust (si falta), reco y llama-cli. La ventana Tauri es opcional (--desktop).
set -euo pipefail

REPO="https://github.com/DarioDGR12/Reco-AI"
BIN_DIR="$HOME/.local/bin"
CARGO_BIN="$HOME/.cargo/bin"
SRC_DIR="$HOME/.cache/reco/src"
WANT_LLAMA=1
WANT_DESKTOP=0

usage() {
  cat <<'EOF'
Instala Reco AI (reco + llama-cli).

  curl -fsSL https://raw.githubusercontent.com/DarioDGR12/Reco-AI/main/scripts/install.sh | bash
  curl -fsSL .../install.sh | bash -s -- --desktop

  --desktop    también compila la ventana Tauri (tarda)
  --no-llama   no bajar llama-cli
  --help
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --desktop) WANT_DESKTOP=1 ;;
    --no-desktop) WANT_DESKTOP=0 ;;
    --cli)
      WANT_LLAMA=0
      WANT_DESKTOP=0
      ;;
    --no-llama) WANT_LLAMA=0 ;;
    --self-test)
      bash -n "${BASH_SOURCE[0]:-$0}"
      echo "ok"
      exit 0
      ;;
    -h|--help)
      usage
      exit 0
      ;;
    *)
      echo "opción desconocida: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
  shift
done

say() { printf '\n==> %s\n' "$*" >&2; }
ok() { printf '    ✓ %s\n' "$*" >&2; }
die() { printf '    ✗ %s\n' "$*" >&2; exit 1; }
have() { command -v "$1" >/dev/null 2>&1; }

mkdir -p "$BIN_DIR" "$CARGO_BIN"
export PATH="$BIN_DIR:$CARGO_BIN:$PATH"
# shellcheck disable=SC1091
[[ -f "$HOME/.cargo/env" ]] && . "$HOME/.cargo/env"

say "Reco AI — instalación"
if have rustc && have cargo; then
  ok "Rust $(rustc --version | awk '{print $2}')"
else
  say "Instalando Rust…"
  have curl || die "necesito curl"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
  export PATH="$BIN_DIR:$CARGO_BIN:$PATH"
  have cargo || die "rustup no dejó cargo en PATH"
  ok "Rust $(rustc --version | awk '{print $2}')"
fi

have git || die "necesito git. En Pop!_OS / Ubuntu: sudo apt install git"
have curl || die "necesito curl"

say "Instalando reco (cargo install --git)…"
# cargo 1.8x+ rejects combining git URL + local path flags.
# Select the workspace package by name. The binary is reco.
cargo install --git "$REPO" --branch main --locked --force reco-cli \
  || cargo install --git "$REPO" --branch main --force reco-cli

[[ -x "$CARGO_BIN/reco" ]] || die "cargo no instaló $CARGO_BIN/reco"
ln -sfn "$CARGO_BIN/reco" "$BIN_DIR/reco"
ok "reco → $CARGO_BIN/reco"

if [[ "$WANT_LLAMA" == 1 ]] && ! have llama-cli && [[ ! -x "$BIN_DIR/llama-cli" ]]; then
  say "Descargando llama-cli…"
  os="$(uname -s)"
  machine="$(uname -m)"
  case "$machine" in
    x86_64|amd64) machine=x64 ;;
    aarch64|arm64) machine=arm64 ;;
  esac
  json="$(curl -fsSL -H 'Accept: application/vnd.github+json' \
    https://api.github.com/repos/ggml-org/llama.cpp/releases/latest)" || json=""
  tag="$(printf '%s' "$json" | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)"
  asset=""
  if [[ -n "$tag" ]]; then
    case "$os" in
      Darwin) asset="llama-${tag}-bin-macos-${machine}.tar.gz" ;;
      Linux) asset="llama-${tag}-bin-ubuntu-${machine}.tar.gz" ;;
    esac
  fi
  if [[ -n "$asset" ]]; then
    tmp="$(mktemp -d)"
    dest="$HOME/.local/share/reco/llama"
    if curl -fsSL -o "$tmp/llama.tgz" \
      "https://github.com/ggml-org/llama.cpp/releases/download/${tag}/${asset}"; then
      tar -xzf "$tmp/llama.tgz" -C "$tmp"
      found="$(find "$tmp" -type f \( -name llama-cli -o -name llama-completion \) | head -1 || true)"
      if [[ -n "$found" ]]; then
        rm -rf "$dest"
        mkdir -p "$dest"
        cp -a "$(dirname "$found")/." "$dest/"
        chmod +x "$dest/$(basename "$found")"
        ln -sfn "$dest/$(basename "$found")" "$BIN_DIR/llama-cli"
        "$CARGO_BIN/reco" config set llama-cli "$BIN_DIR/llama-cli" >/dev/null || true
        ok "llama-cli $tag → $BIN_DIR/llama-cli"
      fi
    fi
    rm -rf "$tmp"
  fi
  have llama-cli || ok "sin llama-cli (puedes instalarlo luego). reco --demo sigue funcionando"
elif have llama-cli; then
  ok "llama-cli ya está"
fi

if [[ "$WANT_DESKTOP" == 1 ]]; then
  say "Ventana Tauri (opcional)…"
  if have git && have npm; then
    mkdir -p "$(dirname "$SRC_DIR")"
    if [[ -d "$SRC_DIR/.git" ]]; then
      git -C "$SRC_DIR" fetch --depth 1 origin main
      git -C "$SRC_DIR" checkout -q -B main FETCH_HEAD || true
    else
      rm -rf "$SRC_DIR"
      git clone --depth 1 --branch main "$REPO" "$SRC_DIR"
    fi
    if [[ -f "$SRC_DIR/scripts/build-desktop.sh" ]]; then
      (cd "$SRC_DIR" && bash scripts/build-desktop.sh) || true
    fi
  else
    ok "sin git/npm: salto la ventana. reco --tui funciona"
  fi
fi

path_line='export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"'
for rc in "$HOME/.bashrc" "$HOME/.profile" "$HOME/.zshrc"; do
  if [[ -f "$rc" ]] && ! grep -q '.local/bin:$HOME/.cargo/bin' "$rc" 2>/dev/null; then
    printf '\n# Reco AI\n%s\n[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"\n' "$path_line" >>"$rc"
    ok "PATH añadido a $rc"
  fi
done

say "Listo"
echo
echo "  En ESTA terminal:"
echo "    export PATH=\"\$HOME/.local/bin:\$HOME/.cargo/bin:\$PATH\""
echo "    source \"\$HOME/.cargo/env\""
echo "    reco"
echo
echo "  (o cierra la terminal, abre otra, y escribe: reco)"
echo
export PATH="$BIN_DIR:$CARGO_BIN:$PATH"
if have reco; then
  "$CARGO_BIN/reco" --list || true
  echo
  echo "  reco            # menú · flechas + enter"
fi
