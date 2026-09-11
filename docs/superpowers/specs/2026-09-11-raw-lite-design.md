# raw-lite: RAW Photo Converter — Design Spec

Status: approved for implementation planning
Date: 2026-09-11

## Purpose

A free, open-source, cross-platform desktop app that converts RAW camera
files (CR2, ARW, NEF, DNG, and other mainstream formats) into smaller JPEGs,
for non-technical photographers who just want to get shrunk copies of their
photos onto another device (phone, cloud, chat app) without a full photo
management suite.

This is v1 of a longer-term direction ("a Lightroom for amateurs"), but v1
deliberately does the smallest useful slice: batch RAW → JPEG conversion.
Library management, non-destructive editing, and develop-style adjustments
are explicitly out of scope for v1 and are not designed here — they will get
their own spec if and when they're built, so this design does not add
extension points for them.

## Non-goals (v1)

- No photo library/catalog, no thumbnail grid, no per-photo editing.
- No AirDrop/cloud/transfer integration — output lands in a folder, the OS
  file manager (Finder/Explorer/whatever Linux DE) handles the rest.
- No memory-card/DCIM auto-detection.
- No code signing / notarization (users will see an unsigned-app warning on
  first run on macOS and Windows).
- No standalone CLI entry point in v1 (see Architecture — the conversion
  logic is a separate library so a CLI can be added later without a
  rewrite, but v1 ships GUI only).

## Target users & platform

General public, not programmers. macOS, Windows, and Linux, all first-class
(no platform is primary/secondary). AirDrop was the original motivating use
case but is explicitly not integrated — it's a manual step the user does
themselves after conversion, same as any other transfer method.

## Approach: Tauri, not Electron

Considered Electron (bundles full Chromium + Node per app, larger memory
footprint, slower startup) vs Tauri (Rust backend, renders through the
OS's own WebView — WKWebView on macOS, WebView2 on Windows, WebKitGTK on
Linux). Frontend code is HTML/CSS/JS either way, so UI capability is
equivalent.

Chosen: **Tauri**, on memory/startup footprint grounds — the OS-provided
WebView avoids shipping a full browser engine per app, which matters for a
lightweight utility a user runs occasionally rather than keeps open.
Trade-off accepted: backend logic is written in Rust rather than
Node/TypeScript, which is a new language for the developer; this was an
explicit choice to prioritize runtime footprint over implementation
familiarity.

RAW decoding: an external `dcraw`/`dcraw_emu` binary, invoked as a child
process, one prebuilt binary bundled per target platform. This avoids
compiling a RAW-decoding library (e.g. libraw) per platform inside the
Tauri build and gives the broadest, most battle-tested format coverage.

## Architecture

Two pieces:

1. **`raw-lite-core`** — a Rust library crate with no UI dependency. Public
   surface: given a list of input file paths, an output directory, and
   conversion parameters (max dimension, JPEG quality), it invokes
   `dcraw`/`dcraw_emu` per file, reports progress, and returns a summary of
   successes and failures. This is the seam that lets a CLI or a future
   Lightroom-style app reuse the same conversion logic without depending on
   Tauri or any GUI code.
2. **`raw-lite-app`** — the Tauri application. Rust backend calls into
   `raw-lite-core`; the frontend (HTML/CSS/JS) is a single-screen UI that
   talks to the backend over Tauri's command/event bridge (invoke commands
   to start a conversion, listen for progress events).

No plugin system, no dynamic module loading — "separate crate" is the only
extension mechanism, deliberately minimal.

## UI flow

Single-window, single-screen app:

1. On open, prompt the user to pick an output folder (native folder-picker
   dialog).
2. Show a drag-and-drop zone: user drags in RAW files and/or folders
   (folders are scanned recursively for supported RAW extensions).
3. Conversion starts automatically on drop. A progress bar shows
   `N / total` files processed.
4. On completion, the output folder is opened automatically in the native
   file manager (Finder/Explorer/xdg-open equivalent), and a summary is
   shown: how many succeeded, and — if any failed — a list of failed
   filenames with a one-line reason each.

No thumbnail previews, no per-file selection UI, no settings panel wired
into v1's primary flow — conversion parameters (max dimension, JPEG
quality) ship with a sane default (matching what was manually validated
during the RICOH GR batch: 2000px long edge; quality is user-tunable but
defaults toward smaller files, since "make it smaller" was the explicit
feedback that drove this project) and are exposed as advanced/optional
settings, not a required step in the golden path.

## Error handling

Per-file: if `dcraw`/`dcraw_emu` fails or the format is unsupported, that
file is skipped and recorded as a failure; the batch continues. Nothing
stops the whole run over one bad file — non-technical users expect "drop
photos in, get photos out," and a hard stop on the 47th file out of 200
would be a worse experience than a completion summary that says "197
converted, 3 failed: reasons X, Y, Z."

## Format scope

Mainstream RAW formats as supported by `dcraw`/`dcraw_emu` (CR2, ARW, NEF,
DNG, and the other formats dcraw already handles) — not limited to DNG,
since the public, three-platform release is aimed at a broad camera-owning
audience, not just DNG-shooting cameras (Ricoh/Leica/Pentax).

## Licensing

MIT.

## Testing

- `raw-lite-core`: unit/integration tests that run real `dcraw` conversions
  against a small set of sample RAW files (one per major format family) and
  assert on output existence, dimensions, and the failure path for a
  corrupt/unsupported input.
- `raw-lite-app`: manual verification of the drag-and-drop → progress →
  completion flow on each target platform before release, since Tauri
  frontend E2E tooling is less established than Electron's; automated UI
  testing is not required for v1.

## Distribution

Unsigned builds for v1 on all three platforms — users will need to manually
bypass Gatekeeper (macOS) / SmartScreen (Windows) warnings on first launch.
Code signing and notarization are deferred until there's a validated user
base to justify the ongoing cost (Apple Developer Program fee + Windows
code-signing certificate).
