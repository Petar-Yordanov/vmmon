use crate::devices::bus::pio::PioDevice;

#[derive(Debug, Clone)]
pub struct CmosRtc {
    index: u8,
}

impl CmosRtc {
    pub fn new() -> Self {
        Self { index: 0 }
    }

    #[inline]
    fn is_port(port: u16) -> bool {
        port == 0x70 || port == 0x71
    }

    #[inline]
    fn to_bcd(v: u8) -> u8 {
        ((v / 10) << 4) | (v % 10)
    }

    fn read_reg(&self, reg: u8) -> u8 {
        use std::time::{SystemTime, UNIX_EPOCH};

        let secs = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();

        let days = secs / 86_400;
        let mut rem = secs % 86_400;
        let hour = (rem / 3600) as u8;
        rem %= 3600;
        let min = (rem / 60) as u8;
        let sec = (rem % 60) as u8;

        let (year, month, day) = civil_from_days(days as i64);

        let year2 = (year % 100) as u8;
        let century = (year / 100) as u8;

        match reg {
            0x00 => Self::to_bcd(sec),
            0x02 => Self::to_bcd(min),
            0x04 => Self::to_bcd(hour),
            0x07 => Self::to_bcd(day as u8),
            0x08 => Self::to_bcd(month as u8),
            0x09 => Self::to_bcd(year2),

            0x32 => Self::to_bcd(century),

            0x0A => 0x26,
            0x0B => 0x02,
            0x0C => 0x00,
            0x0D => 0x80,

            _ => 0x00,
        }
    }
}

impl PioDevice for CmosRtc {
    fn handles_port(&self, port: u16) -> bool {
        Self::is_port(port)
    }

    fn io_out(&mut self, port: u16, data: &[u8]) {
        if data.is_empty() {
            return;
        }
        let v = data[0];

        match port {
            0x70 => {
                // bit 7 is NMI disable, lower 7 bits is index
                self.index = v & 0x7F;
            }
            0x71 => {
                // ignore CMOS writes for now
            }
            _ => {}
        }
    }

    fn io_in(&mut self, port: u16, data: &mut [u8]) {
        let v = match port {
            0x70 => self.index,
            0x71 => self.read_reg(self.index),
            _ => 0x00,
        };

        for b in data.iter_mut() {
            *b = v;
        }
    }
}

fn civil_from_days(z: i64) -> (i32, i32, i32) {
    // days since 1970-01-01 (unix start date)
    let z = z + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let doe = z - era * 146097;
    let yoe = (doe - doe / 1460 + doe / 36524 - doe / 146096) / 365;
    let y = yoe as i32 + (era * 400) as i32;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = (doy - (153 * mp + 2) / 5 + 1) as i32;
    let m = (mp + if mp < 10 { 3 } else { -9 }) as i32;
    let year = y + if m <= 2 { 1 } else { 0 };
    (year, m, d)
}
