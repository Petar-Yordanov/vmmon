use std::path::PathBuf;

use crate::vmm::{Result, VmmonError};

use super::cli::CliArgs;

#[derive(Debug, Clone)]
pub struct VmConfig {
    pub boot_iso_path: PathBuf,
    pub disk_img_path: PathBuf,
    pub mem_size_bytes: usize,
}

impl VmConfig {
    pub fn from_cli(args: CliArgs) -> Result<Self> {
        if args.mem_mib < 64 {
            return Err(VmmonError::config("mem_mib must be >= 64"));
        }

        if !has_ext(&args.boot_iso, "iso") {
            return Err(VmmonError::config(format!(
                "boot_iso must be an .iso file, got '{}'",
                args.boot_iso.display()
            )));
        }

        if !has_ext(&args.disk_img, "img") {
            return Err(VmmonError::config(format!(
                "disk_img must be an .img file, got '{}'",
                args.disk_img.display()
            )));
        }

        Ok(Self {
            boot_iso_path: args.boot_iso,
            disk_img_path: args.disk_img,
            mem_size_bytes: args.mem_mib * 1024 * 1024,
        })
    }
}

fn has_ext(path: &std::path::Path, want: &str) -> bool {
    path.extension()
        .and_then(|s| s.to_str())
        .map(|s| s.eq_ignore_ascii_case(want))
        .unwrap_or(false)
}
