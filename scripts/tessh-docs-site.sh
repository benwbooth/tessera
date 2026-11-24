#!/usr/bin/env bash
# tessh's documentation as a site, made by Tessera's own Guide renderer so it
# reads as the Guide tab does (contents, highlighting, bash/tessh tabs):
#   scripts/tessh-docs-site.sh [OUT]   (OUT: docs/tessh, served at /tessh/)
# TESSERA: the tessera binary to render with (default: tessera on PATH).
# The Pages workflow serves docs/ as the site: run this and commit docs/tessh
# when the guides change.
set -euo pipefail
root=$(cd "$(dirname "$0")/.." && pwd)
out=${1:-$root/docs/tessh}
tessera=${TESSERA:-tessera}
head=$root/tessh/doc/site/head.html
mkdir -p "$out"
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

# page LANG FILE TITLE: one guide as a page, the two guides linked at the top
# of the contents.
page() {
  local lang=$1 file=$2 title=$3
  "$tessera" --guide-html "$lang" "$work/$lang.html"
  local on_g='' on_a=''
  if [ "$lang" = tessh ]; then on_g=' class="on"'; else on_a=' class="on"'; fi
  local switch="<div class='home'><a href='https://github.com/benwbooth/tessera/tree/main/tessh'>tessh on GitHub</a></div><div class='guides'><a href='index.html'$on_g>Guide</a><a href='advanced.html'$on_a>Advanced</a></div>"
  {
    printf '<!doctype html><html lang="en"><head><meta charset="utf-8"><meta name="viewport" content="width=device-width, initial-scale=1"><title>%s</title>\n' "$title"
    cat "$head"
    printf '</head><body>'
    sed "0,/<nav>/s||<nav>$switch|" "$work/$lang.html"
    printf '</body></html>\n'
  } > "$out/$file"
}

page tessh index.html "The tessh guide"
page tessh-advanced advanced.html "Advanced tessh Scripting Guide"
echo "tessh docs: $out"
