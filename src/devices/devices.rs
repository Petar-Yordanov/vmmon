use crate::devices::bus::pio::PioBus;
use crate::devices::legacy::dma8237::Dma8237;
use crate::devices::legacy::pic8259::Pic8259;
use crate::devices::pci::bus::PciBus;
use crate::devices::pci::config_io::PciConfigIo;
use crate::devices::rtc::cmos::CmosRtc;
use crate::devices::serial::uart16550::Uart16550;
use crate::guest::memory::GuestMemory;
use std::cell::RefCell;
use std::path::PathBuf;
use std::rc::Rc;

pub struct Devices {
    pub pio: PioBus,
    pub pci: Rc<RefCell<PciBus>>,
}

impl Devices {
    pub fn new(disk_img_path: PathBuf) -> Self {
        Self {
            pio: PioBus::new(),
            pci: Rc::new(RefCell::new(PciBus::new(disk_img_path))),
        }
    }

    pub fn register_default_platform(&mut self) {
        self.pio.register(Box::new(Uart16550::new(0x3F8))); // COM1
        self.pio.register(Box::new(Pic8259::new()));
        self.pio.register(Box::new(CmosRtc::new()));
        self.pio.register(Box::new(Dma8237::new()));
        self.pio
            .register(Box::new(PciConfigIo::new(self.pci.clone())));
    }

    pub fn mmio_read(&mut self, mem: &mut GuestMemory, addr: u64, data: &mut [u8]) -> bool {
        self.pci.borrow_mut().mmio_read(mem, addr, data)
    }

    pub fn mmio_write(&mut self, mem: &mut GuestMemory, addr: u64, data: &[u8]) -> bool {
        self.pci.borrow_mut().mmio_write(mem, addr, data)
    }
}
