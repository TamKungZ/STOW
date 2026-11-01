#!/usr/bin/env bash
set -euo pipefail

target=$1
version=$2
revision=${3:-1}

case "$target" in
  x86_64-unknown-linux-gnu) arch=amd64; triplet=x86_64-linux-gnu ;;
  aarch64-unknown-linux-gnu) arch=arm64; triplet=aarch64-linux-gnu ;;
  *) echo "unsupported target: $target" >&2; exit 1 ;;
esac

rel="target/${target}/release"
tilde='~'
deb_version="${version//-/$tilde}-${revision}"
maintainer="TamKungZ_ <dev@tamkungz.me>"
homepage="https://github.com/TamKungZ/STOW"
libdir="usr/lib/${triplet}"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT
mkdir -p dist

copyright() {
  local dir=$1
  mkdir -p "$dir"
  {
    echo "Format: https://www.debian.org/doc/packaging-manuals/copyright-format/1.0/"
    echo "Upstream-Name: STOW"
    echo "Source: ${homepage}"
    echo
    echo "Files: *"
    echo "Copyright: 2026 TamKungZ_"
    echo "License: BSD-3-Clause"
    sed -e 's/^$/./' -e 's/^/ /' LICENSE
  } > "$dir/copyright"
}

control() {
  local dir=$1 package=$2 section=$3 depends=$4 summary=$5 description=$6
  mkdir -p "$dir/DEBIAN"
  local size
  size=$(du -sk --exclude=DEBIAN "$dir" | cut -f1)
  cat > "$dir/DEBIAN/control" <<CONTROL
Package: ${package}
Version: ${deb_version}
Architecture: ${arch}
Maintainer: ${maintainer}
Installed-Size: ${size}
Depends: ${depends}
Section: ${section}
Priority: optional
Multi-Arch: same
Homepage: ${homepage}
Description: ${summary}
 ${description}
CONTROL
}

runtime="$work/libstow0"
mkdir -p "$runtime/$libdir"
install -m 0644 "$rel/libstow.so" "$runtime/$libdir/libstow.so.${version%%-*}"
strip --strip-unneeded "$runtime/$libdir/libstow.so.${version%%-*}" || true
ln -s "libstow.so.${version%%-*}" "$runtime/$libdir/libstow.so.0"
copyright "$runtime/usr/share/doc/libstow0"
control "$runtime" libstow0 libs "libc6 (>= 2.34), libgcc-s1" \
  "STOW archive library, shared library" \
  "Shared library for reading and writing STOW archives through a C ABI."
mkdir -p "$runtime/DEBIAN"
echo "libstow 0 libstow0 (>= ${deb_version%%-*})" > "$runtime/DEBIAN/shlibs"
printf '#!/bin/sh\nset -e\nif [ "$1" = configure ]; then ldconfig; fi\n' > "$runtime/DEBIAN/postinst"
printf '#!/bin/sh\nset -e\ncase "$1" in remove|purge) ldconfig ;; esac\n' > "$runtime/DEBIAN/postrm"
chmod 0755 "$runtime/DEBIAN/postinst" "$runtime/DEBIAN/postrm"
dpkg-deb --root-owner-group --build "$runtime" "dist/libstow0_${deb_version}_${arch}.deb"

devel="$work/libstow-dev"
mkdir -p "$devel/usr/include" "$devel/$libdir"
install -m 0644 stow-ffi/include/stow.h "$devel/usr/include/stow.h"
install -m 0644 "$rel/libstow.a" "$devel/$libdir/libstow.a"
ln -s libstow.so.0 "$devel/$libdir/libstow.so"
copyright "$devel/usr/share/doc/libstow-dev"
control "$devel" libstow-dev libdevel "libstow0 (= ${deb_version})" \
  "STOW archive library, development files" \
  "Header, static library and linker symlink for building against libstow."
dpkg-deb --root-owner-group --build "$devel" "dist/libstow-dev_${deb_version}_${arch}.deb"
