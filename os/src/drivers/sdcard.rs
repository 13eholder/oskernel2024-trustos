use spin::Mutex;
use visionfive2_sd::{SDIo, SleepOps, Vf2SdDriver};

use crate::{
    boards::CLOCK_FREQ,
    config::mm::KERNEL_ADDR_OFFSET,
    drivers::{BaseDriver, BlockDriver, DeviceType},
    timer::get_time,
};

pub struct SdIoImpl;
pub const SDIO_BASE: usize = 0x16020000;
pub const KERNEL_SDIO_BASE: usize = SDIO_BASE + KERNEL_ADDR_OFFSET;

impl SDIo for SdIoImpl {
    fn read_reg_at(&self, offset: usize) -> u32 {
        let addr = (KERNEL_SDIO_BASE + offset) as *mut u32;
        unsafe { addr.read_volatile() }
    }
    fn write_reg_at(&mut self, offset: usize, val: u32) {
        let addr = (KERNEL_SDIO_BASE + offset) as *mut u32;
        unsafe { addr.write_volatile(val) }
    }
    fn read_data_at(&self, offset: usize) -> u64 {
        let addr = (KERNEL_SDIO_BASE + offset) as *mut u64;
        unsafe { addr.read_volatile() }
    }
    fn write_data_at(&mut self, offset: usize, val: u64) {
        let addr = (KERNEL_SDIO_BASE + offset) as *mut u64;
        unsafe { addr.write_volatile(val) }
    }
}

pub struct SleepOpsImpl;

impl SleepOps for SleepOpsImpl {
    fn sleep_ms(ms: usize) {
        sleep_ms(ms)
    }
    fn sleep_ms_until(ms: usize, f: impl FnMut() -> bool) {
        sleep_ms_until(ms, f)
    }
}

fn sleep_ms(ms: usize) {
    let start = get_time();
    while get_time() - start < ms * CLOCK_FREQ / 1000 {
        core::hint::spin_loop();
    }
}

fn sleep_ms_until(ms: usize, mut f: impl FnMut() -> bool) {
    let start = get_time();
    while get_time() - start < ms * CLOCK_FREQ / 1000 {
        if f() {
            return;
        }
        core::hint::spin_loop();
    }
}

pub struct Vf2BlkDev(Mutex<Vf2SdDriver<SdIoImpl, SleepOpsImpl>>);

impl Vf2BlkDev {
    pub fn new_device() -> Self {
        let mut vf2_driver = Vf2SdDriver::new(SdIoImpl);
        vf2_driver.init();
        Vf2BlkDev(Mutex::new(vf2_driver))
        // Vf2BlkDev(Mutex::new(Vf2SdDriver::new(SdIoImpl)))
    }
}

impl BaseDriver for Vf2BlkDev {
    fn device_name(&self) -> &str {
        "visionfive2-sdcard"
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Block
    }
}

impl BlockDriver for Vf2BlkDev {
    fn block_size(&self) -> usize {
        1024
    }

    fn num_blocks(&self) -> usize {
        // sdcard 32GB磁盘空间
        32 * 1024 * 1024 * 1024 / self.block_size()
    }

    fn read_block(&mut self, block_id: usize, buf: &mut [u8]) {
        self.0.lock().read_block(block_id, buf);
    }

    fn write_block(&mut self, block_id: usize, buf: &[u8]) {
        self.0.lock().write_block(block_id, buf);
    }

    fn flush(&mut self) {}
}
