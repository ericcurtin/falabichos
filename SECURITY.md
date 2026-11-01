# Security Policy

## Supported versions

Falabichos is pre-1.0. Security fixes are made on the `main` branch and released in the next version.

## What counts as a security issue here

Falabichos is an offline game: it has no accounts, no server and makes no network connections. Relevant issues are, for example:

- a crafted **save file** or **content/asset file** (RON, `.px`, `.song`, `.lang`) that crashes the game, hangs it or reads or writes outside the data directory;
- a vulnerable dependency that affects shipped binaries;
- problems in the release or CI workflows (leaked secrets, unsafe permissions).

Because the game loads content from RON files (and optionally from a directory via `FALABICHOS_ASSETS` / `--assets`), please treat untrusted mods like untrusted programs: they cannot run code, but they can be malformed.

## Reporting a vulnerability

Please **do not open a public issue**. Report it privately through GitHub's *Security advisories → Report a vulnerability* on this repository, or email **security@falabichos.invalid** (maintainers: replace this placeholder with a monitored address before publishing).

Include what you found, how to reproduce it, and the version/platform. You will get an acknowledgement within a week; we aim to fix confirmed issues within 30 days and will credit you in the changelog unless you prefer otherwise.

## Hardening already in place

- Rust, with `unsafe_code = "forbid"` in every crate except the tiny Android/iOS entry-point glue, which uses `deny` plus a local allow for the `#[no_mangle]` entry points.
- Saves are written atomically (temp file + rename) with a `.bak` of the previous version, parsed with strict RON, and versioned with explicit migrations; unknown future versions are rejected rather than guessed.
- Content is validated at load time and by `falabichos-lint` in CI.
