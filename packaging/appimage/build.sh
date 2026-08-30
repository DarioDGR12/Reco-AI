#!/usr/bin/env bash
# Build a terminal AppImage for the reco CLI.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../.." && pwd)"
BIN="${ROOT}/target/release/reco"
OUT="${ROOT}/dist"
ARCH="$(uname -m)"
APPDIR="${OUT}/RecoAI.AppDir"

if [[ ! -x "${BIN}" ]]; then
  echo "missing ${BIN} — run: cargo build --release -p reco-cli" >&2
  exit 1
fi

mkdir -p "${APPDIR}/usr/bin" "${APPDIR}/usr/share/applications" "${OUT}"
cp "${BIN}" "${APPDIR}/usr/bin/reco"
cp "$(dirname "$0")/RecoAI.desktop" "${APPDIR}/usr/share/applications/reco.desktop"
cp "$(dirname "$0")/RecoAI.desktop" "${APPDIR}/reco.desktop"
cat > "${APPDIR}/AppRun" <<'EOF'
#!/bin/sh
HERE="$(dirname "$(readlink -f "$0")")"
exec "${HERE}/usr/bin/reco" "$@"
EOF
chmod +x "${APPDIR}/AppRun" "${APPDIR}/usr/bin/reco"

if command -v appimagetool >/dev/null 2>&1; then
  appimagetool "${APPDIR}" "${OUT}/RecoAI-${ARCH}.AppImage"
  echo "wrote ${OUT}/RecoAI-${ARCH}.AppImage"
else
  echo "AppDir listo en ${APPDIR}"
  echo "Instala appimagetool y vuelve a correr este script para el .AppImage."
fi
