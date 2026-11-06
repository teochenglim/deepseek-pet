# GitHub Description Suggestion

```
A macOS-native, lightweight desktop pet driven by Claude Code and GLM (Z.ai) hooks. Powered by the Big Blue Fish VP9-alpha animation set, rendered through a Tauri v2 sprite pipeline.
```


# PRD: deepseek-pet

## 1. Project Overview

**deepseek-pet** is a macOS-first lightweight desktop pet application. It uses [`golitter/coding-agent-pet`](https://github.com/golitter/coding-agent-pet) as its technical skeleton (Tauri v2 + Rust + HTML/CSS/JS), replaces the default pixel-art character with the **106 VP9-alpha WebM animations** from [`Bai-Cute/big-blue-fish-desktop-pet`](https://github.com/Bai-Cute/big-blue-fish-desktop-pet) (the "Big Blue Fish"), and drives animation state transitions through Claude Code and GLM (Z.ai) Hook events.

**Core constraints**: Keep the installed footprint as small as possible, keep memory usage around the ~50 MB level, and achieve a transparent, borderless, always-on-top native window experience on macOS.


## 2. Goals and Non-Goals

### 2.1 Goals

| ID | Goal | Measurement |
|---|---|---|
| G1 | Display Big Blue Fish animations in a transparent, always-on-top window on macOS | Window background is transparent; window stays above other windows |
| G2 | Respond to Claude Code Hook events by switching animations | `PreToolUse` → thinking/working; `Stop` → back to idle |
| G3 | Respond to GLM Z.ai Hook events by switching animations | Same as G2; event channel is reused |
| G4 | Use the original Big Blue Fish VP9 WebM assets, not redrawn art | Asset source is traceable; license compliance is maintained |
| G5 | Automatically build and release macOS `.dmg` / `.app` via GitHub Actions | Release triggered by tag push |
| G6 | Installed size and runtime memory significantly lower than comparable Electron solutions | App bundle < 60 MB; idle memory < 80 MB |

### 2.2 Non-Goals (not before 1.0.0)

- Native Windows support (WSL2 + Claude Code scenarios can be evaluated later)
- Built-in LLM, OCR, chat bubbles, or other AI features from the original `big-blue-fish` project
- Multi-character / multi-skin system
- User-editable animation mapping UI
- Mac App Store distribution


## 3. Technical Architecture

### 3.1 Skeleton Choice: Why `coding-agent-pet`

| Dimension | `coding-agent-pet` | `big-blue-fish-desktop-pet` | Decision |
|---|---|---|---|
| Runtime | Tauri v2 (Rust + system WebView) | .NET | **Choose Tauri**: native macOS WebView, no Electron overhead |
| Animation system | Sprite sheet (8 columns × 9 rows, 1536×1872, 57 frames) | 106 VP9 WebM videos | **Needs adaptation**: WebM → sprite sheet |
| Hook integration | Existing `pet-hook.sh` + TCP/Unix Socket channel | No Agent Hooks | **Reuse directly** |
| Asset source | Pixel-art character | Big Blue Fish WebM | **Replace assets** |
| License | Needs confirmation, but code structure is suitable for fork | Assets MIT (WebM); code AGPL-3.0 | Assets usable; preserve NOTICE |

**Conclusion**: Fork `coding-agent-pet`, keep its Tauri shell, Hook protocol, multi-session aggregation, and window-management logic; **replace only the animation assets and state mapping**.

### 3.2 Animation Rendering Pipeline: WebM → Sprite Sheet

The `coding-agent-pet` renderer expects a fixed sprite-sheet format: **8 columns × 9 rows, each cell 192×208 px, total 1536×1872**. The `big-blue-fish` WebM files are separate videos, possibly 640×360 or 1280×720 at 60 fps.

**An offline asset-conversion pipeline must be built** to convert the 106 WebM files into a sprite sheet compatible with the `coding-agent-pet` renderer during the build stage.

#### 3.2.1 State Row → WebM Mapping Strategy

The 9 state rows in `coding-agent-pet` roughly correspond to: `idle`, `running-right`, `running-left`, `review`, `waiting`, `waving`, `jumping`, `failed`, etc. The 106 Big Blue Fish animations cover idle, click, drag, daily, meal, seasonal, and holiday scenes.

**v0.1.0 does not attempt to map all 106 segments**. First select a **minimum viable state set**, then expand in later versions:

| Sprite State | Mapped Big Blue Fish Animation (example) | Notes |
|---|---|---|
| `idle` (row 0) | Idle / breathing loop | Most-used; prioritize fidelity |
| `running-right` (row 1) | Work/run loop | Used during Claude Code `PreToolUse` |
| `running-left` (row 2) | Drag/move animation | Used when dragging the window |
| `review` (row 3) | Thinking/observing animation | Used while waiting for model response |
| `waiting` (row 4) | Waiting/still animation | Used during permission confirmation or input wait |
| `failed` (row 8) | Failure/sad animation | Used on `StopFailure` or error |

Each state row can hold **at most 8 frames** (the column limit in `coding-agent-pet`). Any WebM with more than 8 frames must be downsampled to ≤ 8 frames.

#### 3.2.2 Conversion Toolchain

**Recommended approach: FFmpeg + custom Python script + `Spritfy` for verification/fallback**

1. **FFmpeg frame extraction and scaling** (preserve alpha):
   ```bash
   ffmpeg -i input.webm -vf "fps=8,scale=192:208:force_original_aspect_ratio=decrease,pad=192:208:(ow-iw)/2:(oh-ih)/2:color=black@0" -frames:v 8 frame_%02d.png
   ```
   WebM encoded with `-c:v libvpx-vp9` and alpha will decode to PNG with the transparent channel preserved.

2. **Python script to compose the sprite sheet**: Use `Pillow` to stitch up to 8 PNG frames per row onto a 1536×1872 canvas, and output `spritesheet.png` plus `pet.json` (recording frames per row and frame duration).

3. **`Spritfy` / `GIFM` as helpers**: If you need to quickly verify frame extraction from a single WebM, Spritfy or GIFM can convert to APNG/WebP and then extract frames, but **the final pipeline must be based on the FFmpeg script** to ensure reproducibility and CI executability.

**The conversion script should live at `tools/convert-assets.py`**, with input `assets/webm/*.webm` and output `desktop/src/assets/spritesheet.png` + `pet.json`.

#### 3.2.3 Cropping and Alignment

The native aspect ratio of the Big Blue Fish is not necessarily 192×208. The script must perform **crop + letterbox**: first crop transparent margins, then scale proportionally to fit within 192×208, and finally center on a transparent canvas. Avoid stretching.

### 3.3 Hook Event Channel

`coding-agent-pet` already implements a mature Hook channel:

```
Claude Code / Codex ─→ pet-hook.sh ─┐
                                      ├→ TCP loopback (127.0.0.1:17361) or Unix Socket
OpenCode ─→ opencode-plugin.ts ─────┘         │
                                               ↓
                                    Tauri Rust backend
                                               ↓
                                    Frontend sprite animation renderer
```

**deepseek-pet strategy**: **Keep the channel as-is; change only the event-to-sprite-state mapping table**.

#### 3.3.1 Claude Code Hook Configuration

In `~/.claude/settings.json` or project-level `.claude/settings.json`:

```json
{
  "hooks": {
    "PreToolUse": [
      { "type": "command", "command": "~/.deepseek-pet/pet-hook.sh thinking" }
    ],
    "PostToolUse": [
      { "type": "command", "command": "~/.deepseek-pet/pet-hook.sh working" }
    ],
    "Stop": [
      { "type": "command", "command": "~/.deepseek-pet/pet-hook.sh idle" }
    ],
    "StopFailure": [
      { "type": "command", "command": "~/.deepseek-pet/pet-hook.sh failed" }
    ]
  }
}
```

`pet-hook.sh` sends event messages to `tcp://127.0.0.1:17361`. The exact protocol must be confirmed during 0.1.0 by reading the `coding-agent-pet` `pet-hook.sh` source.

#### 3.3.2 GLM Z.ai Hook

Z.ai's Hook mechanism must be verified in a real environment. Prefer reusing the same `pet-hook.sh` and TCP channel; if Z.ai uses a different config format, only adjust the **Hook configuration template**—no changes to the Tauri backend event-handling logic are needed.

**The acceptance criterion for 0.3.0 is: when either Claude Code or Z.ai emits an event, the sprite switches to the corresponding state.**

### 3.4 macOS Window Configuration

`coding-agent-pet` is a Tauri v2 app. The key settings for a transparent, always-on-top macOS window are in `tauri.conf.json` and the Rust side:

```json
{
  "app": {
    "windows": [
      {
        "label": "pet",
        "width": 300,
        "height": 300,
        "decorations": false,
        "transparent": true,
        "alwaysOnTop": true,
        "skipTaskbar": true,
        "resizable": false,
        "visible": true
      }
    ],
    "macOSPrivateApi": true
  }
}
```

**Note**: Newer Tauri v2 versions may no longer require `macOSPrivateApi` to achieve a transparent window, but keep this setting during 0.1.0 to ensure consistent behavior on target macOS versions. If the build warns that this field is deprecated, remove it per the Tauri version notes and verify transparency.

`skipTaskbar: true` (or `activationPolicy: accessory` on macOS) keeps the pet out of the Dock and Cmd+Tab.


## 4. Release Roadmap

| Version | Theme | Core Deliverable |
|---|---|---|
| **0.1.0** | Skeleton + Static Fish | Fork runs; transparent always-on-top window shows a static Big Blue Fish frame; Hook channel receives events and logs them |
| **0.2.0** | Asset Pipeline + Minimal Animation | WebM→sprite-sheet conversion script complete; `idle` / `running` / `failed` states usable |
| **0.3.0** | Claude Code + Z.ai Integration | Hook events from both Agents drive sprite-state switching |
| **0.4.0** | Interaction and State Priority | Drag, click, hover interactions; `waiting > running > idle > failed` priority logic |
| **1.0.0** | Stable Release | 15–20 core states covered; GitHub Actions automatically releases macOS `.dmg`; signing/notarization; complete docs |


## 5. Risks and Mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| VP9 alpha loses alpha in some FFmpeg builds | Sprite background turns black | Use `libvpx-vp9` decoding with `-pix_fmt rgba`; verify a single frame visually early in 0.2.0 |
| License compatibility of `coding-agent-pet` fork | Cannot legally distribute | First task in 0.1.0: confirm `golitter/coding-agent-pet` LICENSE; preserve original copyright notices; preserve `big-blue-fish` MIT NOTICE for assets |
| Z.ai Hook format differs from Claude Code | Delayed integration | Complete the loop with documented Claude Code first in 0.3.0; treat Z.ai as incremental adaptation |
| Mapping 106 WebM to 9 state rows inevitably requires trade-offs | Some animations invisible in 1.0.0 | Record "mapped" and "unmapped" lists in `pet.json`; keep unmapped files in the repo for future versions |
| GitHub Actions macOS runner signing limits | Cannot produce double-click-installable `.dmg` | Use ad-hoc or unsigned `.app` + manual Gatekeeper bypass for 0.1.0–0.4.0; resolve Developer ID signing before 1.0.0 |


## 6. Execution Summary for Coding Agent

```
Repo: github.com/teochenglim/deepseek-pet
Starting point: fork github.com/golitter/coding-agent-pet into the target repo
Phase 1 goal (v0.1.0):
  1. Push the full golitter/coding-agent-pet codebase to deepseek-pet main
  2. Download one idle still frame from big-blue-fish-desktop-pet and replace frame 0 of the idle row in the default sprite sheet
  3. Confirm the Tauri build passes and a transparent always-on-top Big Blue Fish still frame is visible on macOS
  4. Configure the Claude Code Stop hook; confirm the Tauri backend receives the event and prints it to console
Do not:
  - Change the Hook channel protocol
  - Implement real-time WebM playback (use the sprite-sheet pipeline)
  - Aim for full mapping of all 106 segments
  - Handle Windows
```


# RELEASE / 0.1.0.md — Skeleton and Static Fish

## Theme

**See a Big Blue Fish on macOS, and it knows when Claude Code stops.**

## Scope

### Must Complete

- [ ] Fork `golitter/coding-agent-pet` to `teochenglim/deepseek-pet`; preserving git history or squashing are both acceptable
- [ ] Run `python setup.py` on macOS and confirm the original pixel-art character displays correctly (baseline verification)
- [ ] Obtain one idle still frame from `big-blue-fish-desktop-pet` (PNG or extracted from WebM) and replace frame 0 of the `idle` row in the `coding-agent-pet` sprite sheet
- [ ] Build `.app`, launch on macOS, confirm: transparent background, borderless, always-on-top, not in Dock
- [ ] Configure `~/.claude/settings.json` `Stop` hook to call `pet-hook.sh idle`; confirm the Tauri backend receives the event (console log is sufficient; animation switching not required)

### Explicitly Out of Scope

- Batch WebM conversion
- Z.ai Hook
- Animation loops
- Drag interaction
- GitHub Actions

## Acceptance Criteria

| ID | Condition |
|---|---|
| A1.1 | `cargo tauri build` succeeds on macOS and produces `.app` |
| A1.2 | Window is transparent after launch; no white/black rectangle background |
| A1.3 | Window stays above other apps (verify by covering with Finder or a browser) |
| A1.4 | Sprite `idle` row shows the Big Blue Fish still frame, not the original pixel character |
| A1.5 | Running any task in Claude Code and triggering `Stop` produces the corresponding event message in terminal or Tauri logs |

## Technical Notes

- The sprite-sheet path follows `desktop/src/assets/` in `coding-agent-pet`; first locate the renderer entry point that reads the sprite sheet to confirm you are replacing the correct file.
- `pet-hook.sh` is under `desktop/` or near the repo root; read its argument parsing and TCP send logic. Do not modify it in `0.1.0`—only call it.
- If `macOSPrivateApi` causes compile warnings or errors, record the exact Tauri version and error. In `0.1.0` it is acceptable to comment out the field first and test transparency; restore it if transparency fails.


# RELEASE / 0.2.0.md — Asset Pipeline and Minimal Animation

## Theme

**Automatically produce the first playable sprite sheet from 106 WebM files.**

## Scope

### Must Complete

- [ ] Write `tools/convert-assets.py` to: take a WebM directory as input and output `spritesheet.png` (1536×1872) + `pet.json` (frames per row, frame duration, source file)
- [ ] Conversion pipeline supports **at least 3 states**: `idle`, `running-right`, `failed`
- [ ] Each state extracts **≤ 8 frames** from the corresponding Big Blue Fish WebM, scales proportionally, and centers in a 192×208 transparent cell
- [ ] Replace the `0.1.0` static placeholder with the generated sprite sheet; the three states can be manually triggered in the app (via code state change or temporary Hook verification)
- [ ] `pet.json` records source filename, frame count, and original resolution for each WebM
- [ ] Add `NOTICE` noting assets from `Bai-Cute/big-blue-fish-desktop-pet` (MIT) and code skeleton from `golitter/coding-agent-pet`

### Explicitly Out of Scope

- Z.ai Hook
- All 9 state rows
- Drag interaction
- CI automation

## Acceptance Criteria

| ID | Condition |
|---|---|
| A2.1 | `python tools/convert-assets.py --input assets/webm --output desktop/src/assets` succeeds in one shot on a clean macOS environment (with FFmpeg + Python Pillow) |
| A2.2 | Output `spritesheet.png` is exactly 1536×1872; rows 0 / 1 / 8 contain visible Big Blue Fish frames; other rows are transparent or placeholders |
| A2.3 | Manually setting state to `idle` / `running-right` / `failed` in the app switches the view to the corresponding Big Blue Fish animation (loop or one-shot; loop preferred) |
| A2.4 | Idle memory after app launch does not exceed **120 MB** (read from Activity Monitor; 0.2.0 may be above the 1.0.0 target) |
| A2.5 | `NOTICE` contains both the `big-blue-fish` asset license and the `coding-agent-pet` code license |

## Technical Notes

- Use FFmpeg `scale` filter with `force_original_aspect_ratio=decrease` + `pad` to avoid stretching the fish.
- If alpha is lost after frame extraction for a WebM, check whether `libvpx` `-auto-alt-ref` or pixel format was misused; first use `ffprobe` on a single frame to confirm `pix_fmt` includes `alpha`.
- The frame-duration unit in `pet.json` must align with `coding-agent-pet`'s `STATE_SPECS` (milliseconds or frame interval; follow source).
- If time permits, use `Spritfy` on the same WebM in parallel and compare frame-extraction results as a cross-check of pipeline correctness.


# RELEASE / 0.3.0.md — Claude Code + Z.ai Dual-Hook Integration

## Theme

**When the Agent works, the fish moves; when the Agent stops, the fish rests.**

## Scope

### Must Complete

- [ ] Full Claude Code Hook mapping: `PreToolUse` → `running-right`, `PostToolUse` → `idle` (or a sustained `running` state based on animation semantics), `Stop` → `idle`, `StopFailure` → `failed`
- [ ] GLM Z.ai Hook configuration: verify Z.ai's Hook event format; if different from Claude Code, add an adapter script `tools/zai-hook-bridge.sh` that converts Z.ai events into the same TCP message as `pet-hook.sh`
- [ ] Preserve Tauri backend multi-session logic: when multiple Claude Code / Z.ai sessions run simultaneously, aggregate animation state using `coding-agent-pet`'s priority rules (`waiting > running > idle > failed`, follow source)
- [ ] Provide `install-hooks.sh` that automatically writes Hook entries into `~/.claude/settings.json` and the corresponding Z.ai config file (idempotent; repeated runs do not duplicate entries)

### Explicitly Out of Scope

- All 9 state rows
- Interaction animations
- CI automation

## Acceptance Criteria

| ID | Condition |
|---|---|
| A3.1 | Claude Code executes any Bash tool call; fish switches from `idle` to `running-right`, then back to `idle` when the tool completes |
| A3.2 | Claude Code task fails (can be triggered intentionally); fish switches to `failed` |
| A3.3 | Z.ai emits at least one Hook event; fish changes to the corresponding state |
| A3.4 | Two Claude Code sessions run simultaneously, one `running` and one `idle`; fish remains `running` |
| A3.5 | `install-hooks.sh` on a clean `~/.claude/settings.json` leaves valid JSON and Hooks recognized by Claude Code |

## Technical Notes

- Locate Z.ai's Hook config path on a real machine (possibly `~/.zcode/` or project-level config). Do not assume it is identical to Claude Code.
- If Z.ai supports command hooks, point directly to `pet-hook.sh`; if not, consider filesystem watching or polling as a fallback, but `0.3.0` prioritizes official Hooks.
- Multi-session aggregation logic already exists in `coding-agent-pet`'s Rust backend; read the `session` and `event endpoint` code. Do not rewrite it.


# RELEASE / 0.4.0.md — Interaction and State Priority

## Theme

**The fish responds not only to the Agent, but also to you.**

## Scope

### Must Complete

- [ ] Drag: when dragging the window, switch to a drag-specific animation in `running-left` / `running-right`; return to previous Agent state on release
- [ ] Click: single click triggers a short animation (e.g., `waving` or a Big Blue Fish click-reaction WebM) without interrupting Agent working state
- [ ] Hover: hover triggers a jump/attention animation (refer to `coding-agent-pet` hover behavior)
- [ ] Full state priority implementation: `waiting` (permission confirmation) highest, then `running`, then `idle`, and `failed` when no other state applies
- [ ] Right-click menu: at least "Quit" and "Temporarily Hide" (restorable via menu-bar icon)

### Explicitly Out of Scope

- Full coverage of all 9 state rows (done in 1.0.0)
- Settings panel

## Acceptance Criteria

| ID | Condition |
|---|---|
| A4.1 | Fish plays movement animation while dragging window; returns to a reasonable state within 500 ms after release |
| A4.2 | While Agent is `running`, clicking the fish plays a one-shot interaction animation and then automatically returns to `running` |
| A4.3 | When a permission-confirmation event occurs, fish shows `waiting` even if the Agent simultaneously has a `running` event |
| A4.4 | Right-click "Quit" fully terminates the process with no lingering background process |


# RELEASE / 1.0.0.md — Release Vision

> **1.0.0 is not the next sprint goal. It is the endpoint definition of deepseek-pet as a "stable, usable product."**

## The 1.0.0 Promise

### Animation Coverage

- Map **at least 15–20** of the 106 WebM files from `big-blue-fish-desktop-pet` to the 9 state rows in `coding-agent-pet`, covering:
  - Idle loop
  - Agent working/thinking
  - Drag movement
  - Permission waiting
  - Successful completion
  - Failure/error
  - Click reaction
  - Hover reaction
  - One holiday or special-event animation (optional)
- `pet.json` fully records each animation's source WebM, frame window, frame duration, and mapped state.
- Unmapped WebM files remain in the repo and are listed in README for community extension.

### Distribution and Installation

- GitHub Actions automatically builds macOS `.app` and `.dmg` on `v*` tag push.
- **Apple Silicon (arm64)** is the default target; Intel builds are an optional job.
- Provide an **ad-hoc signed** `.dmg` with README instructions for first-launch Gatekeeper bypass (right-click → Open). If possible, obtain Developer ID and notarize.
- Provide **Homebrew Cask** installation: `brew install --cask teochenglim/deepseek-pet/deepseek-pet`.

### Experience Baseline

- App bundle size **≤ 60 MB**.
- Idle memory **≤ 80 MB** (Activity Monitor, read 5 minutes after launch).
- Cold start to fish visible **≤ 3 seconds** (Apple Silicon).
- Transparent always-on-top window behaves consistently on macOS 14 / 15 / 26.

### Agent Integration

- One-click Hook installation for both Claude Code and GLM Z.ai (`install-hooks.sh`).
- Stable multi-session aggregation: 3+ simultaneous Agent sessions do not crash or stutter; state priority correct.
- Provide `uninstall-hooks.sh` to cleanly remove all Hook entries.

### Documentation

- `README.md`: installation, Hook configuration, state mapping table, known limitations.
- `CONTRIBUTING.md`: how to use the asset-conversion script; how to add a new state mapping.
- `NOTICE`: `big-blue-fish` asset MIT, `coding-agent-pet` code license, and any third-party dependency licenses.
- `RELEASE/` retains release notes for every version.

### Explicitly Not in 1.0.0

- Native Windows support
- AI chat / OCR / built-in models
- User-customizable animation mapping UI
- Mac App Store


## 1.0.0 "Done" Definition

```
[ ] All acceptance criteria from 0.1.0 through 0.4.0 pass
[ ] 15+ WebM mappings complete; idle / running / waiting / failed core states human-tested
[ ] GitHub Actions release pipeline produces an installable .dmg from a clean tag
[ ] Homebrew Cask available
[ ] Cold start and Hook integration verified on at least two macOS machines (Apple Silicon, different macOS versions)
[ ] NOTICE and README license information complete
```


**First instruction to Coding Agent**: Start with `RELEASE/0.1.0.md`. First complete the fork, verify the baseline build, replace one still frame, and verify transparent always-on-top and Claude Code `Stop` event delivery. **Do not start 0.2.0 asset-pipeline work until all 0.1.0 acceptance criteria pass.**