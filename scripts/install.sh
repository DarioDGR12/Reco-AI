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

# sudo needs a real TTY when this script is piped from curl.
apt_install() {
  have apt-get || return 1
  if [[ -r /dev/tty ]]; then
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -y "$@" </dev/tty
  else
    echo "    corre: sudo apt install -y $*" >&2
    return 1
  fi
}

# llama.cpp marks binary builds as prerelease. /releases/latest is v0.4.0
# (nightly-tag.txt only) and has no llama-cli — that left Reco in demo mode.
llama_download_url() {
  local os machine pattern json
  os="$(uname -s)"
  machine="$(uname -m)"
  case "$machine" in
    x86_64|amd64) machine=x64 ;;
    aarch64|arm64) machine=arm64 ;;
  esac
  case "$os" in
    Darwin) pattern="llama-b[0-9]+-bin-macos-${machine}\\.tar\\.gz" ;;
    Linux) pattern="llama-b[0-9]+-bin-ubuntu-${machine}\\.tar\\.gz" ;;
    *) return 1 ;;
  esac
  json="$(curl -fsSL -H 'Accept: application/vnd.github+json' -H 'User-Agent: Reco-AI-install' \
    'https://api.github.com/repos/ggml-org/llama.cpp/releases?per_page=20')" || return 1
  printf '%s' "$json" | grep -oE "https://[^\"[:space:]]+${pattern}" | head -1
}

write_llama_wrapper() {
  local dest="$1"
  local wrapper="$2"
  cat >"$wrapper" <<EOF
#!/bin/sh
DIR="$dest"
export LD_LIBRARY_PATH="\$DIR\${LD_LIBRARY_PATH:+:\$LD_LIBRARY_PATH}"
export DYLD_LIBRARY_PATH="\$DIR\${DYLD_LIBRARY_PATH:+:\$DYLD_LIBRARY_PATH}"
exec "\$DIR/llama-cli" "\$@"
EOF
  chmod +x "$wrapper"
}

install_llama_cli() {
  local dest="$HOME/.local/share/reco/llama"
  if [[ -x "$dest/llama-cli" ]]; then
    write_llama_wrapper "$dest" "$BIN_DIR/llama-cli"
    write_llama_wrapper "$dest" "$CARGO_BIN/llama-cli"
    "$CARGO_BIN/reco" config set llama-cli "$dest/llama-cli" >/dev/null || true
    ok "llama-cli ya está → $dest/llama-cli"
    return 0
  fi
  if have llama-cli; then
    local existing
    existing="$(command -v llama-cli)"
    "$CARGO_BIN/reco" config set llama-cli "$existing" >/dev/null || true
    ok "llama-cli ya está → $existing"
    return 0
  fi

  say "Descargando llama-cli (llama.cpp)…"
  local url tmp found
  url="$(llama_download_url || true)"
  if [[ -z "$url" ]]; then
    echo "    no encontré un tarball ubuntu/macos en los releases bXXXX" >&2
    return 1
  fi
  tmp="$(mktemp -d)"
  if ! curl -fL --retry 3 -o "$tmp/llama.tgz" "$url"; then
    rm -rf "$tmp"
    echo "    no pude bajar $url" >&2
    return 1
  fi
  tar -xzf "$tmp/llama.tgz" -C "$tmp"
  found="$(find "$tmp" -type f \( -name llama-cli -o -name llama-completion \) | head -1 || true)"
  if [[ -z "$found" ]]; then
    rm -rf "$tmp"
    echo "    el tarball no traía llama-cli" >&2
    return 1
  fi
  rm -rf "$dest"
  mkdir -p "$dest"
  cp -a "$(dirname "$found")/." "$dest/"
  if [[ ! -x "$dest/llama-cli" && -x "$dest/llama-completion" ]]; then
    ln -sfn "$dest/llama-completion" "$dest/llama-cli"
  fi
  chmod +x "$dest/llama-cli" 2>/dev/null || true
  write_llama_wrapper "$dest" "$BIN_DIR/llama-cli"
  write_llama_wrapper "$dest" "$CARGO_BIN/llama-cli"
  "$CARGO_BIN/reco" config set llama-cli "$dest/llama-cli" >/dev/null || true
  rm -rf "$tmp"
  if [[ ! -x "$dest/llama-cli" ]]; then
    echo "    no quedó $dest/llama-cli" >&2
    return 1
  fi
  ok "llama-cli → $dest/llama-cli"
  ok "PATH → $BIN_DIR/llama-cli"
}

install_desktop_window() {
  if [[ "$(uname -s)" == Linux ]] && have apt-get; then
    if ! have npm || ! pkg-config --exists webkit2gtk-4.1 2>/dev/null; then
      say "Instalando paquetes de Tauri (sudo)…"
      apt_install nodejs npm pkg-config build-essential libgtk-3-dev \
        libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev \
        patchelf libssl-dev \
        || apt_install nodejs npm pkg-config build-essential libgtk-3-dev \
          libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev \
          patchelf libssl-dev \
        || return 1
    fi
  fi
  have npm || {
    echo "    falta npm. sudo apt install -y nodejs npm" >&2
    return 1
  }
  mkdir -p "$(dirname "$SRC_DIR")"
  if [[ -d "$SRC_DIR/.git" ]]; then
    git -C "$SRC_DIR" fetch --depth 1 origin main
    git -C "$SRC_DIR" checkout -q -B main FETCH_HEAD || true
  else
    rm -rf "$SRC_DIR"
    git clone --depth 1 --branch main "$REPO" "$SRC_DIR"
  fi
  [[ -f "$SRC_DIR/scripts/build-desktop.sh" ]] || return 1
  (cd "$SRC_DIR" && bash scripts/build-desktop.sh) || return 1
  if [[ -x "$BIN_DIR/reco-desktop" || -x "$CARGO_BIN/reco-desktop" ]]; then
    ok "ventana → $BIN_DIR/reco-desktop"
    return 0
  fi
  echo "    build-desktop.sh no dejó reco-desktop en PATH" >&2
  return 1
}

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

if [[ "$WANT_LLAMA" == 1 ]]; then
  install_llama_cli || {
    echo >&2
    echo "    ✗ sin llama-cli: el chat queda en modo demo." >&2
    echo "    El endpoint /releases/latest de llama.cpp es v0.4.0 (sin binarios)." >&2
    echo "    Reintenta el instalador sin --no-llama." >&2
    exit 1
  }
fi

if [[ "$WANT_DESKTOP" == 1 ]]; then
  say "Ventana Tauri…"
  install_desktop_window || {
    echo >&2
    echo "    reco quedó instalado, pero la ventana NO." >&2
    echo "    En Pop!_OS / Ubuntu corre esto en la terminal (no con pipe):" >&2
    echo "      sudo apt install -y nodejs npm pkg-config build-essential libgtk-3-dev \\" >&2
    echo "        libwebkit2gtk-4.1-dev libayatana-appindicator3-dev librsvg2-dev patchelf libssl-dev" >&2
    echo "      curl -fsSL $REPO/raw/main/scripts/install.sh -o /tmp/reco-install.sh" >&2
    echo "      bash /tmp/reco-install.sh --desktop" >&2
    echo "    Luego:  reco desktop" >&2
  }
fi

path_line='export PATH="$HOME/.local/bin:$HOME/.cargo/bin:$PATH"'
for rc in "$HOME/.bashrc" "$HOME/.profile" "$HOME/.zshrc"; do
  touch "$rc"
  if ! grep -q '.local/bin:$HOME/.cargo/bin' "$rc" 2>/dev/null; then
    printf '\n# Reco AI — cualquier terminal nueva\n%s\n[ -f "$HOME/.cargo/env" ] && . "$HOME/.cargo/env"\n' "$path_line" >>"$rc"
    ok "PATH añadido a $rc"
  fi
done
if have fish; then
  fish_cfg="$HOME/.config/fish/config.fish"
  mkdir -p "$(dirname "$fish_cfg")"
  touch "$fish_cfg"
  if ! grep -q '.local/bin' "$fish_cfg" 2>/dev/null; then
    printf '\n# Reco AI\nset -gx PATH $HOME/.local/bin $HOME/.cargo/bin $PATH\n' >>"$fish_cfg"
    ok "PATH añadido a $fish_cfg"
  fi
fi

say "Listo"
echo
echo "  Cualquier terminal nueva (cópialo):"
echo "    export PATH=\"\$HOME/.local/bin:\$HOME/.cargo/bin:\$PATH\""
echo "    [ -f \"\$HOME/.cargo/env\" ] && . \"\$HOME/.cargo/env\""
echo "    reco"
echo
export PATH="$BIN_DIR:$CARGO_BIN:$PATH"
if have reco; then
  "$CARGO_BIN/reco" --list || true
  echo
  echo "  reco            # menú · flechas + enter"
  if have reco-desktop; then
    echo "  reco desktop    # ventana Tauri"
  fi
fi
