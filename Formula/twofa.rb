# Bootstrap source formula. The tag release workflow automatically replaces it
# with the checksummed binary formula after building and testing both architectures.
class Twofa < Formula
  desc "Generate TOTP 2FA codes from the terminal"
  homepage "https://github.com/lalkalol1907/2fa-cli"
  url "https://github.com/lalkalol1907/2fa-cli.git", tag: "v0.1.0"
  license "MIT"
  head "https://github.com/lalkalol1907/2fa-cli.git", branch: "master"

  depends_on "rust" => :build
  depends_on :macos

  def install
    system "cargo", "install", *std_cargo_args
    bash_completion.install "completions/2fa.bash" => "2fa"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/2fa --version") unless build.head?
    assert_match "Generate TOTP", shell_output("#{bin}/2fa --help")

    # Exercise account enumeration without accessing the real Keychain.
    (testpath/".config/2fa/accounts.json").write('{"accounts":["brew-test"]}')
    assert_equal "brew-test\n", shell_output("TWOFA_COMPLETE=1 #{bin}/2fa")
    assert_match "not found", shell_output("#{bin}/2fa missing-account 2>&1", 1)
    assert_match "complete -F _2fa 2fa", shell_output("#{bin}/2fa completions bash")
  end
end
