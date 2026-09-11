use image::{DynamicImage, ImageFormat, codecs::jpeg::JpegEncoder, imageops::FilterType};
use std::{
    ffi::OsString,
    fmt, fs,
    fs::{File, OpenOptions},
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

const DECODER_TIMEOUT: Duration = Duration::from_secs(60);
const DECODER_POLL_INTERVAL: Duration = Duration::from_millis(10);

const RAW_EXTENSIONS: &[&str] = &[
    "3fr", "arw", "cr2", "cr3", "dcr", "dng", "erf", "fff", "iiq", "kdc", "mef", "mos", "mrw",
    "nef", "nrw", "orf", "pef", "raf", "raw", "rw2", "rwl", "sr2", "srf", "srw", "x3f",
];

#[derive(Debug)]
pub enum Error {
    InvalidOptions(&'static str),
    Io(io::Error),
}

impl fmt::Display for Error {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidOptions(message) => formatter.write_str(message),
            Self::Io(error) => error.fmt(formatter),
        }
    }
}

impl std::error::Error for Error {}

impl From<io::Error> for Error {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ConversionOptions {
    max_dimension: u32,
    quality: u8,
}

impl ConversionOptions {
    pub fn new(max_dimension: u32, quality: u8) -> Result<Self, Error> {
        if max_dimension == 0 {
            return Err(Error::InvalidOptions(
                "max dimension must be greater than zero",
            ));
        }
        if !(1..=100).contains(&quality) {
            return Err(Error::InvalidOptions(
                "JPEG quality must be between 1 and 100",
            ));
        }
        Ok(Self {
            max_dimension,
            quality,
        })
    }

    pub fn max_dimension(self) -> u32 {
        self.max_dimension
    }

    pub fn quality(self) -> u8 {
        self.quality
    }
}

pub fn is_supported_raw(path: impl AsRef<Path>) -> bool {
    path.as_ref()
        .extension()
        .and_then(|extension| extension.to_str())
        .is_some_and(|extension| RAW_EXTENSIONS.contains(&extension.to_ascii_lowercase().as_str()))
}

pub fn discover_raw_files(paths: &[PathBuf]) -> Result<Vec<PathBuf>, Error> {
    let mut files = Vec::new();
    for path in paths {
        collect_raw_files(path, &mut files)?;
    }
    files.sort_by(|left, right| {
        left.file_name()
            .cmp(&right.file_name())
            .then_with(|| left.cmp(right))
    });
    files.dedup();
    Ok(files)
}

fn collect_raw_files(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), Error> {
    let metadata = fs::metadata(path)?;
    if metadata.is_file() {
        if is_supported_raw(path) {
            files.push(path.canonicalize()?);
        }
    } else if metadata.is_dir() {
        for entry in fs::read_dir(path)? {
            let entry = entry?;
            let entry_path = entry.path();
            if entry.file_type()?.is_symlink() {
                let Ok(target) = fs::metadata(&entry_path) else {
                    continue;
                };
                if target.is_file() && is_supported_raw(&entry_path) {
                    files.push(entry_path.canonicalize()?);
                }
                // ponytail: directory symlinks stay skipped; add cycle detection before following them.
                continue;
            }
            collect_raw_files(&entry_path, files)?;
        }
    }
    Ok(())
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConvertedFile {
    pub input: PathBuf,
    pub output: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversionFailure {
    pub input: PathBuf,
    pub reason: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ConversionSummary {
    pub converted: Vec<ConvertedFile>,
    pub failed: Vec<ConversionFailure>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ConversionProgress {
    pub processed: usize,
    pub total: usize,
    pub input: PathBuf,
    pub succeeded: bool,
}

pub fn convert_batch(
    decoder: &Path,
    inputs: &[PathBuf],
    output_directory: &Path,
    options: ConversionOptions,
    progress: impl FnMut(ConversionProgress),
) -> Result<ConversionSummary, Error> {
    convert_batch_with_decoder(
        inputs,
        output_directory,
        options,
        |input| run_decoder(decoder, input),
        progress,
    )
}

fn convert_batch_with_decoder(
    inputs: &[PathBuf],
    output_directory: &Path,
    options: ConversionOptions,
    mut decode: impl FnMut(&Path) -> Result<Vec<u8>, String>,
    mut progress: impl FnMut(ConversionProgress),
) -> Result<ConversionSummary, Error> {
    fs::create_dir_all(output_directory)?;
    if !output_directory.is_dir() {
        return Err(Error::InvalidOptions("output path must be a directory"));
    }

    let mut summary = ConversionSummary::default();
    for (index, input) in inputs.iter().enumerate() {
        let succeeded = match decode(input)
            .and_then(|bytes| convert_one(input, output_directory, options, &bytes))
        {
            Ok(output) => {
                summary.converted.push(ConvertedFile {
                    input: input.clone(),
                    output,
                });
                true
            }
            Err(reason) => {
                summary.failed.push(ConversionFailure {
                    input: input.clone(),
                    reason,
                });
                false
            }
        };
        progress(ConversionProgress {
            processed: index + 1,
            total: inputs.len(),
            input: input.clone(),
            succeeded,
        });
    }
    Ok(summary)
}

fn run_decoder(decoder: &Path, input: &Path) -> Result<Vec<u8>, String> {
    run_decoder_with_timeout(decoder, input, DECODER_TIMEOUT)
}

// LibRaw's dcraw_emu is not CLI-compatible with classic dcraw: `-c` means
// "write to stdout" in dcraw but "set adjust-maximum threshold" (numeric) in
// dcraw_emu, which instead selects stdout via `-Z -`.
fn is_dcraw_emu(decoder: &Path) -> bool {
    decoder
        .file_stem()
        .and_then(|stem| stem.to_str())
        .is_some_and(|stem| stem.to_ascii_lowercase().starts_with("dcraw_emu"))
}

fn decoder_args(decoder: &Path) -> &'static [&'static str] {
    if is_dcraw_emu(decoder) {
        &["-w", "-q", "3", "-H", "2", "-o", "1", "-Z", "-"]
    } else {
        &["-c", "-w", "-q", "3", "-H", "2", "-o", "1"]
    }
}

fn run_decoder_with_timeout(
    decoder: &Path,
    input: &Path,
    timeout: Duration,
) -> Result<Vec<u8>, String> {
    let mut child = Command::new(decoder)
        .args(decoder_args(decoder))
        .arg(input)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| format!("could not start {}: {error}", decoder.display()))?;
    let (Some(stdout), Some(stderr)) = (child.stdout.take(), child.stderr.take()) else {
        kill_and_reap(&mut child);
        return Err("could not capture decoder output".to_owned());
    };
    let stdout_reader = read_pipe(stdout);
    let stderr_reader = read_pipe(stderr);
    let started = Instant::now();

    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) if started.elapsed() >= timeout => {
                kill_and_reap(&mut child);
                let _ = join_pipe(stdout_reader, "stdout");
                let _ = join_pipe(stderr_reader, "stderr");
                return Err(format!("decoder timed out after {timeout:?}"));
            }
            Ok(None) => {
                thread::sleep(DECODER_POLL_INTERVAL.min(timeout.saturating_sub(started.elapsed())))
            }
            Err(error) => {
                kill_and_reap(&mut child);
                let _ = join_pipe(stdout_reader, "stdout");
                let _ = join_pipe(stderr_reader, "stderr");
                return Err(format!("could not wait for decoder: {error}"));
            }
        }
    };

    let stdout = join_pipe(stdout_reader, "stdout")?;
    let stderr = join_pipe(stderr_reader, "stderr")?;
    if status.success() {
        Ok(stdout)
    } else {
        let stderr = String::from_utf8_lossy(&stderr);
        let reason = stderr.trim();
        Err(if reason.is_empty() {
            format!("decoder exited with {status}")
        } else {
            reason.lines().next().unwrap_or(reason).to_owned()
        })
    }
}

fn read_pipe(mut pipe: impl Read + Send + 'static) -> JoinHandle<io::Result<Vec<u8>>> {
    thread::spawn(move || {
        let mut bytes = Vec::new();
        pipe.read_to_end(&mut bytes)?;
        Ok(bytes)
    })
}

fn join_pipe(reader: JoinHandle<io::Result<Vec<u8>>>, name: &str) -> Result<Vec<u8>, String> {
    reader
        .join()
        .map_err(|_| format!("decoder {name} reader failed"))?
        .map_err(|error| format!("could not read decoder {name}: {error}"))
}

fn kill_and_reap(child: &mut Child) {
    let _ = child.kill();
    let _ = child.wait();
}

fn convert_one(
    input: &Path,
    output_directory: &Path,
    options: ConversionOptions,
    bytes: &[u8],
) -> Result<PathBuf, String> {
    let decoded = image::load_from_memory_with_format(bytes, ImageFormat::Pnm)
        .map_err(|error| format!("invalid decoder output: {error}"))?;
    let image = resize_without_upscaling(decoded, options.max_dimension());
    let (output, file) =
        create_output_file(input, output_directory).map_err(|error| error.to_string())?;

    if let Err(error) = encode_jpeg(file, &image, options.quality()) {
        let _ = fs::remove_file(&output);
        return Err(error);
    }
    Ok(output)
}

fn resize_without_upscaling(image: DynamicImage, max_dimension: u32) -> DynamicImage {
    if image.width() <= max_dimension && image.height() <= max_dimension {
        image
    } else {
        image.resize(max_dimension, max_dimension, FilterType::Lanczos3)
    }
}

fn create_output_file(input: &Path, output_directory: &Path) -> io::Result<(PathBuf, File)> {
    let stem = input
        .file_stem()
        .filter(|stem| !stem.is_empty())
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "input has no filename"))?;

    for number in 1_u32.. {
        let mut filename = OsString::from(stem);
        if number > 1 {
            filename.push(format!("-{number}"));
        }
        filename.push(".jpg");
        let path = output_directory.join(filename);
        match OpenOptions::new().write(true).create_new(true).open(&path) {
            Ok(file) => return Ok((path, file)),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error),
        }
    }
    unreachable!()
}

fn encode_jpeg(file: File, image: &DynamicImage, quality: u8) -> Result<(), String> {
    JpegEncoder::new_with_quality(file, quality)
        .encode_image(image)
        .map_err(|error| format!("could not write JPEG: {error}"))
}

#[cfg(test)]
mod decoder_args_tests {
    use super::*;

    #[test]
    fn selects_dcraw_emu_args_for_dcraw_emu_variants() {
        for name in [
            "dcraw_emu",
            "dcraw_emu-aarch64-apple-darwin",
            "dcraw_emu-x86_64-pc-windows-msvc.exe",
            "DCRAW_EMU",
        ] {
            assert_eq!(
                decoder_args(Path::new(name)),
                &["-w", "-q", "3", "-H", "2", "-o", "1", "-Z", "-"],
                "expected dcraw_emu args for {name}"
            );
        }
    }

    #[test]
    fn selects_classic_dcraw_args_otherwise() {
        for name in ["dcraw", "dcraw.exe", "some-other-decoder"] {
            assert_eq!(
                decoder_args(Path::new(name)),
                &["-c", "-w", "-q", "3", "-H", "2", "-o", "1"],
                "expected classic dcraw args for {name}"
            );
        }
    }
}

#[cfg(test)]
mod conversion_tests {
    use super::*;
    use image::GenericImageView;

    static NEXT_TEST_DIR: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

    struct TestDir(PathBuf);

    impl TestDir {
        fn new() -> Self {
            let path = std::env::temp_dir().join(format!(
                "raw-lite-conversion-{}-{}-{}",
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

        #[cfg(unix)]
        fn executable(&self, name: &str, script: &str) -> PathBuf {
            use std::os::unix::fs::PermissionsExt;

            let path = self.0.join(name);
            fs::write(&path, script).unwrap();
            let mut permissions = fs::metadata(&path).unwrap().permissions();
            permissions.set_mode(0o700);
            fs::set_permissions(&path, permissions).unwrap();
            path
        }
    }

    impl Drop for TestDir {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    fn ppm(width: u32, height: u32) -> Vec<u8> {
        let mut bytes = format!("P6\n{width} {height}\n255\n").into_bytes();
        bytes.extend((0..width * height).flat_map(|pixel| {
            let value = (pixel % 255) as u8;
            [value, 255 - value, value / 2]
        }));
        bytes
    }

    #[test]
    fn converts_ppm_to_bounded_jpeg() {
        let output = TestDir::new();
        let options = ConversionOptions::new(2, 75).unwrap();

        let summary = convert_batch_with_decoder(
            &[PathBuf::from("photo.dng")],
            &output.0,
            options,
            |_| Ok(ppm(4, 2)),
            |_| {},
        )
        .unwrap();

        assert_eq!(summary.converted.len(), 1);
        assert!(summary.failed.is_empty());
        assert_eq!(
            image::open(&summary.converted[0].output)
                .unwrap()
                .dimensions(),
            (2, 1)
        );
    }

    #[test]
    fn does_not_upscale_small_images() {
        let output = TestDir::new();

        let summary = convert_batch_with_decoder(
            &[PathBuf::from("photo.dng")],
            &output.0,
            ConversionOptions::new(2000, 75).unwrap(),
            |_| Ok(ppm(4, 2)),
            |_| {},
        )
        .unwrap();

        assert_eq!(
            image::open(&summary.converted[0].output)
                .unwrap()
                .dimensions(),
            (4, 2)
        );
    }

    #[test]
    fn continues_after_failure_and_reports_each_progress_step() {
        let output = TestDir::new();
        let mut progress = Vec::new();

        let summary = convert_batch_with_decoder(
            &[PathBuf::from("bad.dng"), PathBuf::from("good.dng")],
            &output.0,
            ConversionOptions::new(2000, 75).unwrap(),
            |input| {
                if input == Path::new("bad.dng") {
                    Err("decoder rejected file".into())
                } else {
                    Ok(ppm(2, 1))
                }
            },
            |step| progress.push(step),
        )
        .unwrap();

        assert_eq!(summary.converted.len(), 1);
        assert_eq!(summary.failed.len(), 1);
        assert_eq!(summary.failed[0].reason, "decoder rejected file");
        assert_eq!(progress.len(), 2);
        assert_eq!(
            (
                progress[0].processed,
                progress[0].total,
                progress[0].succeeded
            ),
            (1, 2, false)
        );
        assert_eq!(
            (
                progress[1].processed,
                progress[1].total,
                progress[1].succeeded
            ),
            (2, 2, true)
        );
    }

    #[test]
    fn does_not_overwrite_existing_output() {
        let output = TestDir::new();
        fs::write(output.0.join("photo.jpg"), b"keep me").unwrap();

        let summary = convert_batch_with_decoder(
            &[PathBuf::from("photo.dng")],
            &output.0,
            ConversionOptions::new(2000, 75).unwrap(),
            |_| Ok(ppm(2, 1)),
            |_| {},
        )
        .unwrap();

        assert_eq!(fs::read(output.0.join("photo.jpg")).unwrap(), b"keep me");
        assert_eq!(summary.converted[0].output, output.0.join("photo-2.jpg"));
    }

    #[cfg(unix)]
    #[test]
    fn decoder_timeout_becomes_a_failure_without_hanging() {
        let root = TestDir::new();
        let decoder = root.executable("slow-dcraw", "#!/bin/sh\nexec sleep 5\n");
        let started = std::time::Instant::now();

        let summary = convert_batch_with_decoder(
            &[PathBuf::from("slow.dng")],
            &root.0.join("output"),
            ConversionOptions::new(2000, 75).unwrap(),
            |input| run_decoder_with_timeout(&decoder, input, std::time::Duration::from_millis(25)),
            |_| {},
        )
        .unwrap();

        assert!(started.elapsed() < std::time::Duration::from_secs(1));
        assert!(summary.converted.is_empty());
        assert_eq!(summary.failed[0].reason, "decoder timed out after 25ms");
    }

    #[test]
    fn leaves_no_partial_file_when_image_data_is_invalid() {
        let output = TestDir::new();

        let summary = convert_batch_with_decoder(
            &[PathBuf::from("broken.dng")],
            &output.0,
            ConversionOptions::new(2000, 75).unwrap(),
            |_| Ok(b"not an image".to_vec()),
            |_| {},
        )
        .unwrap();

        assert!(summary.converted.is_empty());
        assert_eq!(summary.failed.len(), 1);
        assert_eq!(fs::read_dir(&output.0).unwrap().count(), 0);
    }
}
