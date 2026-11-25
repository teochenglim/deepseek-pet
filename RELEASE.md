# deepseek-pet release log and summary

> **Current state:** v0.1.0 built and checked; waiting on a manual macOS visual pass. Statuses are updated with the code.

## 1. Summary

macOS desktop pet: Tauri v2 shell forked from coding-agent-pet, Big Blue Fish animations, driven by Claude Code and Z.ai hooks.

| Fact | Value |
|---|---|
| Language / runtime | Rust + Tauri v2, HTML/JS frontend in `desktop/src` |
| Pinned dependency | none (no server) |
| Licence | Apache-2.0 (+ third-party, see NOTICE) |
| Artifacts per tag | macOS arm64 `.app`/`.dmg` (+ `.sha256`), GitHub Release with notes from `RELEASE/<tag>.md` |

## 2. Release status

| Release | Theme | Status | Requirements |
|---|---|---|---|
| [v1.0.0](RELEASE/v1.0.0.md) | Stable release | planned | R-P1, R-D1, R-M1 |
| [v0.9.7](RELEASE/v0.9.7.md) | Show Claude context is a floating window of its own; Show Big Blue Fish always redraws the pet | active | R-X1 |
| [v0.9.6](RELEASE/v0.9.6.md) | The pet never disappears after Show Claude context; Show Big Blue Fish redraws it; note fades after 20 s | active | R-X1 |
| [v0.9.5](RELEASE/v0.9.5.md) | Statusline removed; Show Claude context reads the transcript and speaks in the pet's bubble | active | R-X1 |
| [v0.9.4](RELEASE/v0.9.4.md) | Statusline is just the avatar; Show Claude context is its own feature | active | R-X1 |
| [v0.9.3](RELEASE/v0.9.3.md) | Statusline and context display split into two functions | active | R-X1 |
| [v0.9.2](RELEASE/v0.9.2.md) | Statusline text rows stay aligned in Claude Code | active | R-X1 |
| [v0.9.1](RELEASE/v0.9.1.md) | Sharper statusline avatar, `make statusline-preview` | active | R-X1 |
| [v0.9.0](RELEASE/v0.9.0.md) | Claude Code statusline with the avatar, Show context | active | R-X1 |
| [v0.8.12](RELEASE/v0.8.12.md) | Security scan: cookie 0.18.3, deps to latest | active | R-D1 |
| [v0.8.11](RELEASE/v0.8.11.md) | Security scan: url 2.5.9 | active | R-D1 |
| [v0.8.10](RELEASE/v0.8.10.md) | README install / upgrade / repair split | active | R-D1 |
| [v0.8.9](RELEASE/v0.8.9.md) | Stuck tap (merge-conflict cask) recovery | active | R-D1 |
| [v0.8.8](RELEASE/v0.8.8.md) | Working animation ends when agent goes quiet | active | R-K1 |
| [v0.8.7](RELEASE/v0.8.7.md) | Release flow never diverges | active | R-D1 |
| [v0.8.6](RELEASE/v0.8.6.md) | Homebrew cask deprecation fixes | active | R-D1 |
| [v0.8.5](RELEASE/v0.8.5.md) | Homebrew install fix | active | R-D1 |
| [v0.8.4](RELEASE/v0.8.4.md) | README with animated gallery | active | R-K1 |
| [v0.8.2](RELEASE/v0.8.2.md) | Varied default animations | active | R-K1 |
| [v0.8.3](RELEASE/v0.8.3.md) | English/Chinese button, ZCode animates | active | R-K1 |
| [v0.8.1](RELEASE/v0.8.1.md) | Semgrep gate fix | active | R-D1 |
| [v0.8.0](RELEASE/v0.8.0.md) | More agents, one active source, better default animations | active | R-K2, R-K1 |
| [v0.7.0](RELEASE/v0.7.0.md) | Map animations to hook events | active | R-K1 |
| [v0.6.1](RELEASE/v0.6.1.md) | Standalone app | active | R-D1 |
| [v0.6.0](RELEASE/v0.6.0.md) | Play any animation by hand | active | R-M1, R-I1, R-P1 |
| [v0.5.0](RELEASE/v0.5.0.md) | Dock, menu bar, floating | active | R-W2, R-I1 |
| [v0.4.1](RELEASE/v0.4.1.md) | CI and supply-chain fixes | active | R-D1 |
| [v0.4.0](RELEASE/v0.4.0.md) | Interaction and priority | active | R-I1, R-S1 |
| [v0.3.0](RELEASE/v0.3.0.md) | Claude Code + Z.ai hooks | active | R-H1, R-H2, R-S1 |
| [v0.2.0](RELEASE/v0.2.0.md) | Asset pipeline | active | R-C1, R-L1 |
| [v0.1.0](RELEASE/v0.1.0.md) | Skeleton + static fish | active | R-W1, R-W2, R-A1, R-H1 |

## 3. How a release is cut

1. ACs before code (red tests first). 2. `make check` green. 3. Tick checklist, update this file, ROADMAP, AGENTS. 4. Agent hands over a one-line commit message; the **user** commits and runs `make release VERSION=x.y.z`. 5. CI gate then macOS build then GitHub Release.

## 4. Changelog

(none yet)

## 5. Verified and unverified

| Area | Verified by | Not exercised / limits |
|---|---|---|
| v0.1.0 build + hook | `make check`, `make build`, live Stop event to the backend | transparency, always-on-top, Dock absence, real Claude Code hook, dmg install |
