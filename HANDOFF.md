# raw-lite — Handoff

Status: **design approved, no code written yet.** Next step is
`superpowers:writing-plans` to turn the spec into an implementation plan.

## What this project is

Cross-platform (macOS/Windows/Linux) desktop app that batch-converts RAW
camera files (CR2, ARW, NEF, DNG, etc.) into smaller JPEGs, for
non-technical users. v1 scope is conversion only — no library management,
no editing. Full rationale and decisions: see
`docs/superpowers/specs/2026-09-11-raw-lite-design.md`.

## Key decisions (already made, don't re-litigate without reason)

- **Tauri**, not Electron — chosen for memory/startup footprint (OS
  WebView vs bundled Chromium), even though it means writing backend logic
  in Rust rather than TypeScript/Node.
- **External `dcraw`/`dcraw_emu` binary** for RAW decoding, one prebuilt
  binary bundled per platform — not a Rust RAW-decoding crate, not
  sharp/libvips.
- **Two crates**: `raw-lite-core` (pure Rust conversion logic, no UI
  dependency) + `raw-lite-app` (Tauri shell). This split exists so a CLI or
  a future GUI can reuse `raw-lite-core` without a rewrite — but v1 ships
  GUI only, no CLI entry point yet.
- **Format scope**: mainstream RAW formats broadly (whatever
  `dcraw`/`dcraw_emu` supports), not DNG-only — this was upgraded mid-design
  once "macOS-only, DNG-only" was replaced with "public, three-platform
  release."
- **Error handling**: skip bad/unsupported files, keep converting, show a
  failure list at the end. Never hard-stop a batch on one bad file.
- **No AirDrop/transfer integration** — output goes to a folder the user
  picked, OS file manager handles the rest. This was the original
  motivating use case (a same-day task: converting Ricoh GR RAW files and
  AirDropping them to an iPhone) but was deliberately descoped from the
  tool itself.
- **Unsigned release for v1** — no Apple notarization, no Windows code
  signing, deferred until there's a validated user base.
- **MIT license.**

## Explicitly out of scope for v1 (don't add without a new design pass)

- Thumbnail grid / library / catalog / non-destructive editing (the
  "Lightroom for amateurs" long-term direction — deliberately not designed
  yet, gets its own spec later if pursued).
- SD-card / DCIM auto-detection.
- Standalone CLI.
- Code signing / notarization.

## Repo state

- `git init` done, identity set (Ray Tien / ray.tien0907@gmail.com — this
  repo is outside `~/Documents/work`, so the personal identity applies, not
  a work one).
- One commit: the design spec.
- No source code, no Cargo.toml / package.json / Tauri scaffold yet.

## Next step

Run `superpowers:writing-plans` against the spec to produce an
implementation plan (phased: `raw-lite-core` with tests first, then the
Tauri shell wired to it). Do not skip straight to scaffolding — the spec
was approved but not yet turned into a plan.
