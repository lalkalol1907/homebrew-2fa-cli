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
Homebrew builds from source and installs Bash completions. Rust is a build
dependency; it is not needed to run the installed executable.

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

## Publish the first Homebrew release

Run these steps yourself after reviewing the changes. The repository itself
is the tap, so no second GitHub repository is needed.

1. Commit the source, MIT license, README, formula, scripts, and CI.
2. Tag that commit as `v0.1.0` and push the commit and tag to GitHub.
3. Run `bash scripts/update-homebrew.sh`. It downloads the tagged archive,
   verifies its version, and updates the formula URL and SHA-256.
4. Commit and push the updated formula.
5. Verify the published package:

   ```sh
   brew tap lalkalol1907/2fa-cli https://github.com/lalkalol1907/2fa-cli
   brew install --build-from-source lalkalol1907/2fa-cli/twofa
   brew test lalkalol1907/2fa-cli/twofa
   brew audit --strict lalkalol1907/2fa-cli/twofa
   ```

Before step 3, the formula references the release Git tag. Once updated, it
uses a checksummed release archive. Do not move a published tag. For later
releases, update the Cargo version and lockfile, publish a matching tag,
then run `bash scripts/update-homebrew.sh` and commit the updated formula.

For inclusion in `homebrew/core`, publish a stable release and follow the
[Homebrew acceptance policy](https://docs.brew.sh/Acceptable-Formulae) and
[submission guide](https://docs.brew.sh/Adding-Software-to-Homebrew).
Acceptance is decided by Homebrew maintainers. The custom tap is usable
independently of that process.

## License

MIT; see [LICENSE](LICENSE).
