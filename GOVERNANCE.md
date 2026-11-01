# Governance

Falabichos wants a large, friendly, long-lived community. This document says how decisions get made so that
nobody has to guess.

## Roles

- **Contributors**: anyone who opens an issue, a discussion or a pull request.
- **Reviewers**: trusted contributors who review pull requests in an area. Native-speaker reviewers own
  Portuguese content; artists own the art style; Rust reviewers own code. A reviewer approval is
  required from the relevant area.
- **Maintainers**: merge pull requests, cut releases, moderate, and keep the project healthy. The initial
  maintainer is the repository owner; new maintainers are invited after sustained, high-quality contribution.

## How decisions are made

1. **Everyday changes** (content, bug fixes, small features): one approving review from the relevant area, CI green.
2. **Big changes** (new engine version, new crate, changes to the content format or save format, new platform):
   open an issue or discussion first; lazy consensus for at least one week; a maintainer records the decision
   in the issue and, if it is architectural, in `docs/ARCHITECTURE.md`.
3. **Portuguese correctness**: when a native speaker and a non-native contributor disagree, the native speaker
   from Portugal decides. When native speakers disagree, prefer the most widely understood standard form and
   note the regional variant in the item's `note` field.
4. **Disagreements** are resolved by the maintainers; the Code of Conduct applies to the discussion.

## Principles

- Learners first: if a change makes the game more fun but teaches wrong Portuguese, it does not ship.
- European Portuguese (pt-PT), post-1990 spelling, clearly marked where uncertain (`needs_native_review`).
- Everything is **Apache-2.0**, original or permissively licensed, and recorded in THIRD_PARTY.md.
- Native-only: no web/WASM build targets.
- The content is data: teachers and native speakers must never need Rust to contribute.

## Releases

Semantic versioning once 1.0 is reached; before that, minor versions may change formats but never break saves
without a migration (see `crates/falabichos-core/src/save.rs`). Releases are tags `vX.Y.Z`; the release workflow
builds artifacts for all five platforms. `CHANGELOG.md` follows *Keep a Changelog*.

## Changing this document

Via pull request, with a maintainer approval after a one-week comment period.
