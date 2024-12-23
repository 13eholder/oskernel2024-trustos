use super::sync::THREAD_MAX_NUM;

pub const USER_STACK_SIZE: usize = 1024 * 1024 * 8; // 8MB
pub const KERNEL_STACK_SIZE: usize = 4096 * 2;
#[cfg(feature = "board_qemu")]
pub const KERNEL_HEAP_SIZE: usize = 0x3_000_000; // 48MB
#[cfg(not(feature = "board_qemu"))]
pub const KERNEL_HEAP_SIZE: usize = 0x20_000_000; // 512MB
pub const USER_HEAP_SIZE: usize = 0x10_000_000;
pub const PRE_ALLOC_PAGES: usize = 8;

pub const PAGE_SIZE: usize = 0x1000;
pub const PAGE_SIZE_BITS: usize = 0xc;

pub const KERNEL_ADDR_OFFSET: usize = 0xffff_ffc0_0000_0000;

/// When directly map: vpn = ppn + kernel direct offset
pub const KERNEL_PGNUM_OFFSET: usize = KERNEL_ADDR_OFFSET >> PAGE_SIZE_BITS;

pub const USER_SPACE_SIZE: usize = 0x30_0000_0000;

/// User Space layout
/// TrapContext GuardPage Stack GuardPage Mmap Heap Elf
pub const USER_TRAP_CONTEXT_TOP: usize = USER_SPACE_SIZE;
pub const USER_STACK_TOP: usize = USER_TRAP_CONTEXT_TOP - PAGE_SIZE * THREAD_MAX_NUM;
pub const MMAP_TOP: usize = USER_TRAP_CONTEXT_TOP
    - PAGE_SIZE * THREAD_MAX_NUM
    - USER_STACK_SIZE * THREAD_MAX_NUM
    - PAGE_SIZE;

/// Dynamic linked interpreter address range in user space
pub const DL_INTERP_OFFSET: usize = 0x15_0000_0000;
/// Kernel Stack Start
pub const KSTACK_TOP: usize = usize::MAX - PAGE_SIZE + 1;

/// boot
pub const HART_START_ADDR: usize = 0x80200000;
