# OBS Gamepad Plugin

I wanted to display my controller inputs while speedrunning, and I just wanted
to draw some transparent shapes for the buttons. I didn't want to set up a
browser capture for something like https://gamepadviewer.com, and
[input-overlay](https://github.com/univrsal/input-overlay) was a bit complicated
for my use case (though if I ever wanted more features it seems really cool and
customizable), so I just wrote a little plugin to do it myself.

![celeste gamepad](https://user-images.githubusercontent.com/9326885/192166090-2c68091a-9d97-49f3-999b-1218656dddab.png)

![test gamepad](https://user-images.githubusercontent.com/9326885/192166326-8fb34abb-c0b8-44cc-b949-d9eaf4e64b10.png)

## Configuration

You don't have to open OBS to tweak your configs, just
`cargo run my-config.toml` and it'll show your overlay in a separate window.
Both the plugin and the standalone window support live-reloading. If you tweak
your config file and save, the changes will show up in your overlay. Check out
[the example](example.toml) to see the config options.

## Installation

There's an [install script](install.sh) for linux, but there's nothing platform
specific about the code itself so you should be able to manually install for
Mac/Windows by placing the shared library in your OBS plugin dir.
