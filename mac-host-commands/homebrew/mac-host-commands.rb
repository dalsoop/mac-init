class MacHostCommands < Formula
  desc "Mac host management CLI — Proxmox, Synology, TrueNAS integration"
  homepage "https://github.com/dalsoop/mac-host-commands"
  url "https://github.com/dalsoop/mac-host-commands.git", tag: "v0.1.0"
  license "MIT"

  depends_on "rust" => :build
  depends_on "macfuse"
  depends_on "sshpass"

  def install
    cd "cli" do
      system "cargo", "install", *std_cargo_args
    end

    # Install scripts
    (libexec/"scripts").install Dir["scripts/*"]

    # Install CUE schemas
    (share/"mac-host-commands/cue").install Dir["cue/*"]
  end

  def post_install
    ohai "Run 'mac-host-commands init' to complete setup"
  end

  test do
    assert_match "Mac 호스트 관리 도구", shell_output("#{bin}/mac-host-commands --help")
  end
end
