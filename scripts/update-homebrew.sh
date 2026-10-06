#!/bin/bash
# Download published binaries and generate the formula. No Git changes.
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
version="$(sed -n 's/^version = "\([^" ]*\)"$/\1/p' "$root/Cargo.toml" | head -n 1)"
tag="${1:-v$version}"
[[ "$tag" == "v$version" ]] || { echo "Expected tag v$version" >&2; exit 1; }
base="https://github.com/lalkalol1907/2fa-cli/releases/download/$tag"
archives="$(mktemp -d -t twofa-release.XXXXXX)"
trap 'rm -rf "$archives"' EXIT
for target in aarch64-apple-darwin x86_64-apple-darwin; do
  filename="2fa-$version-$target.tar.gz"
  curl --fail --location --retry 3 --output "$archives/$filename" "$base/$filename"
done
ruby "$root/scripts/generate-homebrew.rb" "$archives" "$root/Formula/twofa.rb" "$base"
