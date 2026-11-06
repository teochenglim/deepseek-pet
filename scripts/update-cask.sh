#!/usr/bin/env bash
# Render the Homebrew cask for a released dmg. Usage: scripts/update-cask.sh X.Y.Z path/to/DeepseekPet_X.Y.Z_aarch64.dmg > deepseek-pet.rb
set -euo pipefail
cd "$(dirname "$0")/.."
VER="${1:?version}"; DMG="${2:?dmg path}"
SHA=$(shasum -a 256 "$DMG" | cut -d' ' -f1)
sed "s/__VERSION__/$VER/; s/__SHA256__/$SHA/" packaging/homebrew/deepseek-pet.rb
