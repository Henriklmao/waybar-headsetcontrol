# [HeadsetControl](https://github.com/Sapd/HeadsetControl) integration for [Waybar](https://github.com/Alexays/Waybar)

A lightweight and basic integration of HeadsetControl features into your waybar config
> Only works with headphones supported by HeadsetControl. See [here](https://github.com/Sapd/HeadsetControl/tree/master#supported-headsets)
>
## Features

* Shows battery percentage of your wireless headphones.

> Probably only works when no more than a single pair of headphones are connected.

* [Coming Soon] Toggle your sidetone with a single click on the widget

## Installation

### Manual install

1. Clone and unzip the repository
2. Move or copy waybar-headsetcontrol.sh to ~/.config/waybar/
3. Grant permissions via terminal:  

```sh
chmod +x ~/.config/waybar/waybar-headsetcontrol.sh
```

4. Open the following config with your prefered [editor](https://neovim.io/) ~/.config/waybar/config.jsonc:  
  Add the following to "modules-right" \(center or left is also possible\):

```sh
"waybar-headsetcontrol"
```

> Depending on your oder of modules, you may have to append a comma to the previous entry.

5. In the same file, append

```sh
Work in progress
```

### Automatic install script

> Install script still is work in progress.
