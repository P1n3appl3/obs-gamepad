#!/usr/bin/env bash

set -ex

plugin_dir=$HOME/.config/obs-studio/plugins/gamepad/bin/64bit
mkdir -p $plugin_dir
cargo build --release
ln -fs $(pwd)/target/release/libgamepad.so $plugin_dir/libgamepad.so
