# Content format

All game content is plain text under `assets/`. You never need Rust to add or fix it. After editing, run

```sh
cargo run -p falabichos-tools --bin falabichos-lint
```

It must report **0 errors**. Warnings are worth reading (missing IPA, nouns without gender, ...).

## General rules

- Content files use **RON** (Rusty Object Notation): `( field: value, ... )`, lists in `[ ]`, strings in `"..."`,
  `// comments` allowed. Optional fields are written *without* `Some(...)`: `ipa: "ˈkazɐ"`.
- **Unknown fields are errors**, so a typo like `ipaa:` is caught by the linter instead of silently ignored.
- **IDs** are lowercase `snake_case` ASCII (`pastel_de_nata`). They are used in save files, so **never rename an id after release**.
- Every Portuguese text has an **English gloss**. Every word has **IPA** (European Portuguese, no slashes).
- Anything a native speaker has not checked carries `needs_native_review: true`.
- Use UTF-8 and real accented characters (`não`, `pão`), post-1990 spelling.

## Directory layout

| Path | Contains |
|------|----------|
| `content/tiles.ron` | the tile legend: map character → art + behaviour |
| `content/story.ron` | start position, intro dialogue, the flag that ends Part 1 |
| `content/pronunciation.ron` | one tip per `PronTag` (shown after wrong answers and in the Bichodex) |
| `content/seals.ron` | the Selos (badges) |
| `content/vocab/<category>.ron` | list of vocabulary items |
| `content/grammar/<concept>.ron` | one grammar concept with its exercises |
| `content/bichos/<id>.ron` | one Bicho species |
| `content/towns/<town>/town.ron` | town metadata |
| `content/towns/<town>/maps/<map>.ron` | one map each |
| `content/towns/<town>/dialogues/*.ron` | lists of dialogues |
| `content/towns/<town>/challenges/*.ron` | trainers and Mestres |
| `art/` | palette, font, `tiles/ bichos/ chars/ seals/` sprites (see ART_PIPELINE.md) |
| `music/*.song` | chiptune songs and sound effects |
| `audio/pt-PT/<category>/<id>.ogg` | recorded pronunciation clips (see AUDIO_GUIDE.md) |
| `i18n/<lang>/ui.lang` | interface strings (see LOCALISATION.md) |

## Vocabulary: `content/vocab/<category>.ron`

A file is a list of items. Worked example (`vocab/cafe.ron`):

```ron
[
    (
        id: "bica", pt: "bica", en: ["espresso"], category: "cafe",
        kind: Noun, gender: Fem,
        ipa: "ˈbikɐ", pron_tags: [ReducedVowel],
        example: (pt: "Uma bica, se faz favor.", en: "An espresso, please."),
        note: "The Lisbon and southern word for espresso. In Porto it is 'um cimbalino'.",
        forms: ["bicas"],
        needs_native_review: true,
    ),
]
```

| Field | Required | Meaning |
|-------|----------|---------|
| `id` | yes | unique, snake_case, never renamed |
| `pt` | yes | the headword **without article**; nouns get their article from `gender` |
| `en` | yes | English meanings; the first is the main translation (used in exercises) |
| `category` | yes | theme; distractors come from the same category first; Mestres can draw from a category |
| `kind` | no | `Noun` (default), `Verb`, `Adjective`, `Phrase`, `Other` |
| `gender` | nouns | `Masc` / `Fem` (drives the article exercise) |
| `plural` | no | irregular plural |
| `ipa` | **should** | broad IPA for European Portuguese, no slashes; the linter warns when missing |
| `audio` | no | override of the clip slot; the default slot is `<category>/<id>` |
| `pron_tags` | no | pt-PT pronunciation points: `NasalVowel NasalDiphthong ReducedVowel SibilantS OpenClosedVowel Digraph RhoticR SoftC SilentLetter DarkL` |
| `example` | no | `(pt: "...", en: "...")`; 3-8 words also enables the *order the words* exercise |
| `note` | no | usage note (register, regional variants, traps) |
| `not_pt_br` | no | the Brazilian word learners should *not* use (shown in the explanation) |
| `forms` | no | other surface forms (plurals, feminine, conjugations) used by the dialogue word lookup |
| `level` | no | 1 (default) to 5 |
| `needs_native_review` | no | `true` until a native speaker has checked the entry |

## Grammar: `content/grammar/<concept>.ron`

One concept per file: a notebook page plus its exercises.

```ron
(
    id: "ser_estar",
    title: "Ser vs estar",
    summary: "Two verbs for 'to be'. ...",
    rules: ["Ser: identity, origin, ...", "Estar: location and temporary states ..."],
    examples: [(pt: "Eu sou português.", en: "I am Portuguese.")],
    pt_pt_note: "Optional: how pt-PT differs from pt-BR here.",
    exercises: [
        (
            id: "se_1", kind: FillBlank,
            prompt: "Eu ___ português.", context: "I am Portuguese.",
            options: ["sou", "estou"], answer: ["sou"],
            explanation: "Nationality is identity: ser.",
        ),
    ],
    needs_native_review: true,
)
```

Exercise `kind` and how fields are used:

| kind | `prompt` | `options` | `answer` | the player |
|------|----------|-----------|----------|------------|
| `Choice` | the question | the choices | the correct option text(s) | picks one |
| `FillBlank` | sentence with `___` | choices for the gap | the right choice | picks one |
| `Type` | the question | (none) | accepted spellings; first is canonical | types it (accent-tolerant, **no typo tolerance**) |
| `Order` | the English meaning (`prompt_lang: En`) | (none) | the correct sentence(s); the words become shuffled tiles | orders the tiles |
| `Translate` | an English sentence (`prompt_lang: En`, at most 36 characters) | (none) | accepted Portuguese sentences (at most 60 characters; case, final punctuation and accents are forgiven per the settings, typos are not) | types the whole sentence; the hint key reveals `word_hints` |

Other fields: `prompt_lang: En` (default `Pt`; colours the prompt), `context` (small line under the prompt), `hint`,
`word_hints: [("bridge", "ponte"), ...]` (only for `Translate`: English/Portuguese pairs in sentence order; the first pairs that fit on one 31-character line are shown, so put the hardest words first; the linter warns if a Portuguese hint word is not in any accepted answer),
`explanation` (shown after answering: **always write one**), `needs_native_review`. Exercise ids are global.

## Bichos: `content/bichos/<id>.ron`

```ron
(
    id: "galo", name: "Galo", dex_no: 1, kind: Vocab,
    theme: "farm animals and the countryside",
    flavor_pt: "O Galo canta ao nascer do sol e acorda a vila inteira.",
    flavor_en: "The Galo sings at sunrise and wakes the whole village.",
    vocab: ["galo", "galinha", "vaca", "cavalo", "porco", "ovelha", "quinta", "campo", "ovo"],
    art: "galo",            // assets/art/bichos/galo.px
    trust_needed: 5,        // correct answers to win it
    cry_hz: 520,            // pitch of its procedural cry
    needs_native_review: true,
)
```

`kind: Grammar` Bichos use `grammar: ["ser_estar", ...]` (concept ids) instead of `vocab`. `dex_no` must be unique.
A Bicho's *skills* are its vocab ids plus every exercise of its grammar concepts. Its **level** is derived from how well
the player has learned them.

## Towns and maps

`towns/<town>/town.ron`: `id`, `name`, `blurb`, `inspired_by`, `start_map`.

A **map** is rows of tile characters. Characters come from `tiles.ron` (`.` grass, `"` tall grass, `T` tree, `D` door,
`#` wall, `~` water, `_` calçada, ... and indoor `F` wood floor, `X` counter, ...). All rows must have the same length.

```ron
(
    id: "vila_azul", name: "Vila Azul", town: "vila_azul", music: "town",
    rows: [ "TTTT...", "T..RRR.", ... ],          // use \" for the tall grass character
    warps:    [(x: 6, y: 5, map: "casa_avo", to_x: 4, to_y: 6, facing: Up)],
    npcs:     [(id: "padeiro", x: 13, y: 9, art: "padeiro", facing: Right, dialogue: "padeiro", wander: true)],
    signs:    [(x: 8, y: 6, dialogue: "sign_casa_avo")],
    triggers: [(x: 5, y: 5, dialogue: "x", when: [Flag("a")], push_back: true)],
    encounters: [(species: "sardinha", weight: 50), (species: "gaivota", weight: 35)],
    encounter_rate: 18,                            // % chance per step in tall grass
)
```

- `warps`: stepping on `(x, y)` fades to `map` at `(to_x, to_y)`. Doors are normal walkable tiles with a warp on them.
- `npcs`: `art` is an id from `art/chars.txt`; `dialogue` is a dialogue id; `show_when: [Cond...]` hides the NPC until
  conditions hold (the old man blocking the road disappears after the first Selo);
  `trainer: (sight: 3, challenge: "treino_x", spot_dialogue: "x_spot")` makes the NPC challenge you when you enter their
  line of sight (until you beat them); `wander: true` lets them stroll within 2 tiles.
- NPCs behind a *counter* tile (`X`, `U`, `A`) can be talked to across it.
- Check a map with `cargo run -p falabichos-tools --bin falabichos-mapcheck -- <map_id>`: it prints the map with
  NPCs (`@`), warps (`>`), signs (`?`), triggers (`!`), and reports unreachable warps/NPCs, bad ids and non-walkable arrivals.

## Dialogue: `towns/<town>/dialogues/*.ron`

A dialogue has **branches**; the first whose `when` conditions hold is played.

```ron
(
    id: "avo",
    branches: [
        (when: [Flag("beat_joaquim")], lines: [
            (who: "Avó", pt: "Ganhaste o Selo do Polvo!", en: "You won the Octopus Seal!",
             words: {"ganhaste": "you won"}),
        ], then: [Heal]),
        (lines: [ (who: "Avó", pt: "Bom dia! Dormiste bem?", en: "Good morning! Did you sleep well?", register: "tu") ],
         then: [SetFlag("met_avo")]),
    ],
)
```

- Line fields: `who` (speaker, omit for narration/signs), `pt`, `en`, `words` (hand glosses for the word lookup, lower-case),
  `audio` (clip), `register` (`"tu"` or `"formal"`, informational), `needs_native_review`.
- The player can reveal `en` with the gloss key, then move over words: the game looks them up in `words`, then in
  the vocabulary (`pt`, `plural`, `forms`), trying 3-, 2- then 1-word phrases.
- Conditions (`when`, `show_when`): `Flag("x")`, `NotFlag("x")`, `HasBicho("galo")`, `HasSeal("selo_polvo")`, `PartyAtLeast(1)`.
- Actions (`then`): `SetFlag`, `ClearFlag`, `Heal`, `Challenge("id")`, `GiveBicho("id")`, `Warp(map: .., x: .., y: ..)`, `Save`.

## Trainers and Mestres: `towns/<town>/challenges/*.ron`

```ron
(
    id: "mestre_joaquim", name: "Mestre Joaquim", kind: Mestre,
    theme: "Ordering food politely",
    pool: (categories: ["cafe"], grammar: ["polite_requests", "articles_gender"]),   // also: vocab: ["id", ...]
    questions: 7, pass_correct: 5, confidence: 3,
    art: "joaquim", reward_flag: "beat_joaquim", seal: "selo_polvo",
    win_dialogue: "joaquim_win", lose_dialogue: "joaquim_lose",
)
```

`pass_correct` out of `questions` wins; `confidence` is the number of mistakes you can afford before failing.
No hints in tests. A `Mestre` should award a `seal`.

## `story.ron`

`start_map/start_x/start_y`, `intro_dialogue`, `finale_flag` (when set, `finale_dialogue` plays once).
