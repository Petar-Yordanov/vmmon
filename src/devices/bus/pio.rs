pub trait PioDevice {
    fn handles_port(&self, port: u16) -> bool;

    fn io_in(&mut self, port: u16, data: &mut [u8]);
    fn io_out(&mut self, port: u16, data: &[u8]);
}

pub struct PioBus {
    devices: Vec<Box<dyn PioDevice>>,
}

impl PioBus {
    pub fn new() -> Self {
        Self {
            devices: Vec::new(),
        }
    }

    pub fn register(&mut self, dev: Box<dyn PioDevice>) {
        self.devices.push(dev);
    }

    pub fn io_out(&mut self, port: u16, data: &[u8]) -> bool {
        for dev in self.devices.iter_mut() {
            if dev.handles_port(port) {
                dev.io_out(port, data);
                return true;
            }
        }
        false
    }

    pub fn io_in(&mut self, port: u16, data: &mut [u8]) -> bool {
        for dev in self.devices.iter_mut() {
            if dev.handles_port(port) {
                dev.io_in(port, data);
                return true;
            }
        }
        false
    }
}
