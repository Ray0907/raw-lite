# raw-lite

Cross-platform (macOS/Windows/Linux) desktop app that batch-converts RAW
camera files into smaller JPEGs. Drop your files, pick an output folder,
get JPEGs — no library, no editing, no learning curve.

## Why

RAW files are huge and most photo apps that read them assume you want a
whole editing/catalog workflow. raw-lite does one thing: turn a folder of
RAW files into a folder of reasonably-sized JPEGs you can actually share.

## Features

- Broad RAW format support (`.cr2`, `.arw`, `.nef`, `.dng`, and more —
  whatever the bundled `dcraw`/`dcraw_emu` decoder handles).
- Drag-and-drop files or whole folders; recurses subfolders.
- One bad/corrupt file never stops the batch — failures are collected and
  shown at the end.
- Sane defaults (2000px long edge, quality 75) with optional advanced
  controls.
- No catalog, no thumbnails, no editing, no SD-card auto-detection —
  conversion only.

## Status

v1, unsigned (no Apple notarization / Windows code signing yet). Core
conversion logic and desktop shell are implemented and tested; packaging
per-platform `dcraw_emu` binaries and real-RAW-file/manual QA across all
three OSes is still outstanding before a public release build.

## Project layout

```
crates/raw-lite-core/   pure Rust RAW→JPEG conversion logic, no UI deps
raw-lite-app/           Tauri v2 desktop shell (frontend + Rust backend)
```

`raw-lite-core` has no dependency on the app shell, so it can be reused by
a CLI or a different UI later without a rewrite.

## Development

Requires Rust (2024 edition) and Node.js.

```bash
# run the desktop app in dev mode
cd raw-lite-app
npm install
npm run tauri dev

# run tests / checks from repo root
cargo check --workspace
cargo test --workspace
```

Development and `cargo test` resolve the decoder via (in order):
`RAW_LITE_DCRAW` env var → a decoder next to the app executable →
`dcraw_emu`/`dcraw` on `PATH`. See
[`raw-lite-app/src-tauri/binaries/README.md`](raw-lite-app/src-tauri/binaries/README.md)
for how release builds bundle a real per-platform `dcraw_emu` binary.

## Docs

- [`PRODUCT.md`](PRODUCT.md) — product scope, users, principles.
- [`DESIGN.md`](DESIGN.md) — design decisions and rationale.

## License

MIT — see [`LICENSE`](LICENSE).
