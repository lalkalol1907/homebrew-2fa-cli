class Twofa < Formula
  desc "Generate TOTP 2FA codes from the terminal"
  homepage "https://github.com/lalkalol1907/2fa-cli"
  version "1.0.0"
  license "MIT"

  on_macos do
    on_arm do
      url "https://github.com/lalkalol1907/2fa-cli/releases/download/v1.0.0/2fa-1.0.0-aarch64-apple-darwin.tar.gz"
      sha256 "3321b6b2709cb270b188d2a72c27203f6b9c3e54a156266d7916007dcbfe18e7"
    end
    on_intel do
      url "https://github.com/lalkalol1907/2fa-cli/releases/download/v1.0.0/2fa-1.0.0-x86_64-apple-darwin.tar.gz"
      sha256 "c53b5613f29b36f72d4c0f3206fa1e78d402e4f7d7ea77323b2fd26b47dc78f8"
    end
  end

  depends_on macos: :sonoma

  def install
    bin.install "2fa"
    bash_completion.install "completions/2fa.bash" => "2fa"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/2fa --version")
    assert_match "Generate TOTP", shell_output("#{bin}/2fa --help")
    (testpath/".config/2fa/accounts.json").write('{"accounts":["brew-test"]}')
    assert_equal "brew-test\n", shell_output("TWOFA_COMPLETE=1 #{bin}/2fa")
    assert_match "not found", shell_output("#{bin}/2fa missing-account 2>&1", 1)
    assert_match "complete -F _2fa 2fa", shell_output("#{bin}/2fa completions bash")
  end
end
