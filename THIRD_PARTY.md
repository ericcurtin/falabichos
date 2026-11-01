# Third-party software and assets

The project's **own** code, content, art and audio data are licensed under the **Apache License 2.0 only**
(see [LICENSE](LICENSE) and [NOTICE](NOTICE)). This file records everything that is *not* ours.

## Assets

| Asset | Source | Licence |
|-------|--------|---------|
| Pixel art (tiles, characters, Bichos, seals), palette | Original, authored for this project as `.px` text | Apache-2.0 |
| Bitmap font (`assets/art/font.txt`): Latin, pt-PT diacritics, IPA | Original, drawn for this project | Apache-2.0 |
| Music and sound effects (`assets/music/*.song`, rendered at runtime) | Original compositions, synthesised in Rust | Apache-2.0 |
| Fonts | **None.** No third-party font is bundled. The game renders with its own bitmap font, which covers the Portuguese diacritics and the IPA symbols needed for pt-PT. | n/a |
| Audio recordings | **None yet.** Contributions are Apache-2.0 (see docs/AUDIO_GUIDE.md). | n/a |

The Apache-2.0 `LICENSE` text was copied verbatim from the canonical licence text.

## Rust dependencies

Resolved from `Cargo.lock` (about 480 crates for all targets, including Bevy's dependency tree). Direct
dependencies:

| Crate | Used for | Licence |
|-------|----------|---------|
| `bevy` 0.19.1 (pinned exactly) | window, GPU, audio, input in the client | MIT OR Apache-2.0 |
| `serde` | data model derive | MIT OR Apache-2.0 |
| `ron` | RON parsing (content, saves) | MIT OR Apache-2.0 |
| `thiserror` | error types | MIT OR Apache-2.0 |
| `png` | screenshots and optional PNG art | MIT OR Apache-2.0 |
| `proptest` (dev) | property tests | MIT OR Apache-2.0 |

The whole resolved tree contains only permissive licences. The full set found in `Cargo.lock`:
`MIT`, `Apache-2.0` (incl. `WITH LLVM-exception`), `Zlib`, `BSD-2-Clause`, `BSD-3-Clause`, `ISC`, `BSL-1.0`,
`CC0-1.0`, `MIT-0`, `Unlicense`, `0BSD`, `Unicode-3.0`. A few crates are offered under `... OR LGPL-2.1-or-later`
(`r-efi`); we use them under their MIT / Apache-2.0 option, so nothing copyleft is combined into the game.
Check it yourself with `cargo metadata --format-version 1 | jq -r '.packages[].license' | sort | uniq -c`.

Android build tooling (Gradle, Android Gradle Plugin, `androidx.games:games-activity` and AndroidX, all
Apache-2.0) is used only to package the APK/AAB.

## What this project does NOT contain

No sprites, music, maps, names, or trademarks from any commercial game franchise, and no code or data
copied from them.
