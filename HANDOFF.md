# raw-lite — Handoff

Status: **v1 implementation complete and uncommitted for review.**

## Implemented

- Cargo workspace with `raw-lite-core` and `raw-lite-app`.
- `raw-lite-core`: deterministic recursive RAW discovery (including file symlinks, never directory symlinks), validated resize/quality options, external `dcraw`-compatible process invocation with a 60-second timeout, PNM decode, no-upscale resize, collision-safe JPEG output, per-file failure continuation, progress, and summaries.
- Core tests use tiny generated PPM data and Unix fake executables. `tests/real_dcraw.rs` is an ignored integration seam for `RAW_LITE_DCRAW` + `RAW_LITE_SAMPLE_RAW`.
- `raw-lite-app`: Tauri 2 backend bridge, off-thread conversion, overlap guard, progress events, output-folder opening, decoder resolution, vanilla TypeScript drag/drop UI, native folder/file dialogs, advanced defaults (2000 px / quality 75), and completion/failure states.
- Release-only Tauri config wires `binaries/dcraw_emu` as an external binary without breaking ordinary development checks.
- MIT license, product context, design record, and implementation plan are present locally.

## Decoder resolution

1. `RAW_LITE_DCRAW`
2. `dcraw_emu` or `dcraw` beside the app executable
3. `dcraw_emu` or `dcraw` on `PATH`

Release filenames and packaging instructions are in `raw-lite-app/src-tauri/binaries/README.md`.

## Remaining release work

- Supply vetted `dcraw_emu` binaries for each target triple plus required third-party license/source notices.
- Add licensed CR2/ARW/NEF/DNG fixtures and run the ignored real-decoder integration test on each platform.
- Perform the spec's manual drag/drop → progress → completion verification on macOS, Windows, and Linux.
- Signing/notarization remains intentionally out of scope.

## Verification policy

During implementation, targeted `cargo test <name>` commands, regular `cargo check --workspace`, frontend builds, formatting, and Clippy were used. Final verification on 2026-09-11: `cargo check --workspace` passed, Clippy passed with warnings denied, the frontend production build passed, and `cargo test --workspace` passed 17 tests with the env-driven real RAW test ignored. Do not commit until Ray reviews the uncommitted changes.
