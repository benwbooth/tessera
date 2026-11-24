#!/usr/bin/env bash
# The single-file tessh for macOS (CI's macOS runners; also by hand on a
# Mac): tessh against Python's stable ABI, with a CPython and uv, built into
# the launcher (a signed program can't have data appended, as Linux's has).
# No download at the first run, as on Linux. Output: dist/tessh-<arch>-macos.
#
#   scripts/tessh-static-macos.sh
set -euo pipefail
cd "$(dirname "$0")/.."
case "$(uname -m)" in arm64|aarch64) arch=aarch64 ;; x86_64) arch=x86_64 ;; *) echo "unknown arch $(uname -m)"; exit 1 ;; esac
export MACOSX_DEPLOYMENT_TARGET="${MACOSX_DEPLOYMENT_TARGET:-11.0}"
work="$PWD/target/tessh-macos"
rm -rf "$work"; mkdir -p "$work/pylib" "$work/root/bin" dist

# libpython to link against (uv's own CPython is linked in at run time, any 3.12+).
pbs="https://github.com/astral-sh/python-build-standalone/releases/download/20261123/cpython-3.12.16%2B20261123-${arch}-apple-darwin-install_only_stripped.tar.gz"
curl -fsSL "$pbs" | tar xz -C "$work"
cp "$work/python/lib/libpython3.12.dylib" "$work/pylib/libpython3.dylib"
chmod u+w "$work/pylib/libpython3.dylib"
install_name_tool -id @rpath/libpython3.dylib "$work/pylib/libpython3.dylib"
cat > "$work/pyo3.txt" <<EOF
implementation=CPython
version=3.12
shared=true
abi3=true
lib_name=python3
lib_dir=$work/pylib
pointer_width=64
suppress_build_script_link_lines=false
EOF

# tessh (its own binary first: the launcher's is also called tessh).
# (Full LTO, one codegen unit, as Linux's release files.)
PYO3_CONFIG_FILE="$work/pyo3.txt" CARGO_PROFILE_RELEASE_LTO=fat CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 \
  cargo build --release --locked -p tessh --no-default-features --features standalone
cp target/release/tessh "$work/root/bin/tessh"
strip -x "$work/root/bin/tessh"
otool -L "$work/root/bin/tessh"

# uv.
curl -fsSL "https://github.com/astral-sh/uv/releases/download/0.13.9/uv-${arch}-apple-darwin.tar.gz" | tar xz -C "$work"
cp "$work/uv-${arch}-apple-darwin/uv" "$work/root/bin/uv"
chmod 755 "$work/root/bin/"*

# The CPython it runs with (its tests, IDLE and Tk left out).
py=3.15.2
curl -fsSL "https://github.com/astral-sh/python-build-standalone/releases/download/20261123/cpython-${py}%2B20261123-${arch}-apple-darwin-install_only_stripped.tar.gz" | tar xz -C "$work/root"
(cd "$work/root/python/lib" && rm -rf python3.*/test python3.*/idlelib python3.*/tkinter python3.*/turtledemo python3.*/turtle.py \
  python3.*/lib-dynload/_tkinter* python3.*/lib-dynload/*_test*.so tcl* tk* itcl* thread* libtcl* libtk* pkgconfig && find . -name '*.a' -delete)
rm -rf "$work/root/python/share" "$work/root/python/bin" "$work/root/python/lib"/python3.*/site-packages/pip*
echo "cpython-${py}-macos-${arch}-none" > "$work/root/pyname"
echo none > "$work/root/libc"

# The payload, built into the launcher.
(cd "$work/root" && tar -cf - --uid 0 --gid 0 .) | zstd -19 --long=27 -T0 -q > "$work/payload.tar.zst"
# (The launcher built for size, as the flake's.)
TESSH_PAYLOAD="$work/payload.tar.zst" CARGO_PROFILE_RELEASE_OPT_LEVEL=z CARGO_PROFILE_RELEASE_LTO=fat \
  CARGO_PROFILE_RELEASE_CODEGEN_UNITS=1 CARGO_PROFILE_RELEASE_PANIC=abort CARGO_PROFILE_RELEASE_STRIP=symbols \
  cargo build --release --locked -p tessh-launcher
out="dist/tessh-${arch}-macos"
cp target/release/tessh "$out"
# (Ad hoc: Apple Silicon runs nothing unsigned.)
codesign --force --sign - "$out"
codesign --verify --strict "$out"
ls -l "$out"
