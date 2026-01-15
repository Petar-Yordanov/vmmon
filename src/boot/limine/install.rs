use crate::{
    boot::elf::{ElfLoadResult, LoadedSeg},
    guest::{layout::GuestLayout, memory::GuestMemory},
    vmm::{Result, VmmonError},
};

use super::spec::*;

#[derive(Clone, Copy, Debug)]
struct Range {
    base: u64,
    len: u64,
}

impl Range {
    fn end(self) -> u64 {
        self.base.saturating_add(self.len)
    }
}

fn align_down(x: u64, a: u64) -> u64 {
    x & !(a - 1)
}

fn align_up(x: u64, a: u64) -> u64 {
    (x + (a - 1)) & !(a - 1)
}

fn write_u64(mem: &mut GuestMemory, gpa: u64, v: u64) {
    mem.write(gpa, &v.to_le_bytes());
}

fn read_u64(mem: &GuestMemory, gpa: u64) -> u64 {
    let mut buf = [0u8; 8];
    mem.read(gpa, &mut buf);
    u64::from_le_bytes(buf)
}

fn find_request_in_ranges(mem: &GuestMemory, ranges: &[Range], magic: [u64; 4]) -> Option<u64> {
    let needle = [
        magic[0].to_le_bytes(),
        magic[1].to_le_bytes(),
        magic[2].to_le_bytes(),
        magic[3].to_le_bytes(),
    ]
    .concat();

    for r in ranges {
        let len = r.len as usize;
        if len < needle.len() {
            continue;
        }

        let hay = mem.slice(r.base, len);

        for off in 0..=(hay.len() - needle.len()) {
            if &hay[off..off + needle.len()] == needle.as_slice() {
                return Some(r.base + off as u64);
            }
        }
    }

    None
}

fn kernel_loaded_ranges(load: &ElfLoadResult) -> Vec<Range> {
    let mut out = Vec::new();
    for s in &load.segs {
        out.push(Range {
            base: s.paddr,
            len: s.memsz,
        });
    }
    out
}

struct GuestBump {
    cur: u64,
    end: u64,
}

impl GuestBump {
    fn new(base: u64, size: u64) -> Self {
        Self {
            cur: base,
            end: base + size,
        }
    }

    fn alloc(&mut self, size: u64, align: u64) -> Result<u64> {
        let aligned = align_up(self.cur, align);
        let next = aligned
            .checked_add(size)
            .ok_or_else(|| VmmonError::boot("limine bump overflow"))?;

        if next > self.end {
            return Err(VmmonError::boot("limine bump out of space"));
        }

        self.cur = next;
        Ok(aligned)
    }
}

fn build_memmap_entries(
    guest_mem_size: u64,
    kernel_ranges: Range,
    bootinfo_ranges: Range,
    framebuffer_ranges: Option<Range>,
) -> Vec<LimineMemmapEntry> {
    let mut reserved = Vec::<Range>::new();
    reserved.push(Range {
        base: 0,
        len: 0x10_0000,
    });
    reserved.push(kernel_ranges);
    reserved.push(bootinfo_ranges);
    if let Some(fb) = framebuffer_ranges {
        reserved.push(fb);
    }

    reserved.sort_by_key(|r| r.base);
    let mut merged = Vec::<Range>::new();
    for r in reserved {
        if let Some(last) = merged.last_mut() {
            if r.base <= last.end() {
                let new_end = last.end().max(r.end());
                last.len = new_end - last.base;
                continue;
            }
        }
        merged.push(r);
    }

    let mut entries = Vec::<LimineMemmapEntry>::new();

    entries.push(LimineMemmapEntry {
        base: 0,
        length: 0x10_0000,
        typ: LIMINE_MEMMAP_RESERVED,
    });

    let mut cur = 0x10_0000;
    for r in &merged {
        if r.end() <= 0x10_0000 {
            continue;
        }
        let r0 = r.base.max(0x10_0000);
        if cur < r0 {
            entries.push(LimineMemmapEntry {
                base: cur,
                length: r0 - cur,
                typ: LIMINE_MEMMAP_USABLE,
            });
        }
        cur = cur.max(r.end());
    }
    if cur < guest_mem_size {
        entries.push(LimineMemmapEntry {
            base: cur,
            length: guest_mem_size - cur,
            typ: LIMINE_MEMMAP_USABLE,
        });
    }

    entries.push(LimineMemmapEntry {
        base: kernel_ranges.base,
        length: kernel_ranges.len,
        typ: LIMINE_MEMMAP_KERNEL_AND_MODULES,
    });
    entries.push(LimineMemmapEntry {
        base: bootinfo_ranges.base,
        length: bootinfo_ranges.len,
        typ: LIMINE_MEMMAP_BOOTLOADER_RECLAIMABLE,
    });
    if let Some(fb) = framebuffer_ranges {
        entries.push(LimineMemmapEntry {
            base: fb.base,
            length: fb.len,
            typ: LIMINE_MEMMAP_FRAMEBUFFER,
        });
    }

    entries
}

fn kernel_phys_span(load: &ElfLoadResult) -> Range {
    let mut lo = u64::MAX;
    let mut hi = 0u64;

    for s in &load.segs {
        lo = lo.min(s.paddr);
        hi = hi.max(s.paddr.saturating_add(s.memsz));
    }

    if lo == u64::MAX {
        lo = 0;
    }

    let base = align_down(lo, 0x1000);
    let end = align_up(hi, 0x1000);
    Range {
        base,
        len: end - base,
    }
}

pub fn install_minimal_limine(
    mem: &mut GuestMemory,
    layout: &GuestLayout,
    load: &ElfLoadResult,
    guest_mem_size_bytes: u64,
) -> Result<()> {
    let kernel_ranges = kernel_phys_span(load);

    let bootinfo_base = 0x0090_0000u64;
    let bootinfo_size = 0x0020_0000u64;
    let bootinfo_ranges = Range {
        base: bootinfo_base,
        len: bootinfo_size,
    };

    let fb_base = 0x00b0_0000u64;
    let fb_w = 800u64;
    let fb_h = 600u64;
    let fb_pitch = fb_w * 4;
    let fb_size = fb_pitch * fb_h;
    let framebuffer_ranges = Some(Range {
        base: fb_base,
        len: fb_size,
    });

    let mut bump = GuestBump::new(bootinfo_base, bootinfo_size);

    let ranges = kernel_loaded_ranges(load);

    let memmap_req_magic = [
        LIMINE_COMMON_MAGIC0,
        LIMINE_COMMON_MAGIC1,
        LIMINE_MEMMAP_MAGIC2,
        LIMINE_MEMMAP_MAGIC3,
    ];
    let hhdm_req_magic = [
        LIMINE_COMMON_MAGIC0,
        LIMINE_COMMON_MAGIC1,
        LIMINE_HHDM_MAGIC2,
        LIMINE_HHDM_MAGIC3,
    ];
    let fb_req_magic = [
        LIMINE_COMMON_MAGIC0,
        LIMINE_COMMON_MAGIC1,
        LIMINE_FRAMEBUFFER_MAGIC2,
        LIMINE_FRAMEBUFFER_MAGIC3,
    ];

    let memmap_req_gpa = find_request_in_ranges(mem, &ranges, memmap_req_magic)
        .ok_or_else(|| VmmonError::boot("failed to find Limine MemoryMapRequest in guest"))?;

    let hhdm_req_gpa = find_request_in_ranges(mem, &ranges, hhdm_req_magic)
        .ok_or_else(|| VmmonError::boot("failed to find Limine HHDMRequest in guest"))?;

    let fb_req_gpa = find_request_in_ranges(mem, &ranges, fb_req_magic);

    let entries = build_memmap_entries(
        guest_mem_size_bytes,
        kernel_ranges,
        bootinfo_ranges,
        framebuffer_ranges,
    );

    let entry_bytes = (core::mem::size_of::<LimineMemmapEntry>() as u64) * (entries.len() as u64);
    let entries_gpa = bump.alloc(entry_bytes, 8)?;
    for (i, e) in entries.iter().enumerate() {
        let off = entries_gpa + (i as u64) * (core::mem::size_of::<LimineMemmapEntry>() as u64);
        mem.write(off + 0, &e.base.to_le_bytes());
        mem.write(off + 8, &e.length.to_le_bytes());
        mem.write(off + 16, &e.typ.to_le_bytes());
    }

    let ptr_table_bytes = 8u64 * (entries.len() as u64);
    let ptrs_gpa = bump.alloc(ptr_table_bytes, 8)?;
    for i in 0..entries.len() {
        let p = entries_gpa + (i as u64) * (core::mem::size_of::<LimineMemmapEntry>() as u64);
        write_u64(mem, ptrs_gpa + (i as u64) * 8, p);
    }

    let memmap_resp_gpa = bump.alloc(core::mem::size_of::<LimineMemmapResponse>() as u64, 8)?;
    write_u64(mem, memmap_resp_gpa + 0, 0); // revision
    write_u64(mem, memmap_resp_gpa + 8, entries.len() as u64);
    write_u64(mem, memmap_resp_gpa + 16, ptrs_gpa);

    write_u64(
        mem,
        memmap_req_gpa + LIMINE_REQUEST_RESPONSE_OFF,
        memmap_resp_gpa,
    );

    let hhdm_offset = 0xffff_8000_0000_0000u64;

    let hhdm_resp_gpa = bump.alloc(core::mem::size_of::<LimineHhdmResponse>() as u64, 8)?;
    write_u64(mem, hhdm_resp_gpa + 0, 0);
    write_u64(mem, hhdm_resp_gpa + 8, hhdm_offset);

    write_u64(
        mem,
        hhdm_req_gpa + LIMINE_REQUEST_RESPONSE_OFF,
        hhdm_resp_gpa,
    );

    if let Some(fb_req_gpa) = fb_req_gpa {
        mem.write_zeros(fb_base, fb_size as usize);

        let fb_struct_gpa = bump.alloc(core::mem::size_of::<LimineFramebuffer>() as u64, 8)?;
        write_u64(mem, fb_struct_gpa + 0, fb_base);
        write_u64(mem, fb_struct_gpa + 8, fb_w);
        write_u64(mem, fb_struct_gpa + 16, fb_h);
        write_u64(mem, fb_struct_gpa + 24, fb_pitch);

        mem.write(fb_struct_gpa + 32, &(32u16).to_le_bytes());
        mem.write(fb_struct_gpa + 34, &[1u8]);
        mem.write(fb_struct_gpa + 35, &[8u8]);
        mem.write(fb_struct_gpa + 36, &[16u8]);
        mem.write(fb_struct_gpa + 37, &[8u8]);
        mem.write(fb_struct_gpa + 38, &[8u8]);
        mem.write(fb_struct_gpa + 39, &[8u8]);
        mem.write(fb_struct_gpa + 40, &[0u8]);

        let edid_size_off = fb_struct_gpa + 48;
        write_u64(mem, edid_size_off, 0);
        write_u64(mem, edid_size_off + 8, 0);

        let fb_ptrs_gpa = bump.alloc(8, 8)?;
        write_u64(mem, fb_ptrs_gpa, fb_struct_gpa);

        let fb_resp_gpa =
            bump.alloc(core::mem::size_of::<LimineFramebufferResponse>() as u64, 8)?;
        write_u64(mem, fb_resp_gpa + 0, 0); // revision
        write_u64(mem, fb_resp_gpa + 8, 1); // count
        write_u64(mem, fb_resp_gpa + 16, fb_ptrs_gpa);

        write_u64(mem, fb_req_gpa + LIMINE_REQUEST_RESPONSE_OFF, fb_resp_gpa);
    }

    let mm_ptr = read_u64(mem, memmap_req_gpa + LIMINE_REQUEST_RESPONSE_OFF);
    if mm_ptr == 0 {
        return Err(VmmonError::boot("memmap response pointer stayed null"));
    }

    Ok(())
}
