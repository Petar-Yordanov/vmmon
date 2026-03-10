<a id="readme-top"></a>

<!-- PROJECT LOGO -->
<br />
<div align="center">
  <a href="https://github.com/Petar-Yordanov/vmmon">
    <img src="assets/image.jpeg" alt="Logo" width="200" height="200">
  </a>

  <h3 align="center">VMMon</h3>

  <p align="center">
    VMMon is a Virtual Machine Monitor for Linux to run MicrOS (a 64-bit Rust kernel) 
  </p>
</div>

<!-- TABLE OF CONTENTS -->
<details>
  <summary>Table of Contents</summary>
  <ol>
    <li>
      <a href="#about-the-project">About The Project</a>
      <ul>
        <li><a href="#built-with">Built With</a></li>
      </ul>
    </li>
    <li>
      <a href="#getting-started">Getting Started</a>
      <ul>
        <li><a href="#prerequisites">Prerequisites</a></li>
        <li><a href="#installation">Installation</a></li>
      </ul>
    </li>
    <li><a href="#usage">Usage</a></li>
    <li><a href="#roadmap">Roadmap</a></li>
    <li><a href="#license">License</a></li>
  </ol>
</details>


<!-- ABOUT THE PROJECT -->
## About The Project

A minimal Type-2 Virtual Machine Monitor (VMM) built on Linux KVM that **direct-boots** my x86_64 Rust kernel (Limine protocol) without ISO/BIOS/UEFI.  
The VMM loads the kernel ELF into guest RAM, enters 64-bit long mode, provides the required Limine request/response data, and then runs the VM with basic PCI + virtio devices.

<div align="center">

[![CI](https://github.com/Petar-Yordanov/vmmon/actions/workflows/build.yml/badge.svg?branch=main)](https://github.com/Petar-Yordanov/vmmon/actions/workflows/build.yml)

</div>


<p align="right">(<a href="#readme-top">back to top</a>)</p>

### Built With

* [![Rust][rust-badge]][rust-url]
* [![Linux][linux-badge]][linux-url]
* [![GitHub Actions][gha-badge]][gha-url]

<!-- Badges -->
[rust-badge]: https://img.shields.io/badge/Rust-000000?style=for-the-badge&logo=rust&logoColor=white
[rust-url]: https://www.rust-lang.org/

[linux-badge]: https://img.shields.io/badge/Linux-FCC624?style=for-the-badge&logo=linux&logoColor=black
[linux-url]: https://www.kernel.org/

[gha-badge]: https://img.shields.io/badge/GitHub%20Actions-2088FF?style=for-the-badge&logo=githubactions&logoColor=white
[gha-url]: https://github.com/features/actions

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- GETTING STARTED -->
## Getting Started

`vmmon` is a minimal Type-2 Virtual Machine Monitor built on Linux KVM. It runs on modern Linux distros with KVM enabled and uses QEMU tooling only for building/managing disk images (not for running the VM).

### Prerequisites

- Linux host (Ubuntu 24.04 / Fedora 41 tested)
- Hardware virtualization enabled in BIOS/UEFI (Intel VT-x / AMD-V)
- KVM device available: `/dev/kvm`
- Rust stable toolchain
- Build tooling: clang + lld, plus standard build essentials
- QEMU tools (for `qemu-img` / image utilities)

* Essentials setup on Ubuntu/Debian
  ```sh
  sudo apt-get update
  sudo apt-get install -y \
    build-essential pkg-config \
    clang lld \
    make \
    ca-certificates curl \
    qemu-system-x86 qemu-utils
  ```

* Install Rust
  ```sh
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
    source "$HOME/.cargo/env"
    rustup toolchain install stable
    rustup default stable
  ```

* Setup user
  ```sh
    ls -l /dev/kvm
    sudo usermod -aG kvm "$USER"
  ```

### Installation

1. Clone the repo
   ```sh
   git clone git@github.com:Petar-Yordanov/vmmon.git
   ```
2. Build
  ```sh
    # Debug
    cargo build -p vmmon

    # Release
    cargo build -p vmmon --release
   ```

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- USAGE EXAMPLES -->
## Usage

```sh
  # Run (Debug)
  cargo run -p vmmon -- ./kernel.elf --mem-mib 1024
```

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Roadmap

- [x] **VMM foundation**
  - [x] Create KVM VM + vCPU(s)
  - [x] Allocate guest RAM + map it into KVM
  - [x] vCPU run loop + exit dispatch (PIO/MMIO)
  - [x] Debug output from guest

- [x] **Boot**
  - [x] Enter 64-bit long mode (page tables + control regs)
  - [x] Load MicrOS ELF into guest memory
  - [x] Implement minimal Limine protocol support
    - [x] Memory map response
    - [x] HHDM response
    - [x] Framebuffer response
  - [x] Jump to kernel entry
  - [x] Reach MicrOS kernel initialization
  - [x] Reach MicrOS storage / VFS selftests

- [ ] **Interrupts and machine control**
  - [ ] KVM irqchip setup
  - [ ] Local APIC interrupt delivery
  - [ ] APIC timer compatible delivery
  - [ ] Shutdown handling
  - [ ] Reboot handling

- [x] **Time devices**
  - [x] CMOS RTC support
  - [x] RTC / time data compatible with MicrOS expectations

- [x] **Legacy devices**
  - [x] Uart16550
  - [x] Pic8259
  - [x] CmosRtc
  - [x] Dma8237

- [x] **PCI**
  - [x] PCI config space access
  - [x] PCI enumeration support
  - [x] BAR probing support
  - [x] BAR mapping model for virtio PCI devices
  - [x] Virtio PCI capability exposure

- [x] **virtio-blk**
  - [x] Common configuration support
  - [x] Device configuration support
  - [x] Notify path
  - [x] Host-file-backed disk image backend
  - [x] Polling-based request processing
  - [x] MicrOS virtio-blk bring-up
  - [x] MicrOS FAT32 / ext2 mount compatibility
  - [x] MicrOS VFS read / write / append compatibility
  - [ ] Interrupt-driven request completion

- [ ] **virtio-input**
  - [ ] Keyboard events
  - [ ] Mouse events
  - [ ] MicrOS virtio-input compatibility
  - [ ] Interrupt-driven input delivery

- [ ] **MicrOS runtime support**
  - [ ] Scheduler / timer runtime compatibility
  - [ ] Userspace bootstrap compatibility
  - [ ] Window manager compatibility
  - [ ] GUI runtime compatibility

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- LICENSE -->
## License

Distributed under the MIT License. See `LICENSE.md` for details.

<p align="right">(<a href="#readme-top">back to top</a>)</p>
