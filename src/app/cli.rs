use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "vmmon",
    about = "Minimal KVM-based VMM that boots a Limine ISO and attaches a separate disk image",
    disable_help_subcommand = true
)]
pub struct CliArgs {
    pub boot_iso: PathBuf,

    #[arg(long)]
    pub disk_img: PathBuf,

    #[arg(long, default_value_t = 512)]
    pub mem_mib: usize,
}
