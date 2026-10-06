#!/usr/bin/env ruby
require "digest"
require "fileutils"
require "open3"

root = File.expand_path("..", __dir__)
version = File.read("#{root}/Cargo.toml")[/^version = "([^"]+)"$/, 1]
archive_dir, output, base_url = ARGV
abort "Usage: ruby scripts/generate-homebrew.rb ARCHIVE_DIR OUTPUT [BASE_URL]" unless archive_dir && output
base_url ||= "https://github.com/lalkalol1907/2fa-cli/releases/download/v#{version}"
values = %w[aarch64-apple-darwin x86_64-apple-darwin].to_h do |target|
  filename = "2fa-#{version}-#{target}.tar.gz"
  path = File.join(archive_dir, filename)
  abort "Missing release archive: #{path}" unless File.file?(path)
  stored_version, status = Open3.capture2("tar", "-xOzf", path, "VERSION")
  abort "Archive version mismatch: #{filename}" unless status.success? && stored_version.strip == version
  [target, ["#{base_url}/#{filename}", Digest::SHA256.file(path).hexdigest]]
end
arm_url, arm_sha = values.fetch("aarch64-apple-darwin")
intel_url, intel_sha = values.fetch("x86_64-apple-darwin")
formula = <<~RUBY
  class Twofa < Formula
    desc "Generate TOTP 2FA codes from the terminal"
    homepage "https://github.com/lalkalol1907/2fa-cli"
    version "#{version}"
    license "MIT"

    on_macos do
      on_arm do
        url "#{arm_url}"
        sha256 "#{arm_sha}"
      end
      on_intel do
        url "#{intel_url}"
        sha256 "#{intel_sha}"
      end
    end

    depends_on macos: :sonoma

    def install
      bin.install "2fa"
      bash_completion.install "completions/2fa.bash" => "2fa"
    end

    test do
      assert_match version.to_s, shell_output("\#{bin}/2fa --version")
      assert_match "Generate TOTP", shell_output("\#{bin}/2fa --help")
      (testpath/".config/2fa/accounts.json").write('{"accounts":["brew-test"]}')
      assert_equal "brew-test\\n", shell_output("TWOFA_COMPLETE=1 \#{bin}/2fa")
      assert_match "not found", shell_output("\#{bin}/2fa missing-account 2>&1", 1)
      assert_match "complete -F _2fa 2fa", shell_output("\#{bin}/2fa completions bash")
    end
  end
RUBY
FileUtils.mkdir_p(File.dirname(File.expand_path(output)))
File.write(output, formula)
puts "Generated #{output} for #{version}; no compiler dependencies"
