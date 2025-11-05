#!/usr/bin/env bash
# scripts/dev-setup.sh
# Compatibility wrapper for docs that reference scripts/dev-setup.sh
# If the canonical installer exists at scripts/setup/setup-testnet-node.sh, this wrapper will invoke it.
# Otherwise it prints a helpful message explaining the canonical path.

set -euo pipefail

# Resolve repo root and wrapper location
SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(git rev-parse --show-toplevel 2>/dev/null || printf "%s" "$SCRIPT_DIR/..")"

CANONICAL="$REPO_ROOT/scripts/setup/setup-testnet-node.sh"

if [ -f "$CANONICAL" ]; then
  echo "Invoking canonical setup script: $CANONICAL"
  # Preserve positional args (so callers can pass options)
  exec bash "$CANONICAL" "$@"
else
  cat <<'EOF'
ERROR: canonical setup script not found: scripts/setup/setup-testnet-node.sh

The documentation references scripts/dev-setup.sh as a convenience wrapper. The recommended canonical installer is:
  scripts/setup/setup-testnet-node.sh

To fix:
- Either create scripts/setup/setup-testnet-node.sh (the canonical installer), or
- Update docs to reference the canonical path.

You can run the canonical script manually (if present) with:
  bash scripts/setup/setup-testnet-node.sh

This wrapper will fail until the canonical installer exists.
EOF
  exit 1
fi