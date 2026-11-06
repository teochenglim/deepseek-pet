# Cask template. Copy to Casks/deepseek-pet.rb in the tap repo
# github.com/teochenglim/homebrew-deepseek-pet (the release workflow fills version and sha256).
cask "deepseek-pet" do
  version "__VERSION__"
  sha256 "__SHA256__"

  url "https://github.com/teochenglim/deepseek-pet/releases/download/v#{version}/DeepseekPet_#{version}_aarch64.dmg"
  name "deepseek-pet"
  desc "Desktop pet driven by Claude Code and Z.ai hooks"
  homepage "https://github.com/teochenglim/deepseek-pet"

  depends_on arch: :arm64
  depends_on macos: ">= :ventura"

  app "DeepseekPet.app"

  # Unsigned/ad-hoc builds are quarantined by Gatekeeper. Remove this stanza once notarized.
  postflight do
    system_command "/usr/bin/xattr", args: ["-dr", "com.apple.quarantine", "#{appdir}/DeepseekPet.app"]
  end

  zap trash: "~/.deepseek-pet"
end
