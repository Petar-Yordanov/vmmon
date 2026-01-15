#![allow(dead_code)]

pub const LIMINE_COMMON_MAGIC0: u64 = 0xc7b1_dd30_df4c_8b88;
pub const LIMINE_COMMON_MAGIC1: u64 = 0x0a82_e883_a194_f07b;

pub const LIMINE_MEMMAP_MAGIC2: u64 = 0x67cf_3d9d_378a_806f;
pub const LIMINE_MEMMAP_MAGIC3: u64 = 0xe304_acdf_c50c_3c62;

pub const LIMINE_HHDM_MAGIC2: u64 = 0x48dc_f1cb_8ad2_b852;
pub const LIMINE_HHDM_MAGIC3: u64 = 0x6398_4e95_9a98_244b;

pub const LIMINE_FRAMEBUFFER_MAGIC2: u64 = 0x9d58_27dc_d881_dd75;
pub const LIMINE_FRAMEBUFFER_MAGIC3: u64 = 0xa314_8604_f6fa_b11b;

pub const LIMINE_REQUEST_RESPONSE_OFF: u64 = 40;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct LimineMemmapEntry {
    pub base: u64,
    pub length: u64,
    pub typ: u64,
}

pub const LIMINE_MEMMAP_USABLE: u64 = 0;
pub const LIMINE_MEMMAP_RESERVED: u64 = 1;
pub const LIMINE_MEMMAP_ACPI_RECLAIMABLE: u64 = 2;
pub const LIMINE_MEMMAP_ACPI_NVS: u64 = 3;
pub const LIMINE_MEMMAP_BAD_MEMORY: u64 = 4;
pub const LIMINE_MEMMAP_BOOTLOADER_RECLAIMABLE: u64 = 5;
pub const LIMINE_MEMMAP_KERNEL_AND_MODULES: u64 = 6;
pub const LIMINE_MEMMAP_FRAMEBUFFER: u64 = 7;

#[repr(C)]
pub struct LimineMemmapResponse {
    pub revision: u64,
    pub entry_count: u64,
    pub entries: *const *const LimineMemmapEntry,
}

#[repr(C)]
pub struct LimineHhdmResponse {
    pub revision: u64,
    pub offset: u64,
}

#[repr(C)]
pub struct LimineFramebuffer {
    pub address: u64,
    pub width: u64,
    pub height: u64,
    pub pitch: u64,
    pub bpp: u16,
    pub memory_model: u8,
    pub red_mask_size: u8,
    pub red_mask_shift: u8,
    pub green_mask_size: u8,
    pub green_mask_shift: u8,
    pub blue_mask_size: u8,
    pub blue_mask_shift: u8,
    pub unused: [u8; 7],
    pub edid_size: u64,
    pub edid: u64,
}

#[repr(C)]
pub struct LimineFramebufferResponse {
    pub revision: u64,
    pub framebuffer_count: u64,
    pub framebuffers: *const *const LimineFramebuffer,
}
