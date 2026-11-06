# deepseek-pet: agent guide

macOS desktop pet. Tauri v2 (Rust + system WebView, plain HTML/JS, no bundler) forked from `golitter/coding-agent-pet` (`desktop/cross-platform/`), with Big Blue Fish animations from `Bai-Cute/big-blue-fish-desktop-pet` converted to PNG frames. Driven by Claude Code and Z.ai hooks. Goal and requirements: [PRD.md](PRD.md), [ARCH/00_requirements_traceability.md](ARCH/00_requirements_traceability.md). No server, no database, no Docker.

## Where things stand

- Built: v0.1.0 code done and `make check` green; .app and .dmg build. Awaiting the human visual pass (see Limits in RELEASE/v0.1.0.md). 0.2.0-1.0.0 planned. Status table: [RELEASE.md](RELEASE.md).
- Next: finish v0.3.0 (hooks + app icon built, see RELEASE/v0.3.0.md), then 0.4.0.
- Reference clones `coding-agent-pet/` and `big-blue-fish-desktop-pet/` are git-ignored read-only references. Never edit them; copy what is needed into `desktop/cross-platform/`, `assets/`.

Code map:

| Path | What |
|---|---|
| `desktop/cross-platform/src/` | frontend: `animator.js` (frame loader/state machine), `main.js`, `bubble.js`, `interaction-controller.js`, `hit-test.js`, `style.css` |
| `assets/big-blue-fish/frames/<state>/NN.png` | runtime frames; `PROVENANCE.md` records the source WebM per frame |
| `desktop/cross-platform/src-tauri/src/` | `lib.rs` app setup, `aggregator.rs` multi-session state priority, `commands.rs`, `config.rs` (socket path, settings), `watcher.rs` |
| `desktop/cross-platform/src-tauri/tauri.conf.json` | window: transparent, borderless, alwaysOnTop, skipTaskbar |
| `desktop/cross-platform/hooks/` | `pet-hook.sh` (Claude/Codex), `opencode-plugin.ts`; protocol is frozen |
| `desktop/docs/` | upstream docs (Chinese), reference only |
| `tools/convert-assets.py` | WebM -> frames; `state-map.json` picks the clip per state; `make convert-assets` |
| `docs/hooks/` | example hook configs |
| `tools/install_hooks.py`, `scripts/install-hooks.sh`, `scripts/uninstall-hooks.sh` | idempotent Claude Code + ZCode hook installer (marker `# deepseek-pet`) |
| `packaging/homebrew/` | cask template; `scripts/update-cask.sh` renders it; release job pushes to the tap |

## Commands

`make help`. `make check` = fmt-check + clippy + lint-frontend + ac-map + test + version-check; green before claiming anything works. `make setup` first (npm ci). `make run` dev, `make build` bundle.
ACs are rows `| ACM.m.n |` in `RELEASE/vX.Y.Z.md`; each needs >= 2 Rust tests `fn ac_M_m_n_<slug>` under `desktop/cross-platform/src-tauri/{src,tests}` (`scripts/ac-map.sh`; `planned` releases are checked for requirement coverage only).

## Git & release

The user does ALL commits, pushes, tags and releases. The agent never runs `git commit/push/tag`, `make bump`, `make release`. Hand over a one-line commit message. The agent cannot run `/compact`; tell the user when context is heavy.

## Rules

- Do not change the hook channel protocol or `pet-hook.sh` argument parsing; only the event-to-state mapping.
- No real-time WebM playback; frames only. No Windows work before 1.0.0.
- No `unwrap()`/`expect()` outside tests in Rust. Keep `tauri` capabilities minimal (an unpermissioned plugin fails silently).
- New dependency: say why in the release file. Changed behaviour: update docs in the same change.
- Licences: keep `NOTICE` accurate. Assets are Apache-2.0 (not MIT as PRD first said); code skeleton MIT per upstream README (no LICENSE file upstream; re-confirm before 1.0.0).
- Bundle <= 60 MB and idle memory <= 80 MB are budgets (1.0.0); 0.2.0 may reach 120 MB.

## Working method

Read the release file first. Probe the real thing (run hook, read socket) before assuming. Order: tests -> code -> manual macOS pass -> docs -> `make check`. Scratch files go outside the repo. Report honestly what was not exercised: transparency, always-on-top and hook delivery need a human macOS check.

## Pitfalls already paid for

- Renderer loads per-state PNG frame dirs plus `frames-manifest.json`; there is no 8x9 sprite sheet (PRD was wrong).
- The big-blue-fish repo has 106 WebM files (the 212 you see in `ls` includes a `.mask` per clip); filenames are Chinese. (I wrongly reported 212 earlier.)
- Hook protocol (verified in the copied code): `pet-hook.sh claude-code|codex` reads the hook JSON on stdin (not `idle`/`thinking` as PRD said). Event-to-state mapping lives in `config.json` `state_map`. macOS endpoint default is `tcp://127.0.0.1:17361` (`config.rs`); the Unix socket path is non-mac only. Upstream hard-codes the `desktop/cross-platform` layout (config.rs, hooks common.py), so keep it.
- `macOSPrivateApi` is set in `tauri.conf.json` and the `macos-private-api` Cargo feature; remove both together or neither.
- Actions are SHA-pinned (checkout, codeql upload-sarif and Trivy digest copied from ~/code/glove). `scripts/pin-actions.py` mislabeled checkout as v4.4.0, so verify `# vX.Y.Z` comments after any bump.
- Security: `.github/workflows/security.yml` runs Semgrep (p/rust, p/javascript, p/github-actions, p/secrets, p/owasp-top-ten + `.semgrep/rules.yaml`) and Trivy fs. Only ERROR fails the gate; unwrap/innerHTML rules are WARNING because upstream code has 40 unwrap hits. Run locally with `semgrep scan --config .semgrep/rules.yaml --exclude coding-agent-pet --exclude big-blue-fish-desktop-pet .`.
- Do not put destructive shell strings (even inside heredocs) in Bash commands; the harness blocks them. Write such files with the editor tool.
- ZCode (Z.ai) is a separate agent with Claude-compatible hook payloads; its config is `~/.zcode/cli/config.json` and project-level hooks are ignored by ZCode. Failure event is `PostToolUseFailure`. Never run the installer against the real `~/.claude` or `~/.zcode` in tests; use `--claude-settings/--zcode-config` temp paths.
- `terminal_events` deletes the session, so never list an event there whose state should stay visible (StopFailure was, and hid `failed`).
