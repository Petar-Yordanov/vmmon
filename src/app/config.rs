use std::path::PathBuf;

use crate::vmm::{Result, VmmonError};

use super::cli::CliArgs;

#[derive(Debug, Clone)]
pub struct VmConfig {
    pub kernel_path: PathBuf,
    pub mem_size_bytes: usize,
}

impl VmConfig {
    pub fn from_cli(args: CliArgs) -> Result<Self> {
        if args.mem_mib < 64 {
            return Err(VmmonError::config("mem_mib must be >= 64"));
        }

        Ok(Self {
            kernel_path: args.kernel_elf,
            mem_size_bytes: args.mem_mib * 1024 * 1024,
        })
    }
}
