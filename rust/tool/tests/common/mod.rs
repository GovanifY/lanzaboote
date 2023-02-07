use std::ffi::OsStr;
use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Output;

use anyhow::{Context, Result};
use assert_cmd::Command;
use rand::distributions::Alphanumeric;
use rand::{thread_rng, Rng};
use serde_json::json;

/// Create a mock generation link.
///
/// Creates the generation link using the specified version inside a mock profiles directory
/// (mimicking /nix/var/nix/profiles). Returns the path to the generation link.
pub fn setup_generation_link(
    tmpdir: &Path,
    profiles_directory: &Path,
    version: u64,
) -> Result<PathBuf> {
    let toplevel = setup_toplevel(tmpdir).context("Failed to setup toplevel")?;
    // Explicitly set modification time so that snapshot test of os-release reliably works.
    filetime::set_file_mtime(&toplevel, filetime::FileTime::zero())?;

    let bootspec = json!({
        "v1": {
          "init": format!("init-v{}", version),
          "initrd": toplevel.join("initrd"),
          "kernel": toplevel.join("kernel"),
          "kernelParams": [
            "amd_iommu=on",
            "amd_iommu=pt",
            "iommu=pt",
            "kvm.ignore_msrs=1",
            "kvm.report_ignored_msrs=0",
            "udev.log_priority=3",
            "systemd.unified_cgroup_hierarchy=1",
            "loglevel=4"
          ],
          "label": "LanzaOS",
          "toplevel": toplevel,
          "system": "x86_64-linux",
          "specialisation": {},
          "extensions": {
            "lanzaboote": { "osRelease": toplevel.join("os-release") }
          }
        }
    });

    let generation_link_path = profiles_directory.join(format!("system-{}-link", version));
    fs::create_dir(&generation_link_path)?;

    let bootspec_path = generation_link_path.join("boot.json");
    let mut file = fs::File::create(bootspec_path)?;
    file.write_all(&serde_json::to_vec(&bootspec)?)?;

    Ok(generation_link_path)
}

/// Setup a mock toplevel inside a temporary directory.
///
/// Accepts the temporary directory as a parameter so that the invoking function retains control of
/// it (and when it goes out of scope).
fn setup_toplevel(tmpdir: &Path) -> Result<PathBuf> {
    // Generate a random toplevel name so that multiple toplevel paths can live alongside each
    // other in the same directory.
    let toplevel = tmpdir.join(format!("toplevel-{}", random_string(8)));
    fs::create_dir(&toplevel)?;

    let test_systemd = systemd_location_from_env()?;
    let test_systemd_stub = format!("{test_systemd}/lib/systemd/boot/efi/linuxx64.efi.stub");

    let initrd_path = toplevel.join("initrd");
    let kernel_path = toplevel.join("kernel");
    let nixos_version_path = toplevel.join("nixos-version");
    let kernel_modules_path = toplevel.join("kernel-modules/lib/modules/6.1.1");

    // To simplify the test setup, we use the systemd stub for all PE binaries used by lanzatool.
    // Lanzatool doesn't care whether its actually a kernel or initrd but only whether it can
    // manipulate the PE binary with objcopy and/or sign it with sbsigntool. For testing lanzatool
    // in isolation this should suffice.
    fs::copy(&test_systemd_stub, initrd_path)?;
    fs::copy(&test_systemd_stub, kernel_path)?;
    fs::write(nixos_version_path, b"23.05")?;
    fs::create_dir_all(kernel_modules_path)?;

    Ok(toplevel)
}

fn random_string(length: usize) -> String {
    thread_rng()
        .sample_iter(&Alphanumeric)
        .take(length)
        .map(char::from)
        .collect()
}

/// Call the `lanzaboote install` command.
pub fn lanzaboote_install(
    config_limit: u64,
    esp_mountpoint: &Path,
    generation_links: impl IntoIterator<Item = impl AsRef<OsStr>>,
) -> Result<Output> {
    // To simplify the test setup, we use the systemd stub here instead of the lanzaboote stub. See
    // the comment in setup_toplevel for details.
    let test_systemd = systemd_location_from_env()?;
    let test_systemd_stub = format!("{test_systemd}/lib/systemd/boot/efi/linuxx64.efi.stub");

    let test_loader_config_path = tempfile::NamedTempFile::new()?;
    let test_loader_config = r"timeout 0\nconsole-mode 1\n";
    fs::write(test_loader_config_path.path(), test_loader_config)?;

    let mut cmd = Command::cargo_bin("lzbt")?;
    let output = cmd
        .env("LANZABOOTE_STUB", &test_systemd_stub)
        .arg("install")
        .arg("--systemd")
        .arg(test_systemd)
        .arg("--systemd-boot-loader-config")
        .arg(test_loader_config_path.path())
        .arg("--public-key")
        .arg("tests/fixtures/uefi-keys/db.pem")
        .arg("--private-key")
        .arg("tests/fixtures/uefi-keys/db.key")
        .arg("--efi-boot-path")
        .arg(test_systemd_stub)
        .arg("--configuration-limit")
        .arg(config_limit.to_string())
        .arg(esp_mountpoint)
        .args(generation_links)
        .output()?;

    // Print debugging output.
    // This is a weird hack to make cargo test capture the output.
    // See https://github.com/rust-lang/rust/issues/12309
    print!("{}", String::from_utf8(output.stdout.clone())?);
    print!("{}", String::from_utf8(output.stderr.clone())?);

    // Also walk the entire ESP mountpoint and print each path for debugging
    for entry in walkdir::WalkDir::new(esp_mountpoint) {
        println!("{}", entry?.path().display());
    }

    Ok(output)
}

/// Read location of systemd installation from an environment variable.
fn systemd_location_from_env() -> Result<String> {
    let error_msg = "TEST_SYSTEMD environment variable is not set. TEST_SYSTEMD has to point to a systemd installation.
On a system with Nix installed, you can set it with: export TEST_SYSTEMD=$(nix-build '<nixpkgs>' -A systemd)";
    std::env::var("TEST_SYSTEMD").context(error_msg)
}
