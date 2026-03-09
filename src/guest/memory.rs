use memmap2::MmapMut;

use crate::vmm::Result;

pub struct GuestMemory {
    mmap: MmapMut,
    size: usize,
}

impl GuestMemory {
    pub fn new(size_bytes: usize) -> Result<Self> {
        let mmap = MmapMut::map_anon(size_bytes)?;
        Ok(Self {
            mmap,
            size: size_bytes,
        })
    }

    pub fn size_bytes(&self) -> usize {
        self.size
    }

    pub fn as_ptr_u64(&self) -> u64 {
        self.mmap.as_ptr() as u64
    }

    pub fn write(&mut self, gpa: u64, bytes: &[u8]) {
        let start = gpa as usize;
        let end = start + bytes.len();
        self.mmap[start..end].copy_from_slice(bytes);
    }

    pub fn read(&self, gpa: u64, out: &mut [u8]) {
        out.copy_from_slice(self.slice(gpa, out.len()));
    }

    pub fn write_zeros(&mut self, gpa: u64, len: usize) {
        let start = gpa as usize;
        let end = start + len;
        self.mmap[start..end].fill(0);
    }

    pub fn slice_mut(&mut self, gpa: u64, len: usize) -> &mut [u8] {
        let start = gpa as usize;
        let end = start + len;
        &mut self.mmap[start..end]
    }

    pub fn slice(&self, gpa: u64, len: usize) -> &[u8] {
        let start = gpa as usize;
        let end = start + len;
        &self.mmap[start..end]
    }

    pub fn write_u8(&mut self, gpa: u64, v: u8) {
        self.write(gpa, &[v]);
    }

    pub fn read_u8(&self, gpa: u64) -> u8 {
        let mut b = [0u8; 1];
        self.read(gpa, &mut b);
        b[0]
    }

    pub fn write_u16(&mut self, gpa: u64, v: u16) {
        self.write(gpa, &v.to_le_bytes());
    }

    pub fn read_u16(&self, gpa: u64) -> u16 {
        let mut b = [0u8; 2];
        self.read(gpa, &mut b);
        u16::from_le_bytes(b)
    }

    pub fn write_u32(&mut self, gpa: u64, v: u32) {
        self.write(gpa, &v.to_le_bytes());
    }

    pub fn read_u32(&self, gpa: u64) -> u32 {
        let mut b = [0u8; 4];
        self.read(gpa, &mut b);
        u32::from_le_bytes(b)
    }

    pub fn write_u64(&mut self, gpa: u64, v: u64) {
        self.write(gpa, &v.to_le_bytes());
    }

    pub fn read_u64(&self, gpa: u64) -> u64 {
        let mut b = [0u8; 8];
        self.read(gpa, &mut b);
        u64::from_le_bytes(b)
    }
}
