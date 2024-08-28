#!/usr/bin/env bash

set -ex

plugin_dir="$HOME/.config/obs-studio/plugins/gamepad"
mkdir -p "$plugin_dir"/bin/64bit
src=$(realpath "$(dirname "${BASH_SOURCE[0]}")")
cd "$src"
target=target
if command -v jq &>/dev/null; then
  target=$(cargo metadata --format-version 1 | jq .target_directory -r)
fi
cargo build --release
cp -f "$target/release/libgamepad.so" "$plugin_dir/bin/64bit/obs-gamepad.so"
cp -f "$target/release/obs-gamepad" "$plugin_dir/obs-gamepad-tester"
cp -rf layouts "$plugin_dir"
