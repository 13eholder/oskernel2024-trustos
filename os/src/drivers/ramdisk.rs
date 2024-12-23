use spin::Mutex;

use crate::drivers::{BaseDriver, BlockDriver, DeviceType};

struct MemBlock(usize);

impl MemBlock {
    const BLOCK_SIZE: usize = 512;
    pub fn block_ref(&self, block_id: usize, len: usize) -> &[u8] {
        unsafe {
            core::slice::from_raw_parts((self.0 + block_id * Self::BLOCK_SIZE) as *const u8, len)
        }
    }
    pub fn block_refmut(&self, block_id: usize, len: usize) -> &mut [u8] {
        unsafe {
            core::slice::from_raw_parts_mut((self.0 + block_id * Self::BLOCK_SIZE) as *mut u8, len)
        }
    }
}

extern "C" {
    fn sd_start();
    fn sd_end();
}

pub struct MemBlockWrapper(Mutex<MemBlock>);

impl MemBlockWrapper {
    pub fn new_device() -> Self {
        MemBlockWrapper(Mutex::new(MemBlock(sd_start as usize)))
    }
}

impl BaseDriver for MemBlockWrapper {
    fn device_name(&self) -> &str {
        "mem_block"
    }

    fn device_type(&self) -> DeviceType {
        DeviceType::Block
    }
}

impl BlockDriver for MemBlockWrapper {
    fn block_size(&self) -> usize {
        512
    }

    fn num_blocks(&self) -> usize {
        (sd_end as usize - sd_start as usize) / 512
    }

    fn read_block(&mut self, block_id: usize, buf: &mut [u8]) {
        let blk = self.0.lock();
        buf.copy_from_slice(blk.block_ref(block_id, buf.len()));
    }

    fn write_block(&mut self, block_id: usize, buf: &[u8]) {
        let blk = self.0.lock();
        blk.block_refmut(block_id, buf.len()).copy_from_slice(buf);
    }

    fn flush(&mut self) {}
}
