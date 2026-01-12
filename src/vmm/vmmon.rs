use crate::devices::serial::uart16550::Uart16550;
use kvm_ioctls::VcpuExit;

use crate::{
    app::config::VmConfig,
    boot::{elf, long_mode},
    guest::{layout::GuestLayout, memory::GuestMemory},
    kvm::{host::KvmHost, vcpu::KvmVcpu, vm::KvmVm},
    vmm::{Result, VmExitInfo, VmExitReason},
};

pub struct Vmmon {
    _host: KvmHost,
    _vm: KvmVm,
    vcpu: KvmVcpu,
    _guest: GuestMemory,

    uart: Uart16550,
}

impl Vmmon {
    pub fn build(cfg: VmConfig) -> Result<Self> {
        let host = KvmHost::new()?;
        let vm = host.create_vm()?;

        let layout = GuestLayout::default_layout(cfg.mem_size_bytes);

        let mut guest = GuestMemory::new(cfg.mem_size_bytes)?;
        vm.map_guest_memory(&guest)?;

        let elf_bytes = std::fs::read(&cfg.kernel_path)?;
        let load = elf::load_elf64(&mut guest, &layout, &elf_bytes)?;

        let vcpu = vm.create_vcpu(0)?;
        vcpu.setup_cpuid(&host)?;

        let sregs_template = vcpu.vcpu.get_sregs()?;
        let boot_state =
            long_mode::build_long_mode_boot_state(&mut guest, &layout, &load, sregs_template)?;

        if let Err(e) = vcpu.set_sregs(&boot_state.sregs) {
            eprintln!("KVM_SET_SREGS failed: {e}");
            return Err(e);
        }

        if let Err(e) = vcpu.set_regs(&boot_state.regs) {
            eprintln!("KVM_SET_REGS failed: {e}");
            return Err(e);
        }

        let uart = Uart16550::new(0x3F8);

        Ok(Self {
            _host: host,
            _vm: vm,
            vcpu,
            _guest: guest,
            uart,
        })
    }

    pub fn run(&mut self) -> Result<VmExitInfo> {
        loop {
            match self.vcpu.run()? {
                VcpuExit::Hlt => return Ok(VmExitInfo::hlt()),
                VcpuExit::Shutdown => return Ok(VmExitInfo::shutdown()),
                VcpuExit::InternalError => {
                    return Ok(VmExitInfo {
                        reason: VmExitReason::InternalError,
                    });
                }
                VcpuExit::IoOut(port, data) => {
                    if port == 0xE9 {
                        for &b in data {
                            print!("{}", b as char);
                        }
                        use std::io::Write;
                        let _ = std::io::stdout().flush();
                        continue;
                    }

                    if self.uart.handles_port(port) {
                        self.uart.io_out(port, data);
                        continue;
                    }

                    return Ok(VmExitInfo {
                        reason: VmExitReason::Unhandled(format!(
                            "io out port={:#x} len={}",
                            port,
                            data.len()
                        )),
                    });
                }

                VcpuExit::IoIn(port, data) => {
                    if self.uart.handles_port(port) {
                        self.uart.io_in(port, data);
                        continue;
                    }

                    return Ok(VmExitInfo {
                        reason: VmExitReason::Unhandled(format!(
                            "io in port={:#x} len={}",
                            port,
                            data.len()
                        )),
                    });
                }
                other => {
                    return Ok(VmExitInfo {
                        reason: VmExitReason::Unhandled(format!("{other:?}")),
                    });
                }
            }
        }
    }
}
