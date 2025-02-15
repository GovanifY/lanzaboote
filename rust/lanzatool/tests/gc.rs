use std::fs;
use std::path::{Path, PathBuf};

use anyhow::Result;
use tempfile::tempdir;

mod common;

#[test]
fn keep_only_configured_number_of_generations() -> Result<()> {
    let esp_mountpoint = tempdir()?;
    let tmpdir = tempdir()?;
    let profiles = tempdir()?;
    let generation_links: Vec<PathBuf> = [1, 2, 3]
        .into_iter()
        .map(|v| {
            common::setup_generation_link(tmpdir.path(), profiles.path(), v)
                .expect("Failed to setup generation link")
        })
        .collect();
    let stub_count = || count_files(&esp_mountpoint.path().join("EFI/Linux")).unwrap();

    // Install all 3 generations.
    let output0 = common::lanzaboote_install(0, esp_mountpoint.path(), generation_links.clone())?;
    assert!(output0.status.success());
    assert_eq!(stub_count(), 3);

    // Call `lanzatool install` again with a config limit of 2 and assert that one is deleted.
    let output1 = common::lanzaboote_install(2, esp_mountpoint.path(), generation_links)?;
    assert!(output1.status.success());
    assert_eq!(stub_count(), 2);

    Ok(())
}

fn count_files(path: &Path) -> Result<usize> {
    Ok(fs::read_dir(path)?.count())
}
