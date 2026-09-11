# Product

<!-- impeccable:product-schema 1 -->

## Platform

web

## Stack

Tauri 2 desktop shell with a vanilla TypeScript/Vite frontend and a Rust conversion core. The approved spec delegates the smallest frontend implementation consistent with HTML/CSS/JS.

## Users

Non-technical photographers on macOS, Windows, and Linux who need smaller copies of RAW photos for a phone, cloud storage, or chat.

## Product Purpose

Convert dropped RAW files or folders into smaller JPEGs with one output-folder choice and clear batch progress. Success means users get usable JPEGs without learning a photo-management suite.

## Positioning

A lightweight, occasional-use converter: broad `dcraw`/`dcraw_emu` format support without a catalog, editing workflow, or bundled browser engine.

## Operating Context

Users choose an output folder, drop RAW files or folders, wait for batch conversion, then continue in the operating system's file manager.

## Capabilities and Constraints

- Recursively discover mainstream RAW formats.
- Convert independently so one bad file does not stop the batch.
- Default to a 2000 px long edge and JPEG quality 75, with optional advanced controls.
- Tauri shell calls a reusable UI-independent Rust crate and a bundled platform decoder.
- No catalog, editing, thumbnails, transfer integration, memory-card detection, standalone CLI, signing, or notarization in v1.
- MIT license for raw-lite; third-party decoder distribution obligations remain separate release work.

## Evidence on Hand

The approved product and flow are documented in `DESIGN.md`. No logo, photography, testimonials, benchmarks, or bundled decoder binaries are present and none should be fabricated.

## Product Principles

- Make the golden path output folder → drop → converted folder.
- Keep technical decoder details out of routine use but make failures actionable.
- Never lose an existing output file.
- Prefer operating-system capabilities over duplicate transfer or file-management UI.
