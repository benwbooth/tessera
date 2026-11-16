# Homebrew cask template; the Release workflow fills it in and pushes it to
# the homebrew-tap repository as Casks/tessera.rb.
cask "tessera" do
  arch arm: "arm64", intel: "x86_64"

  version "@VERSION@"
  sha256 arm:   "@SHA_ARM@",
         intel: "@SHA_INTEL@"

  url "https://github.com/@REPO@/releases/download/@TAG@/Tessera-#{version}-macos-#{arch}.dmg"
  name "Tessera"
  desc "A terminal in the spirit of iTerm2"
  homepage "https://github.com/@REPO@"

  depends_on macos: ">= :ventura"

  app "Tessera.app"

  # The app is signed ad hoc, not notarized: let macOS open it.
  postflight do
    system_command "/usr/bin/xattr",
                   args: ["-dr", "com.apple.quarantine", "#{appdir}/Tessera.app"]
  end

  zap trash: [
    "~/.config/tessera",
    "~/.local/state/tessera",
  ]
end
