cask "deepseek-pet" do
  version "0.8.4"
  sha256 "332a501fe586548fd9c0afca97e3aa17cd62c321bb54989aa111f4b55a3485a6"

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
