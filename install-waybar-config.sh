#!/bin/bash
# Auto-configure Waybar for wb-headsetcontrol

WAYBAR_CONFIG="${HOME}/.config/waybar/config.jsonc"
MODULE_NAME="custom/headsetcontrol"
BINARY_PATH="/usr/bin/wb-headset"

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

echo "Adding wb-headsetcontrol to Waybar..."

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
    "format": "{icon}",
    "format-icons": ["󰋎"],
    "on-click": "alacritty -e $BINARY_PATH"
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

echo "✅ Waybar configuration updated!"
