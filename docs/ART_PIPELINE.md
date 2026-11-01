# Art, font and music pipeline

There is **no external art**. Everything is described in small text files (or optional PNGs) and rendered by
`crates/falabichos-media`, at runtime in the game and by the tools. This means diffs are readable, artists need no
special software, and everything is licensed Apache-2.0.

## Palette (`assets/art/palette.txt`)

```text
K = #1b1a2e  ink
W = #fff8e8  cream white
. = transparent
```

One character per colour. The master palette has 31 opaque colours; `falabichos-lint` errors above 32.

## Sprites and tiles (`.px`)

Located in `assets/art/{tiles,bichos,seals,ui,chars}/<id>.px`.

```text
size: 16x16        required: frame size
outline: K         optional: auto 1 px outline in that palette letter
repeat: 8x8        optional: the grid is a pattern tiled over `size`
bob: yes           optional: add a 2nd frame = the first shifted down 1 px
................   one row per line; '.' transparent; letters from palette.txt
---                starts the next animation frame
```

**Tiles** are 16×16 and referenced from `content/tiles.ron` (`art: ["water_a", "water_b"]` animates by cycling frames, or give one
`.px` with several frames). **Bichos**: see [BICHO_DESIGN.md](BICHO_DESIGN.md).

### Characters (`art/chars.txt` + `art/chars/_body_*.px`)

People are *recolourable templates*: the body files use role digits instead of palette letters:

`1` hair · `2` skin · `3` shirt · `4` pants · `5` shoes · `6` extra (apron / scarf / badge)

and `art/chars.txt` defines each character in one line:

```text
joaquim: body=a hair=K skin=M shirt=W pants=n shoes=K extra=r
```

A template holds 6 frames: down stand/walk, up stand/walk, left stand/walk (right is the mirrored left). Add a new person with one line;
add a new silhouette by drawing a new `_body_<x>.px` (underscore files are templates, not sprites).

### PNG instead of text

Export an RGBA PNG whose width is the frame size and height is `frames × frame size` (frames stacked vertically) to
`assets/art/<category>/<id>.png`. A PNG **replaces** a `.px` of the same name, so artists can use Aseprite, Pixelorama, LibreSprite, etc.

## Font (`assets/art/font.txt`)

Original 5×7 bitmap font, drawn for this project, covering ASCII, the Portuguese diacritics (composed from base letters in code), and the IPA symbols used for
pt-PT (ɐ ɛ ɔ ə ɨ ʃ ʒ ʁ ɾ ɲ ʎ ŋ ɫ ˈ ː, and nasal vowels via combining tilde). One glyph per line:

```text
a ..... ..... .###. ....# .#### #...# .#### .....
```

8 rows of 5 pixels (rows 0-6 caps, 2-6 x-height, 7 descender). The linter checks that every character in the content and UI strings exists in the font, so a
missing glyph is caught before players see a `?`.

## Previewing

```sh
cargo run -p falabichos-tools --bin falabichos-preview -- out/
```

writes `out/tiles.png`, `bichos.png`, `chars.png`, `seals.png`, `font.png` (4× contact sheets) and every song as a `.wav`.
For the real in-game look, `cargo run -p falabichos -- --screenshots out/` replays the story and saves PNGs of key screens.

## Music

Songs and sound effects are text files in `assets/music/*.song`, rendered by a small chiptune synthesiser:

```text
name: town
bpm: 108               a "step" is a sixteenth note
loop: yes
lead = square 0.35 : E5:2 -:1 D5:1 C5:2 D5:2      NOTE:STEPS ; '-' or '.' is a rest
bass = triangle 0.7 : C3:4 G2:4 A2:4 E3:4
perc = noise 0.15 : h:2 .:2 h:2 .:2               noise: x = hit, h = hat
slide = square 0.4 : C5>C6:2                      NOTE>NOTE:STEPS is a pitch slide
```

Waves: `square` (50 % duty), `pulse` (25 %), `triangle`, `saw`, `noise`. The second number is the channel volume. Channels loop to the length of the
longest one. Notes are `C4`, `F#3`, `Bb5`. Sound effects use the same format with `loop: no` and `sfx_` names the game looks up
(`sfx_blip sfx_select sfx_confirm sfx_cancel sfx_bump sfx_correct sfx_wrong sfx_door sfx_encounter sfx_levelup sfx_save sfx_hint`).
Assign music to a map with `music: "city"`. Everything is **original**: do not transcribe existing melodies.
