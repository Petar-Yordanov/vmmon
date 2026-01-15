use crate::devices::bus::pio::PioDevice;

#[derive(Debug, Clone)]
pub struct Dma8237 {
    dma1: [u8; 0x10],
    dma2: [u8; 0x20],
}

impl Dma8237 {
    pub fn new() -> Self {
        Self {
            dma1: [0; 0x10],
            dma2: [0; 0x20],
        }
    }

    #[inline]
    fn handles(port: u16) -> bool {
        (0x0000..=0x000F).contains(&port)
            || (0x0080..=0x008F).contains(&port)
            || (0x00C0..=0x00DF).contains(&port)
    }
}

impl PioDevice for Dma8237 {
    fn handles_port(&self, port: u16) -> bool {
        Self::handles(port)
    }

    fn io_out(&mut self, port: u16, data: &[u8]) {
        for &b in data {
            match port {
                0x0000..=0x000F => {
                    let idx = (port - 0x0000) as usize;
                    self.dma1[idx] = b;
                }
                0x00C0..=0x00DF => {
                    let idx = (port - 0x00C0) as usize;
                    if idx < self.dma2.len() {
                        self.dma2[idx] = b;
                    }
                }
                0x0080..=0x008F => {
                    // safe to ignore for now.
                    let _ = b;
                }
                _ => {}
            }
        }
    }

    fn io_in(&mut self, port: u16, data: &mut [u8]) {
        let v = match port {
            0x0000..=0x000F => {
                let idx = (port - 0x0000) as usize;
                self.dma1[idx]
            }
            0x00C0..=0x00DF => {
                let idx = (port - 0x00C0) as usize;
                self.dma2.get(idx).copied().unwrap_or(0)
            }
            0x0080..=0x008F => 0,
            _ => 0,
        };

        for out in data.iter_mut() {
            *out = v;
        }
    }
}
