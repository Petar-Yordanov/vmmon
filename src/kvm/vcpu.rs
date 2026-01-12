use kvm_bindings::{KVM_MAX_CPUID_ENTRIES, kvm_regs, kvm_sregs};
use kvm_ioctls::VcpuFd;

use crate::{kvm::host::KvmHost, vmm::Result};

pub struct KvmVcpu {
    pub vcpu: VcpuFd,
}

impl KvmVcpu {
    pub fn setup_cpuid(&self, host: &KvmHost) -> Result<()> {
        let cpuid = host.kvm.get_supported_cpuid(KVM_MAX_CPUID_ENTRIES)?;
        self.vcpu.set_cpuid2(&cpuid)?;
        Ok(())
    }

    pub fn set_regs(&self, regs: &kvm_regs) -> Result<()> {
        self.vcpu.set_regs(regs)?;
        Ok(())
    }

    pub fn set_sregs(&self, sregs: &kvm_sregs) -> Result<()> {
        self.vcpu.set_sregs(sregs)?;
        Ok(())
    }

    pub fn run(&mut self) -> Result<kvm_ioctls::VcpuExit<'_>> {
        Ok(self.vcpu.run()?)
    }
}
