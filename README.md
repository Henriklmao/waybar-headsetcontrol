# [HeadsetControl](https://github.com/Sapd/HeadsetControl) integration for [Waybar](https://github.com/Alexays/Waybar)

A simple Rust and [ratatui](https://ratatui.rs/) based integration of HeadsetControl features into Waybar. Shows wireless headphone battery status with colored icons and provides an interactive TUI for sidetone control.

> Only works with headphones supported by HeadsetControl. See: [the official repo](https://github.com/Sapd/HeadsetControl/tree/master?tab=readme-ov-file#supported-devices)

## Features

- **Battery Display** - Shows battery percentage with color-coded icons (green >50%, yellow 15-50%, red <15%)
- **Sidetone Control** - Interactive TUI to adjust sidetone levels (0-128)
- **Configurable Keys** - Customize all keybindings and default sidetone via interactive config or terminal

## Installation

### AUR (Arch User Repository) Coming soon

```bash
yay -S wb-headsetcontrol
# or
paru -S wb-headsetcontrol
```

After installation, the post_install hook will display instructions for Waybar configuration.

### Manual Build

1. Clone the repository:

```bash
git clone https://github.com/Henriklmao/waybar-headsetcontrol.git
cd waybar-headsetcontrol
```

2. Build with Cargo:

```bash
cargo build --release
```

3. Install the binary:

```bash
sudo install -Dm 755 target/release/wb-headset /usr/bin/wb-headset
```

## Configuration

### Waybar

Add to your `~/.config/waybar/config.jsonc`:

```jsonc
{
  "modules-right": ["custom/headsetcontrol"],

  "custom/headsetcontrol": {
    "exec": "/usr/bin/wb-headset --waybar-status",
    "return-type": "json",
    "interval": 10,
    "on-click": "alacritty -e /usr/bin/wb-headset",
    "tooltip-format": "Headset Battery\nClick to open menu",
  },
}
```

### Keybindings & Settings

Configuration is automatically created at `~/.config/wb-headsetcontrol/config.toml`.

To configure interactively, either:

- Run: `wb-headset --config`
- Or press `c` inside the tui to interactively set your bindings.

Default keybindings:

- **a/s** - Decrease sidetone by 1/10
- **d/w** - Increase sidetone by 1/10
- **f** - Set sidetone to full (128)
- **e** - Set sidetone to none (0)
- **c** - Open configuration menu
- **v** - Toggle verbose mode (reserved for future use)
- **q/ESC** - Quit

## Usage

Launch the interactive TUI:

```bash
wb-headset
```

Get battery status for Waybar:

```bash
wb-headset --waybar-status
```
