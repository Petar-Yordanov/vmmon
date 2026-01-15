use std::cell::RefCell;
use std::rc::Rc;

use crate::devices::bus::pio::PioDevice;
use crate::devices::pci::bus::PciBus;

#[derive(Debug)]
pub struct PciConfigIo {
    addr: u32,
    pci: Rc<RefCell<PciBus>>,
}

impl PciConfigIo {
    pub fn new(pci: Rc<RefCell<PciBus>>) -> Self {
        Self { addr: 0, pci }
    }

    #[inline]
    fn enabled(&self) -> bool {
        (self.addr & 0x8000_0000) != 0
    }

    #[inline]
    fn decode_bdf(&self) -> (u8, u8, u8) {
        let bus = ((self.addr >> 16) & 0xff) as u8;
        let dev = ((self.addr >> 11) & 0x1f) as u8;
        let func = ((self.addr >> 8) & 0x07) as u8;
        (bus, dev, func)
    }

    #[inline]
    fn reg_dword_aligned(&self) -> u16 {
        (self.addr & 0xfc) as u16
    }

    #[inline]
    fn data_port_offset(port: u16) -> u8 {
        (port - 0xCFC) as u8
    }

    fn read_data(&mut self, port: u16, data: &mut [u8]) {
        let mut out = [0u8; 4];

        let val = if self.enabled() {
            let (bus, dev, func) = self.decode_bdf();
            let reg = self.reg_dword_aligned();
            self.pci.borrow_mut().read_config_dword(bus, dev, func, reg)
        } else {
            0xffff_ffff
        };

        out.copy_from_slice(&val.to_le_bytes());

        let base = Self::data_port_offset(port) as usize;
        for (i, b) in data.iter_mut().enumerate() {
            let idx = base + i;
            *b = if idx < 4 { out[idx] } else { 0xff };
        }
    }

    fn write_data(&mut self, port: u16, data: &[u8]) {
        if !self.enabled() {
            return;
        }

        let (bus, dev, func) = self.decode_bdf();
        let reg = self.reg_dword_aligned();

        let mut cur = self.pci.borrow_mut().read_config_dword(bus, dev, func, reg);
        let mut bytes = cur.to_le_bytes();

        let base = Self::data_port_offset(port) as usize;
        for (i, &b) in data.iter().enumerate() {
            let idx = base + i;
            if idx < 4 {
                bytes[idx] = b;
            }
        }

        cur = u32::from_le_bytes(bytes);
        self.pci
            .borrow_mut()
            .write_config_dword(bus, dev, func, reg, cur);
    }
}

impl PioDevice for PciConfigIo {
    fn handles_port(&self, port: u16) -> bool {
        matches!(port, 0xCF8 | 0xCFC..=0xCFF)
    }

    fn io_in(&mut self, port: u16, data: &mut [u8]) {
        match port {
            0xCF8 => {
                let bytes = self.addr.to_le_bytes();
                for (i, b) in data.iter_mut().enumerate() {
                    *b = *bytes.get(i).unwrap_or(&0);
                }
            }
            0xCFC..=0xCFF => self.read_data(port, data),
            _ => {
                for b in data.iter_mut() {
                    *b = 0xff;
                }
            }
        }
    }

    fn io_out(&mut self, port: u16, data: &[u8]) {
        match port {
            0xCF8 => {
                let mut bytes = self.addr.to_le_bytes();
                for (i, &b) in data.iter().enumerate() {
                    if i < 4 {
                        bytes[i] = b;
                    }
                }
                self.addr = u32::from_le_bytes(bytes);
            }
            0xCFC..=0xCFF => self.write_data(port, data),
            _ => {}
        }
    }
}
