#!/usr/bin/env bash
# tessh's documentation as a site: scripts/tessh-docs-site.sh OUT
# The guide, the language reference and the Advanced tessh Scripting Guide,
# as an mdBook (contents, search, themes), its tessh code coloured by tessh's
# own highlighting (tree-sitter-tessh's md_html example). The Pages workflow
# puts it at /tessh/. Needs cargo and mdbook.
set -euo pipefail
out=$(realpath -m "${1:?usage: tessh-docs-site.sh OUT}")
root=$(cd "$(dirname "$0")/.." && pwd)
work=$(mktemp -d)
trap 'rm -rf "$work"' EXIT

cargo build --release --quiet --manifest-path "$root/Cargo.toml" -p tree-sitter-tessh --example md_html
hl=${CARGO_TARGET_DIR:-$root/target}/release/examples/md_html

cp "$root/tessh/doc/site/book.toml" "$root/tessh/doc/site/tessh.css" "$work/"
src=$work/src
mkdir -p "$src/advanced"
# (Links as the site has them: the guide, the reference and the chapters side by side.)
sed -e 's|(doc/guide.md)|(guide.md)|g' -e 's|(\.\./docs/TESSH.md)|(reference.md)|g' \
    -e 's|(doc/advanced/|(advanced/|g' -e 's|(\.\./README.md)|(https://github.com/benwbooth/tessera)|g' \
    "$root/tessh/README.md" | "$hl" > "$src/README.md"
"$hl" < "$root/tessh/doc/guide.md" > "$src/guide.md"
sed -e 's|(\.\./tessh/doc/guide.md)|(guide.md)|g' "$root/docs/TESSH.md" | "$hl" > "$src/reference.md"
for f in "$root"/tessh/doc/advanced/*.md; do
  "$hl" < "$f" > "$src/advanced/$(basename "$f")"
done

# The contents: the chapters in the guide's order, each by its first heading.
{
  echo "# Summary"
  echo
  echo "[tessh](README.md)"
  echo
  echo "- [The tessh guide](guide.md)"
  echo "- [Language reference](reference.md)"
  echo "- [Advanced tessh Scripting Guide](advanced/00-introduction.md)"
  for f in $(cd "$root/tessh/doc/advanced" && ls *.md | LC_ALL=C sort | grep -v '^00-' | awk '/^[0-9]/{print; next} {rest=rest $0 "\n"} END{printf "%s", rest}'); do
    t=$(head -1 "$root/tessh/doc/advanced/$f" | sed 's/^#* *//')
    echo "  - [$t](advanced/$f)"
  done
} > "$src/SUMMARY.md"

mdbook build "$work" --dest-dir "$out"
echo "tessh docs: $out"
