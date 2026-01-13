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

    pub fn write_u64(&mut self, gpa: u64, v: u64) {
        self.write(gpa, &v.to_le_bytes());
    }

    pub fn write_u32(&mut self, gpa: u64, v: u32) {
        self.write(gpa, &v.to_le_bytes());
    }

    pub fn as_ptr_u64(&self) -> u64 {
        self.mmap.as_ptr() as u64
    }

    pub fn write(&mut self, gpa: u64, bytes: &[u8]) {
        let start = gpa as usize;
        let end = start + bytes.len();
        self.mmap[start..end].copy_from_slice(bytes);
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
}
