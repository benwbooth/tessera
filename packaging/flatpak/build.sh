#!/bin/sh
# Runs inside flatpak-builder (see io.github.benwbooth.Tessera.yml).
set -eu

# Zig 0.15 for libghostty-vt (Ghostty's build).
ZIG=0.15.2
arch=$(uname -m)
curl -fsSL "https://ziglang.org/download/$ZIG/zig-$arch-linux-$ZIG.tar.xz" | tar -xJ
export PATH="$PWD/zig-$arch-linux-$ZIG:$PATH"
zig version

export QMAKE="$(command -v qmake6 || command -v qmake)"
echo "Qt $("$QMAKE" -query QT_VERSION)"
cargo build --release --locked

install -Dm755 target/release/tessera /app/bin/tessera
install -Dm644 packaging/linux/io.github.benwbooth.Tessera.desktop /app/share/applications/io.github.benwbooth.Tessera.desktop
install -Dm644 packaging/linux/io.github.benwbooth.Tessera.metainfo.xml /app/share/metainfo/io.github.benwbooth.Tessera.metainfo.xml
install -Dm644 icons/tessera.svg /app/share/icons/hicolor/scalable/apps/io.github.benwbooth.Tessera.svg
install -Dm644 packaging/icons/tessera-512.png /app/share/icons/hicolor/512x512/apps/io.github.benwbooth.Tessera.png
