#!/bin/bash
# Auto-configure Waybar for wb-headsetcontrol

WAYBAR_CONFIG="${HOME}/.config/waybar/config.jsonc"
MODULE_NAME="custom/headsetcontrol"
BINARY_PATH="/usr/bin/wb-headset"

# Detect available terminal (alacritty preferred, then kitty)
if command -v alacritty &> /dev/null; then
    TERMINAL_CMD="alacritty"
elif command -v kitty &> /dev/null; then
    TERMINAL_CMD="kitty"
else
    echo "Error: Neither alacritty nor kitty found"
    exit 1
fi

# Check if Waybar config exists
if [ ! -f "$WAYBAR_CONFIG" ]; then
    echo "Waybar config not found at $WAYBAR_CONFIG - skipping auto-config"
    exit 0
fi

# Check if module already added to modules-right
if grep -q "\"$MODULE_NAME\"" "$WAYBAR_CONFIG"; then
    echo "wb-headsetcontrol already configured in Waybar"
    exit 0
fi

echo "Adding wb-headsetcontrol to Waybar (using $TERMINAL_CMD)..."

# Add module to modules-right array (after first module for safety)
sed -i '/modules-right.*\[/,/\]/s/"cpu"/"'$MODULE_NAME'", "cpu"/' "$WAYBAR_CONFIG"

# Add module config block before the final closing brace
# The on-click and on-right-click handlers are now in the JSON output from the binary
if ! grep -q "\"$MODULE_NAME\":" "$WAYBAR_CONFIG"; then
    # Insert module config
    cat >> "$WAYBAR_CONFIG" << EOF

  "$MODULE_NAME": {
    "exec": "$BINARY_PATH --waybar-status",
    "return-type": "json",
    "interval": 10,
    "format": "{text}",
    "tooltip": true,
    "on-click": "$TERMINAL_CMD $BINARY_PATH",
    "on-right-click": "bash -c '$BINARY_PATH --toggle-sidetone &'"
  },
EOF
fi

# Try to restart Waybar if running
if pgrep -f "^/usr/bin/waybar" > /dev/null 2>&1; then
    echo "Restarting Waybar..."
    pkill -f "^/usr/bin/waybar"
    sleep 1
    waybar &
fi

echo "✓ Waybar configuration updated!"
