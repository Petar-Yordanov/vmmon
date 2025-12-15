use kvm_bindings::KVM_MAX_CPUID_ENTRIES;
use kvm_bindings::*;
use kvm_ioctls::{Kvm, VcpuExit, VcpuFd};
use memmap2::MmapMut;

const GUEST_MEM_SIZE: usize = 64 * 1024 * 1024; // 64MB
const GUEST_ENTRY: u64 = 0x1000;

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() -> Result<()> {
    let kvm = Kvm::new()?;
    let vm = kvm.create_vm()?;

    let mut guest_mem = MmapMut::map_anon(GUEST_MEM_SIZE)?;

    let mem_region = kvm_userspace_memory_region {
        slot: 0,
        flags: 0,
        guest_phys_addr: 0,
        memory_size: GUEST_MEM_SIZE as u64,
        userspace_addr: guest_mem.as_ptr() as u64,
    };
    unsafe { vm.set_user_memory_region(mem_region) }?;

    load_guest_code(&mut guest_mem)?;

    let vcpu_fd = vm.create_vcpu(0)?;

    // CPUID
    setup_cpuid(&kvm, &vcpu_fd)?;

    // Basic real-mode segments
    setup_sregs(&vcpu_fd)?;

    setup_regs(&vcpu_fd)?;
    run_vcpu(vcpu_fd)?;

    Ok(())
}

fn load_guest_code(mem: &mut MmapMut) -> Result<()> {
    let code = [
        0xF4, // hlt
        0xEB, 0xFD, // jmp $
    ];

    let start = GUEST_ENTRY as usize;
    mem[start..start + code.len()].copy_from_slice(&code);
    Ok(())
}

fn setup_regs(vcpu: &VcpuFd) -> Result<()> {
    let mut regs = vcpu.get_regs()?;
    regs.rip = GUEST_ENTRY;
    regs.rflags = 0x2;
    regs.rsp = 0x8000;
    vcpu.set_regs(&regs)?;
    Ok(())
}

fn run_vcpu(mut vcpu: VcpuFd) -> Result<()> {
    loop {
        match vcpu.run()? {
            VcpuExit::Hlt => {
                println!("Guest executed HLT");
            }
            VcpuExit::Shutdown => {
                println!("Guest shutdown");
                break;
            }
            exit => {
                println!("Unhandled exit: {:?}", exit);
                break;
            }
        }
    }
    Ok(())
}

fn setup_cpuid(kvm: &Kvm, vcpu: &VcpuFd) -> Result<()> {
    let cpuid = kvm.get_supported_cpuid(KVM_MAX_CPUID_ENTRIES)?;
    vcpu.set_cpuid2(&cpuid)?;
    Ok(())
}

fn setup_sregs(vcpu: &VcpuFd) -> Result<()> {
    let mut sregs = vcpu.get_sregs()?;

    sregs.cs.base = 0;
    sregs.cs.selector = 0;

    vcpu.set_sregs(&sregs)?;
    Ok(())
}
