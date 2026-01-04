class CommitLsp < Formula
  desc "LSP Server for providing linting and autocompletion for git commit messages"
  homepage "https://github.com/texel-sensei/commit-lsp"
  url "https://github.com/texel-sensei/commit-lsp/archive/refs/tags/v0.2.0.tar.gz"
  sha256 "cb49ee98375f78be4359bb62e2b970ce3bdc7f8cb3ae17b6e8ed9b9ddd2e75d2"
  license "MIT"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args
  end

  test do
    system "#{bin}/commit-lsp", "--version"
  end
end
