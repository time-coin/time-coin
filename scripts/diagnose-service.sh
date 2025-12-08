#!/bin/bash
# Diagnostic script to check systemd service configuration

echo "=== Checking systemd service file ==="
echo ""

SERVICE_FILE="/etc/systemd/system/timed.service"

if [ -f "$SERVICE_FILE" ]; then
    echo "Service file exists at: $SERVICE_FILE"
    echo ""
    echo "=== Service file contents ==="
    cat "$SERVICE_FILE"
    echo ""
    echo "=== Checking WorkingDirectory ==="
    WORKING_DIR=$(grep "^WorkingDirectory=" "$SERVICE_FILE" | cut -d= -f2)
    echo "WorkingDirectory is set to: $WORKING_DIR"
    echo ""
    
    if [ -n "$WORKING_DIR" ]; then
        if [ -d "$WORKING_DIR" ]; then
            echo "✅ Directory exists: $WORKING_DIR"
            ls -la "$WORKING_DIR" | head -10
        else
            echo "❌ Directory does NOT exist: $WORKING_DIR"
            echo ""
            echo "Expanded path would be:"
            eval echo "$WORKING_DIR"
        fi
    else
        echo "⚠️  No WorkingDirectory specified in service file"
    fi
else
    echo "❌ Service file not found at: $SERVICE_FILE"
fi

echo ""
echo "=== Checking for directory ==="
ls -la /root/.timecoin 2>/dev/null || echo "❌ /root/.timecoin does not exist"
echo ""
ls -la ~/.timecoin 2>/dev/null || echo "❌ ~/.timecoin does not exist"
echo ""
echo "Current user: $(whoami)"
echo "HOME variable: $HOME"
