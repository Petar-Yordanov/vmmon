use goblin::elf::Elf;
use goblin::elf::reloc::R_X86_64_RELATIVE;

use crate::{
    guest::{layout::GuestLayout, memory::GuestMemory},
    vmm::{Result, VmmonError},
};

#[derive(Debug, Clone)]
pub struct ElfLoadResult {
    pub entry_virt: u64,
    pub load_base: u64,
    pub segs: Vec<LoadedSeg>,
}

#[derive(Debug, Clone)]
pub struct LoadedSeg {
    pub vaddr: u64,
    pub memsz: u64,
    pub paddr: u64,
}

pub fn load_elf64(
    mem: &mut GuestMemory,
    layout: &GuestLayout,
    elf_bytes: &[u8],
) -> Result<ElfLoadResult> {
    let elf = Elf::parse(elf_bytes).map_err(|e| VmmonError::elf(e.to_string()))?;

    let load_base = layout.kernel_load_phys;

    let mut segs = Vec::new();
    for ph in elf
        .program_headers
        .iter()
        .filter(|ph| ph.p_type == goblin::elf::program_header::PT_LOAD)
    {
        let vaddr = ph.p_vaddr as u64;
        let paddr = load_base + vaddr;

        let file_off = ph.p_offset as usize;
        let file_sz = ph.p_filesz as usize;
        let mem_sz = ph.p_memsz as usize;

        if file_off + file_sz > elf_bytes.len() {
            return Err(VmmonError::elf("segment out of bounds"));
        }

        mem.write(paddr, &elf_bytes[file_off..file_off + file_sz]);
        if mem_sz > file_sz {
            mem.write_zeros(paddr + file_sz as u64, mem_sz - file_sz);
        }

        segs.push(LoadedSeg {
            vaddr,
            memsz: ph.p_memsz as u64,
            paddr,
        });
    }

    for rela in &elf.dynrelas {
        let r_type = rela.r_type;
        let r_off = rela.r_offset;
        let addend = rela.r_addend.unwrap_or(0);

        if r_type == R_X86_64_RELATIVE {
            let where_gpa = load_base + r_off;
            let value = load_base.wrapping_add(addend as u64);
            mem.write_u64(where_gpa, value);
        } else {
            return Err(VmmonError::elf(format!(
                "unsupported relocation type {} at r_offset={:#x}",
                r_type, r_off
            )));
        }
    }

    let entry_virt = load_base + (elf.entry as u64);

    Ok(ElfLoadResult {
        entry_virt,
        load_base,
        segs,
    })
}
