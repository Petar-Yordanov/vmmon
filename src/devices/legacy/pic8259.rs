use crate::devices::bus::pio::PioDevice;

#[derive(Debug, Clone)]
pub struct Pic8259 {
    pic1_imr: u8,
    pic2_imr: u8,
}

impl Pic8259 {
    pub fn new() -> Self {
        Self {
            pic1_imr: 0,
            pic2_imr: 0,
        }
    }

    #[inline]
    fn is_port(port: u16) -> bool {
        matches!(port, 0x20 | 0x21 | 0xA0 | 0xA1)
    }
}

impl PioDevice for Pic8259 {
    fn handles_port(&self, port: u16) -> bool {
        Self::is_port(port)
    }

    fn io_out(&mut self, port: u16, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        let v = data[0];

        match port {
            // data ports (IMR)
            0x21 => self.pic1_imr = v,
            0xA1 => self.pic2_imr = v,

            // command ports ignore for now
            0x20 | 0xA0 => {}

            _ => {}
        }
    }

    fn io_in(&mut self, port: u16, data: &mut [u8]) {
        let v = match port {
            0x21 => self.pic1_imr,
            0xA1 => self.pic2_imr,
            0x20 | 0xA0 => 0x00,
            _ => 0x00,
        };

        for b in data.iter_mut() {
            *b = v;
        }
    }
}
