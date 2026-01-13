#[derive(Debug, Clone)]
pub struct GuestLayout {
    pub kernel_load_phys: u64,
    pub page_tables_phys: u64,
    pub gdt_phys: u64,
    pub boot_stack_top: u64,
}

impl GuestLayout {
    pub fn default_layout(_mem_size_bytes: usize) -> Self {
        Self {
            kernel_load_phys: 0x0020_0000, // load kernel at 2MiB
            page_tables_phys: 0x0040_0000, // keep tables away from kernel
            gdt_phys: 0x003f_0000,
            boot_stack_top: 0x0080_0000,
        }
    }
}
