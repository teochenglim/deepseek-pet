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
| [v0.4.0](RELEASE/v0.4.0.md) | Interaction and priority | planned | R-I1, R-S1 |
| [v0.3.0](RELEASE/v0.3.0.md) | Claude Code + Z.ai hooks | planned | R-H1, R-H2, R-S1 |
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
