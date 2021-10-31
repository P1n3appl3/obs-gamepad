#!/usr/bin/env bash

set -ex

mkdir -p $HOME/config/obs-studio/plugins/gamepad/bin/64bit
cargo build --release
ln -fs $(pwd)/target/release/libgamepad.so $HOME/.config/obs-studio/plugins/gamepad/bin/64bit/libgamepad.so
