# Localisation: translating the game's interface

The **interface** (menus, battle messages, settings) is translatable. The **learning content** is Portuguese with
an **English** gloss: the game currently teaches *English speakers*. Adding source languages for the glosses
(so Spanish or French speakers could learn European Portuguese) is on the [roadmap](../ROADMAP.md); this document covers the
UI, which is already structured for it.

## How it works

UI strings live in `assets/i18n/<lang>/ui.lang`: plain text, one `key = value` per line, `#` comments.

```text
menu.save = Save
battle.caught = {name} trusts you now and joins your team!
battle.progress = Question {n}/{total}  (need {need} right)
```

- `{name}`-style **placeholders** are filled in by the game: keep them exactly, in any position that suits your language.
- `\n` in a value is a line break.
- Keys are fixed (they are used by the code and checked by a unit test); only translate the **values**.
- Missing keys fall back to English, so a partial translation never breaks the game.

## Add a language

1. `cp -r assets/i18n/en assets/i18n/es` and translate the values in `assets/i18n/es/ui.lang`.
2. Run `cargo run -p falabichos-tools --bin falabichos-lint`: it lists **missing** and **unknown** keys against English, and checks that every
   character you used exists in the bitmap font (`assets/art/font.txt`).
3. If you need a character the font lacks (for example `ñ` is there, but `¿` or `ß` may not be), add a glyph: 5 columns × 8 rows of `#` and `.`
   (see the header of `font.txt`). Accents are composed automatically from the base letter (á à â ã é è ê í ì î ó ò ô õ ú ù û ü ç ñ).
4. Select the language: set `ui_lang: "es"` in `settings.ron` in the game's data directory. A language picker in the settings menu is a
   [good first issue](GOOD_FIRST_ISSUES.md).

## Limits today

- The bitmap font covers Latin scripts. Chinese, Japanese, Korean, Arabic, Hebrew, Cyrillic, Greek or Thai UI translations need font work first
  (a good community project: new glyph sets in `font.txt`, or a licence-compatible font renderer).
- Right-to-left layouts are not supported yet.
- Plural rules are handled by writing both forms into the string (the UI avoids counted nouns).
