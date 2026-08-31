#!/usr/bin/env bash
# Reco AI — instalación completa (CLI + llama-cli + ventana Tauri).
#
#   curl -fsSL https://raw.githubusercontent.com/DarioDGR12/Reco-AI/main/scripts/install.sh | bash
#   curl -fsSL .../install.sh | bash -s -- --cli
#   ./scripts/install.sh --no-desktop
#
# Variables: RECO_REPO_URL  RECO_BIN_DIR  RECO_SRC_DIR  RECO_LLAMA_VARIANT
set -euo pipefail

REPO_URL="${RECO_REPO_URL:-https://github.com/DarioDGR12/Reco-AI}"
BIN_DIR="${RECO_BIN_DIR:-$HOME/.local/bin}"
SRC_DIR="${RECO_SRC_DIR:-$HOME/.cache/reco/src}"
WANT_LLAMA=1
WANT_DESKTOP=1
CLI_ONLY=0
SELF_TEST=0

usage() {
  cat <<'EOF'
Reco AI — instala todo lo necesario para chatear y servir modelos locales.

Uso: install.sh [opciones]

  --cli           solo el binario reco (sin llama-cli ni ventana)
  --no-llama      no descargar llama-cli
  --no-desktop    no compilar la ventana Tauri
  --help          esta ayuda

Un comando:
  curl -fsSL https://raw.githubusercontent.com/DarioDGR12/Reco-AI/main/scripts/install.sh | bash
EOF
}

while [[ $# -gt 0 ]]; do
  case "$1" in
    --cli)
      CLI_ONLY=1
      WANT_LLAMA=0
      WANT_DESKTOP=0
      ;;
    --no-llama) WANT_LLAMA=0 ;;
    --no-desktop) WANT_DESKTOP=0 ;;
    --self-test) SELF_TEST=1 ;;
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

# Logs always go to stderr so `ROOT="$(ensure_source)"` never captures them.
log() { printf '\n\033[1m==>\033[0m %s\n' "$*" >&2; }
ok() { printf '    \033[32m✓\033[0m %s\n' "$*" >&2; }
warn() { printf '    \033[33m!\033[0m %s\n' "$*" >&2; }
die() { printf '    \033[31m✗\033[0m %s\n' "$*" >&2; exit 1; }

have() { command -v "$1" >/dev/null 2>&1; }

os="$(uname -s)"
arch="$(uname -m)"
case "$arch" in
  x86_64|amd64) arch="x64" ;;
  aarch64|arm64) arch="arm64" ;;
esac

mkdir -p "$BIN_DIR"

ensure_path() {
  case ":$PATH:" in
    *":$BIN_DIR:"*) ;;
    *) export PATH="$BIN_DIR:$PATH" ;;
  esac
  if [[ -f "$HOME/.cargo/env" ]]; then
    # shellcheck disable=SC1091
    . "$HOME/.cargo/env"
  fi
}

ensure_rust() {
  if have cargo && have rustc; then
    ok "Rust $(rustc --version | awk '{print $2}')"
    return
  fi
  log "Instalando Rust (rustup)…"
  have curl || die "necesito curl"
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
  # shellcheck disable=SC1091
  . "$HOME/.cargo/env"
  have cargo || die "rustup terminó pero cargo no está en PATH"
  ok "Rust $(rustc --version | awk '{print $2}')"
}

ensure_git() {
  have git && return
  if have sudo && [[ -f /etc/debian_version ]]; then
    log "Instalando git…"
    sudo apt-get update -qq
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq git
  fi
  have git || die "necesito git (sudo apt install git / brew install git)"
}

# Real checkout only. `curl | bash` sets BASH_SOURCE to /dev/fd/… — ignore that.
local_checkout() {
  local script="${BASH_SOURCE[0]:-}"
  case "$script" in
    ""|/dev/*|/proc/*) return 1 ;;
  esac
  [[ -f "$script" ]] || return 1
  local here
  here="$(cd "$(dirname "$script")/.." && pwd)"
  [[ -f "$here/crates/reco-cli/Cargo.toml" ]] || return 1
  printf '%s' "$here"
}

# Path on stdout only. All chatter is stderr.
ensure_source() {
  local here
  if here="$(local_checkout)"; then
    printf '%s' "$here"
    return 0
  fi
  ensure_git
  log "Preparando fuente en $SRC_DIR…"
  mkdir -p "$(dirname "$SRC_DIR")"
  if [[ -d "$SRC_DIR/.git" ]]; then
    git -C "$SRC_DIR" fetch --depth 1 origin main >&2
    git -C "$SRC_DIR" checkout -q -B main FETCH_HEAD >&2 \
      || git -C "$SRC_DIR" reset --hard origin/main >&2
  else
    rm -rf "$SRC_DIR"
    git clone --depth 1 --branch main "$REPO_URL" "$SRC_DIR" >&2
  fi
  [[ -f "$SRC_DIR/crates/reco-cli/Cargo.toml" ]] \
    || die "el clone en $SRC_DIR está incompleto. Borra esa carpeta y reintenta."
  printf '%s' "$SRC_DIR"
}

install_reco() {
  local root="${1:-}"
  log "Instalando reco…"
  ensure_path
  if [[ -n "$root" && -f "$root/crates/reco-cli/Cargo.toml" ]]; then
    cargo install --path "$root/crates/reco-cli" --locked --force \
      || cargo install --path "$root/crates/reco-cli" --force
  else
    cargo install --git "$REPO_URL" --path crates/reco-cli --locked --force \
      || cargo install --git "$REPO_URL" --path crates/reco-cli --force
  fi
  local reco_bin="$HOME/.cargo/bin/reco"
  [[ -x "$reco_bin" ]] || die "cargo install no dejó $reco_bin"
  mkdir -p "$BIN_DIR"
  ln -sfn "$reco_bin" "$BIN_DIR/reco"
  ensure_path
  if ! have reco; then
    die "reco no está en PATH. Añade $HOME/.cargo/bin y $BIN_DIR, abre otra terminal, y vuelve a probar."
  fi
  ok "reco $($reco_bin --version 2>/dev/null | head -1) → $reco_bin"
}

llama_asset() {
  local tag="$1"
  local variant="${RECO_LLAMA_VARIANT:-}"
  if [[ -n "$variant" ]]; then
    printf 'llama-%s-bin-%s.tar.gz' "$tag" "$variant"
    return
  fi
  case "$os" in
    Darwin)
      printf 'llama-%s-bin-macos-%s.tar.gz' "$tag" "$arch"
      ;;
    Linux)
      if have nvidia-smi && [[ "$arch" == "x64" ]]; then
        printf 'llama-%s-bin-ubuntu-vulkan-x64.tar.gz' "$tag"
      else
        printf 'llama-%s-bin-ubuntu-%s.tar.gz' "$tag" "$arch"
      fi
      ;;
    *)
      return 1
      ;;
  esac
}

install_llama() {
  [[ "$WANT_LLAMA" == 1 ]] || return 0
  if have llama-cli; then
    ok "llama-cli ya está: $(command -v llama-cli)"
    reco config set llama-cli "$(command -v llama-cli)" >/dev/null || true
    return
  fi
  if [[ -x "$BIN_DIR/llama-cli" ]]; then
    ok "llama-cli ya está: $BIN_DIR/llama-cli"
    reco config set llama-cli "$BIN_DIR/llama-cli" >/dev/null || true
    return
  fi

  have curl || die "necesito curl para bajar llama-cli"
  log "Descargando llama-cli (llama.cpp)…"
  local api json tag asset url tmp dest
  api="https://api.github.com/repos/ggml-org/llama.cpp/releases/latest"
  json="$(curl -fsSL -H 'Accept: application/vnd.github+json' "$api")" || die "no pude leer releases de llama.cpp"
  tag="$(printf '%s' "$json" | sed -n 's/.*"tag_name":[[:space:]]*"\([^"]*\)".*/\1/p' | head -1)"
  [[ -n "$tag" ]] || die "no pude leer el tag de llama.cpp"
  asset="$(llama_asset "$tag")" || {
    warn "no hay binario oficial de llama.cpp para $os/$arch"
    warn "compila el tuyo: https://github.com/ggml-org/llama.cpp y reco config set llama-cli /ruta"
    return 0
  }
  url="https://github.com/ggml-org/llama.cpp/releases/download/${tag}/${asset}"
  tmp="$(mktemp -d)"
  dest="$HOME/.local/share/reco/llama"
  mkdir -p "$dest"
  if ! curl -fsSL -o "$tmp/llama.tgz" "$url"; then
    if [[ "$asset" == *vulkan* ]]; then
      asset="llama-${tag}-bin-ubuntu-${arch}.tar.gz"
      url="https://github.com/ggml-org/llama.cpp/releases/download/${tag}/${asset}"
      curl -fsSL -o "$tmp/llama.tgz" "$url" || {
        warn "no pude bajar $asset"
        rm -rf "$tmp"
        return 0
      }
    else
      warn "no pude bajar $url"
      rm -rf "$tmp"
      return 0
    fi
  fi
  tar -xzf "$tmp/llama.tgz" -C "$tmp"
  local found
  found="$(find "$tmp" -type f \( -name llama-cli -o -name llama-completion \) -perm -u+x | head -1 || true)"
  if [[ -z "$found" ]]; then
    found="$(find "$tmp" -type f \( -name llama-cli -o -name llama-completion \) | head -1 || true)"
  fi
  if [[ -z "$found" ]]; then
    warn "el archivo de llama.cpp no traía llama-cli"
    rm -rf "$tmp"
    return 0
  fi
  rm -rf "$dest"
  mkdir -p "$dest"
  cp -a "$(dirname "$found")/." "$dest/"
  chmod +x "$dest/$(basename "$found")"
  ln -sfn "$dest/$(basename "$found")" "$BIN_DIR/llama-cli"
  # Some builds ship llama-cli; keep the real name too.
  if [[ "$(basename "$found")" != "llama-cli" ]]; then
    ln -sfn "$dest/$(basename "$found")" "$BIN_DIR/$(basename "$found")"
  fi
  reco config set llama-cli "$BIN_DIR/llama-cli" >/dev/null || true
  ok "llama-cli $tag → $BIN_DIR/llama-cli"
  rm -rf "$tmp"
}

linux_desktop_deps() {
  [[ "$os" == "Linux" ]] || return 0
  if pkg-config --exists webkit2gtk-4.1 2>/dev/null; then
    return 0
  fi
  if have sudo && [[ -f /etc/debian_version ]]; then
    log "Instalando WebKitGTK (ventana Tauri)…"
    sudo apt-get update -qq
    sudo DEBIAN_FRONTEND=noninteractive apt-get install -y -qq \
      libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf libgtk-3-dev pkg-config \
      || warn "faltan deps de WebKit; la ventana se omitirá"
  else
    warn "falta WebKitGTK. En Debian/Ubuntu:"
    warn "  sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf"
  fi
}

install_desktop() {
  [[ "$WANT_DESKTOP" == 1 ]] || return 0
  local root="$1"
  if ! have npm; then
    warn "sin npm: no compilo la ventana. Instala Node 20+ y vuelve a correr este script."
    return 0
  fi
  if [[ "$os" == "Linux" ]]; then
    linux_desktop_deps
    if ! pkg-config --exists webkit2gtk-4.1 2>/dev/null; then
      warn "sin WebKitGTK: salto la ventana. reco run --tui sigue funcionando."
      return 0
    fi
  fi
  log "Compilando la ventana Prueba (Tauri)… esto tarda la primera vez"
  if ! (cd "$root" && CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-$root/target}" bash scripts/build-desktop.sh); then
    warn "no pude compilar reco-desktop. Usa reco run --tui o instala las deps y reintenta."
    return 0
  fi
  local bin=""
  for candidate in \
    "${CARGO_TARGET_DIR:-$root/target}/release/reco-desktop" \
    "$root/target/release/reco-desktop" \
    "$root/crates/reco-desktop/src-tauri/target/release/reco-desktop"; do
    if [[ -x "$candidate" ]]; then
      bin="$candidate"
      break
    fi
  done
  [[ -n "$bin" ]] || {
    warn "build-desktop terminó sin binario"
    return 0
  }
  mkdir -p "$HOME/.cargo/bin"
  cp -f "$bin" "$BIN_DIR/reco-desktop"
  cp -f "$bin" "$HOME/.cargo/bin/reco-desktop"
  chmod +x "$BIN_DIR/reco-desktop" "$HOME/.cargo/bin/reco-desktop"
  ok "reco-desktop → $BIN_DIR/reco-desktop"
}

install_completions() {
  have reco || return 0
  local shell_name
  shell_name="$(basename "${SHELL:-bash}")"
  case "$shell_name" in
    bash|zsh|fish)
      log "Completados ($shell_name)…"
      reco setup --completions "$shell_name" || true
      ;;
  esac
}

finish() {
  ensure_path
  log "Listo"
  echo
  echo "  Añade esto a tu shell si reco no aparece:"
  echo "    export PATH=\"$BIN_DIR:\$HOME/.cargo/bin:\$PATH\""
  echo
  if have reco; then
    reco setup || reco doctor || true
  fi
  echo
  echo "  Siguiente:"
  echo "    reco            # menú · flechas + enter"
  echo "    reco setup"
  echo "    reco desktop"
}

if [[ "$SELF_TEST" == 1 ]]; then
  out="$(local_checkout || true)"
  if [[ "$out" == *$'\n'* ]]; then
    die "local_checkout no debe imprimir logs (salió con salto de línea)"
  fi
  if [[ -n "$out" && ! -f "$out/crates/reco-cli/Cargo.toml" ]]; then
    die "local_checkout devolvió una ruta inválida: $out"
  fi
  captured="$(
    log "ping-log"
    ok "ping-ok"
    printf 'ONLYPATH'
  )"
  if [[ "$captured" != "ONLYPATH" ]]; then
    die "log/ok se colaron en stdout: [$captured]"
  fi
  ok "self-test del instalador"
  exit 0
fi

ensure_path
log "Reco AI — instalación completa"
ensure_rust
ROOT=""
if ROOT="$(local_checkout)"; then
  ok "fuente local $ROOT"
fi
install_reco "$ROOT"
install_llama
if [[ "$WANT_DESKTOP" == 1 ]]; then
  ROOT="$(ensure_source)"
  ok "fuente $ROOT"
  install_desktop "$ROOT"
fi
install_completions
finish
