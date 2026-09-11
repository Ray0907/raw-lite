use image::GenericImageView;
use raw_lite_core::{ConversionOptions, convert_batch};
use std::{fs, path::PathBuf};

#[test]
#[ignore = "requires RAW_LITE_DCRAW and RAW_LITE_SAMPLE_RAW"]
fn converts_a_real_raw_sample() {
    let decoder = PathBuf::from(std::env::var_os("RAW_LITE_DCRAW").expect("RAW_LITE_DCRAW"));
    let input =
        PathBuf::from(std::env::var_os("RAW_LITE_SAMPLE_RAW").expect("RAW_LITE_SAMPLE_RAW"));
    let output = std::env::temp_dir().join(format!("raw-lite-real-{}", std::process::id()));
    let _ = fs::remove_dir_all(&output);

    let summary = convert_batch(
        &decoder,
        &[input],
        &output,
        ConversionOptions::new(2000, 75).unwrap(),
        |_| {},
    )
    .unwrap();

    assert!(summary.failed.is_empty(), "{:?}", summary.failed);
    let dimensions = image::open(&summary.converted[0].output)
        .unwrap()
        .dimensions();
    assert!(dimensions.0 <= 2000 && dimensions.1 <= 2000);
    let _ = fs::remove_dir_all(output);
}
