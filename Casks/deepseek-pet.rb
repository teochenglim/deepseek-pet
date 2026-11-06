cask "deepseek-pet" do
  version "0.8.5"
  sha256 "92ae435e72fcd27c20e1a1e7d4ca9e4aca5341782ec1cc78fdc1e5f256d1f462"

  url "https://github.com/teochenglim/deepseek-pet/releases/download/v#{version}/DeepseekPet_#{version}_aarch64.dmg"
  name "deepseek-pet"
  desc "Desktop pet driven by Claude Code and Z.ai hooks"
  homepage "https://github.com/teochenglim/deepseek-pet"

  depends_on arch: :arm64
  depends_on macos: :ventura

  app "DeepseekPet.app"

  # Unsigned/ad-hoc builds are quarantined by Gatekeeper. Remove this stanza once notarized.
  postflight_steps do
    run "/usr/bin/xattr", args: ["-dr", "com.apple.quarantine", "{{appdir}}/DeepseekPet.app"],
                          writable_paths: ["{{appdir}}/DeepseekPet.app"], must_succeed: false
  end

  zap trash: "~/.deepseek-pet"
end
