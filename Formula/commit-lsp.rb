class CommitLsp < Formula
  desc "LSP Server for providing linting and autocompletion for git commit messages"
  homepage "https://github.com/texel-sensei/commit-lsp"
  url "https://github.com/texel-sensei/commit-lsp/archive/refs/tags/v0.2.1.tar.gz"
  sha256 "bdf753d4b0ac047165df033f1e7b3ae84ea2f25c92a249790e44751e2c4312af"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    system "#{bin}/commit-lsp", "--version"
  end
end
