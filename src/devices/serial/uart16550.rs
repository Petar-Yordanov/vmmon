use crate::devices::bus::pio::PioDevice;

#[derive(Debug, Clone)]
pub struct Uart16550 {
    base_port: u16,

    ier: u8, // interrupt enable
    lcr: u8, // line control (DLAB bit 7)
    mcr: u8, // modem control
    fcr: u8, // fifo control (write-only)
    scr: u8, // scratch

    dll: u8, // divisor latch low
    dlm: u8, // divisor latch high
}

impl Uart16550 {
    pub fn new(base_port: u16) -> Self {
        Self {
            base_port,
            ier: 0,
            lcr: 0,
            mcr: 0,
            fcr: 0,
            scr: 0,
            dll: 1,
            dlm: 0,
        }
    }

    #[inline]
    pub fn handles_port(&self, port: u16) -> bool {
        port >= self.base_port && port <= self.base_port + 7
    }

    #[inline]
    fn offset(&self, port: u16) -> u16 {
        port - self.base_port
    }

    #[inline]
    fn dlab(&self) -> bool {
        (self.lcr & 0x80) != 0
    }

    pub fn io_out(&mut self, port: u16, data: &[u8]) {
        let off = self.offset(port);

        for &b in data {
            self.write_reg(off, b);
        }
    }

    pub fn io_in(&mut self, port: u16, data: &mut [u8]) {
        let off = self.offset(port);

        for out in data.iter_mut() {
            *out = self.read_reg(off);
        }
    }

    fn write_reg(&mut self, off: u16, val: u8) {
        match off {
            0 => {
                if self.dlab() {
                    self.dll = val;
                } else {
                    print!("{}", val as char);
                    use std::io::Write;
                    let _ = std::io::stdout().flush();
                }
            }
            1 => {
                if self.dlab() {
                    self.dlm = val;
                } else {
                    self.ier = val;
                }
            }
            2 => {
                // write-only
                self.fcr = val;
            }
            3 => {
                self.lcr = val;
            }
            4 => {
                self.mcr = val;
            }
            7 => {
                self.scr = val;
            }
            _ => {
                // ignore other regs for now
            }
        }
    }

    fn read_reg(&mut self, off: u16) -> u8 {
        match off {
            0 => {
                if self.dlab() {
                    self.dll
                } else {
                    // RBR (no input)
                    0
                }
            }
            1 => {
                if self.dlab() {
                    self.dlm
                } else {
                    self.ier
                }
            }
            2 => {
                // IIR -> no interrupt pending
                0x01
            }
            3 => self.lcr,
            4 => self.mcr,
            5 => {
                // LSR
                0x60
            }
            6 => {
                // MSR
                0x00
            }
            7 => self.scr,
            _ => 0,
        }
    }
}

impl PioDevice for Uart16550 {
    fn handles_port(&self, port: u16) -> bool {
        Uart16550::handles_port(self, port)
    }

    fn io_in(&mut self, port: u16, data: &mut [u8]) {
        Uart16550::io_in(self, port, data)
    }

    fn io_out(&mut self, port: u16, data: &[u8]) {
        Uart16550::io_out(self, port, data)
    }
}
