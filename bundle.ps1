$bundle_dir="obs-gamepad\bin\64bit"
New-Item -ItemType Directory -Path $bundle_dir -Force
$target="target"
if (Get-Command jq) {
    $target="$(cargo metadata --format-version 1 | jq .target_directory -r)"
}
echo "using target dir at $target"
cargo build --release
cp -Force "$target\release\gamepad.dll" "$bundle_dir\obs-gamepad.dll"
cp -Force "$target\release\obs-gamepad.exe" "$bundle_dir\obs-gamepad-tester.exe"
mkdir -Force "$bundle_dir\layouts"
cp -Force -Recurse layouts "$bundle_dir\" -Filter *.toml
