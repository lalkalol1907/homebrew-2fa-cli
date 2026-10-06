#!/bin/bash
# Update the formula from a published release; never commits or pushes changes.
set -euo pipefail

root="$(cd "$(dirname "$0")/.." && pwd)"
version="$(sed -n 's/^version = "\([^"]*\)"$/\1/p' "$root/Cargo.toml" | head -n 1)"
tag="${1:-v$version}"
if [[ "$tag" != "v$version" ]]; then
  echo "Tag must match Cargo.toml: v$version" >&2
  exit 1
fi
url="https://github.com/lalkalol1907/2fa-cli/archive/refs/tags/$tag.tar.gz"
archive="$(mktemp -t twofa-release.XXXXXX)"
trap 'rm -f "$archive"' EXIT
curl --fail --location --retry 3 --output "$archive" "$url"
tar -tzf "$archive" >/dev/null
sha="$(shasum -a 256 "$archive" | cut -d ' ' -f 1)"
if ! tar -xOzf "$archive" "2fa-cli-${tag#v}/Cargo.toml" | grep -Fxq "version = \"$version\""; then
  echo "Archive version does not match Cargo.toml" >&2
  exit 1
fi
formula="$root/Formula/twofa.rb"
if ! grep -q '^  url ' "$formula"; then
  echo "Formula URL is missing" >&2
  exit 1
fi
updated="$(mktemp -t twofa-formula.XXXXXX)"
trap 'rm -f "$archive" "$updated"' EXIT
awk -v url="$url" -v sha="$sha" '
  /^  url / { print "  url \"" url "\""; print "  sha256 \"" sha "\""; next }
  /^  sha256 / { next }
  { print }
' "$formula" > "$updated"
cat "$updated" > "$formula"
echo "Updated $formula for $tag (SHA-256: $sha)"
