# deepseek-pet: agent guide

macOS desktop pet. Tauri v2 (Rust + system WebView, plain HTML/JS, no bundler) forked from `golitter/coding-agent-pet` (`desktop/cross-platform/`), with Big Blue Fish animations from `Bai-Cute/big-blue-fish-desktop-pet` converted to PNG frames. Driven by Claude Code and Z.ai hooks. Goal and requirements: [PRD.md](PRD.md), [ARCH/00_requirements_traceability.md](ARCH/00_requirements_traceability.md). No server, no database, no Docker.

## Where things stand

- Built: v0.1.0 code done and `make check` green; .app and .dmg build. Awaiting the human visual pass (see Limits in RELEASE/v0.1.0.md). 0.2.0-1.0.0 planned. Status table: [RELEASE.md](RELEASE.md).
- Next: v0.6.0 (Play animation menu) and v0.5.0 (Dock, menu bar, floating) await a manual pass. Planned: v0.6.1 standalone app (bundle frames/config/hooks, done in code, needs a manual install pass), v0.7.0 map hook events to animations (built, needs a manual pass), v0.8.0 (built, needs a manual pass with each real agent) install hooks for Codex, Gemini, Qwen, Kimi, Copilot, Cursor, OpenCode from the menu, Active source, better default animations. Then 1.0.0.
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
| `assets/big-blue-fish/frames/extras/` | all 106 clips (16 palette PNG frames each) + `catalog.json`; loaded on demand by `playExtra`, never at startup. Rebuild with `make convert-extras`; titles in `tools/clip-titles.json` |
| `desktop/cross-platform/src-tauri/src/home.rs` | installed-app layout: read-only files in `Contents/Resources`, user state and hook scripts in `~/.deepseek-pet` (`DEEPSEEK_PET_HOME` overrides) |
| `desktop/cross-platform/src-tauri/src/mapping.rs` | `~/.deepseek-pet/mapping.json` model (agent -> event -> state / clip / off), agent and event tables, shared by the tray menu; hook scripts read the same file (`common.py` `apply_mapping`) |
| `desktop/cross-platform/src-tauri/src/menus.rs` | tray menu (Play animation, Map animations), right-click popup, and one `parse_action`/`run` for both |
| `desktop/cross-platform/src-tauri/src/catalog.rs` | reads the catalog, groups it for the tray "Play animation" submenu |
| `desktop/cross-platform/hooks/agents.json` | the one table of supported agents (config path, format, events, native event names); read by the installer, `common.py` and the Rust menus |
| `tools/install_hooks.py`, `scripts/install-hooks.sh`, `scripts/uninstall-hooks.sh` | idempotent installer for every agent in agents.json plus `status` (marker `# deepseek-pet`); the menu's Install hooks runs it |
| `desktop/cross-platform/src-tauri/src/{hooks,settings}.rs` | menu rows and installer driver; `settings.json` (active source) |
| `packaging/homebrew/`, `Casks/` | cask template; `scripts/update-cask.sh` renders it into `Casks/deepseek-pet.rb`; this repo is its own tap (`brew tap teochenglim/deepseek-pet <repo url>`); the release job commits the cask to main |

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
- CI is deliberately small: `security.yml` (Semgrep, Trivy, cargo-deny, daily) and `release.yml` (tag build and publish). `make check` and `make audit` run locally before `make release`; the release job does not re-run them. Do not re-add Dependabot or a macOS CI job without asking.
- `src-tauri/deny.toml` ignores and `src-tauri/.cargo/audit.toml` must stay in sync; each ignored advisory is a transitive tauri dependency, recheck on every tauri bump.
- `cargo-deny` needs `license` in `Cargo.toml`; macOS-only `targets` in `deny.toml` hide the gtk/webkit2gtk trees.
- The app is `ActivationPolicy::Regular` on purpose (Dock icon, requested after 0.4.0); do not restore Accessory. The menu-bar icon must be `icons/tray.png` with `icon_as_template(false)`: the default window icon rendered blank there.
- Extras must never enter `frames-manifest.json` (the renderer loads every row at startup, 106 clips would be about 270 MB decoded). `tools/requirements.txt` is scanned by Trivy: keep Pillow on a CVE-free version.
- Installed `.app` vs repo run: `home::bundle_resources(exe)` decides. In a bundle, frames and defaults come from `Contents/Resources` and `config.json`, `hooks/`, `runtime/` from `~/.deepseek-pet`; a repo run keeps the old repo-relative paths. A new `bundle.resources` entry must be added to `tauri.conf.json` and covered by `ac_0_6_6_*`.
- Mapping rules live in two places that must agree: `mapping.rs` (menu, file writer) and `common.py` `apply_mapping` (what a hook does with an entry). Agents and their events live only in `hooks/agents.json` (installer, hook scripts and menus all read it); a new agent also needs its `format` writer in `install_hooks.py` and a source case in `pet-hook.sh`. Clips only fire on the live socket, not from the sessions-folder watcher.
- Active source filters in `ActivityAggregator::compute_change`; hidden sources are still tracked. Default animations live in `config.example.json` `state_map`; bump `config_version` there when changing them so `home::upgrade_state_map` pushes them to existing configs. Agent formats (Kimi, Copilot, Cursor, Qwen, OpenCode) are written from docs and untested against the real tools.
