# Homebrew cask. This repo is its own tap: brew tap teochenglim/deepseek-pet https://github.com/teochenglim/deepseek-pet
# The release workflow re-renders this file from packaging/homebrew/deepseek-pet.rb after each tag.
cask "deepseek-pet" do
  version "__VERSION__"
  sha256 "__SHA256__"

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
