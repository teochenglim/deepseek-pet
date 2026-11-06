# Requirements traceability

Every requirement is cited by at least one AC in `RELEASE/vX.Y.Z.md` (`make ac-map`).

| ID | Requirement | Source | Release |
|---|---|---|---|
| R-W1 | Window is transparent and borderless on macOS | PRD G1 | 0.1.0 |
| R-W2 | Window is always on top and absent from Dock / Cmd+Tab | PRD G1, 3.4 | 0.1.0 |
| R-A1 | Pet art comes from the original Big Blue Fish WebM, not redrawn | PRD G4 | 0.1.0 |
| R-H1 | Claude Code hook events reach the Tauri backend and drive state | PRD G2, 3.3.1 | 0.1.0, 0.3.0 |
| R-C1 | Offline converter turns WebM into the renderer's frame format | PRD 3.2 | 0.2.0 |
| R-L1 | NOTICE carries asset and code-skeleton licenses | PRD 5 | 0.2.0 |
| R-H2 | Z.ai hook events drive state through the same channel | PRD G3, 3.3.2 | 0.3.0 |
| R-S1 | Multi-session aggregation and state priority are correct | PRD 0.3/0.4 | 0.3.0, 0.4.0 |
| R-I1 | Drag, click, hover, right-click menu respond | PRD 0.4.0 | 0.4.0 |
| R-P1 | Footprint: bundle <= 60 MB, idle memory <= 80 MB, cold start <= 3 s | PRD G6 | 1.0.0 |
| R-D1 | Tag push builds and publishes macOS .dmg; Homebrew Cask | PRD G5 | 1.0.0 |
| R-M1 | Mapped / unmapped animations recorded; 15+ states covered | PRD 1.0.0 | 1.0.0 |

## PRD corrections found while scaffolding

- Renderer format: `coding-agent-pet` loads `frames/<state>/NN.png` + `frames-manifest.json` (see `desktop/cross-platform/src/animator.js`), not a 1536x1872 sprite sheet. R-C1 targets that format.
- Asset count: 212 `.webm` files (PRD said 106), under `code/BigBlueFish.Windows/assets/fish/`.
- Asset licence: Apache-2.0 (PRD said MIT). Code skeleton: MIT per its README.
- Hook channel in the copied code is a Unix socket (`/tmp/kotori-pet.sock`, `config.rs`); PRD's TCP 17361 must be re-verified in 0.1.0 before documenting hooks.
