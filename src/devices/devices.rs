use crate::devices::bus::pio::PioBus;
use crate::devices::legacy::dma8237::Dma8237;
use crate::devices::legacy::pic8259::Pic8259;
use crate::devices::pci::bus::PciBus;
use crate::devices::pci::config_io::PciConfigIo;
use crate::devices::rtc::cmos::CmosRtc;
use crate::devices::serial::uart16550::Uart16550;
use std::cell::RefCell;
use std::rc::Rc;

pub struct Devices {
    pub pio: PioBus,
    pub pci: Rc<RefCell<PciBus>>,
}

impl Devices {
    pub fn new() -> Self {
        Self {
            pio: PioBus::new(),
            pci: Rc::new(RefCell::new(PciBus::new())),
        }
    }

    pub fn register_default_platform(&mut self) {
        self.pio.register(Box::new(Uart16550::new(0x3F8))); //COM1
        self.pio.register(Box::new(Pic8259::new()));
        self.pio.register(Box::new(CmosRtc::new()));
        self.pio.register(Box::new(Dma8237::new()));
        self.pio
            .register(Box::new(PciConfigIo::new(self.pci.clone())));
    }
}
