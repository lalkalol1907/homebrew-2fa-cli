# 2fa

Generate TOTP codes in the terminal on macOS. Secrets are stored in macOS
Keychain; `~/.config/2fa/accounts.json` contains account names only.
Accepts base32 secrets, `otpauth://` URLs, and PNG/JPEG QR images, including
base64 image data URIs. Copy a code to the clipboard with `-c`.

## Homebrew

After the first release tag and formula have been published:

```sh
brew tap lalkalol1907/2fa-cli https://github.com/lalkalol1907/2fa-cli
brew install lalkalol1907/2fa-cli/twofa
```

The formula is named `twofa`; the executable is `2fa`. This is a third-party
tap, so installing it does not require acceptance into `homebrew/core`.
The binary release formula downloads a ready-made executable for Apple Silicon
or Intel and installs Bash completions. It has no Rust or LLVM dependencies.
Requires macOS 14 (Sonoma) or newer. The source formula remains active until
the binary release is published and `scripts/update-homebrew.sh` is run.

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

## Publish a binary Homebrew release

The repository itself is the tap; no second GitHub repository is needed.
The existing tags remain unchanged. The next version is **0.1.1**.

1. Commit and push these changes yourself, including `Cargo.lock`.
2. Create and push **v0.1.1** on that commit. The Binary release workflow builds
   and tests Apple Silicon and Intel binaries, then publishes both archives,
   their checksums and a generated `twofa.rb` in a GitHub Release.
3. Wait for Binary release to finish successfully, then run:

   ```sh
   bash scripts/update-homebrew.sh
   ```

   This downloads both published archives, checks their versions, and replaces
   `Formula/twofa.rb` with the binary formula containing their SHA-256 checksums.
   It removes the Rust build dependency. It never commits or pushes changes.
4. Commit and push the updated formula, then verify:

   ```sh
   brew update
   brew upgrade lalkalol1907/2fa-cli/twofa
   brew test lalkalol1907/2fa-cli/twofa
   ```

For a fresh installation, use the tap and install commands above. Do not pass
`--build-from-source`. Publishing the release alone does not update the tap;
step 4 is required. Existing Rust/LLVM installations are not removed by upgrading
2fa; `brew autoremove` can remove dependencies no longer required by any package.

For later releases, update the Cargo version and lockfile, publish a matching
tag, then repeat steps 3–4. Never move a published tag. The binary formula is
for this custom tap; it is not a source-built `homebrew/core` submission.

To build a local native archive for inspection:

```sh
bash scripts/package-release.sh
```

Archives include the binary, MIT license, version marker and Bash completions.
Packaging rejects non-system dynamic library dependencies.

## License

MIT; see [LICENSE](LICENSE).
