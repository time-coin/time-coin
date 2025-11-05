#!/usr/bin/env bash
# scripts/ci/check-readme-scripts.sh
# Checks README.md and docs/*.md for references to scripts/* and verifies file existence.
# Exits non-zero if any referenced script path is missing.

set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || echo ".")"
MISSES=()

# Search target files: README.md and all docs/*.md
FILES_TO_SCAN=("$ROOT_DIR/README.md")
while IFS= read -r -d '' f; do FILES_TO_SCAN+=("$f"); done < <(find "$ROOT_DIR/docs" -maxdepth 2 -type f -name '*.md' -print0 2>/dev/null || true)

echo "Scanning ${#FILES_TO_SCAN[@]} markdown file(s) for scripts/ references..."

for md in "${FILES_TO_SCAN[@]}"; do
  [ -f "$md" ] || continue
  # Find occurrences like scripts/... or ./scripts/... or /scripts/...
  # Avoid matching URLs with http(s)://...scripts/ by excluding "http" prefix
  while IFS= read -r match; do
    # Normalize leading ./ or / to repo-relative path
    candidate="${match#./}"
    candidate="${candidate#/}"
    # If candidate contains a trailing punctuation, strip it
    candidate="${candidate%%[)\\],;\"' ]*}"
    # Only consider up to whitespace or punctuation
    if [[ ! -e "$ROOT_DIR/$candidate" ]]; then
      MISSES+=("$candidate (referenced in $md)")
    fi
  done < <(grep -oE '(?<!http[s]?:\/\/)(\./|/)?scripts\/[A-Za-z0-9._\/-]+' "$md" | sed 's/\\)$//' || true)
done

if [ "${#MISSES[@]}" -ne 0 ]; then
  echo "ERROR: The following referenced script files are missing in the repository:"
  for m in "${MISSES[@]}"; do
    echo "  - $m"
  done
  echo ""
  echo "Please ensure these paths exist or update the documentation to the correct paths."
  exit 1
fi

echo "OK: All referenced scripts exist."
exit 0