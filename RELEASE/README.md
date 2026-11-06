# Release plan

Roll-up: [`../RELEASE.md`](../RELEASE.md).

| Release | Theme | Exit gate | Depends on |
|---|---|---|---|
| [v0.1.0](v0.1.0.md) | Skeleton + static fish | transparent on-top fish still; Stop hook event logged | - |
| [v0.2.0](v0.2.0.md) | Asset pipeline | `tools/convert-assets.py` yields idle/running-right/failed frames | 0.1.0 |
| [v0.3.0](v0.3.0.md) | Claude Code + Z.ai | both agents' hooks switch state | 0.2.0 |
| [v0.6.0](v0.6.0.md) | Play any animation | 106 clips playable from the menu-bar icon | 0.5.0 |
| [v0.5.0](v0.5.0.md) | Dock, menu bar, floating | app in Dock and menu bar, floats on every Space | 0.4.1 |
| [v0.4.1](v0.4.1.md) | CI fixes | `make check`/`make audit` green, release publishes a dmg | 0.4.0 |
| [v0.4.0](v0.4.0.md) | Interaction + priority | drag/click/hover/menu, priority rules | 0.3.0 |
| [v1.0.0](v1.0.0.md) | Stable | signed dmg via CI, Cask, 15+ states | 0.4.0 |

SemVer. Patches are fixes without AC change.
