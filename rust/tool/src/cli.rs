use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use crate::install;
use crate::signature::KeyPair;

#[derive(Parser)]
pub struct Cli {
    #[clap(subcommand)]
    commands: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Install(InstallCommand),
}

#[derive(Parser)]
struct InstallCommand {
    /// Systemd-boot loader config
    #[arg(long)]
    systemd_boot_loader_config: PathBuf,

    /// sbsign Public Key
    #[arg(long)]
    public_key: PathBuf,

    /// sbsign Private Key
    #[arg(long)]
    private_key: PathBuf,

    /// Configuration limit
    #[arg(long, default_value_t = 1)]
    configuration_limit: usize,

    /// EFI Bootloader Path (e.g. systemd/lib/systemd/boot/efi/systemd-bootx64.efi)
    #[arg(long)]
    efi_boot_path: PathBuf,

    /// EFI system partition mountpoint (e.g. efiSysMountPoint)
    esp: PathBuf,

    /// List of generation links (e.g. /nix/var/nix/profiles/system-*-link)
    generations: Vec<PathBuf>,
}

impl Cli {
    pub fn call(self) -> Result<()> {
        self.commands.call()
    }
}

impl Commands {
    pub fn call(self) -> Result<()> {
        match self {
            Commands::Install(args) => install(args),
        }
    }
}

fn install(args: InstallCommand) -> Result<()> {
    let lanzaboote_stub =
        std::env::var("LANZABOOTE_STUB").context("Failed to read LANZABOOTE_STUB env variable")?;

    let key_pair = KeyPair::new(&args.public_key, &args.private_key);

    install::Installer::new(
        PathBuf::from(lanzaboote_stub),
        args.systemd_boot_loader_config,
        key_pair,
        args.configuration_limit,
        args.esp,
        args.efi_boot_path,
        args.generations,
    )
    .install()
}
