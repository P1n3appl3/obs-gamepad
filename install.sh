#!/usr/bin/env bash

set -ex

plugin_dir="$HOME"/.config/obs-studio/plugins/gamepad/
mkdir -p "$plugin_dir"
cargo build --release
src=$(realpath "$(dirname "${BASH_SOURCE[0]}")")
ln -fs "$src"/target/release/libgamepad.so "$plugin_dir"/bin/64bit/libgamepad.so
ln -fs "$src"/example.toml "$plugin_dir"/example.toml
