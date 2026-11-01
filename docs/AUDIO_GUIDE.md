# Audio guide: recording pt-PT pronunciation

Pronunciation is central to learning European Portuguese. The game has a **slot system** for native-speaker clips and
degrades gracefully (reading the IPA) when a clip is missing. We are looking for **voice contributors from Portugal**.

## How slots work

Each vocabulary item has a *slot*: `audio/pt-PT/<category>/<id>.ogg`. For `bica` in category `cafe`:

```
assets/audio/pt-PT/cafe/bica.ogg
```

You do **not** need to edit any data: drop the file in the right place and the game uses it (an item may override the slot with its `audio:` field).
Dialogue lines can also set `audio: "avo/bom_dia"` in the data to point at a clip.

Where clips play:

- after you answer a vocabulary question (right or wrong);
- in **listening exercises**: when a clip exists, "which word did you hear?" uses it instead of the IPA reading (press Menu to replay);
- on dialogue lines that reference a clip.

## Recording spec

| | |
|--|--|
| Content | the headword **with its article for nouns**, exactly as in the game (`a bica`, `o pastel de nata`, `bom dia`), then silence. One take per file. |
| Voice | native speaker from Portugal, standard (Lisbon) or clearly noted regional; natural but clear, not exaggerated |
| Format | **Ogg Vorbis, mono, 44.1 kHz** (quality 4-5, about 64-96 kbps) |
| Length | 0.4-2.5 s; trim silence to about 100 ms at the start and 150 ms at the end |
| Level | peak around -3 dBFS, loudness about -18 LUFS; consistent across all files |
| Noise | quiet room, pop filter, no clipping, no music, no breaths; remove clicks |
| Naming | exactly `<id>.ogg` in `<category>/`, lowercase ASCII |
| Licence | by contributing a file you agree it is licensed under **Apache-2.0** (and that you have the right to license it). Add yourself to `CREDITS` in the PR if you wish. |

Free tools: [Audacity](https://www.audacityteam.org/) to record/trim, `ffmpeg -i in.wav -ac 1 -ar 44100 -c:a libvorbis -q:a 4 out.ogg` to convert.

## Checking your work

```sh
cargo run -p falabichos-tools --bin falabichos-lint -- --missing-audio    # lists vocab still without a clip + the path to record
cargo run -p falabichos-tools --bin falabichos-lint                       # warns about clips whose name matches no slot (typos)
```

The summary line shows `N/M vocab items have a recording`.

## What is not implemented yet

- Recorded dialogue for whole lines is supported by the data format but no dialogue has audio yet.
- Speech recognition (checking the learner's own pronunciation) is deliberately out of scope until a permissively licensed offline option exists.
