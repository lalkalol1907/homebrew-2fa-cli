# 2fa

Generate TOTP codes in the terminal on macOS. Secrets are stored in macOS
Keychain; `~/.config/2fa/accounts.json` contains account names only.
Accepts base32 secrets, `otpauth://` URLs, and PNG/JPEG QR images, including
base64 image data URIs. Copy a code to the clipboard with `-c`.

## Homebrew

After the first release tag and formula have been published:

```sh
brew install lalkalol1907/2fa-cli/twofa
```

The formula is named `twofa`; the executable is `2fa`. This is a third-party
tap, so installing it does not require acceptance into `homebrew/core`.
The binary release formula downloads a ready-made executable for Apple Silicon
or Intel and installs Bash completions. It has no Rust or LLVM dependencies.
Requires macOS 14 (Sonoma) or newer. The source formula remains active until
the first binary release workflow successfully delivers the generated formula.

```sh
2fa add github                 # Prompts for a secret without echoing it
pbpaste | 2fa add github       # Read a secret or otpauth URL from stdin
2fa add github qr.png          # Read a QR image
2fa                           # Show all codes
2fa github -c                 # Show and copy one code
2fa list
2fa rm github
2fa --version
```

Prefer the secret prompt or stdin over passing secrets as command arguments,
which may be recorded in shell history. Homebrew's Bash completion loader can
load the installed completion; alternatively add
`eval "$(2fa completions bash)"` to `~/.bashrc`.

## Build and verify

```sh
cargo build --release --locked
cargo test --locked
cargo fmt --check
cargo clippy --locked --all-targets -- -D warnings
```

CI checks Apple Silicon and Intel macOS, plus builds and tests the Homebrew
formula against the current checkout. The formula test uses a temporary account
index and does not write secrets to Keychain.

## Release process

1. Update `Cargo.toml` and `Cargo.lock` to the new version with your changes.
2. Merge into `master`.
3. Create and push the matching tag, for example **v0.1.1**.

That is all the release author needs to do. The Binary release workflow:

- Checks that the tag belongs to master and matches the Cargo version.
- Builds, tests and packages Apple Silicon and Intel binaries.
- Installs both packages through Homebrew and runs the formula's functional tests.
- Publishes the binary archives, SHA-256 checksums and formula in a GitHub Release.
- Automatically commits the generated `Formula/twofa.rb` to master using
  GitHub Actions' built-in token. The formula downloads binaries without Rust or LLVM.

No local release scripts, manual formula commits or release PRs are required.
The generated commit changes only the formula. Delivery retries re-read its
current SHA, and an older release cannot replace a newer formula. Re-running
an already published release reuses its published formula and does not replace
its binaries. Never move a published tag.

The workflow requests `contents: write`. The repository must allow GitHub Actions
updates to `Formula/twofa.rb` on master. If branch protection requires all changes
to go through a PR, configure an automation actor with permission to bypass that
rule; the ordinary built-in token cannot bypass protected branch rules. Otherwise
binary publication succeeds but formula delivery fails visibly in Actions.

Existing users get the new binary with `brew update` and `brew upgrade twofa`.
This repository is a custom tap; delivery does not depend on `homebrew/core`.

CI uses `scripts/` to package binaries, generate the checksummed formula and
deliver it to master. Archives include the binary, MIT license, version marker
and Bash completions. Packaging rejects non-system dynamic library dependencies.

## License

MIT; see [LICENSE](LICENSE).
