# Bundled decoder binaries

Release builds require a real `dcraw_emu` binary for each target, named as Tauri expects:

- `dcraw_emu-aarch64-apple-darwin`
- `dcraw_emu-x86_64-apple-darwin`
- `dcraw_emu-x86_64-pc-windows-msvc.exe`
- `dcraw_emu-x86_64-unknown-linux-gnu`

`npm run bundle` merges `tauri.release.conf.json`, whose `bundle.externalBin` packages the matching target binary. Development and ordinary checks do not require a placeholder: they resolve `RAW_LITE_DCRAW`, then a decoder beside the app executable, then `dcraw_emu`/`dcraw` on `PATH`.

Do not add a fake placeholder executable: it can accidentally ship as the decoder.

Verified on macOS (aarch64): copying a real Homebrew `dcraw_emu` (from
the `libraw` package) to `dcraw_emu-aarch64-apple-darwin`, then running
`npm run bundle`, produces a `.app` that converts a real RAW file with
`PATH` scrubbed of Homebrew — confirming the bundled binary is what
actually gets used, not something found on the host. `x86_64-apple-darwin`,
Windows, and Linux binaries are still needed and untested.
