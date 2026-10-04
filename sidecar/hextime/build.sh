#!/usr/bin/env bash
# Builds the HexTime sidecar binary that HexStickyNote bundles.
#
#   HEXTIME_DIR=~/code/HexTime ./sidecar/hextime/build.sh
#
# Output: src-tauri/binaries/hextime-server-<rust target triple>
# Requires: uv, npm, and a HexTime checkout (default: ../HexTime next to this repo).
set -euo pipefail

REPO_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
HEXTIME_DIR="$(cd "${HEXTIME_DIR:-$REPO_DIR/../HexTime}" && pwd)"
SIDECAR_DIR="$REPO_DIR/sidecar/hextime"
OUT_DIR="$REPO_DIR/src-tauri/binaries"
WORK_DIR="$(mktemp -d)"
trap 'rm -rf "$WORK_DIR"' EXIT

TRIPLE="$(rustc -vV | sed -n 's/^host: //p')"
EXT=""
[[ "$TRIPLE" == *windows* ]] && EXT=".exe"

echo "==> HexTime: $HEXTIME_DIR"

echo "==> Building the HexTime frontend"
(cd "$HEXTIME_DIR/frontend" && npm ci && npm run build)

echo "==> Packaging the HexTime backend with PyInstaller"
SEP=":"
[[ "$TRIPLE" == *windows* ]] && SEP=";"
(
  cd "$HEXTIME_DIR/backend"

  # PDF export (newer HexTime versions) needs ReportLab's fonts in the bundle
  EXTRA_ARGS=()
  if uv run --frozen python -c "import reportlab" 2>/dev/null; then
    EXTRA_ARGS+=(--collect-data reportlab)
  fi

  # PyInstaller is added for this run only; HexTime's dependencies stay untouched
  uv run --frozen --with pyinstaller pyinstaller \
    --onefile \
    --noconfirm \
    --name hextime-server \
    --paths "$PWD" \
    --add-data "$PWD/alembic.ini${SEP}." \
    --add-data "$PWD/alembic${SEP}alembic" \
    --add-data "$PWD/app/static${SEP}app/static" \
    --collect-submodules app \
    --collect-submodules uvicorn \
    --collect-data tzdata \
    "${EXTRA_ARGS[@]}" \
    --distpath "$WORK_DIR/dist" \
    --workpath "$WORK_DIR/build" \
    --specpath "$WORK_DIR" \
    "$SIDECAR_DIR/hextime_server.py"
)

mkdir -p "$OUT_DIR"
cp "$WORK_DIR/dist/hextime-server$EXT" "$OUT_DIR/hextime-server-$TRIPLE$EXT"
echo "==> Done: $OUT_DIR/hextime-server-$TRIPLE$EXT"
