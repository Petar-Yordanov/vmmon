use kvm_bindings::kvm_pit_config;
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

        vm.set_tss_address(0xfffb_d000)?;
        vm.set_identity_map_address(0xfffb_c000)?;
        vm.create_irq_chip()?;
        vm.create_pit2(kvm_pit_config::default())?;

        Ok(crate::kvm::vm::KvmVm { vm })
    }
}
