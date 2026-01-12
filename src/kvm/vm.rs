use kvm_bindings::kvm_userspace_memory_region;
use kvm_ioctls::{VcpuFd, VmFd};

use crate::{guest::memory::GuestMemory, vmm::Result};

pub struct KvmVm {
    pub vm: VmFd,
}

impl KvmVm {
    pub fn map_guest_memory(&self, mem: &GuestMemory) -> Result<()> {
        let region = kvm_userspace_memory_region {
            slot: 0,
            flags: 0,
            guest_phys_addr: 0,
            memory_size: mem.size_bytes() as u64,
            userspace_addr: mem.as_ptr_u64(),
        };
        unsafe { self.vm.set_user_memory_region(region) }?;
        Ok(())
    }

    pub fn create_vcpu(&self, id: u8) -> Result<crate::kvm::vcpu::KvmVcpu> {
        let vcpu: VcpuFd = self.vm.create_vcpu(id.into())?;
        Ok(crate::kvm::vcpu::KvmVcpu { vcpu })
    }
}
