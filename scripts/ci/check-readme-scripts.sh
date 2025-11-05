#!/usr/bin/env bash
# scripts/ci/check-readme-scripts.sh
# Checks README.md and docs/*.md for references to scripts/* and verifies file existence.
# Exits non-zero if any referenced script path is missing.
#
# Portable adjustments made for Git Bash / MSYS:
# - Avoid use of [[ ... =~ ... ]] with complex character classes (can fail on some bash builds)
# - Use a case-based loop to trim trailing punctuation
# - Use a safe grep pattern with hyphen at start of class

set -euo pipefail

ROOT_DIR="$(git rev-parse --show-toplevel 2>/dev/null || pwd)"
MISSES=()

# Start with README.md
FILES_TO_SCAN=("$ROOT_DIR/README.md")

# Add docs/*.md (if any)
while IFS= read -r -d '' f; do
  FILES_TO_SCAN+=("$f")
done < <(find "$ROOT_DIR/docs" -maxdepth 2 -type f -name '*.md' -print0 2>/dev/null || true)

echo "Scanning ${#FILES_TO_SCAN[@]} markdown file(s) for scripts/ references..."

for md in "${FILES_TO_SCAN[@]}"; do
  [ -f "$md" ] || continue

  # Extract substrings that start with scripts/ (or ./scripts/ or /scripts/) up to the next whitespace/punctuation
  # grep will return one match per line; use || true so absence of matches doesn't cause failure
  while IFS= read -r match; do
    # Normalize leading ./ or / to repo-relative path
    candidate="${match#./}"
    candidate="${candidate#/}"

    # Strip trailing punctuation characters if present: ) , ] ; " ' space
    # Use a case statement to avoid =~ and maintain portability
    while [ -n "$candidate" ]; do
      last_char="${candidate: -1}"
      case "$last_char" in
        ')'|','|']'|';'|'"'|"'"|' ')
          candidate=${candidate%?}
          ;;
        *)
          break
          ;;
      esac
    done

    # Skip empty results
    if [ -z "$candidate" ]; then
      continue
    fi

    # Check existence relative to repo root
    if [ ! -e "$ROOT_DIR/$candidate" ]; then
      MISSES+=("$candidate (referenced in $md)")
    fi
  done < <(grep -oE '(\./|/)?scripts/[-A-Za-z0-9._/]*' "$md" || true)
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