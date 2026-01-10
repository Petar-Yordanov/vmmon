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
  # Run (debug)
  cargo run -p vmmon
```

<p align="right">(<a href="#readme-top">back to top</a>)</p>

## Roadmap

- [ ] **VMM core**
  - [ ] Create KVM VM + vCPU
  - [ ] Allocate guest RAM and register `KVM_SET_USER_MEMORY_REGION`
  - [ ] vCPU run loop with exit dispatch (PIO + MMIO)
  - [ ] Basic debug console (port `0xE9` or 16550 COM1)

- [ ] **Boot to 64-bit (long mode)**
  - [ ] Build guest page tables (PML4/PDPT/PD/PT)
  - [ ] Map low identity region (bootstrap + page tables + stack)
  - [ ] Map kernel virtual addresses to the chosen physical load addresses (supports higher-half)
  - [ ] Set CR0/CR4/EFER + 64-bit segments, then run a 64-bit smoke test

- [ ] **Direct-load the kernel (no bootloader)**
  - [ ] ELF64 loader: copy `PT_LOAD` segments into guest RAM, zero BSS
  - [ ] Allocate a guest stack, set initial `RSP`
  - [ ] Set initial `RIP` to kernel entry (or a tiny trampoline)

- [ ] **Limine protocol support (boot contract)**
  - [ ] Locate Limine request pointers in the kernel image (ELF section)
  - [ ] Allocate and fill required Limine responses in guest RAM (minimum set used by the kernel)
  - [ ] Write response pointers back into the requests
  - [ ] Jump to kernel entry and reach kernel early log

- [ ] **Minimal platform**
  - [ ] In-kernel irqchip
  - [ ] Clean shutdown/reset handling

- [ ] **PCI**
  - [ ] PCI config space emulation
  - [ ] BAR routing to MMIO/PIO handlers

- [ ] **Virtio (to run the OS normally)**
  - [ ] virtio-pci capabilities (common/notify/isr/device cfg)
  - [ ] virtqueue implementation
  - [ ] virtio-blk
  - [ ] virtio-input
  - [ ] Interrupt delivery

<p align="right">(<a href="#readme-top">back to top</a>)</p>

<!-- LICENSE -->
## License

Distributed under the MIT License. See `LICENSE.md` for details.

<p align="right">(<a href="#readme-top">back to top</a>)</p>
