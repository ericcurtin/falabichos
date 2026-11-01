// SPDX-License-Identifier: Apache-2.0
//! Templates for `falabichos-new`.

pub fn is_id(s: &str) -> bool {
    !s.is_empty()
        && s.chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || c == '_')
}

/// Returns `(relative path under assets/, file contents)` pairs.
pub fn files_for(kind: &str, id: &str) -> Result<Vec<(String, String)>, String> {
    if !is_id(id) {
        return Err(format!(
            "`{id}` is not a valid id (lowercase letters, digits and `_` only)"
        ));
    }
    Ok(match kind {
        "vocab" => vec![(format!("content/vocab/{id}.ron"), vocab(id))],
        "grammar" => vec![(format!("content/grammar/{id}.ron"), grammar(id))],
        "bicho" => vec![
            (format!("content/bichos/{id}.ron"), bicho(id)),
            (format!("art/bichos/{id}.px"), bicho_art(id)),
        ],
        "town" => vec![
            (format!("content/towns/{id}/town.ron"), town(id)),
            (format!("content/towns/{id}/maps/{id}_outside.ron"), map(id)),
            (format!("content/towns/{id}/dialogues/{id}.ron"), dialogues(id)),
        ],
        other => {
            return Err(format!(
                "unknown kind `{other}` (use: vocab, grammar, bicho, town)"
            ));
        }
    })
}

fn vocab(id: &str) -> String {
    format!(
        r#"// Vocabulary: category `{id}`. One entry per word or phrase.
// Full field reference: docs/CONTENT_FORMAT.md. Run `cargo run -p falabichos-tools --bin falabichos-lint` to check.
[
    (
        id: "{id}_example",              // never rename once released: saves refer to it
        pt: "exemplo",                   // European Portuguese (pt-PT), post-1990 spelling, no article
        en: ["example"],                 // first entry is the main translation
        category: "{id}",
        kind: Noun,                      // Noun | Verb | Adjective | Phrase | Other
        gender: Masc,                    // Masc | Fem (nouns)
        ipa: "ɨzˈẽplu",                  // pt-PT IPA, no slashes
        pron_tags: [ReducedVowel, NasalVowel],
        example: (pt: "Este é um exemplo.", en: "This is an example."),
        needs_native_review: true,       // set to false once a native speaker has checked it
    ),
]
"#
    )
}

fn grammar(id: &str) -> String {
    format!(
        r#"// Grammar concept `{id}`: a notebook page plus its practice exercises.
(
    id: "{id}",
    title: "Title of the concept",
    summary: "One sentence explaining it.",
    rules: [
        "Rule one.",
        "Rule two.",
    ],
    examples: [
        (pt: "Exemplo em português.", en: "Example in English."),
    ],
    pt_pt_note: "How European Portuguese differs from Brazilian Portuguese here (optional).",
    exercises: [
        (
            id: "{id}_1",                       // globally unique
            kind: FillBlank,                    // Choice | FillBlank | Type | Order
            prompt: "Eu ___ português.",
            context: "I am Portuguese.",
            options: ["sou", "estou"],
            answer: ["sou"],
            explanation: "Why this is right.",
        ),
        (
            id: "{id}_2",
            kind: Order,
            prompt: "I am eating.",
            prompt_lang: En,
            answer: ["Estou a comer."],
        ),
        (
            id: "{id}_3",
            kind: Type,
            prompt: "you (informal) speak",
            prompt_lang: En,
            answer: ["falas"],
        ),
    ],
    needs_native_review: true,
)
"#
    )
}

fn bicho(id: &str) -> String {
    format!(
        r#"// A Bicho species. Link it to vocab ids (or grammar concept ids for a grammar Bicho).
(
    id: "{id}",
    name: "{name}",
    dex_no: 99,                               // must be unique: see the other files in this folder
    kind: Vocab,                              // Vocab | Grammar
    theme: "what the words are about",
    flavor_pt: "Uma frase curta sobre o Bicho.",
    flavor_en: "A short sentence about the Bicho.",
    vocab: ["some_vocab_id", "another_vocab_id", "a_third_vocab_id"],
    art: "{id}",                              // assets/art/bichos/{id}.px
    trust_needed: 5,
    cry_hz: 440,
    needs_native_review: true,
)
"#,
        name = {
            let mut c = id.chars();
            c.next()
                .map(|f| f.to_uppercase().collect::<String>() + c.as_str())
                .unwrap_or_default()
        }
    )
}

fn bicho_art(id: &str) -> String {
    format!(
        "# Bicho: {id}. 16x16, palette letters from art/palette.txt, auto-outline, idle bob.\n# See docs/BICHO_DESIGN.md for the style guide.\nsize: 16x16\noutline: K\nbob: yes\n................\n................\n................\n.....yyyyyy.....\n....yYYYYYYy....\n...yYYKYYKYYy...\n...yYYYYYYYYy...\n...yYYYooYYYy...\n...yYYYYYYYYy...\n....yyyyyyyy....\n................\n................\n................\n................\n................\n................\n"
    )
}

fn town(id: &str) -> String {
    format!(
        r#"(
    id: "{id}",
    name: "{id}",
    blurb: "One line about the town.",
    inspired_by: "Real-world region that inspired it",
    start_map: "{id}_outside",
    needs_native_review: true,
)
"#
    )
}

fn map(id: &str) -> String {
    format!(
        r#"// Map for town `{id}`. Tile characters are defined in assets/content/tiles.ron.
// Edit the rows below: every row must have the same length.
// Run `cargo run -p falabichos-tools --bin falabichos-mapcheck -- {id}_outside` to see problems.
(
    id: "{id}_outside",
    name: "{id}",
    town: "{id}",
    music: "town",
    rows: [
        "TTTTTTTTTTTTTTTT",
        "T..............T",
        "T..............T",
        "T..............T",
        "T..............T",
        "T..............T",
        "TTTTTTTTTTTTTTTT",
    ],
    npcs: [
        (id: "{id}_walker", x: 5, y: 3, art: "turista", dialogue: "{id}_hello"),
    ],
)
"#
    )
}

fn dialogues(id: &str) -> String {
    format!(
        r#"// Dialogues for `{id}`. The first branch whose `when` conditions hold is played.
[
    (
        id: "{id}_hello",
        branches: [
            (
                lines: [
                    (who: "Pessoa", pt: "Bom dia! Tudo bem?", en: "Good morning! How are you?",
                     words: {{"tudo": "everything"}}, register: "tu", needs_native_review: true),
                ],
            ),
        ],
    ),
]
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn templates_parse_as_ron() {
        for kind in ["vocab", "grammar", "bicho", "town"] {
            let files = files_for(kind, "demo").unwrap();
            assert!(!files.is_empty());
        }
        assert!(files_for("vocab", "Bad Id").is_err());
        assert!(files_for("nope", "x").is_err());
    }
}
