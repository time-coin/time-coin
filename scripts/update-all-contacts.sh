#!/bin/bash
# Update all documentation files

FILES=(
    "README.md"
    "docs/README.md"
    "CONTRIBUTING.md"
    "docs/governance/voting-guide.md"
    "docs/treasury/treasury-overview.md"
)

for file in "${FILES[@]}"; do
    if [ -f "$file" ]; then
        echo "Updating $file..."
        sed -i.bak 's/@TIMEcoinOfficial/@TIMEcoin515010/g' "$file"
        sed -i.bak 's|https://t\.me/timecoin|https://t.me/+CaN6EflYM-83OTY0|g' "$file"
        sed -i.bak 's|https://t\.co/ISNmAW8gMV|https://t.me/+CaN6EflYM-83OTY0|g' "$file"
        rm -f "${file}.bak"
        echo "  ✓ Updated"
    fi
done

echo "All files updated!"
