#[derive(Debug, Default)]
pub struct PciBus;

impl PciBus {
    pub fn new() -> Self {
        Self
    }

    pub fn read_config_dword(&mut self, _bus: u8, _dev: u8, _func: u8, _reg: u16) -> u32 {
        // "No device present"
        0xffff_ffff
    }

    pub fn write_config_dword(&mut self, _bus: u8, _dev: u8, _func: u8, _reg: u16, _val: u32) {
        // ignore for now
    }
}
