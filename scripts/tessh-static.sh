#!/usr/bin/env bash
# The static tessh, for a release: one file that runs on any x86-64 Linux
# (flake.nix: tessh-static; tessh/launcher). Builds it in its own memory-capped
# scope, checks it's static, and copies it to dist/tessh-x86_64-linux.
set -euo pipefail
cd "$(dirname "$0")/.."
out=$(systemd-run --user --scope -q -p MemoryMax=16G nix build .#tessh-static --no-link --print-out-paths)
mkdir -p dist
install -m755 "$out/bin/tessh" dist/tessh-x86_64-linux
if ldd dist/tessh-x86_64-linux 2>&1 | grep -qv 'not a dynamic executable\|statically linked'; then
  echo "dist/tessh-x86_64-linux isn't static:" >&2
  ldd dist/tessh-x86_64-linux >&2
  exit 1
fi
ls -la dist/tessh-x86_64-linux
sha256sum dist/tessh-x86_64-linux
