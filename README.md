# OBS Gamepad Plugin

I wanted to display my controller inputs while speedrunning celeste (and later
while playing melee), and I just wanted to draw some transparent shapes for
the buttons. I didn't want to set up a browser capture for something like
<https://gamepadviewer.com>, and [input-overlay](https://github.com/univrsal/input-overlay)
was a bit complicated for my use case (though if I ever wanted more features
it seems really cool and customizable), so I just wrote a little plugin to do
it myself.

celeste | melee
--|--
<video src="https://github.com/user-attachments/assets/b3164361-9b0e-4bd0-a1eb-2e8e89ed3d3c"> | <video src="https://github.com/user-attachments/assets/8be063a2-31be-4124-9007-64da3ac5f33b">

## Installation

### Windows

Download the [latest release](https://github.com/P1n3appl3/obs-gamepad/releases/latest) and then [follow these instructions](https://obsproject.com/kb/plugins-guide#install-or-remove-plugins) to copy it into your OBS plugins directory

### Nix

There's a home-manager module for OBS, so you can use this flake's overlay to install the plugin like you would any other:

```nix
programs.obs-studio = { enable = true;
  plugins = with pkgs.obs-studio-plugins; [ obs-gamepad ];
};
```

If you don't use flakes or home-manager, you can use flake-compat and manually override the OBS wrapper to add the plugin:

```nix
pkgs.wrapOBS.override {} { plugins = [ obs-gamepad ]; };
```

### Other Linux Distros

Run `install.sh` (which is just `cargo build` + moving the files around). If you want to write a PKGBUILD/deb/port/rpm/etc. for your distro, just take a look in [flake.nix](flake.nix) for the required native deps.

## Usage

Check out [the example](layouts/example.toml) to see the config options.

You don't have to open OBS to tweak your config, just
`cargo run <my-config.toml>` and it'll show your overlay in a separate window. Both
the OBS plugin and the standalone window support live-reloading, so if you tweak
your config file and save, the changes should show up in your overlay.

## Future plans

- stick tilt distortion
- backend to read dolphin memory like [m-overlay](https://github.com/bkacjios/m-overlay)
  - will need additional button config for arbitrary bezier paths
  - octagonal gate option
- keyboard backend?
- pass through info about whether or not a backend is connected and render that somehow
- button labels? at that point maybe just add these backends to input-overlay instead...
