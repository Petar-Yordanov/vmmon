use clap::Parser;
use std::path::PathBuf;

#[derive(Debug, Clone, Parser)]
#[command(
    name = "vmmon",
    about = "Minimal KVM-based VMM that direct-boots a kernel ELF",
    disable_help_subcommand = true
)]
pub struct CliArgs {
    pub kernel_elf: PathBuf,

    #[arg(long, default_value_t = 512)]
    pub mem_mib: usize,
}
