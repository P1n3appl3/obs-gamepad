#!/usr/bin/env bash

set -ex

plugin_dir="$HOME/.config/obs-studio/plugins/gamepad"
mkdir -p "$plugin_dir"/bin/64bit
src=$(realpath "$(dirname "${BASH_SOURCE[0]}")")
cd "$src"
target=$(cargo metadata | jq .target_directory -r)
cargo build --release
cp -f "$target"/release/libgamepad.so "$plugin_dir"/bin/64bit
cp -f "$src"/example.toml "$plugin_dir"
