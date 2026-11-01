#!/usr/bin/env bash
set -euo pipefail

target=$1
version=$2
name="stow-${version}-${target}"
out="dist/${name}"
rel="target/${target}/release"

rm -rf "$out"
mkdir -p "$out/include" "$out/lib"
cp stow-ffi/include/stow.h "$out/include/"
cp LICENSE README.md "$out/"

case "$target" in
  *-windows-*)
    mkdir "$out/bin"
    cp "$rel/stow.dll" "$out/bin/"
    cp "$rel/stow.dll.lib" "$rel/stow.lib" "$out/lib/"
    ;;
  *-apple-*)
    cp "$rel/libstow.dylib" "$rel/libstow.a" "$out/lib/"
    ;;
  *)
    cp "$rel/libstow.so" "$rel/libstow.a" "$out/lib/"
    ;;
esac

cd dist
case "$target" in
  *-windows-*) 7z a -tzip "${name}.zip" "$name" > /dev/null ;;
  *) tar -czf "${name}.tar.gz" "$name" ;;
esac
rm -rf "$name"
