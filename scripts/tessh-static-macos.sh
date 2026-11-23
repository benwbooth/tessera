#!/usr/bin/env bash
# The single-file tessh for macOS (CI's macOS runners; also by hand on a
# Mac): tessh against Python's stable ABI, with uv, built into the launcher
# (a signed program can't have data appended, as Linux's has). The Python
# comes from uv at the first run, as on Linux. Output: dist/tessh-<arch>-macos.
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
PYO3_CONFIG_FILE="$work/pyo3.txt" cargo build --release --locked -p tessh --features standalone
cp target/release/tessh "$work/root/bin/tessh"
otool -L "$work/root/bin/tessh"

# uv.
curl -fsSL "https://github.com/astral-sh/uv/releases/download/0.13.9/uv-${arch}-apple-darwin.tar.gz" | tar xz -C "$work"
cp "$work/uv-${arch}-apple-darwin/uv" "$work/root/bin/uv"
chmod 755 "$work/root/bin/"*

# The payload, built into the launcher.
(cd "$work/root" && tar -cf - --uid 0 --gid 0 bin) | zstd -19 -q > "$work/payload.tar.zst"
TESSH_PAYLOAD="$work/payload.tar.zst" cargo build --release --locked -p tessh-launcher
out="dist/tessh-${arch}-macos"
cp target/release/tessh "$out"
# (Ad hoc: Apple Silicon runs nothing unsigned.)
codesign --force --sign - "$out"
codesign --verify --strict "$out"
ls -l "$out"
