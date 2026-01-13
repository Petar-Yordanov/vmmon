use kvm_ioctls::Kvm;

use crate::vmm::Result;

pub struct KvmHost {
    pub kvm: Kvm,
}

impl KvmHost {
    pub fn new() -> Result<Self> {
        Ok(Self { kvm: Kvm::new()? })
    }

    pub fn create_vm(&self) -> Result<crate::kvm::vm::KvmVm> {
        let vm = self.kvm.create_vm()?;
        Ok(crate::kvm::vm::KvmVm { vm })
    }
}
