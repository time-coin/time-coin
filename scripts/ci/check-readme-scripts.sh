#!/bin/bash
# Validates that all script references in README.md exist in the repository
# Exits with non-zero status if any referenced files are missing

set -e

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
README_FILE="$REPO_ROOT/README.md"

if [ ! -f "$README_FILE" ]; then
    echo -e "${RED}Error: README.md not found at $README_FILE${NC}"
    exit 1
fi

echo "Checking README.md for script references..."
echo "Repository root: $REPO_ROOT"
echo ""

# Extract script references from README.md
# Look for patterns like:
# - scripts/something.sh
# - ./scripts/something.sh
# - scripts/setup/something.sh
# We'll extract all references to files under scripts/

MISSING_FILES=()
CHECKED_FILES=()

# Use grep to find lines containing 'scripts/' and extract the paths
# This regex looks for scripts/ followed by any path-like characters
while IFS= read -r line; do
    # Extract all occurrences of scripts/[path] from the line
    # Handles both ./scripts/ and scripts/ and with or without quotes
    echo "$line" | grep -oE '(\./)?scripts/[a-zA-Z0-9_./\-]+\.(sh|md|toml|yml|yaml|txt)' | while read -r script_path; do
        # Remove leading ./ if present
        script_path="${script_path#./}"
        
        # Skip if we've already checked this file
        if [[ " ${CHECKED_FILES[@]} " =~ " ${script_path} " ]]; then
            continue
        fi
        
        CHECKED_FILES+=("$script_path")
        
        full_path="$REPO_ROOT/$script_path"
        if [ -f "$full_path" ]; then
            echo -e "${GREEN}✓${NC} Found: $script_path"
        else
            echo -e "${RED}✗${NC} Missing: $script_path"
            MISSING_FILES+=("$script_path")
        fi
    done
done < "$README_FILE"

echo ""

# Report results
if [ ${#MISSING_FILES[@]} -eq 0 ]; then
    echo -e "${GREEN}All script references are valid!${NC}"
    exit 0
else
    echo -e "${RED}Error: Found ${#MISSING_FILES[@]} missing script reference(s):${NC}"
    for file in "${MISSING_FILES[@]}"; do
        echo -e "${RED}  - $file${NC}"
    done
    echo ""
    echo "Please update README.md to reference existing files or create the missing files."
    exit 1
fi
