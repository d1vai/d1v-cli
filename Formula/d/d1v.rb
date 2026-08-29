class D1v < Formula
  desc "Command-line interface for d1v.ai"
  homepage "https://d1v.ai"
  version "0.1.26"
  license "MIT"

  on_arm do
    url "https://github.com/d1vai/d1v-cli/releases/download/v#{version}/d1v-aarch64-apple-darwin.tar.gz"
    sha256 "REPLACE_AARCH64_SHA256"
  end

  on_intel do
    url "https://github.com/d1vai/d1v-cli/releases/download/v#{version}/d1v-x86_64-apple-darwin.tar.gz"
    sha256 "REPLACE_X86_64_SHA256"
  end

  def install
    bin.install "d1v"
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/d1v --version")
  end
end
