# Bicho design document and art style guide

A **Bicho** is a critter that *embodies* some Portuguese: a theme of words (farm, sea, café, transport, feelings) or a grammar
concept (articles, ser/estar). Its design should make the lesson **memorable** and the creature **lovable**. Everything must be
**original**: no copying, tracing or "inspired by" any existing franchise creature, and no franchise names.

## The Bicho concept checklist

1. **Hook**: how does the creature's *look* or *behaviour* point at what it teaches? Galo (rooster) = farm words; Sardinha = the sea;
   Elétrico (a tram-snail) = transport; Saudade (a shy violet ghost) = feelings; Cegonha (a stork that delivers articles) = *o/a/um/uma*;
   Camaleão (a chameleon that changes colour with its state) = *ser* vs *estar*.
2. **Portuguese roots**: prefer a Portuguese animal, object or cultural icon: *Galo de Barcelos*, azulejo, pastel de nata, sardinha,
   elétrico 28, cegonha, caravel, cork oak, fado guitar (*guitarra portuguesa*), *bacalhau*, *calçada*, lighthouse, ...
3. **Name**: the Portuguese word for the thing, capitalised, `ASCII` id (`galo`, `eletrico`), pronounceable and short.
4. **Personality**: one line of Portuguese flavour text (with English): shy, proud, sleepy, curious.
5. **Words**: at least 3, ideally 8-16 vocabulary items (or a grammar concept with 5+ exercises).

## Sprite specification

| | |
|--|--|
| Size | **16×16 px** per frame (Bicho, tile, character, seal) |
| Format | `assets/art/bichos/<id>.px` (text) **or** `<id>.png` (RGBA, frames stacked vertically; a PNG replaces a same-named `.px`) |
| Frames | 1 (the game adds a 1 px *bob* when `bob: yes`), or 2+ separated by `---` for hand-made animation |
| Palette | only the letters in `assets/art/palette.txt` (**31 colours**: do not add colours without a very good reason; the linter caps it at 32) |
| Outline | `outline: K` adds an automatic 1 px ink outline on the outside of the shape; draw the *inside* only |
| Facing | Bichos face **left** in the sheet; battles mirror them to face the player |
| Shading | light from the top-left: highlight on the upper-left, one darker shade bottom-right, no dithering, no gradients |
| Eyes | 1-2 px, dark ink `K` with an optional white `W` glint: eyes give personality |
| Silhouette | must read at 1× on a small phone screen: a distinct shape beats detail |

The battle screen shows the opponent at **3×** scale (48×48 px on the 240×160 logical screen); the Bichodex at 1× in lists and 3× in details.

## Palette (excerpt)

Full list in `assets/art/palette.txt`. Letters used most:

| | | | |
|--|--|--|--|
| `K` ink `#1b1a2e` | `W` cream `#fff8e8` | `w` pale stone | `g` grey / `G` dark grey |
| `e` green / `E` leaf green / `l` lime / `d` deep green | `b` azulejo blue / `B` sky blue / `c` cyan / `s` sea teal / `n` navy | `r` red / `R` coral / `o` orange / `y` yellow / `Y` pale yellow | `p` pink / `P` plum / `v` violet |
| `t` brown / `T` light brown | `a` sand / `A` pale sand | `h` terracotta / `H` dark terracotta | `m` skin / `M` skin dark |

Use the **azulejo blue + cream** pair and **terracotta** as the project's signature colours.

## `.px` format in one example

```text
# Bicho: galo          (comments start with #)
size: 16x16
outline: K             (auto outline)
bob: yes               (idle animation: second frame shifted down 1 px)
................
......rr.rr.....       (. = transparent, letters = palette)
.....rRRrRRr....
...
```

Rows must be exactly `size` wide and tall. Errors are reported with the file name by `falabichos-lint`.

## Naming

- File and id: lowercase ASCII snake_case: `galo`, `pastel_de_nata`, `eletrico`.
- Display name: the Portuguese word with its proper accents: *Elétrico*, *Camaleão*.
- Dex numbers are unique; new Bichos get the next free number.

## Workflow for artists

1. `falabichos-new bicho <id>` (creates a 16×16 stub) or copy an existing `.px`.
2. Edit; run `falabichos-preview out/` and open `out/bichos.png` (4× contact sheet). Iterate.
3. Make sure it looks good next to the others (same outline, shading and saturation).
4. Open a PR. By contributing you license the art under Apache-2.0.

Open ideas (not yet drawn): *Caravela* (sailing words), *Bacalhau* (food), *Guitarrada* (music words), *Sobreiro* (nature), *Fado* (feelings 2),
*Pastel* variants, *Lume* (fire/weather), *Vinha* (vineyard words), *Moliceiro* (Aveiro boat), *Tamanco* (clothes).
