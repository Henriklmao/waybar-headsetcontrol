#!/bin/bash
set -e

# Configuration
BIN_DIR="${HOME}/.local/bin"
LIB_DIR="${HOME}/.local/lib/waybar"
CMD_NAME="waybar-headsetcontrol-cmd"
LIB_NAME="libwaybar_headsetcontrol.so"

echo "=== Waybar HeadsetControl Installer ==="

# 1. Build HeadsetControl
echo "[1/4] Building HeadsetControl..."
if [ -d "HeadsetControl-master" ]; then
    cd HeadsetControl-master
    mkdir -p build
    cd build
    cmake ..
    make -j$(nproc)
    cd ../..
else
    echo "Error: HeadsetControl-master directory not found!"
    exit 1
fi

# 2. Install HeadsetControl binary
echo "[2/4] Installing HeadsetControl binary to $BIN_DIR..."
mkdir -p "$BIN_DIR"
cp "HeadsetControl-master/build/headsetcontrol" "$BIN_DIR/$CMD_NAME"
chmod +x "$BIN_DIR/$CMD_NAME"
echo "Installed $CMD_NAME"

# 3. Build Rust Module
echo "[3/4] Building Waybar Rust Module..."
cargo build --release

# 4. Install Shared Library
echo "[4/4] Installing Waybar Module library to $LIB_DIR..."
mkdir -p "$LIB_DIR"
cp "target/release/libwaybar.so" "$LIB_DIR/$LIB_NAME"

echo ""
echo "=== Installation Complete! ==="
echo ""
echo "Please ensure $BIN_DIR is in your PATH."
echo ""
echo "Add the following to your Waybar configuration (config.jsonc):"
echo "---------------------------------------------------"
echo "\"custom/headsetcontrol\": {"
echo "    \"exec\": \"$LIB_DIR/$LIB_NAME\","
echo "    \"return-type\": \"json\","
echo "    \"interval\": \"once\""
echo "}"
echo "---------------------------------------------------"
echo "Note: Since this is a CFFI module, usage depends on how your Waybar supports loading dynamic modules."
echo "If you are using standard Waybar, you might need to use 'swaybar' or a compatible loader,"
echo "OR if this was intended as a standalone binary, please check the documentation."
echo "(Currently compiled as a shared library 'cdylib')"
