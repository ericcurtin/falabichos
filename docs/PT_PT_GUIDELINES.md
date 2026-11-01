# European Portuguese (pt-PT) guidelines

Falabichos teaches **European Portuguese**. Many learning resources silently teach the Brazilian standard, so
English-speaking learners (and contributors) fall into predictable traps. This is the checklist. When in doubt:
use the safest well-known pt-PT form and set `needs_native_review: true`.

Native speakers: if anything here is wrong or too prescriptive, please open a PR. You outrank this document.

## 1. Spelling: the 1990 Orthographic Agreement (Acordo Ortográfico)

We use post-1990 spelling, as official in Portugal today.

| Old / avoid | Use | Note |
|-------------|-----|------|
| óptimo, óptima | **ótimo, ótima** | silent consonants dropped |
| eléctrico | **elétrico** | |
| aspecto, espectador | **aspeto, espetador** | silent `c` dropped |
| (keep) facto, contacto | **facto, contacto** | a `c` that *is* pronounced in pt-PT stays (Brazil writes *fato*, *contato*) |
| acção, direcção | **ação, direção** | |
| espectáculo | **espetáculo** | |
| Janeiro, Maio (capitalised months) | **janeiro, maio** | months and days are lower case |
| *pára* (verb) | **para** | the accent on pára (verb) was removed |
| *pêlo, pólo* | **pelo, polo** | |
| *ideia* | **ideia** | the accent on `ei` was removed in paroxytones |

Not changed: the accent on **avô / avó**, **café**, **você**, **português**, **pôr** (verb) vs **por**.
When pt-PT and pt-BR differ even after the agreement (*facto* vs *fato*, *contacto* vs *contato*, *aspeto* vs *aspecto*),
use the pt-PT form.

## 2. Vocabulary: pt-PT vs pt-BR

The linter warns on some pt-BR headwords. Use the left column.

| English | **pt-PT (use)** | pt-BR (avoid) |
|---------|-----------------|---------------|
| bus | **autocarro** | ônibus |
| train | **comboio** | trem |
| mobile phone | **telemóvel** | celular |
| breakfast | **pequeno-almoço** | café da manhã |
| toilet / bathroom | **casa de banho** | banheiro |
| girl | **rapariga** | menina (rapariga is offensive in Brazil) |
| boy | **rapaz** | menino/garoto (rapaz is fine in both) |
| cool, great | **fixe** | legal |
| espresso | **bica** (Lisbon/south), *cimbalino* (Porto), *café* | cafezinho |
| juice | **sumo** | suco |
| fridge | **frigorífico** | geladeira |
| menu | **ementa** | cardápio |
| screen | **ecrã** | tela |
| tram | **elétrico** | bonde |
| bill / check | **a conta** | a conta |
| ice cream | **gelado** | sorvete |
| taxi stop, bus stop | **paragem** | ponto |
| to catch (a bus) | **apanhar** | pegar |
| Mr / Mrs | **senhor / senhora**; *Dona Rosa*, *Sr. Joaquim* | |

## 3. Grammar

### `estar a` + infinitive (not the gerund)
`Estou a comer.` / `Estamos a falar português.` (pt-PT standard) vs `Estou comendo.` (Brazilian). The gerund after
`estar` is heard in the Algarve, Alentejo and Azores, but the standard taught everywhere in Portugal is `a` + infinitive.

### Clitic (pronoun) placement
- Affirmative main clauses: **after the verb with a hyphen** (*enclisis*): `Dá-me água.` `Chamo-me Rita.` `Disse-lhe a verdade.`
- *Negatives, question words, `já`, `também`, `sempre`, `só`, relatives (`que`), `todos` ...* pull the pronoun **before**
  the verb (*proclisis*): `Não me chamo João.` `Quem te disse?` `Também lhe disse.`
- With an infinitive: attach to the infinitive: `Podes ajudar-me?`; futures and conditionals use *mesoclisis*
  (`dar-lhe-ei`), which we do not teach at A1/A2.
- **Never start a sentence with a clitic** (`Me dá...` is Brazilian).

### Tu, você, o senhor / a senhora
- **tu**: friends, family, children, peers. Verb in the 2nd person (`tu falas`, `tu és`). The default for a learner talking to peers.
- **o senhor / a senhora** (or the person's name/title, `a professora`, `o Sr. Joaquim`) with the **3rd-person verb**: polite
  and the safe choice with strangers and elders. `O senhor quer um café?` `Como está a senhora?`
- **você** is *not* the normal polite "you" in Portugal; it can sound cold, distant or even rude. Avoid it in dialogue for
  NPCs unless you want exactly that. (In Brazil it is the everyday "you": a classic trap.)
- Mark register on dialogue lines with `register: "tu"` or `"formal"`.

### Possessives and articles
pt-PT normally keeps the article: **o meu livro, a minha casa**. Dropping it (`meu livro`) is the Brazilian habit.

### Other
- `Queria um café` (I would like a coffee) is the polite way to order; `Quero` alone is blunt.
- `Ter de` + infinitive is the usual "have to" (`Tens de ganhar o Selo`); `ter que` is also heard.
- Prefer `ao pé de`, `perto de` for "near"; `na rua` for "in the street".
- `Vou à praia` (a + a = à): pt-PT uses *ir a* + place. `Vou na praia` is common in colloquial Brazilian Portuguese; avoid it.
- Plural of `-ão`: `pão → pães`, `mão → mãos`, `estação → estações`, `cidadão → cidadãos`: learn each.

## 4. Pronunciation (what makes pt-PT sound like pt-PT)

IPA in this project is a broad transcription of **standard European Portuguese (Lisbon)**. The learning points that
trip up English speakers (each has a `PronTag` and a tip in `pronunciation.ron`):

1. **Reduced unstressed vowels.** Unstressed `e` → /ɨ/ or silent (`leite` ≈ /ˈlejt/), unstressed `o` → /u/ (`obrigado` /obɾiˈgadu/), unstressed `a` → /ɐ/ (`casa` /ˈkazɐ/). The single biggest difference from Brazil.
2. **Nasal vowels.** `bom`, `sim`, `um`, `campo`, `mãe`: the vowel itself is nasal; do not add a clear /n/ or /m/.
3. **Nasal diphthongs** `ão /ɐ̃w̃/`, `ões /õjʃ/`, `ães /ɐ̃jʃ/`, and `-em` at the end of a word (`paragem` /pɐˈɾaʒɐ̃j/).
4. **S and Z.** `s` at the end of a word or before a voiceless consonant is /ʃ/; before a voiced consonant /ʒ/: `dois` /dojʃ/, `mesmo` /ˈmeʒmu/, `estação` /ɨʃtɐˈsɐ̃w/.
5. **Open and closed vowels.** `é /ɛ/` (café), `ê /e/` (você), `ó /ɔ/` (avó), `ô /o/` (avô). Accents carry meaning.
6. **lh /ʎ/, nh /ɲ/, ch /ʃ/.**
7. **R.** Single `r` between vowels is a tap /ɾ/ (caro); word-initial `r` and `rr` are a uvular /ʁ/ (rua, carro). Final `r` is a tap.
8. **Dark L** /ɫ/ at the end of a syllable (azul, Portugal).
9. `x` varies (`táxi` /ˈtaksi/, `exame` /iˈzɐmɨ/, `caixa` /ˈkajʃɐ/): check each word.

## 5. Style for content authors

- Keep sentences short and natural; prefer what people really say (`Tudo bem?`, `Pode ser.`).
- Show register: NPCs speak to the player with *tu* (young, friendly) or *o senhor / a senhora* (shopkeepers to adults).
- Avoid regionalisms unless flagged in `note` (Lisbon `bica` vs Porto `cimbalino`; Azorean and Madeiran forms get their own notes).
- Do not copy dictionary definitions or textbook sentences: write your own.
- Never "correct" a pt-PT form into pt-BR, even if a spell-checker suggests it. Many spell-checkers default to pt-BR.
- Mark it `needs_native_review: true` when unsure.
