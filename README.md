# deepseek-pet

A macOS-native, lightweight desktop pet driven by Claude Code and GLM (Z.ai) hooks. Powered by the Big Blue Fish VP9-alpha animation set, rendered through a Tauri v2 frame pipeline.

**Status:** pre-0.1.0 scaffold. See [RELEASE.md](RELEASE.md) and [PRD.md](PRD.md).

## Try it (once 0.1.0 is built)

```bash
make build          # tauri build -> .app
make run            # dev run
```

Hook setup: see [docs/hooks/claude-settings.example.json](docs/hooks/claude-settings.example.json).

## Credits

Code skeleton from `golitter/coding-agent-pet`; animations from `Bai-Cute/big-blue-fish-desktop-pet`. See [NOTICE](NOTICE).

## Install

Homebrew (Apple Silicon, once a release exists):

```bash
brew install --cask teochenglim/deepseek-pet/deepseek-pet
```

Or download `DeepseekPet_<version>_aarch64.dmg` from the GitHub Releases page and drag the app to Applications. Builds are not notarized yet: first launch needs right-click, Open (the cask clears the quarantine flag for you).
