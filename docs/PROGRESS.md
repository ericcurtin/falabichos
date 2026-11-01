# Falabichos: living progress log

Read this first if you are resuming work. Newest entries at the bottom of each section.

## Environment notes
- Host has no Bevy dev headers; all cargo commands run inside the `falabichos-dev` toolbox container
  (Fedora 42). Packages were installed with `podman exec --user root` (rootless, no sudo).
- `CARGO_TARGET_DIR=/home/ecurtin/git/falabichos-run/target` (NOT inherited by toolbox; wrappers
  `falabichos-run/tc.sh`, `build.sh`, `xrun.sh` set it).
- Bevy pinned: **=0.19.1** (0.20.1 is on crates.io but needs a newer rustc than 1.95).
- Spike (window + sprite) built in ~2 min and ran under Xvfb + llvmpipe (lavapipe).

## Architecture decisions
- Bevy is a thin shell. All game logic AND software rendering of the 240x160 frame live in
  engine-agnostic crates, so the headless sim and screenshots work with no GPU.

## Milestones
- [x] 1. Preflight (toolchain, container, Bevy spike)
- [x] 2a. Workspace skeleton, LICENSE, NOTICE
- [x] 3a. falabichos-core (model, normalize, SRS, exercises, battle, progress, save, i18n), 39 tests
- [ ] 3b. falabichos-content loader + lint
- [ ] 4. falabichos-media (pixel art, font, synth)
- [ ] 5. falabichos-game + Bevy client
- [ ] 6. Vertical-slice content
- [ ] 7. Mobile glue, xtask, CI
- [ ] 8. Community docs
- [ ] 9. Final verification + FINAL_REPORT.md
