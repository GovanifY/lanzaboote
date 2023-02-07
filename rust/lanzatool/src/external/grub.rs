use std::ffi::OsString;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::Command;

use anyhow::{Context, Result};

pub struct KeyPair {
    pub private_key: PathBuf,
    pub public_key: PathBuf,
}

impl KeyPair {
    pub fn new(public_key: &Path, private_key: &Path) -> Self {
        Self {
            public_key: public_key.into(),
            private_key: private_key.into(),
        }
    }

    pub fn sign_and_copy(&self, from: &Path, to: &Path) -> Result<()> {
        let gen_config = Command::new("grub-mkconfig -o /boot/grub/grub.cfg").args(&args).output()?;

        if !gen_config.status.success() {
            std::io::stderr()
                .write_all(&output.stderr)
                .context("Failed to write output of grub-mkconfig to stderr")?;
            return Err(anyhow::anyhow!(
                "Failed to generate GRUB configuration.",
                &args
            ));
        }

        Ok(())
    }
}
