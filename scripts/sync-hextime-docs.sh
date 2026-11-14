#!/usr/bin/env bash
# Copies HexTime's documentation into docs/docs/hextime/ so that it is part of
# the HexStickyNote documentation.
#
# HexTime's pages are written in the HexTime repository and only copied here:
# edit them there, then run this script again. The copy is committed, so the
# documentation builds without a HexTime checkout.
#
#   scripts/sync-hextime-docs.sh            # HexTime next to this repository
#   HEXTIME_DIR=/path/to/HexTime scripts/sync-hextime-docs.sh
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
hextime="${HEXTIME_DIR:-$root/../HexTime}"
source_dir="$hextime/docs/docs"
target_dir="$root/docs/docs/hextime"

if [[ ! -f $source_dir/.nav.yml ]]; then
  echo "HexTime documentation not found in $source_dir (set HEXTIME_DIR)" >&2
  exit 1
fi

rm -rf "$target_dir"
mkdir -p "$target_dir"
cp -r "$source_dir"/. "$target_dir"/

# HexTime's "These pages" explains how to run HexTime's own documentation
# site; here the HexStickyNote one covers that.
rm -f "$target_dir/documentation.md"
sed -i '/documentation\.md/d' "$target_dir/.nav.yml" "$target_dir/index.md"

# Note where the pages come from, under the title of HexTime's overview
revision="$(git -C "$hextime" rev-parse --short HEAD 2>/dev/null || echo unknown)"
note="$(cat <<NOTE
!!! note "From the HexTime repository"
    These pages are copied from HexTime ($revision) by \`scripts/sync-hextime-docs.sh\`;
    edit them in the HexTime repository. Paths such as \`deploy/\` or \`backend/\` refer to
    that repository. How HexStickyNote runs HexTime is described in
    [Time tracking](../Time-tracking.md).
NOTE
)"
awk -v note="$note" '{ print } !done && /^# / { print ""; print note; done = 1 }' \
  "$target_dir/index.md" > "$target_dir/index.md.new"
mv "$target_dir/index.md.new" "$target_dir/index.md"

echo "Copied HexTime documentation ($revision) to $target_dir"
