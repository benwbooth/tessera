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

# Qt WebEngine comes from the BaseApp (in /app), the rest of Qt from the
# runtime (in /usr), but the runtime's qmake only knows /usr. A qmake that
# answers with both, merged: the libraries (and their .prl files) and the
# headers.
QLIBS="$("$QMAKE" -query QT_INSTALL_LIBS)"
QHDRS="$("$QMAKE" -query QT_INSTALL_HEADERS)"
WE_PRL="$(find /app -name 'libQt6WebEngineQuick.prl' 2>/dev/null | head -1)"
if [ -n "$WE_PRL" ] && [ ! -e "$QLIBS/libQt6WebEngineQuick.prl" ]; then
    M="$PWD/qt-merged"
    rm -rf "$M"
    mkdir -p "$M/lib" "$M/include"
    for f in "$QLIBS"/libQt6* "$(dirname "$WE_PRL")"/libQt6*; do ln -sf "$f" "$M/lib/"; done
    for d in "$QHDRS"/*; do ln -sfn "$d" "$M/include/"; done
    WE_H="$(find /app -type d -name QtWebEngineQuick -path '*include*' 2>/dev/null | head -1)"
    if [ -n "$WE_H" ]; then
        for d in "$(dirname "$WE_H")"/*; do ln -sfn "$d" "$M/include/"; done
    fi
    cat > "$M/qmake" <<EOF
#!/bin/sh
if [ "\$1" = -query ]; then
    case "\$2" in
        QT_INSTALL_LIBS) echo "$M/lib"; exit 0 ;;
        QT_INSTALL_HEADERS) echo "$M/include"; exit 0 ;;
    esac
fi
exec "$QMAKE" "\$@"
EOF
    chmod +x "$M/qmake"
    export QMAKE="$M/qmake"
    echo "Qt WebEngine from $(dirname "$WE_PRL"), merged in $M"
fi
cargo build --release --locked

install -Dm755 target/release/tessera /app/bin/tessera
install -Dm755 target/release/tesserad /app/bin/tesserad
install -Dm644 packaging/linux/io.github.benwbooth.Tessera.desktop /app/share/applications/io.github.benwbooth.Tessera.desktop
install -Dm644 packaging/linux/io.github.benwbooth.Tessera.metainfo.xml /app/share/metainfo/io.github.benwbooth.Tessera.metainfo.xml
install -Dm644 icons/tessera.svg /app/share/icons/hicolor/scalable/apps/io.github.benwbooth.Tessera.svg
install -Dm644 packaging/icons/tessera-512.png /app/share/icons/hicolor/512x512/apps/io.github.benwbooth.Tessera.png
