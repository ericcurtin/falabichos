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
2. Run `cargo run -p falabichos-tools --bin falabichos-lint`: it lists **missing** and **unknown** keys against English, checks that every
   `{placeholder}` is kept exactly (a different placeholder is an **error**), warns when a string is too long for where the game draws it
   (pause-menu items about 11 characters, settings labels about 26, toasts, hints and captions one 38-character line; longer texts wrap), and checks that every
   character you used exists in the bitmap font (`assets/art/font.txt`).
3. If you need a character the font lacks (for example `ñ`, `¿` and `¡` are there, but `ß`, `œ` and `ë`/`ï` are not), add a glyph: 5 columns × 8 rows of `#` and `.`
   (see the header of `font.txt`). Accents are composed automatically from the base letter (á à â ã é è ê í ì î ó ò ô õ ú ù û ü ç ñ).
4. Give your language a name in its own language: `lang.name = Español`. It is what the **Language** row in Settings shows.
5. Select the language: *Settings > Language* (from the title screen or the pause menu) cycles through every folder under
   `assets/i18n/` and the interface changes at once. The choice is saved as `ui_lang` in `settings.ron`.

## Languages today

| Code | Language | Status |
|------|----------|--------|
| `en` | English | the source language |
| `es` | Español | complete, **not reviewed by a native speaker** (written by an AI coding assistant) |
| `fr` | Français | complete, **not reviewed by a native speaker** (written by an AI coding assistant) |

Please review them, or add another language. A translation does not have to be perfect to be merged.

## Limits today

- The bitmap font covers Latin scripts. Chinese, Japanese, Korean, Arabic, Hebrew, Cyrillic, Greek or Thai UI translations need font work first
  (a good community project: new glyph sets in `font.txt`, or a licence-compatible font renderer).
- Right-to-left layouts are not supported yet.
- Plural rules are handled by writing both forms into the string (the UI avoids counted nouns).
