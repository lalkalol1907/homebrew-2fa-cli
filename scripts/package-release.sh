#!/bin/bash
set -euo pipefail
root="$(cd "$(dirname "$0")/.." && pwd)"
cd "$root"
version="$(sed -n 's/^version = "\([^"]*\)"$/\1/p' Cargo.toml | head -n 1)"
case "$(uname -m)" in
  arm64) target=aarch64-apple-darwin ;;
  x86_64) target=x86_64-apple-darwin ;;
  *) echo "Unsupported architecture" >&2; exit 1 ;;
esac
[[ "$(uname -s)" == Darwin ]] || { echo "macOS is required" >&2; exit 1; }
export MACOSX_DEPLOYMENT_TARGET=14.0
cargo build --release --locked
[[ "$(target/release/2fa --version)" == "2fa $version" ]]
if otool -L target/release/2fa | tail -n +2 | grep -Ev '^\s*(/usr/lib/|/System/Library/)' | grep -q .; then
  echo "Binary has non-system dynamic library dependencies" >&2
  otool -L target/release/2fa >&2
  exit 1
fi
stage="$(mktemp -d -t twofa-package.XXXXXX)"
trap 'rm -rf "$stage"' EXIT
cp target/release/2fa LICENSE "$stage/"
cp -R completions "$stage/"
printf '%s\n' "$version" > "$stage/VERSION"
mkdir -p dist
archive="$root/dist/2fa-$version-$target.tar.gz"
COPYFILE_DISABLE=1 tar -czf "$archive" -C "$stage" 2fa LICENSE VERSION completions
shasum -a 256 "$archive"
