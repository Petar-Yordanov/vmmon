use std::{
    fs::OpenOptions,
    io::{Read, Seek, SeekFrom, Write},
    path::PathBuf,
};

use crate::guest::memory::GuestMemory;

const PCI_VENDOR_ID_REDHAT_QUMRANET: u16 = 0x1af4;

const PCI_CAP_ID_VENDOR_SPECIFIC: u8 = 0x09;
const PCI_STATUS_CAP_LIST: u16 = 1 << 4;

const VIRTIO_PCI_CAP_COMMON_CFG: u8 = 1;
const VIRTIO_PCI_CAP_NOTIFY_CFG: u8 = 2;
const VIRTIO_PCI_CAP_ISR_CFG: u8 = 3;
const VIRTIO_PCI_CAP_DEVICE_CFG: u8 = 4;

const VIRTIO_PCI_DEVICE_ID_BLOCK: u16 = 0x1042;
const VIRTIO_PCI_DEVICE_ID_INPUT: u16 = 0x1052;

const VIRTIO_STATUS_FEATURES_OK: u8 = 8;
const VIRTIO_STATUS_FAILED: u8 = 128;

const BAR4_MMIO_BASE_BLK: u32 = 0xfeb0_0000;
const BAR4_MMIO_BASE_INPUT: u32 = 0xfeb0_4000;
const BAR4_MMIO_SIZE: u32 = 0x4000;

const COMMON_CFG_OFF: u32 = 0x0000;
const ISR_CFG_OFF: u32 = 0x1000;
const DEVICE_CFG_OFF: u32 = 0x2000;
const NOTIFY_CFG_OFF: u32 = 0x3000;
const REGION_LEN: u32 = 0x1000;

const QUEUE_SIZE: u16 = 128;
const NUM_QUEUES_BLK: u16 = 1;
const NUM_QUEUES_INPUT: u16 = 2;

const VIRTQ_DESC_F_NEXT: u16 = 1;
const VIRTQ_DESC_F_WRITE: u16 = 2;

const VIRTIO_BLK_T_IN: u32 = 0;
const VIRTIO_BLK_T_OUT: u32 = 1;

const VIRTIO_BLK_S_OK: u8 = 0;
const VIRTIO_BLK_S_IOERR: u8 = 1;

const VIRTIO_INPUT_CFG_UNSET: u8 = 0x00;
const VIRTIO_INPUT_CFG_ID_NAME: u8 = 0x01;
const VIRTIO_INPUT_CFG_ID_SERIAL: u8 = 0x02;
const VIRTIO_INPUT_CFG_ID_DEVIDS: u8 = 0x03;
const VIRTIO_INPUT_CFG_PROP_BITS: u8 = 0x10;
const VIRTIO_INPUT_CFG_EV_BITS: u8 = 0x11;
const VIRTIO_INPUT_CFG_ABS_INFO: u8 = 0x12;

const EV_SYN: usize = 0;
const EV_KEY: usize = 1;
const EV_REL: usize = 2;

const REL_X: usize = 0;
const REL_Y: usize = 1;
const REL_WHEEL: usize = 8;

const BTN_LEFT: usize = 0x110;
const BTN_RIGHT: usize = 0x111;
const BTN_MIDDLE: usize = 0x112;

#[derive(Debug)]
pub struct PciBus {
    devices: Vec<PciFunction>,
}

#[derive(Debug)]
struct PciFunction {
    bus: u8,
    dev: u8,
    func: u8,
    config: [u8; 256],
    bar4_addr: u32,
    bar4_size: u32,
    bar_probe_mask_low: bool,
    bar_probe_mask_high: bool,
    kind: PciFunctionKind,
}

#[derive(Debug)]
enum PciFunctionKind {
    VirtioBlk(VirtioBlkState),
    VirtioInput(VirtioInputState),
}

#[derive(Debug)]
struct VirtioBlkState {
    common: VirtioCommonCfgState,
    queue: VirtioQueueState,
    isr_status: u8,
    capacity_sectors: u64,
    backend: std::fs::File,
}

#[derive(Debug)]
struct VirtioInputState {
    common: VirtioCommonCfgState,
    queues: [VirtioQueueState; 2],
    isr_status: u8,
    config_select: u8,
    config_subsel: u8,
}

#[derive(Debug, Clone, Default)]
struct VirtioCommonCfgState {
    device_feature_select: u32,
    driver_feature_select: u32,
    driver_features: [u32; 2],
    msix_config: u16,
    device_status: u8,
    config_generation: u8,
    queue_select: u16,
}

#[derive(Debug, Clone)]
struct VirtioQueueState {
    size: u16,
    msix_vector: u16,
    enable: u16,
    notify_off: u16,
    desc_addr: u64,
    driver_addr: u64,
    device_addr: u64,
    last_avail_idx: u16,
}

impl Default for VirtioQueueState {
    fn default() -> Self {
        Self {
            size: QUEUE_SIZE,
            msix_vector: 0xffff,
            enable: 0,
            notify_off: 0,
            desc_addr: 0,
            driver_addr: 0,
            device_addr: 0,
            last_avail_idx: 0,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct VirtqDesc {
    addr: u64,
    len: u32,
    flags: u16,
    next: u16,
}

impl PciBus {
    pub fn new(disk_img_path: PathBuf) -> Self {
        let meta_len = std::fs::metadata(&disk_img_path)
            .map(|m| m.len())
            .unwrap_or(0);
        let capacity_sectors = meta_len / 512;

        let backend = OpenOptions::new()
            .read(true)
            .write(true)
            .open(&disk_img_path)
            .unwrap_or_else(|e| {
                panic!(
                    "failed to open virtio-blk backend '{}': {e}",
                    disk_img_path.display()
                )
            });

        Self {
            devices: vec![
                PciFunction::new_virtio_blk(
                    0,
                    4,
                    0,
                    BAR4_MMIO_BASE_BLK,
                    capacity_sectors,
                    backend,
                ),
                PciFunction::new_virtio_input(0, 5, 0, BAR4_MMIO_BASE_INPUT),
            ],
        }
    }

    pub fn read_config_dword(&mut self, bus: u8, dev: u8, func: u8, reg: u16) -> u32 {
        let Some(pci_fn) = self.find_mut(bus, dev, func) else {
            return 0xffff_ffff;
        };

        match reg {
            0x20 => {
                if pci_fn.bar_probe_mask_low {
                    pci_fn.bar_probe_mask_low = false;
                    let mask = (!(pci_fn.bar4_size - 1)) & 0xffff_fff0;
                    return mask | 0x4;
                }
            }
            0x24 => {
                if pci_fn.bar_probe_mask_high {
                    pci_fn.bar_probe_mask_high = false;
                    return 0;
                }
            }
            _ => {}
        }

        read_u32(&pci_fn.config, reg as usize).unwrap_or(0xffff_ffff)
    }

    pub fn write_config_dword(&mut self, bus: u8, dev: u8, func: u8, reg: u16, val: u32) {
        let Some(pci_fn) = self.find_mut(bus, dev, func) else {
            return;
        };

        match reg {
            0x04 => {
                write_u16(&mut pci_fn.config, 0x04, (val & 0xffff) as u16);
                let status = read_u16(&pci_fn.config, 0x06).unwrap_or(0);
                write_u16(&mut pci_fn.config, 0x06, status);
            }
            0x20 => {
                if val == 0xffff_ffff {
                    pci_fn.bar_probe_mask_low = true;
                } else {
                    let preserved_flags = 0x4u32;
                    let addr = val & 0xffff_fff0;
                    pci_fn.bar4_addr = addr;
                    let stored = (addr & 0xffff_fff0) | preserved_flags;
                    write_u32(&mut pci_fn.config, 0x20, stored);
                }
            }
            0x24 => {
                if val == 0xffff_ffff {
                    pci_fn.bar_probe_mask_high = true;
                } else {
                    write_u32(&mut pci_fn.config, 0x24, 0);
                }
            }
            _ => {
                if (reg as usize) + 4 <= pci_fn.config.len() {
                    write_u32(&mut pci_fn.config, reg as usize, val);
                }
            }
        }
    }

    pub fn mmio_read(&mut self, mem: &mut GuestMemory, addr: u64, data: &mut [u8]) -> bool {
        let addr32 = match u32::try_from(addr) {
            Ok(v) => v,
            Err(_) => return false,
        };

        for pci_fn in &mut self.devices {
            let Some(off) = addr32.checked_sub(pci_fn.bar4_addr) else {
                continue;
            };

            if off >= pci_fn.bar4_size {
                continue;
            }

            data.fill(0);

            match &mut pci_fn.kind {
                PciFunctionKind::VirtioBlk(blk) => {
                    if (COMMON_CFG_OFF..COMMON_CFG_OFF + REGION_LEN).contains(&off) {
                        blk.read_common_cfg(off - COMMON_CFG_OFF, data);
                        return true;
                    }

                    if (ISR_CFG_OFF..ISR_CFG_OFF + REGION_LEN).contains(&off) {
                        if !data.is_empty() {
                            data[0] = blk.isr_status;
                            blk.isr_status = 0;
                        }
                        return true;
                    }

                    if (DEVICE_CFG_OFF..DEVICE_CFG_OFF + REGION_LEN).contains(&off) {
                        blk.read_device_cfg(off - DEVICE_CFG_OFF, data);
                        return true;
                    }

                    if (NOTIFY_CFG_OFF..NOTIFY_CFG_OFF + REGION_LEN).contains(&off) {
                        let _ = mem;
                        return true;
                    }

                    let _ = mem;
                    return true;
                }
                PciFunctionKind::VirtioInput(input) => {
                    if (COMMON_CFG_OFF..COMMON_CFG_OFF + REGION_LEN).contains(&off) {
                        input.read_common_cfg(off - COMMON_CFG_OFF, data);
                        return true;
                    }

                    if (ISR_CFG_OFF..ISR_CFG_OFF + REGION_LEN).contains(&off) {
                        if !data.is_empty() {
                            data[0] = input.isr_status;
                            input.isr_status = 0;
                        }
                        return true;
                    }

                    if (DEVICE_CFG_OFF..DEVICE_CFG_OFF + REGION_LEN).contains(&off) {
                        input.read_device_cfg(off - DEVICE_CFG_OFF, data);
                        return true;
                    }

                    if (NOTIFY_CFG_OFF..NOTIFY_CFG_OFF + REGION_LEN).contains(&off) {
                        let _ = mem;
                        return true;
                    }

                    let _ = mem;
                    return true;
                }
            }
        }

        false
    }

    pub fn mmio_write(&mut self, mem: &mut GuestMemory, addr: u64, data: &[u8]) -> bool {
        let addr32 = match u32::try_from(addr) {
            Ok(v) => v,
            Err(_) => return false,
        };

        for pci_fn in &mut self.devices {
            let Some(off) = addr32.checked_sub(pci_fn.bar4_addr) else {
                continue;
            };

            if off >= pci_fn.bar4_size {
                continue;
            }

            match &mut pci_fn.kind {
                PciFunctionKind::VirtioBlk(blk) => {
                    if (COMMON_CFG_OFF..COMMON_CFG_OFF + REGION_LEN).contains(&off) {
                        blk.write_common_cfg(off - COMMON_CFG_OFF, data);
                        return true;
                    }

                    if (NOTIFY_CFG_OFF..NOTIFY_CFG_OFF + REGION_LEN).contains(&off) {
                        blk.write_notify(mem, off - NOTIFY_CFG_OFF, data);
                        return true;
                    }

                    if (ISR_CFG_OFF..ISR_CFG_OFF + REGION_LEN).contains(&off) {
                        return true;
                    }

                    if (DEVICE_CFG_OFF..DEVICE_CFG_OFF + REGION_LEN).contains(&off) {
                        return true;
                    }

                    return true;
                }
                PciFunctionKind::VirtioInput(input) => {
                    if (COMMON_CFG_OFF..COMMON_CFG_OFF + REGION_LEN).contains(&off) {
                        input.write_common_cfg(off - COMMON_CFG_OFF, data);
                        return true;
                    }

                    if (NOTIFY_CFG_OFF..NOTIFY_CFG_OFF + REGION_LEN).contains(&off) {
                        input.write_notify(mem, off - NOTIFY_CFG_OFF, data);
                        return true;
                    }

                    if (ISR_CFG_OFF..ISR_CFG_OFF + REGION_LEN).contains(&off) {
                        return true;
                    }

                    if (DEVICE_CFG_OFF..DEVICE_CFG_OFF + REGION_LEN).contains(&off) {
                        input.write_device_cfg(off - DEVICE_CFG_OFF, data);
                        return true;
                    }

                    return true;
                }
            }
        }

        false
    }

    fn find_mut(&mut self, bus: u8, dev: u8, func: u8) -> Option<&mut PciFunction> {
        self.devices
            .iter_mut()
            .find(|d| d.bus == bus && d.dev == dev && d.func == func)
    }
}

impl PciFunction {
    fn new_virtio_blk(
        bus: u8,
        dev: u8,
        func: u8,
        bar4_addr: u32,
        capacity_sectors: u64,
        backend: std::fs::File,
    ) -> Self {
        let mut this = Self::new_base(
            bus,
            dev,
            func,
            PCI_VENDOR_ID_REDHAT_QUMRANET,
            VIRTIO_PCI_DEVICE_ID_BLOCK,
            bar4_addr,
        );
        this.write_virtio_caps();
        this.kind = PciFunctionKind::VirtioBlk(VirtioBlkState::new(capacity_sectors, backend));
        this
    }

    fn new_virtio_input(bus: u8, dev: u8, func: u8, bar4_addr: u32) -> Self {
        let mut this = Self::new_base(
            bus,
            dev,
            func,
            PCI_VENDOR_ID_REDHAT_QUMRANET,
            VIRTIO_PCI_DEVICE_ID_INPUT,
            bar4_addr,
        );
        this.write_virtio_caps();
        this.kind = PciFunctionKind::VirtioInput(VirtioInputState::new());
        this
    }

    fn new_base(
        bus: u8,
        dev: u8,
        func: u8,
        vendor_id: u16,
        device_id: u16,
        bar4_addr: u32,
    ) -> Self {
        let mut config = [0u8; 256];

        write_u16(&mut config, 0x00, vendor_id);
        write_u16(&mut config, 0x02, device_id);
        write_u16(&mut config, 0x04, 0x0000);
        write_u16(&mut config, 0x06, PCI_STATUS_CAP_LIST);

        config[0x08] = 0x00;
        config[0x09] = 0x00;
        config[0x0a] = 0x00;
        config[0x0b] = 0x00;

        config[0x0e] = 0x00;
        config[0x34] = 0x40;

        write_u16(&mut config, 0x2c, PCI_VENDOR_ID_REDHAT_QUMRANET);
        write_u16(&mut config, 0x2e, device_id);

        write_u32(&mut config, 0x20, (bar4_addr & 0xffff_fff0) | 0x4);
        write_u32(&mut config, 0x24, 0);

        let dummy = OpenOptions::new()
            .read(true)
            .write(true)
            .open("/dev/null")
            .unwrap();

        Self {
            bus,
            dev,
            func,
            config,
            bar4_addr,
            bar4_size: BAR4_MMIO_SIZE,
            bar_probe_mask_low: false,
            bar_probe_mask_high: false,
            kind: PciFunctionKind::VirtioBlk(VirtioBlkState::new(0, dummy)),
        }
    }

    fn write_virtio_caps(&mut self) {
        self.write_cap16(
            0x40,
            0x50,
            VIRTIO_PCI_CAP_COMMON_CFG,
            4,
            COMMON_CFG_OFF,
            REGION_LEN,
        );

        self.write_cap20(
            0x50,
            0x64,
            VIRTIO_PCI_CAP_NOTIFY_CFG,
            4,
            NOTIFY_CFG_OFF,
            REGION_LEN,
            4,
        );

        self.write_cap16(
            0x64,
            0x74,
            VIRTIO_PCI_CAP_ISR_CFG,
            4,
            ISR_CFG_OFF,
            REGION_LEN,
        );

        self.write_cap16(
            0x74,
            0x00,
            VIRTIO_PCI_CAP_DEVICE_CFG,
            4,
            DEVICE_CFG_OFF,
            REGION_LEN,
        );
    }

    fn write_cap16(
        &mut self,
        off: usize,
        next: u8,
        cfg_type: u8,
        bar: u8,
        cap_off: u32,
        cap_len: u32,
    ) {
        self.config[off + 0] = PCI_CAP_ID_VENDOR_SPECIFIC;
        self.config[off + 1] = next;
        self.config[off + 2] = 16;
        self.config[off + 3] = cfg_type;
        self.config[off + 4] = bar;
        self.config[off + 5] = 0;
        self.config[off + 6] = 0;
        self.config[off + 7] = 0;
        write_u32(&mut self.config, off + 8, cap_off);
        write_u32(&mut self.config, off + 12, cap_len);
    }

    fn write_cap20(
        &mut self,
        off: usize,
        next: u8,
        cfg_type: u8,
        bar: u8,
        cap_off: u32,
        cap_len: u32,
        notify_off_multiplier: u32,
    ) {
        self.config[off + 0] = PCI_CAP_ID_VENDOR_SPECIFIC;
        self.config[off + 1] = next;
        self.config[off + 2] = 20;
        self.config[off + 3] = cfg_type;
        self.config[off + 4] = bar;
        self.config[off + 5] = 0;
        self.config[off + 6] = 0;
        self.config[off + 7] = 0;
        write_u32(&mut self.config, off + 8, cap_off);
        write_u32(&mut self.config, off + 12, cap_len);
        write_u32(&mut self.config, off + 16, notify_off_multiplier);
    }
}

impl VirtioBlkState {
    fn new(capacity_sectors: u64, backend: std::fs::File) -> Self {
        Self {
            common: VirtioCommonCfgState {
                config_generation: 1,
                ..VirtioCommonCfgState::default()
            },
            queue: VirtioQueueState::default(),
            isr_status: 0,
            capacity_sectors,
            backend,
        }
    }

    fn reset(&mut self) {
        self.common = VirtioCommonCfgState {
            config_generation: self.common.config_generation.wrapping_add(1),
            ..VirtioCommonCfgState::default()
        };
        self.queue = VirtioQueueState::default();
        self.isr_status = 0;
    }

    fn read_common_cfg(&mut self, off: u32, data: &mut [u8]) {
        let value = match off {
            0x00 => self.common.device_feature_select as u64,
            0x04 => self.device_feature(self.common.device_feature_select) as u64,
            0x08 => self.common.driver_feature_select as u64,
            0x0c => self.common.driver_features[self.common.driver_feature_select.min(1) as usize]
                as u64,
            0x10 => self.common.msix_config as u64,
            0x12 => NUM_QUEUES_BLK as u64,
            0x14 => self.common.device_status as u64,
            0x15 => self.common.config_generation as u64,
            0x16 => self.common.queue_select as u64,
            0x18 => self.queue.size as u64,
            0x1a => self.queue.msix_vector as u64,
            0x1c => self.queue.enable as u64,
            0x1e => self.queue.notify_off as u64,
            0x20 => (self.queue.desc_addr & 0xffff_ffff) as u64,
            0x24 => (self.queue.desc_addr >> 32) as u64,
            0x28 => (self.queue.driver_addr & 0xffff_ffff) as u64,
            0x2c => (self.queue.driver_addr >> 32) as u64,
            0x30 => (self.queue.device_addr & 0xffff_ffff) as u64,
            0x34 => (self.queue.device_addr >> 32) as u64,
            _ => 0,
        };

        write_value_to_buf(value, data);
    }

    fn write_common_cfg(&mut self, off: u32, data: &[u8]) {
        let value = read_value_from_buf(data);

        match off {
            0x00 => self.common.device_feature_select = value as u32,
            0x08 => self.common.driver_feature_select = value as u32,
            0x0c => {
                let idx = self.common.driver_feature_select.min(1) as usize;
                self.common.driver_features[idx] = value as u32;
            }
            0x10 => self.common.msix_config = value as u16,
            0x14 => self.write_device_status(value as u8),
            0x16 => self.common.queue_select = value as u16,
            0x1a => self.queue.msix_vector = value as u16,
            0x1c => self.queue.enable = value as u16,
            0x20 => {
                self.queue.desc_addr =
                    (self.queue.desc_addr & 0xffff_ffff_0000_0000) | (value as u32 as u64);
            }
            0x24 => {
                self.queue.desc_addr =
                    (self.queue.desc_addr & 0x0000_0000_ffff_ffff) | ((value as u32 as u64) << 32);
            }
            0x28 => {
                self.queue.driver_addr =
                    (self.queue.driver_addr & 0xffff_ffff_0000_0000) | (value as u32 as u64);
            }
            0x2c => {
                self.queue.driver_addr = (self.queue.driver_addr & 0x0000_0000_ffff_ffff)
                    | ((value as u32 as u64) << 32);
            }
            0x30 => {
                self.queue.device_addr =
                    (self.queue.device_addr & 0xffff_ffff_0000_0000) | (value as u32 as u64);
            }
            0x34 => {
                self.queue.device_addr = (self.queue.device_addr & 0x0000_0000_ffff_ffff)
                    | ((value as u32 as u64) << 32);
            }
            _ => {}
        }
    }

    fn read_device_cfg(&self, off: u32, data: &mut [u8]) {
        let value = match off {
            0x00 => self.capacity_sectors,
            _ => 0,
        };

        write_value_to_buf(value, data);
    }

    fn write_notify(&mut self, mem: &mut GuestMemory, _off: u32, _data: &[u8]) {
        let _ = self.process_available(mem);
    }

    fn process_available(&mut self, mem: &mut GuestMemory) -> std::io::Result<()> {
        if self.queue.enable == 0
            || self.queue.desc_addr == 0
            || self.queue.driver_addr == 0
            || self.queue.device_addr == 0
        {
            return Ok(());
        }

        let avail_idx = mem.read_u16(self.queue.driver_addr + 2);
        while self.queue.last_avail_idx != avail_idx {
            let ring_slot = (self.queue.last_avail_idx % self.queue.size) as u64;
            let head = mem.read_u16(self.queue.driver_addr + 4 + ring_slot * 2);
            self.process_one_request(mem, head)?;
            self.queue.last_avail_idx = self.queue.last_avail_idx.wrapping_add(1);
        }

        Ok(())
    }

    fn process_one_request(&mut self, mem: &mut GuestMemory, head: u16) -> std::io::Result<()> {
        let d0 = self.read_desc(mem, head);
        let d1 = if (d0.flags & VIRTQ_DESC_F_NEXT) != 0 {
            self.read_desc(mem, d0.next)
        } else {
            eprintln!(
                "[vmmon][blk] malformed chain: head={} missing second descriptor d0=addr:{:#x} len:{} flags:{:#x} next:{}",
                head, d0.addr, d0.len, d0.flags, d0.next
            );
            self.fail_request(mem, head, 0);
            return Ok(());
        };
        let d2 = if (d1.flags & VIRTQ_DESC_F_NEXT) != 0 {
            self.read_desc(mem, d1.next)
        } else {
            eprintln!(
                "[vmmon][blk] malformed chain: head={} missing status descriptor d0=addr:{:#x} len:{} flags:{:#x} next:{} d1=addr:{:#x} len:{} flags:{:#x} next:{}",
                head, d0.addr, d0.len, d0.flags, d0.next, d1.addr, d1.len, d1.flags, d1.next
            );
            self.fail_request(mem, head, 0);
            return Ok(());
        };

        let req_type = mem.read_u32(d0.addr);
        let _reserved = mem.read_u32(d0.addr + 4);
        let sector = mem.read_u64(d0.addr + 8);
        let sector_off = sector.saturating_mul(512);
        let status_addr = d2.addr;

        match req_type {
            VIRTIO_BLK_T_IN => {
                let mut buf = vec![0u8; d1.len as usize];
                self.backend.seek(SeekFrom::Start(sector_off))?;
                self.backend.read_exact(&mut buf)?;
                mem.write(d1.addr, &buf);
                mem.write_u8(status_addr, VIRTIO_BLK_S_OK);

                let used_len = d1.len + 1;
                self.push_used(mem, head, used_len);
            }
            VIRTIO_BLK_T_OUT => {
                let data = mem.slice(d1.addr, d1.len as usize).to_vec();
                self.backend.seek(SeekFrom::Start(sector_off))?;
                self.backend.write_all(&data)?;
                self.backend.flush()?;
                mem.write_u8(status_addr, VIRTIO_BLK_S_OK);

                let used_len = 1;
                self.push_used(mem, head, used_len);
            }
            _ => {
                mem.write_u8(status_addr, VIRTIO_BLK_S_IOERR);

                let used_len = 1;
                self.push_used(mem, head, used_len);
            }
        }

        Ok(())
    }

    fn fail_request(&mut self, mem: &mut GuestMemory, head: u16, used_len: u32) {
        self.push_used(mem, head, used_len);
    }

    fn push_used(&mut self, mem: &mut GuestMemory, head: u16, used_len: u32) {
        let used_idx = mem.read_u16(self.queue.device_addr + 2);
        let slot = (used_idx % self.queue.size) as u64;
        let elem = self.queue.device_addr + 4 + slot * 8;
        mem.write_u32(elem, head as u32);
        mem.write_u32(elem + 4, used_len);
        mem.write_u16(self.queue.device_addr + 2, used_idx.wrapping_add(1));
        self.isr_status = 1;
    }

    fn read_desc(&self, mem: &GuestMemory, idx: u16) -> VirtqDesc {
        let base = self.queue.desc_addr + (idx as u64) * 16;
        VirtqDesc {
            addr: mem.read_u64(base),
            len: mem.read_u32(base + 8),
            flags: mem.read_u16(base + 12),
            next: mem.read_u16(base + 14),
        }
    }

    fn device_feature(&self, select: u32) -> u32 {
        match select {
            0 => 0,
            1 => 0,
            _ => 0,
        }
    }

    fn write_device_status(&mut self, new_status: u8) {
        if new_status == 0 {
            self.reset();
            return;
        }

        let mut status = new_status;

        if (new_status & VIRTIO_STATUS_FEATURES_OK) != 0 {
            let accepted = self.driver_features_accepted();
            if !accepted {
                status &= !VIRTIO_STATUS_FEATURES_OK;
                status |= VIRTIO_STATUS_FAILED;
            }
        }

        self.common.device_status = status;
    }

    fn driver_features_accepted(&self) -> bool {
        self.common.driver_features[0] == 0 && self.common.driver_features[1] == 0
    }
}

impl VirtioInputState {
    fn new() -> Self {
        let mut q0 = VirtioQueueState::default();
        let mut q1 = VirtioQueueState::default();
        q0.notify_off = 0;
        q1.notify_off = 1;

        Self {
            common: VirtioCommonCfgState {
                config_generation: 1,
                ..VirtioCommonCfgState::default()
            },
            queues: [q0, q1],
            isr_status: 0,
            config_select: VIRTIO_INPUT_CFG_UNSET,
            config_subsel: 0,
        }
    }

    fn reset(&mut self) {
        let mut q0 = VirtioQueueState::default();
        let mut q1 = VirtioQueueState::default();
        q0.notify_off = 0;
        q1.notify_off = 1;

        self.common = VirtioCommonCfgState {
            config_generation: self.common.config_generation.wrapping_add(1),
            ..VirtioCommonCfgState::default()
        };
        self.queues = [q0, q1];
        self.isr_status = 0;
        self.config_select = VIRTIO_INPUT_CFG_UNSET;
        self.config_subsel = 0;
    }

    fn selected_queue_index(&self) -> usize {
        (self.common.queue_select as usize).min(self.queues.len() - 1)
    }

    fn selected_queue(&self) -> &VirtioQueueState {
        &self.queues[self.selected_queue_index()]
    }

    fn selected_queue_mut(&mut self) -> &mut VirtioQueueState {
        let idx = self.selected_queue_index();
        &mut self.queues[idx]
    }

    fn read_common_cfg(&mut self, off: u32, data: &mut [u8]) {
        let q = self.selected_queue();

        let value = match off {
            0x00 => self.common.device_feature_select as u64,
            0x04 => self.device_feature(self.common.device_feature_select) as u64,
            0x08 => self.common.driver_feature_select as u64,
            0x0c => self.common.driver_features[self.common.driver_feature_select.min(1) as usize]
                as u64,
            0x10 => self.common.msix_config as u64,
            0x12 => NUM_QUEUES_INPUT as u64,
            0x14 => self.common.device_status as u64,
            0x15 => self.common.config_generation as u64,
            0x16 => self.common.queue_select as u64,
            0x18 => q.size as u64,
            0x1a => q.msix_vector as u64,
            0x1c => q.enable as u64,
            0x1e => q.notify_off as u64,
            0x20 => (q.desc_addr & 0xffff_ffff) as u64,
            0x24 => (q.desc_addr >> 32) as u64,
            0x28 => (q.driver_addr & 0xffff_ffff) as u64,
            0x2c => (q.driver_addr >> 32) as u64,
            0x30 => (q.device_addr & 0xffff_ffff) as u64,
            0x34 => (q.device_addr >> 32) as u64,
            _ => 0,
        };

        write_value_to_buf(value, data);
    }

    fn write_common_cfg(&mut self, off: u32, data: &[u8]) {
        let value = read_value_from_buf(data);

        match off {
            0x00 => self.common.device_feature_select = value as u32,
            0x08 => self.common.driver_feature_select = value as u32,
            0x0c => {
                let idx = self.common.driver_feature_select.min(1) as usize;
                self.common.driver_features[idx] = value as u32;
            }
            0x10 => self.common.msix_config = value as u16,
            0x14 => self.write_device_status(value as u8),
            0x16 => self.common.queue_select = value as u16,
            0x1a => self.selected_queue_mut().msix_vector = value as u16,
            0x1c => self.selected_queue_mut().enable = value as u16,
            0x20 => {
                let q = self.selected_queue_mut();
                q.desc_addr = (q.desc_addr & 0xffff_ffff_0000_0000) | (value as u32 as u64);
            }
            0x24 => {
                let q = self.selected_queue_mut();
                q.desc_addr = (q.desc_addr & 0x0000_0000_ffff_ffff) | ((value as u32 as u64) << 32);
            }
            0x28 => {
                let q = self.selected_queue_mut();
                q.driver_addr = (q.driver_addr & 0xffff_ffff_0000_0000) | (value as u32 as u64);
            }
            0x2c => {
                let q = self.selected_queue_mut();
                q.driver_addr =
                    (q.driver_addr & 0x0000_0000_ffff_ffff) | ((value as u32 as u64) << 32);
            }
            0x30 => {
                let q = self.selected_queue_mut();
                q.device_addr = (q.device_addr & 0xffff_ffff_0000_0000) | (value as u32 as u64);
            }
            0x34 => {
                let q = self.selected_queue_mut();
                q.device_addr =
                    (q.device_addr & 0x0000_0000_ffff_ffff) | ((value as u32 as u64) << 32);
            }
            _ => {}
        }
    }

    fn read_device_cfg(&self, off: u32, data: &mut [u8]) {
        let mut cfg = [0u8; 136];

        cfg[0] = self.config_select;
        cfg[1] = self.config_subsel;

        let payload = self.build_config_payload();
        cfg[2] = payload.len() as u8;
        cfg[8..8 + payload.len()].copy_from_slice(&payload);

        let start = off as usize;
        if start >= cfg.len() {
            data.fill(0);
            return;
        }

        let n = data.len().min(cfg.len() - start);
        data[..n].copy_from_slice(&cfg[start..start + n]);
        if n < data.len() {
            data[n..].fill(0);
        }
    }

    fn write_device_cfg(&mut self, off: u32, data: &[u8]) {
        let value = read_value_from_buf(data) as u8;

        match off {
            0x00 => self.config_select = value,
            0x01 => self.config_subsel = value,
            _ => {}
        }
    }

    fn build_config_payload(&self) -> Vec<u8> {
        match self.config_select {
            VIRTIO_INPUT_CFG_ID_NAME => b"vmmon virtio input".to_vec(),
            VIRTIO_INPUT_CFG_ID_SERIAL => b"vmmon-virtio-input".to_vec(),
            VIRTIO_INPUT_CFG_ID_DEVIDS => {
                let mut out = Vec::with_capacity(8);
                out.extend_from_slice(&0x0006u16.to_le_bytes());
                out.extend_from_slice(&0x1af4u16.to_le_bytes());
                out.extend_from_slice(&0x1052u16.to_le_bytes());
                out.extend_from_slice(&0x0001u16.to_le_bytes());
                out
            }
            VIRTIO_INPUT_CFG_PROP_BITS => Vec::new(),
            VIRTIO_INPUT_CFG_EV_BITS => self.build_ev_bits_payload(),
            VIRTIO_INPUT_CFG_ABS_INFO => Vec::new(),
            _ => Vec::new(),
        }
    }

    fn build_ev_bits_payload(&self) -> Vec<u8> {
        match self.config_subsel as usize {
            0 => {
                let mut bits = vec![0u8; 1];
                set_bit(&mut bits, EV_SYN);
                set_bit(&mut bits, EV_KEY);
                set_bit(&mut bits, EV_REL);
                bits
            }
            EV_KEY => {
                let mut bits = vec![0u8; (BTN_MIDDLE / 8) + 1];
                set_bit(&mut bits, BTN_LEFT);
                set_bit(&mut bits, BTN_RIGHT);
                set_bit(&mut bits, BTN_MIDDLE);
                trim_trailing_zeros(bits)
            }
            EV_REL => {
                let mut bits = vec![0u8; (REL_WHEEL / 8) + 1];
                set_bit(&mut bits, REL_X);
                set_bit(&mut bits, REL_Y);
                set_bit(&mut bits, REL_WHEEL);
                trim_trailing_zeros(bits)
            }
            _ => Vec::new(),
        }
    }

    fn write_notify(&mut self, mem: &mut GuestMemory, off: u32, _data: &[u8]) {
        let queue_idx = ((off / 4) as usize).min(self.queues.len() - 1);
        let _ = self.process_available(mem, queue_idx);
    }

    fn process_available(&mut self, mem: &mut GuestMemory, queue_idx: usize) -> std::io::Result<()> {
        {
            let q = &self.queues[queue_idx];
            if q.enable == 0 || q.desc_addr == 0 || q.driver_addr == 0 || q.device_addr == 0 {
                return Ok(());
            }
        }

        loop {
            let (avail_idx, last_avail_idx, size, driver_addr) = {
                let q = &self.queues[queue_idx];
                (
                    mem.read_u16(q.driver_addr + 2),
                    q.last_avail_idx,
                    q.size,
                    q.driver_addr,
                )
            };

            if last_avail_idx == avail_idx {
                break;
            }

            let ring_slot = (last_avail_idx % size) as u64;
            let head = mem.read_u16(driver_addr + 4 + ring_slot * 2);

            match queue_idx {
                0 => {
                    break;
                }
                1 => {
                    self.push_used_for_queue(mem, queue_idx, head, 0);
                    let q = &mut self.queues[queue_idx];
                    q.last_avail_idx = q.last_avail_idx.wrapping_add(1);
                }
                _ => break,
            }
        }

        Ok(())
    }

    fn push_used_for_queue(&mut self, mem: &mut GuestMemory, queue_idx: usize, head: u16, used_len: u32) {
        let q = &mut self.queues[queue_idx];
        let used_idx = mem.read_u16(q.device_addr + 2);
        let slot = (used_idx % q.size) as u64;
        let elem = q.device_addr + 4 + slot * 8;
        mem.write_u32(elem, head as u32);
        mem.write_u32(elem + 4, used_len);
        mem.write_u16(q.device_addr + 2, used_idx.wrapping_add(1));
        self.isr_status = 1;
    }

    fn device_feature(&self, select: u32) -> u32 {
        match select {
            0 => 0,
            1 => 0,
            _ => 0,
        }
    }

    fn write_device_status(&mut self, new_status: u8) {
        if new_status == 0 {
            self.reset();
            return;
        }

        let mut status = new_status;

        if (new_status & VIRTIO_STATUS_FEATURES_OK) != 0 {
            let accepted = self.driver_features_accepted();
            if !accepted {
                status &= !VIRTIO_STATUS_FEATURES_OK;
                status |= VIRTIO_STATUS_FAILED;
            }
        }

        self.common.device_status = status;
    }

    fn driver_features_accepted(&self) -> bool {
        self.common.driver_features[0] == 0 && self.common.driver_features[1] == 0
    }
}

fn set_bit(bits: &mut [u8], bit: usize) {
    let byte = bit / 8;
    let mask = 1u8 << (bit % 8);
    if byte < bits.len() {
        bits[byte] |= mask;
    }
}

fn trim_trailing_zeros(mut v: Vec<u8>) -> Vec<u8> {
    while matches!(v.last(), Some(0)) {
        v.pop();
    }
    v
}

fn write_value_to_buf(value: u64, data: &mut [u8]) {
    let bytes = value.to_le_bytes();
    let n = data.len().min(bytes.len());
    data[..n].copy_from_slice(&bytes[..n]);
}

fn read_value_from_buf(data: &[u8]) -> u64 {
    let mut bytes = [0u8; 8];
    let n = data.len().min(bytes.len());
    bytes[..n].copy_from_slice(&data[..n]);
    u64::from_le_bytes(bytes)
}

fn read_u16(buf: &[u8], off: usize) -> Option<u16> {
    let bytes = buf.get(off..off + 2)?;
    Some(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn read_u32(buf: &[u8], off: usize) -> Option<u32> {
    let bytes = buf.get(off..off + 4)?;
    Some(u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn write_u16(buf: &mut [u8], off: usize, v: u16) {
    let b = v.to_le_bytes();
    buf[off..off + 2].copy_from_slice(&b);
}

fn write_u32(buf: &mut [u8], off: usize, v: u32) {
    let b = v.to_le_bytes();
    buf[off..off + 4].copy_from_slice(&b);
}
