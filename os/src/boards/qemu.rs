use crate::config::mm::KERNEL_ADDR_OFFSET;
pub const CLOCK_FREQ: usize = 12500000;
pub const MEMORY_END: usize = 0x8800_0000 + KERNEL_ADDR_OFFSET;

pub const MMIO: &[(usize, usize)] = &[
    (0x0010_0000, 0x00_1000), // VIRT_TEST in virt machine
    (0x0010_1000, 0x00_1000), // VIRT_RTC in virt machine
    (0x1000_1000, 0x00_1000), // Virtio Block in virt machine
];
