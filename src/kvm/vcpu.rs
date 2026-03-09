use kvm_bindings::{KVM_MAX_CPUID_ENTRIES, kvm_regs, kvm_sregs};
use kvm_ioctls::VcpuFd;

use crate::{kvm::host::KvmHost, vmm::Result};

pub struct KvmVcpu {
    pub vcpu: VcpuFd,
}

impl KvmVcpu {
    pub fn setup_cpuid(&self, host: &KvmHost) -> Result<()> {
        let mut cpuid = host.kvm.get_supported_cpuid(KVM_MAX_CPUID_ENTRIES)?;

        for entry in cpuid.as_mut_slice() {
            match (entry.function, entry.index) {
                // Basic processor features
                (0x0000_0001, 0) => {
                    entry.ecx &= !(1 << 21); // x2APIC
                    entry.ecx &= !(1 << 27); // OSXSAVE
                    entry.ecx &= !(1 << 28); // AVX
                    entry.ecx |= 1 << 31; // hypervisor-present

                    entry.ecx &= !(1 << 17); // PCID
                    entry.ecx &= !(1 << 12); // FMA
                    entry.ecx &= !(1 << 29); // F16C
                    entry.ecx &= !(1 << 30); // RDRAND
                }

                // Structured extended features
                (0x0000_0007, 0) => {
                    entry.ebx &= !(1 << 7);  // SMEP
                    entry.ebx &= !(1 << 10); // INVPCID
                    entry.ebx &= !(1 << 18); // RDSEED
                    entry.ebx &= !(1 << 19); // ADX
                    entry.ebx &= !(1 << 20); // SMAP
                    entry.ebx &= !(1 << 26); // AVX512PF
                    entry.ebx &= !(1 << 27); // AVX512ER
                    entry.ebx &= !(1 << 28); // AVX512CD
                    entry.ebx &= !(1 << 29); // SHA
                    entry.ebx &= !(1 << 30); // AVX512BW
                    entry.ebx &= !(1 << 31); // AVX512VL

                    entry.ecx &= !(1 << 16);
                }

                (0x0000_000d, _) => {
                    entry.eax = 0;
                    entry.ebx = 0;
                    entry.ecx = 0;
                    entry.edx = 0;
                }

                (0x8000_0001, 0) => {
                    entry.ecx &= !(1 << 2);
                }

                _ => {}
            }
        }

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
