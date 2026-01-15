pub mod serial {
    pub mod uart16550;
}

pub mod bus {
    pub mod pio;
}

pub mod legacy {
    pub mod dma8237;
    pub mod pic8259;
}
pub mod rtc {
    pub mod cmos;
}
pub mod pci {
    pub mod bus;
    pub mod config_io;
}
pub mod devices;
pub use devices::Devices;
