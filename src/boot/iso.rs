use std::{
    fs::File,
    io::{Read, Seek, SeekFrom},
    path::Path,
};

use crate::vmm::{Result, VmmonError};

const ISO_SECTOR_SIZE: u64 = 2048;
const PVD_SECTOR: u64 = 16;
const VD_TYPE_PRIMARY: u8 = 1;
const VD_STD_ID: &[u8; 5] = b"CD001";

const LIMINE_CONF_CANDIDATES: &[&str] = &[
    "/boot/limine.conf",
    "/limine.conf",
    "/boot/limine.cfg",
    "/limine.cfg",
];

const KERNEL_FALLBACK_CANDIDATES: &[&str] = &[
    "/boot/kernel.elf",
    "/kernel.elf",
    "/boot/micros64.elf",
    "/micros64.elf",
    "/boot/micros.elf",
    "/micros.elf",
];

#[derive(Debug, Clone)]
struct IsoNode {
    extent_lba: u32,
    size: u32,
    is_dir: bool,
}

pub fn extract_kernel_elf_from_limine_iso(iso_path: &Path) -> Result<Vec<u8>> {
    let mut iso = Iso9660::open(iso_path)?;
    let root = iso.root_dir()?;

    for cfg_path in LIMINE_CONF_CANDIDATES {
        if let Ok(cfg_bytes) = iso.read_file_by_path(&root, cfg_path) {
            if let Some(kernel_path) = parse_limine_kernel_path(&cfg_bytes) {
                let candidate_paths = kernel_path_candidates(&kernel_path);

                for candidate in &candidate_paths {
                    if let Ok(kernel_bytes) = iso.read_file_by_path(&root, candidate) {
                        return Ok(kernel_bytes);
                    }
                }

                return Err(VmmonError::boot(format!(
                    "found KERNEL_PATH='{}' in '{}' inside ISO '{}', but failed to read any of these resolved paths: {}",
                    kernel_path,
                    cfg_path,
                    iso_path.display(),
                    candidate_paths.join(", "),
                )));
            }
        }
    }

    for kernel_path in KERNEL_FALLBACK_CANDIDATES {
        if let Ok(kernel_bytes) = iso.read_file_by_path(&root, kernel_path) {
            return Ok(kernel_bytes);
        }
    }

    Err(VmmonError::boot(format!(
        "failed to resolve kernel ELF from ISO '{}'; tried Limine configs ({}) and fallback kernel paths ({})",
        iso_path.display(),
        LIMINE_CONF_CANDIDATES.join(", "),
        KERNEL_FALLBACK_CANDIDATES.join(", "),
    )))
}

fn kernel_path_candidates(path: &str) -> Vec<String> {
    let normalized = normalize_limine_path(path);
    let mut out = Vec::new();

    push_unique(&mut out, normalized.clone());

    if let Some(base) = normalized.strip_prefix('/') {
        push_unique(&mut out, format!("/boot/{base}"));
    }

    if let Some(name) = normalized.rsplit('/').next() {
        push_unique(&mut out, format!("/boot/{name}"));
    }

    out
}

fn push_unique(out: &mut Vec<String>, value: String) {
    if !value.is_empty() && !out.iter().any(|v| v.eq_ignore_ascii_case(&value)) {
        out.push(value);
    }
}

fn parse_limine_kernel_path(cfg_bytes: &[u8]) -> Option<String> {
    let text = std::str::from_utf8(cfg_bytes).ok()?;

    for raw_line in text.lines() {
        let line = raw_line.trim();

        if line.is_empty() || line.starts_with('#') {
            continue;
        }

        let upper = line.to_ascii_uppercase();
        if !upper.starts_with("KERNEL_PATH=") {
            continue;
        }

        let value = line.split_once('=')?.1.trim();
        let value = value.trim_matches('"').trim_matches('\'');

        let normalized = normalize_limine_path(value);
        if !normalized.is_empty() {
            return Some(normalized);
        }
    }

    None
}

fn normalize_limine_path(path: &str) -> String {
    let mut s = path.trim();

    for prefix in [
        "boot:///",
        "boot://",
        "boot:/",
        "file:///",
        "file://",
        "file:/",
    ] {
        if let Some(rest) = s.strip_prefix(prefix) {
            s = rest;
            break;
        }
    }

    let s = s.trim_start_matches('/');

    if s.is_empty() {
        String::new()
    } else {
        format!("/{}", s)
    }
}

struct Iso9660 {
    file: File,
}

impl Iso9660 {
    fn open(path: &Path) -> Result<Self> {
        let file = File::open(path)?;
        Ok(Self { file })
    }

    fn root_dir(&mut self) -> Result<IsoNode> {
        let pvd = self.read_sector(PVD_SECTOR)?;
        validate_pvd(&pvd)?;

        let root_off = 156usize;
        let root = parse_dir_record(&pvd[root_off..])
            .ok_or_else(|| VmmonError::boot("invalid ISO primary volume descriptor root record"))?;

        if !root.is_dir {
            return Err(VmmonError::boot(
                "ISO primary volume descriptor root record is not a directory",
            ));
        }

        Ok(root)
    }

    fn read_sector(&mut self, sector: u64) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; ISO_SECTOR_SIZE as usize];
        self.file
            .seek(SeekFrom::Start(sector.checked_mul(ISO_SECTOR_SIZE).ok_or_else(
                || VmmonError::boot("ISO sector offset overflow"),
            )?))?;
        self.file.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn read_extent(&mut self, lba: u32, size: u32) -> Result<Vec<u8>> {
        let mut buf = vec![0u8; size as usize];
        let offset = (lba as u64)
            .checked_mul(ISO_SECTOR_SIZE)
            .ok_or_else(|| VmmonError::boot("ISO extent offset overflow"))?;
        self.file.seek(SeekFrom::Start(offset))?;
        self.file.read_exact(&mut buf)?;
        Ok(buf)
    }

    fn read_dir_entries(&mut self, dir: &IsoNode) -> Result<Vec<(String, IsoNode)>> {
        if !dir.is_dir {
            return Err(VmmonError::boot("attempted to read entries from non-directory"));
        }

        let data = self.read_extent(dir.extent_lba, dir.size)?;
        let mut out = Vec::new();
        let mut off = 0usize;

        while off < data.len() {
            let rec_len = data[off];
            if rec_len == 0 {
                let next_sector = ((off as u64 / ISO_SECTOR_SIZE) + 1) * ISO_SECTOR_SIZE;
                off = next_sector as usize;
                continue;
            }

            let rec_len_usize = rec_len as usize;
            if off + rec_len_usize > data.len() {
                break;
            }

            let rec = &data[off..off + rec_len_usize];
            if let Some(node) = parse_dir_record(rec) {
                if let Some(ident) = parse_file_identifier(rec) {
                    if ident != "." && ident != ".." {
                        out.push((ident, node));
                    }
                }
            }

            off += rec_len_usize;
        }

        Ok(out)
    }

    fn find_in_dir_case_insensitive(
        &mut self,
        dir: &IsoNode,
        want_name: &str,
    ) -> Result<Option<IsoNode>> {
        let want_norm = normalize_iso_name(want_name);

        for (name, node) in self.read_dir_entries(dir)? {
            if normalize_iso_name(&name) == want_norm {
                return Ok(Some(node));
            }
        }

        Ok(None)
    }

    fn read_file_by_path(&mut self, root: &IsoNode, path: &str) -> Result<Vec<u8>> {
        let parts = split_path(path);
        if parts.is_empty() {
            return Err(VmmonError::boot(format!(
                "invalid ISO file path '{}'",
                path
            )));
        }

        let mut cur = root.clone();

        for part in &parts[..parts.len() - 1] {
            let next = self
                .find_in_dir_case_insensitive(&cur, part)?
                .ok_or_else(|| {
                    VmmonError::boot(format!(
                        "path component '{}' not found while resolving '{}'",
                        part, path
                    ))
                })?;

            if !next.is_dir {
                return Err(VmmonError::boot(format!(
                    "path component '{}' in '{}' is not a directory",
                    part, path
                )));
            }

            cur = next;
        }

        let file_name = parts.last().unwrap();
        let file_node = self
            .find_in_dir_case_insensitive(&cur, file_name)?
            .ok_or_else(|| VmmonError::boot(format!("file '{}' not found in ISO", path)))?;

        if file_node.is_dir {
            return Err(VmmonError::boot(format!(
                "'{}' resolved to a directory, not a file",
                path
            )));
        }

        self.read_extent(file_node.extent_lba, file_node.size)
    }
}

fn validate_pvd(pvd: &[u8]) -> Result<()> {
    if pvd.len() < ISO_SECTOR_SIZE as usize {
        return Err(VmmonError::boot("short read of ISO primary volume descriptor"));
    }

    if pvd[0] != VD_TYPE_PRIMARY {
        return Err(VmmonError::boot(format!(
            "unexpected ISO volume descriptor type {}; expected primary volume descriptor",
            pvd[0]
        )));
    }

    if &pvd[1..6] != VD_STD_ID {
        return Err(VmmonError::boot("invalid ISO9660 standard identifier"));
    }

    if pvd[6] != 1 {
        return Err(VmmonError::boot(format!(
            "unsupported ISO volume descriptor version {}",
            pvd[6]
        )));
    }

    Ok(())
}

fn parse_dir_record(rec: &[u8]) -> Option<IsoNode> {
    if rec.len() < 34 {
        return None;
    }

    let extent_lba = rd_u32_le(rec, 2)?;
    let size = rd_u32_le(rec, 10)?;
    let flags = *rec.get(25)?;
    let is_dir = (flags & 0x02) != 0;

    Some(IsoNode {
        extent_lba,
        size,
        is_dir,
    })
}

fn parse_file_identifier(rec: &[u8]) -> Option<String> {
    let ident_len = *rec.get(32)? as usize;
    let ident = rec.get(33..33 + ident_len)?;

    if ident_len == 1 {
        match ident[0] {
            0 => return Some(".".to_string()),
            1 => return Some("..".to_string()),
            _ => {}
        }
    }

    let raw = std::str::from_utf8(ident).ok()?.trim();
    let no_ver = strip_iso_version_suffix(raw);
    Some(no_ver.to_string())
}

fn strip_iso_version_suffix(name: &str) -> &str {
    if let Some((base, suffix)) = name.rsplit_once(';') {
        if !base.is_empty() && suffix.chars().all(|c| c.is_ascii_digit()) {
            return base;
        }
    }
    name
}

fn normalize_iso_name(name: &str) -> String {
    strip_iso_version_suffix(name)
        .trim()
        .trim_matches('/')
        .to_ascii_lowercase()
}

fn split_path(path: &str) -> Vec<&str> {
    path.split('/').filter(|s| !s.is_empty()).collect()
}

fn rd_u32_le(buf: &[u8], off: usize) -> Option<u32> {
    let bytes = buf.get(off..off + 4)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}
