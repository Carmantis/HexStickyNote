#!/usr/bin/env bash
# Builds the HexTime sidecar binary that HexStickyNote bundles.
#
#   HEXTIME_DIR=~/code/HexTime ./sidecar/hextime/build.sh
#
# Output: src-tauri/binaries/hextime-server-<rust target triple>
# Requires: uv, npm, and a HexTime checkout (default: ../HexTime next to this repo).
# On Windows, run it in Git Bash (as GitHub's Windows runners do).
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

# Paths handed to Python. In Git Bash, $PWD is /c/...; Windows' Python needs C:/...
native() {
  if [[ "$TRIPLE" == *windows* ]] && command -v cygpath >/dev/null; then
    cygpath -m "$1"
  else
    printf '%s\n' "$1"
  fi
}

echo "==> HexTime: $HEXTIME_DIR"

echo "==> Building the HexTime frontend"
(cd "$HEXTIME_DIR/frontend" && npm ci && npm run build)

echo "==> Packaging the HexTime backend with PyInstaller"
SEP=":"
[[ "$TRIPLE" == *windows* ]] && SEP=";"
(
  cd "$HEXTIME_DIR/backend"
  BACKEND="$(native "$PWD")"
  WORK="$(native "$WORK_DIR")"

  # Keep this list in step with HexTime's backend/hextime.spec.
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
    --paths "$BACKEND" \
    --add-data "$BACKEND/alembic.ini${SEP}." \
    --add-data "$BACKEND/alembic${SEP}alembic" \
    --add-data "$BACKEND/app/static${SEP}app/static" \
    --add-data "$BACKEND/app/i18n${SEP}app/i18n" \
    --add-data "$BACKEND/app/reports/fonts${SEP}app/reports/fonts" \
    --collect-submodules app \
    --collect-submodules uvicorn \
    --collect-submodules alembic \
    --collect-data tzdata \
    "${EXTRA_ARGS[@]}" \
    --distpath "$WORK/dist" \
    --workpath "$WORK/build" \
    --specpath "$WORK" \
    "$(native "$SIDECAR_DIR/hextime_server.py")"
)

mkdir -p "$OUT_DIR"
cp "$WORK_DIR/dist/hextime-server$EXT" "$OUT_DIR/hextime-server-$TRIPLE$EXT"
echo "==> Done: $OUT_DIR/hextime-server-$TRIPLE$EXT"
