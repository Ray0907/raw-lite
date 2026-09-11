use raw_lite_core::{
    ConversionFailure, ConversionOptions, ConversionProgress, ConversionSummary, convert_batch,
    discover_raw_files,
};
use serde::Serialize;
use std::{
    ffi::OsString,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};
use tauri::{AppHandle, Emitter, State};
use tauri_plugin_opener::OpenerExt;

fn decoder_filename(base: &str) -> OsString {
    OsString::from(if cfg!(windows) {
        format!("{base}.exe")
    } else {
        base.to_owned()
    })
}

fn resolve_decoder(
    environment_override: Option<OsString>,
    executable_directory: Option<&Path>,
    path_environment: Option<OsString>,
) -> PathBuf {
    if let Some(path) = environment_override.filter(|path| !path.is_empty()) {
        return path.into();
    }

    let names = [decoder_filename("dcraw_emu"), decoder_filename("dcraw")];
    if let Some(directory) = executable_directory {
        for name in &names {
            let candidate = directory.join(name);
            if candidate.is_file() {
                return candidate;
            }
        }
    }
    if let Some(path) = path_environment {
        for directory in std::env::split_paths(&path) {
            for name in &names {
                let candidate = directory.join(name);
                if candidate.is_file() {
                    return candidate;
                }
            }
        }
    }
    PathBuf::from(&names[0])
}

#[derive(Clone, Default)]
struct BatchGate(Arc<AtomicBool>);

impl BatchGate {
    fn acquire(&self) -> Result<BatchPermit, &'static str> {
        self.0
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .map(|_| BatchPermit(self.0.clone()))
            .map_err(|_| "a conversion is already running")
    }
}

struct BatchPermit(Arc<AtomicBool>);

impl Drop for BatchPermit {
    fn drop(&mut self) {
        self.0.store(false, Ordering::Release);
    }
}

struct AppState {
    decoder: PathBuf,
    gate: BatchGate,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProgressPayload {
    processed: usize,
    total: usize,
    filename: String,
    succeeded: bool,
}

impl From<ConversionProgress> for ProgressPayload {
    fn from(progress: ConversionProgress) -> Self {
        Self {
            processed: progress.processed,
            total: progress.total,
            filename: filename(&progress.input),
            succeeded: progress.succeeded,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct FailurePayload {
    filename: String,
    reason: String,
}

impl From<ConversionFailure> for FailurePayload {
    fn from(failure: ConversionFailure) -> Self {
        Self {
            filename: filename(&failure.input),
            reason: failure.reason,
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct SummaryPayload {
    succeeded: usize,
    failed: Vec<FailurePayload>,
    folder_open_error: Option<String>,
}

fn filename(path: &Path) -> String {
    path.file_name()
        .unwrap_or(path.as_os_str())
        .to_string_lossy()
        .into_owned()
}

#[tauri::command]
async fn start_conversion(
    app: AppHandle,
    state: State<'_, AppState>,
    paths: Vec<String>,
    output_directory: String,
    max_dimension: u32,
    quality: u8,
) -> Result<SummaryPayload, String> {
    let permit = state.gate.acquire().map_err(str::to_owned)?;
    let decoder = state.decoder.clone();

    tauri::async_runtime::spawn_blocking(move || {
        let _permit = permit;
        let inputs = discover_raw_files(&paths.into_iter().map(PathBuf::from).collect::<Vec<_>>())
            .map_err(|error| format!("Could not read the dropped files: {error}"))?;
        if inputs.is_empty() {
            return Err("No supported RAW files were found.".to_owned());
        }
        let options =
            ConversionOptions::new(max_dimension, quality).map_err(|error| error.to_string())?;
        let output_path = PathBuf::from(&output_directory);
        let event_app = app.clone();
        let summary = convert_batch(&decoder, &inputs, &output_path, options, move |progress| {
            let _ = event_app.emit("conversion-progress", ProgressPayload::from(progress));
        })
        .map_err(|error| format!("Could not start conversion: {error}"))?;

        Ok(summary_payload(
            summary,
            app.opener()
                .open_path(output_directory, None::<&str>)
                .err()
                .map(|error| error.to_string()),
        ))
    })
    .await
    .map_err(|error| format!("Conversion worker failed: {error}"))?
}

fn summary_payload(
    summary: ConversionSummary,
    folder_open_error: Option<String>,
) -> SummaryPayload {
    SummaryPayload {
        succeeded: summary.converted.len(),
        failed: summary.failed.into_iter().map(Into::into).collect(),
        folder_open_error,
    }
}

fn configured_decoder() -> PathBuf {
    let executable_directory = std::env::current_exe()
        .ok()
        .and_then(|path| path.parent().map(Path::to_path_buf));
    resolve_decoder(
        std::env::var_os("RAW_LITE_DCRAW"),
        executable_directory.as_deref(),
        std::env::var_os("PATH"),
    )
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_opener::init())
        .manage(AppState {
            decoder: configured_decoder(),
            gate: BatchGate::default(),
        })
        .invoke_handler(tauri::generate_handler![start_conversion])
        .run(tauri::generate_context!())
        .expect("raw-lite failed to start");
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{ffi::OsString, fs, path::PathBuf};

    static NEXT_TEST_DIR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "raw-lite-app-{}-{}-{}",
                std::process::id(),
                std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap()
                    .as_nanos(),
                NEXT_TEST_DIR.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            fs::create_dir_all(&path).unwrap();
            Self(path)
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn decoder_environment_override_wins() {
        assert_eq!(
            resolve_decoder(Some(OsString::from("/custom/dcraw")), None, None,),
            PathBuf::from("/custom/dcraw")
        );
    }

    #[test]
    fn bundled_decoder_wins_over_path() {
        let root = TestDir::new();
        let bundled = root.0.join(decoder_filename("dcraw_emu"));
        fs::write(&bundled, b"").unwrap();
        let path_dir = root.0.join("path");
        fs::create_dir(&path_dir).unwrap();
        fs::write(path_dir.join(decoder_filename("dcraw_emu")), b"").unwrap();

        assert_eq!(
            resolve_decoder(
                None,
                Some(&root.0),
                Some(std::env::join_paths([path_dir]).unwrap()),
            ),
            bundled
        );
    }

    #[test]
    fn batch_gate_rejects_overlap_and_resets_on_drop() {
        let gate = BatchGate::default();
        let permit = gate.acquire().unwrap();
        assert!(gate.acquire().is_err());
        drop(permit);
        assert!(gate.acquire().is_ok());
    }
}
