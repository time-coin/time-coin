#!/bin/bash
# Fix systemd service to use correct working directory

set -e

echo "=== Fixing timed.service WorkingDirectory ==="
echo ""

SERVICE_FILE="/etc/systemd/system/timed.service"
CORRECT_DIR="/root/.timecoin"

# Check if the correct directory exists
if [ ! -d "$CORRECT_DIR" ]; then
    echo "Creating $CORRECT_DIR..."
    mkdir -p "$CORRECT_DIR"
    mkdir -p "$CORRECT_DIR/data"
    mkdir -p "$CORRECT_DIR/config"
    mkdir -p "$CORRECT_DIR/logs"
    echo "✅ Created directory structure"
fi

echo "Current service configuration:"
grep "WorkingDirectory=" "$SERVICE_FILE" || echo "No WorkingDirectory set"
echo ""

echo "Updating service file to use: $CORRECT_DIR"

# Update the service file
sudo sed -i "s|^WorkingDirectory=.*|WorkingDirectory=$CORRECT_DIR|" "$SERVICE_FILE"

echo "✅ Service file updated"
echo ""

echo "New service configuration:"
grep "WorkingDirectory=" "$SERVICE_FILE"
echo ""

# Reload systemd
echo "Reloading systemd daemon..."
sudo systemctl daemon-reload
echo "✅ Daemon reloaded"
echo ""

echo "Restarting timed service..."
sudo systemctl restart timed
echo "✅ Service restarted"
echo ""

echo "Waiting 3 seconds for startup..."
sleep 3
echo ""

echo "Service status:"
sudo systemctl status timed --no-pager -l | head -15 || true
