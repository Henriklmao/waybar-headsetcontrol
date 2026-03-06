#!/bin/bash
set -e

# Configuration
INSTALL_DIR="${HOME}/.local/bin/wb-headsetcontrol"
BIN_NAME="wb-headset"
WAYBAR_CONFIG="${HOME}/.config/waybar/config.jsonc"

echo "=== Waybar HeadsetControl Installer ==="

# 1. Build Rust Module
echo "[1/4] Building Waybar Rust Module..."
cargo build --release

# 2. Install to single directory
echo "[2/4] Installing to $INSTALL_DIR..."
mkdir -p "$INSTALL_DIR"
cp "target/release/$BIN_NAME" "$INSTALL_DIR/$BIN_NAME"
chmod +x "$INSTALL_DIR/$BIN_NAME"

# 3. Add to PATH
echo "[3/4] Adding to PATH..."
for rc_file in ~/.bashrc ~/.zshrc ~/.profile; do
    if [ -f "$rc_file" ]; then
        if ! grep -q "wb-headsetcontrol" "$rc_file"; then
            echo "export PATH=\"\$PATH:$INSTALL_DIR\"" >> "$rc_file"
            echo "  ✓ Updated $rc_file"
        fi
    fi
done

# 4. Update Waybar config
echo "[4/4] Updating Waybar config..."
if [ -f "$WAYBAR_CONFIG" ]; then
    if grep -q "\"custom/headsetcontrol\"" "$WAYBAR_CONFIG"; then
        # Config entry exists, update it with proper escaping
        sed -i 's|"exec": "[^"]*--waybar-status[^"]*"|"exec": "'$INSTALL_DIR/$BIN_NAME' --waybar-status"|g' "$WAYBAR_CONFIG"
        sed -i 's|"on-click": "alacritty -e [^"]*wb-headset[^"]*"|"on-click": "alacritty -e '$INSTALL_DIR/$BIN_NAME'"|g' "$WAYBAR_CONFIG"
        echo "  ✓ Updated Waybar config"
    else
        echo "  ⚠ custom/headsetcontrol not found in Waybar config - manual setup required"
    fi
else
    echo "  ⚠ Waybar config not found at $WAYBAR_CONFIG"
fi

echo ""
echo "=== Installation Complete! ==="
echo ""
echo "✓ Installed to: $INSTALL_DIR"
echo "  - Binary: $INSTALL_DIR/$BIN_NAME"
echo ""
echo "✓ Added to PATH in shell configuration files"
echo ""
if [ -f "$WAYBAR_CONFIG" ]; then
    echo "✓ Updated Waybar config at $WAYBAR_CONFIG"
    echo ""
    echo "Restart Waybar to apply changes (e.g., reload your window manager config)"
else
    echo "✓ Setup complete - manually add the following to your Waybar config:"
    echo ""
    echo '  "custom/headsetcontrol": {'
    echo '    "exec": "'$INSTALL_DIR/$BIN_NAME' --waybar-status",'
    echo '    "return-type": "json",'
    echo '    "interval": 10,'
    echo '    "on-click": "alacritty -e '$INSTALL_DIR/$BIN_NAME'",'
    echo '    "tooltip-format": "Headset Battery\nClick to open menu"'
    echo '  }'
fi
