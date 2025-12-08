#!/bin/bash
# Quick fix to create .timecoin directory structure
# Run this to fix the CHDIR error

set -e

echo "Creating .timecoin directory structure..."

# Determine the home directory (use root's home if running as root)
if [ "$USER" = "root" ]; then
    HOME_DIR="/root"
else
    HOME_DIR="$HOME"
fi

echo "Using home directory: $HOME_DIR"

# Create main .timecoin directory
mkdir -p "$HOME_DIR/.timecoin"
echo "✅ Created $HOME_DIR/.timecoin"

# Create subdirectories
mkdir -p "$HOME_DIR/.timecoin/data"
echo "✅ Created $HOME_DIR/.timecoin/data"

mkdir -p "$HOME_DIR/.timecoin/config"
echo "✅ Created $HOME_DIR/.timecoin/config"

mkdir -p "$HOME_DIR/.timecoin/logs"
echo "✅ Created $HOME_DIR/.timecoin/logs"

mkdir -p "$HOME_DIR/.timecoin/data/blockchain"
echo "✅ Created $HOME_DIR/.timecoin/data/blockchain"

# Set ownership if not running as the target user
if [ "$USER" = "root" ] && [ -n "$SUDO_USER" ]; then
    chown -R $SUDO_USER:$SUDO_USER "$HOME_DIR/.timecoin"
    echo "✅ Set ownership to $SUDO_USER"
fi

echo ""
echo "✅ Directory structure created successfully!"
echo ""
echo "Now restart the service with:"
echo "  sudo systemctl restart timed"
