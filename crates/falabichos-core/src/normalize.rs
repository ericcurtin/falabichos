// SPDX-License-Identifier: Apache-2.0
//! Answer normalisation and the accent / typo tolerance policy.

use serde::{Deserialize, Serialize};

/// Remove diacritics from a single character (Portuguese + a few extras).
pub fn strip_accent(c: char) -> char {
    match c {
        'á' | 'à' | 'â' | 'ã' | 'ä' => 'a',
        'é' | 'è' | 'ê' | 'ë' => 'e',
        'í' | 'ì' | 'î' | 'ï' => 'i',
        'ó' | 'ò' | 'ô' | 'õ' | 'ö' => 'o',
        'ú' | 'ù' | 'û' | 'ü' => 'u',
        'ç' => 'c',
        'ñ' => 'n',
        'Á' | 'À' | 'Â' | 'Ã' | 'Ä' => 'A',
        'É' | 'È' | 'Ê' | 'Ë' => 'E',
        'Í' | 'Ì' | 'Î' | 'Ï' => 'I',
        'Ó' | 'Ò' | 'Ô' | 'Õ' | 'Ö' => 'O',
        'Ú' | 'Ù' | 'Û' | 'Ü' => 'U',
        'Ç' => 'C',
        other => other,
    }
}

/// Lowercase, trim, collapse whitespace, drop punctuation. Keeps accents.
pub fn normalize(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut last_space = true;
    for c in s.chars() {
        let c = match c {
            '’' | '‘' => '\'',
            '–' | '—' => '-',
            other => other,
        };
        if c.is_alphanumeric() || c == '-' || c == '\'' {
            for l in c.to_lowercase() {
                out.push(l);
            }
            last_space = false;
        } else if c.is_whitespace() && !last_space {
            out.push(' ');
            last_space = true;
        }
        // other punctuation is dropped
    }
    out.trim().to_string()
}

/// Like [`normalize`] and also strips accents.
pub fn fold(s: &str) -> String {
    normalize(s).chars().map(strip_accent).collect()
}

/// Levenshtein edit distance on chars.
pub fn edit_distance(a: &str, b: &str) -> usize {
    let a: Vec<char> = a.chars().collect();
    let b: Vec<char> = b.chars().collect();
    if a.is_empty() {
        return b.len();
    }
    if b.is_empty() {
        return a.len();
    }
    let mut prev: Vec<usize> = (0..=b.len()).collect();
    let mut cur = vec![0; b.len() + 1];
    for i in 1..=a.len() {
        cur[0] = i;
        for j in 1..=b.len() {
            let cost = usize::from(a[i - 1] != b[j - 1]);
            cur[j] = (prev[j] + 1).min(cur[j - 1] + 1).min(prev[j - 1] + cost);
        }
        std::mem::swap(&mut prev, &mut cur);
    }
    prev[b.len()]
}

/// How forgiving typed answers are.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct AnswerPolicy {
    /// Accept answers that differ only by missing/wrong accents (`pao` for `pão`).
    pub accent_insensitive: bool,
    /// Accept near-misses (one slip in a word of 5+ letters, two in 9+).
    pub typo_tolerance: bool,
}

impl Default for AnswerPolicy {
    fn default() -> Self {
        AnswerPolicy { accent_insensitive: true, typo_tolerance: true }
    }
}

impl AnswerPolicy {
    pub const STRICT: AnswerPolicy = AnswerPolicy { accent_insensitive: false, typo_tolerance: false };
}

/// Result of comparing a typed answer to the accepted answers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Verdict {
    /// Exactly right (after case/punctuation normalisation).
    Exact,
    /// Right except for accents; `expected` is the correctly accented form.
    AccentSlip { expected: String },
    /// Close enough: a small typo; `expected` is the right spelling.
    Typo { expected: String },
    Wrong,
}

impl Verdict {
    pub fn is_correct(&self) -> bool {
        !matches!(self, Verdict::Wrong)
    }
}

/// Maximum edit distance tolerated for a word of `len` chars.
pub fn typo_budget(len: usize) -> usize {
    match len {
        0..=4 => 0,
        5..=8 => 1,
        _ => 2,
    }
}

/// Compare `input` against `accepted`. `allow_typos` lets an exercise opt out of typo tolerance
/// (grammar exercises must not accept `falas` for `falam`).
pub fn check_text(input: &str, accepted: &[String], policy: AnswerPolicy, allow_typos: bool) -> Verdict {
    let inp = normalize(input);
    if inp.is_empty() {
        return Verdict::Wrong;
    }
    for a in accepted {
        if normalize(a) == inp {
            return Verdict::Exact;
        }
    }
    let inp_f = fold(input);
    let policy_accents = policy.accent_insensitive;
    // Accent slips: the folded forms match but the accented forms differ.
    for a in accepted {
        if fold(a) == inp_f {
            return if policy_accents {
                Verdict::AccentSlip { expected: a.clone() }
            } else {
                Verdict::Wrong
            };
        }
    }
    if policy.typo_tolerance && allow_typos {
        for a in accepted {
            let af = fold(a);
            let budget = typo_budget(af.chars().count());
            if budget > 0 && edit_distance(&af, &inp_f) <= budget {
                return Verdict::Typo { expected: a.clone() };
            }
        }
    }
    Verdict::Wrong
}

#[cfg(test)]
mod tests {
    use super::*;

    fn acc(s: &[&str]) -> Vec<String> {
        s.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn normalises_case_and_punctuation() {
        assert_eq!(normalize("  Bom   Dia! "), "bom dia");
        assert_eq!(normalize("Olá, tudo bem?"), "olá tudo bem");
    }

    #[test]
    fn folds_accents() {
        assert_eq!(fold("Pão"), "pao");
        assert_eq!(fold("coração"), "coracao");
        assert_eq!(fold("Água"), "agua");
    }

    #[test]
    fn exact_and_accent() {
        let a = acc(&["pão"]);
        assert_eq!(check_text("Pão!", &a, AnswerPolicy::default(), true), Verdict::Exact);
        assert!(matches!(check_text("pao", &a, AnswerPolicy::default(), true), Verdict::AccentSlip { .. }));
        assert_eq!(check_text("pao", &a, AnswerPolicy::STRICT, true), Verdict::Wrong);
    }

    #[test]
    fn typo_tolerance_respects_length_and_optout() {
        let a = acc(&["autocarro"]);
        assert!(matches!(check_text("autocaro", &a, AnswerPolicy::default(), true), Verdict::Typo { .. }));
        assert_eq!(check_text("autocaro", &a, AnswerPolicy::default(), false), Verdict::Wrong);
        // Short words never get typo tolerance: "mal" must not pass for "mau".
        assert_eq!(check_text("mal", &acc(&["mau"]), AnswerPolicy::default(), true), Verdict::Wrong);
    }

    #[test]
    fn empty_is_wrong() {
        assert_eq!(check_text("  ", &acc(&["a"]), AnswerPolicy::default(), true), Verdict::Wrong);
    }

    #[test]
    fn distance_basics() {
        assert_eq!(edit_distance("kitten", "sitting"), 3);
        assert_eq!(edit_distance("", "abc"), 3);
        assert_eq!(edit_distance("abc", "abc"), 0);
    }
}
