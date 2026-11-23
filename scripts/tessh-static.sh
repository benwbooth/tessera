#!/usr/bin/env bash
# The static tessh, for a release: one file that runs on any Linux of this
# machine's architecture
# (flake.nix: tessh-static; tessh/launcher). Builds it in its own memory-capped
# scope, checks it's static, and copies it to dist/tessh-$arch-linux.
set -euo pipefail
cd "$(dirname "$0")/.."
arch=$(uname -m)
out=$(systemd-run --user --scope -q -p MemoryMax=16G nix build .#tessh-static --no-link --print-out-paths)
mkdir -p dist
install -m755 "$out/bin/tessh" dist/tessh-$arch-linux
if ldd dist/tessh-$arch-linux 2>&1 | grep -qv 'not a dynamic executable\|statically linked'; then
  echo "dist/tessh-$arch-linux isn't static:" >&2
  ldd dist/tessh-$arch-linux >&2
  exit 1
fi
ls -la dist/tessh-$arch-linux
sha256sum dist/tessh-$arch-linux
