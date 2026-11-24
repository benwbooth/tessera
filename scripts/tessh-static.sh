#!/usr/bin/env bash
# The static tessh, for a release: two files for this machine's architecture
# (flake.nix: tessh-static-musl and tessh-static-gnu; tessh/launcher), each
# with its tessh, a CPython and uv inside. Built in their own memory-capped
# scope, checked to be static, copied to dist/tessh-$arch-linux-{musl,gnu}.
#
#   scripts/tessh-static.sh [musl|gnu]…     (both when none is named)
set -euo pipefail
cd "$(dirname "$0")/.."
arch=$(uname -m)
mkdir -p dist
for libc in "${@:-musl gnu}"; do
  for l in $libc; do
    out=$(systemd-run --user --scope -q -p MemoryMax=16G nix build ".#tessh-static-$l" --no-link --print-out-paths)
    f=dist/tessh-$arch-linux-$l
    install -m755 "$out/bin/tessh" "$f"
    if ldd "$f" 2>&1 | grep -qv 'not a dynamic executable\|statically linked'; then
      echo "$f isn't static:" >&2
      ldd "$f" >&2
      exit 1
    fi
    ls -la "$f"
    sha256sum "$f"
  done
done
