# OBS Gamepad Plugin

I wanted to display my controller inputs while speedrunning celeste (and later
while playing melee), and I just wanted to draw some transparent shapes for
the buttons. I didn't want to set up a browser capture for something like
<https://gamepadviewer.com>, and [input-overlay](https://github.com/univrsal/input-overlay)
was a bit complicated for my use case (though if I ever wanted more features
it seems really cool and customizable), so I just wrote a little plugin to do
it myself.

![celeste gamepad](https://user-images.githubusercontent.com/9326885/192166090-2c68091a-9d97-49f3-999b-1218656dddab.png)

![test gamepad](https://user-images.githubusercontent.com/9326885/192166326-8fb34abb-c0b8-44cc-b949-d9eaf4e64b10.png)

## Configuration

You don't have to open OBS to tweak your config, just
`cargo run my-config.toml` and it'll show your overlay in a separate window. Both
the OBS plugin and the standalone window support live-reloading. If you tweak
your config file and save, the changes will show up in your overlay. Check out
[the example](example.toml) to see the config options.

## Installation

``` bash
cargo build --release
plugin_dir="$HOME/.config/obs-studio/plugins/gamepad" # or whatever it is on your platform...
cp -f target/release/libgamepad.so "$plugin_dir"/bin/64bit
cp -f example.toml "$plugin_dir"
```

## Future plans

- stick tilt distortion
- backend to read dolphin memory like [m-overlay](https://github.com/bkacjios/m-overlay)
  - will need additional button config for arbitrary bezier paths
  - octagonal gate option
- keyboard backend?
- pass through info about whether or not a backend is connected and render that somehow
- button labels? at that point maybe just add these backends to input-overlay instead...
