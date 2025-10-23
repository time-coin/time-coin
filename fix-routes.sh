#!/bin/bash
echo "========================================"
echo "Fixing Axum Route Syntax"
echo "========================================"
echo ""
if [ ! -f "Cargo.toml" ]; then
  echo "ERROR: Not in time-coin directory!"
  exit 1
fi
BACKUP="backup_$(date +%Y%m%d_%H%M%S)"
mkdir -p "$BACKUP"
echo "Creating backups in: $BACKUP"
echo ""
FIXED=0
find . -name "*.rs" -type f ! -path "*/target/*" ! -path "*/backups/*" | while read file; do
  if grep -q '":' "$file" 2>/dev/null; then
    echo "Processing: $file"
    cp "$file" "$BACKUP/"
    sed -i 's|"/\([^"]*\):\([a-zA-Z_][a-zA-Z0-9_]*\)"|"/\1{\2}"|g' "$file"
    sed -i 's|":\([a-zA-Z_][a-zA-Z0-9_]*\)"|"{\1}"|g' "$file"
    sed -i 's|:\([a-zA-Z_][a-zA-Z0-9_]*\)/|{\1}/|g' "$file"
    echo "  ✓ Fixed routes in $file"
    FIXED=$((FIXED + 1))
  fi
done
echo ""
echo "========================================"
echo "✓ Complete!"
echo "========================================"
echo "Review changes: git diff"
echo "Test build: cargo build --release"
echo "Backups in: $BACKUP"
