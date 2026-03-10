use kvm_bindings::{kvm_regs, kvm_sregs};

use crate::{
    boot::elf::ElfLoadResult,
    guest::{layout::GuestLayout, memory::GuestMemory},
    vmm::Result,
};

pub struct BootState {
    pub sregs: kvm_sregs,
    pub regs: kvm_regs,
}

pub fn build_long_mode_boot_state(
    mem: &mut GuestMemory,
    layout: &GuestLayout,
    load: &ElfLoadResult,
    mut sregs: kvm_sregs,
    guest_mem_size_bytes: u64,
) -> Result<BootState> {
    let cr3 = build_paging(mem, layout, load, guest_mem_size_bytes)?;
    build_gdt(mem, layout)?;

    let stub_phys: u64 = 0x1000;
    write_long_mode_stub(
        mem,
        stub_phys,
        layout.gdt_phys,
        cr3,
        layout.boot_stack_top,
        load.entry_virt,
    )?;

    sregs.cs.base = 0;
    sregs.cs.selector = 0;
    sregs.ds.base = 0;
    sregs.ds.selector = 0;
    sregs.es.base = 0;
    sregs.es.selector = 0;
    sregs.ss.base = 0;
    sregs.ss.selector = 0;

    let mut regs = kvm_regs::default();
    regs.rip = stub_phys;
    regs.rsp = 0x7000;
    regs.rflags = 0x2;

    Ok(BootState { sregs, regs })
}

fn build_gdt(mem: &mut GuestMemory, layout: &GuestLayout) -> Result<()> {
    let gdt: [u64; 3] = [
        0,
        0x00AF9A000000FFFF, // code
        0x00AF92000000FFFF, // data
    ];

    let mut off = layout.gdt_phys;
    for entry in gdt {
        mem.write(off, &entry.to_le_bytes());
        off += 8;
    }
    Ok(())
}

const HHDM_OFFSET: u64 = 0xffff_8000_0000_0000;

fn build_paging(
    mem: &mut GuestMemory,
    layout: &GuestLayout,
    _load: &ElfLoadResult,
    guest_mem_size_bytes: u64,
) -> Result<u64> {
    let pml4_pa = layout.page_tables_phys;
    let pdpt0_pa = pml4_pa + 0x1000;
    let pd0_pa = pdpt0_pa + 0x1000;

    mem.write_zeros(pml4_pa, 0x1000);
    mem.write_zeros(pdpt0_pa, 0x1000);
    mem.write_zeros(pd0_pa, 0x1000);

    const P: u64 = 1 << 0;
    const W: u64 = 1 << 1;
    const PS: u64 = 1 << 7;

    #[inline(always)]
    fn pte(addr: u64, flags: u64) -> u64 {
        (addr & 0x000f_ffff_ffff_f000) | flags
    }

    mem.write(pml4_pa + 0 * 8, &pte(pdpt0_pa, P | W).to_le_bytes());
    mem.write(pdpt0_pa + 0 * 8, &pte(pd0_pa, P | W).to_le_bytes());

    for i in 0..512u64 {
        let pa = i * 0x200000;
        mem.write(pd0_pa + i * 8, &pte(pa, P | W | PS).to_le_bytes());
    }

    map_hhdm_2mib(
        mem,
        pml4_pa,
        layout.page_tables_phys + 0x4000,
        guest_mem_size_bytes,
    )?;

    Ok(pml4_pa)
}

fn map_hhdm_2mib(
    mem: &mut GuestMemory,
    pml4_pa: u64,
    scratch_pa: u64,
    guest_mem_size_bytes: u64,
) -> Result<()> {
    const P: u64 = 1 << 0;
    const W: u64 = 1 << 1;
    const PS: u64 = 1 << 7;

    #[inline(always)]
    fn pte(addr: u64, flags: u64) -> u64 {
        (addr & 0x000f_ffff_ffff_f000) | flags
    }

    let pml4i = ((HHDM_OFFSET >> 39) & 0x1ff) as u64;

    let pdpt_hi_pa = scratch_pa;
    mem.write_zeros(pdpt_hi_pa, 0x1000);
    mem.write(pml4_pa + pml4i * 8, &pte(pdpt_hi_pa, P | W).to_le_bytes());

    let bytes_per_pdpt: u64 = 0x4000_0000; // 1GiB
    let num_pdpt_entries = (guest_mem_size_bytes + bytes_per_pdpt - 1) / bytes_per_pdpt;

    let mut next_pd_pa = pdpt_hi_pa + 0x1000;

    for pdpt_index in 0..num_pdpt_entries {
        let pd_pa = next_pd_pa;
        next_pd_pa += 0x1000;

        mem.write_zeros(pd_pa, 0x1000);
        mem.write(
            pdpt_hi_pa + (pdpt_index * 8),
            &pte(pd_pa, P | W).to_le_bytes(),
        );

        let chunk_phys_base = pdpt_index * bytes_per_pdpt;
        let chunk_bytes = (guest_mem_size_bytes - chunk_phys_base).min(bytes_per_pdpt);

        let num_pd_entries = ((chunk_bytes + 0x200000 - 1) / 0x200000).min(512);

        for i in 0..num_pd_entries {
            let pa = chunk_phys_base + i * 0x200000;
            mem.write(pd_pa + (i * 8), &pte(pa, P | W | PS).to_le_bytes());
        }
    }

    Ok(())
}

fn write_long_mode_stub(
    mem: &mut GuestMemory,
    stub_phys: u64,
    gdt_phys: u64,
    cr3: u64,
    stack_top: u64,
    kernel_entry: u64,
) -> Result<()> {
    let mut code: Vec<u8> = Vec::new();

    code.extend_from_slice(&[0xFA]);

    // lgdt [disp16]
    // 0F 01 16 imm16
    code.extend_from_slice(&[0x0F, 0x01, 0x16]);
    let lgdt_disp_pos = code.len();
    code.extend_from_slice(&[0x00, 0x00]); // patch disp16

    // mov eax, cr4
    code.extend_from_slice(&[0x66, 0x0F, 0x20, 0xE0]);
    // or eax, 0x20 (PAE)
    code.extend_from_slice(&[0x66, 0x83, 0xC8, 0x20]);
    // mov cr4, eax
    code.extend_from_slice(&[0x66, 0x0F, 0x22, 0xE0]);

    // mov eax, cr3_imm32
    code.extend_from_slice(&[0x66, 0xB8]);
    let cr3_imm_pos = code.len();
    code.extend_from_slice(&[0, 0, 0, 0]); // patch imm32
    // mov cr3, eax
    code.extend_from_slice(&[0x66, 0x0F, 0x22, 0xD8]);

    // mov ecx, 0xC0000080 (EFER)
    code.extend_from_slice(&[0x66, 0xB9, 0x80, 0x00, 0x00, 0xC0]);
    // rdmsr
    code.extend_from_slice(&[0x0F, 0x32]);
    // or eax, 0x100 (LME)
    code.extend_from_slice(&[0x66, 0x0D, 0x00, 0x09, 0x00, 0x00]);
    // wrmsr
    code.extend_from_slice(&[0x0F, 0x30]);

    // mov eax, cr0
    code.extend_from_slice(&[0x66, 0x0F, 0x20, 0xC0]);
    // or eax, 0x80000001 (PG|PE)
    code.extend_from_slice(&[0x66, 0x0D, 0x01, 0x00, 0x00, 0x80]);
    // mov cr0, eax
    code.extend_from_slice(&[0x66, 0x0F, 0x22, 0xC0]);

    // far jmp to 0x08:long_mode_entry_abs
    code.extend_from_slice(&[0x66, 0xEA]);
    let far_jmp_off_pos = code.len();
    code.extend_from_slice(&[0, 0, 0, 0]);
    code.extend_from_slice(&[0x08, 0x00]); // sel16 = 0x08

    // Long mode
    let long_off = code.len() as u32;

    // mov ax, 0x10
    code.extend_from_slice(&[0x66, 0xB8, 0x10, 0x00]);
    // mov ds, ax / es / fs / gs / ss
    code.extend_from_slice(&[0x8E, 0xD8]); // ds
    code.extend_from_slice(&[0x8E, 0xC0]); // es
    code.extend_from_slice(&[0x8E, 0xE0]); // fs
    code.extend_from_slice(&[0x8E, 0xE8]); // gs
    code.extend_from_slice(&[0x8E, 0xD0]); // ss

    // mov rsp, imm64
    code.extend_from_slice(&[0x48, 0xBC]);
    let rsp_imm_pos = code.len();
    code.extend_from_slice(&[0; 8]);

    // mov rax, imm64
    code.extend_from_slice(&[0x48, 0xB8]);
    let entry_imm_pos = code.len();
    code.extend_from_slice(&[0; 8]);

    // jmp rax
    code.extend_from_slice(&[0xFF, 0xE0]);

    let gdt_ptr_off = code.len() as u32;
    let gdt_limit: u16 = (3 * 8 - 1) as u16;
    code.extend_from_slice(&gdt_limit.to_le_bytes());
    code.extend_from_slice(&(gdt_phys as u32).to_le_bytes());

    let gdt_ptr_addr = stub_phys + gdt_ptr_off as u64;
    let disp16 = gdt_ptr_addr as u16;
    code[lgdt_disp_pos..lgdt_disp_pos + 2].copy_from_slice(&disp16.to_le_bytes());

    code[cr3_imm_pos..cr3_imm_pos + 4].copy_from_slice(&(cr3 as u32).to_le_bytes());

    let long_abs = (stub_phys as u32).wrapping_add(long_off);
    code[far_jmp_off_pos..far_jmp_off_pos + 4].copy_from_slice(&long_abs.to_le_bytes());

    code[rsp_imm_pos..rsp_imm_pos + 8].copy_from_slice(&stack_top.to_le_bytes());
    code[entry_imm_pos..entry_imm_pos + 8].copy_from_slice(&kernel_entry.to_le_bytes());

    mem.write(stub_phys, &code);

    Ok(())
}
