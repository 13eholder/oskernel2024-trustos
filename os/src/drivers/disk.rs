use log::info;

use crate::drivers::BlockDriver;

use super::BlockDeviceImpl;

#[cfg(feature = "board_qemu")]
const BLOCK_SIZE: usize = 512;

#[cfg(feature = "board_vf2")]
const BLOCK_SIZE: usize = 1024;

#[cfg(feature = "board_ramdisk")]
const BLOCK_SIZE: usize = 512;

/// A disk device with a cursor.
pub struct Disk {
    block_id: usize,
    offset: usize,
    dev: BlockDeviceImpl,
}

impl Disk {
    /// Create a new disk.
    pub fn new(dev: BlockDeviceImpl) -> Self {
        assert_eq!(BLOCK_SIZE, dev.block_size());
        Self {
            block_id: 0,
            offset: 0,
            dev,
        }
    }

    /// Get the size of the disk.
    pub fn size(&self) -> usize {
        self.dev.num_blocks() * BLOCK_SIZE
    }

    /// Get the position of the cursor.
    pub fn position(&self) -> usize {
        self.block_id * BLOCK_SIZE + self.offset
    }

    /// Set the position of the cursor.
    pub fn set_position(&mut self, pos: usize) {
        self.block_id = pos / BLOCK_SIZE;
        self.offset = pos as usize % BLOCK_SIZE;
    }

    /// Read within one block, returns the number of bytes read.
    pub fn read_one(&mut self, buf: &mut [u8]) -> usize {
        // info!("block id: {}", self.block_id);
        let read_size = if self.offset == 0 && buf.len() >= BLOCK_SIZE {
            // whole block
            self.dev.read_block(self.block_id, &mut buf[0..BLOCK_SIZE]);
            self.block_id += 1;
            BLOCK_SIZE
        } else {
            // partial block
            let mut data = [0u8; BLOCK_SIZE];
            let start = self.offset;
            let count = buf.len().min(BLOCK_SIZE - self.offset);
            if start > BLOCK_SIZE {
                info!("block size: {} start {}", BLOCK_SIZE, start);
            }

            self.dev.read_block(self.block_id, &mut data);
            buf[..count].copy_from_slice(&data[start..start + count]);

            self.offset += count;
            if self.offset >= BLOCK_SIZE {
                self.block_id += 1;
                self.offset -= BLOCK_SIZE;
            }
            count
        };
        read_size
    }

    /// Write within one block, returns the number of bytes written.
    pub fn write_one(&mut self, buf: &[u8]) -> usize {
        let write_size = if self.offset == 0 && buf.len() >= BLOCK_SIZE {
            // whole block
            self.dev.write_block(self.block_id, &buf[0..BLOCK_SIZE]);
            self.block_id += 1;
            BLOCK_SIZE
        } else {
            // partial block
            let mut data = [0u8; BLOCK_SIZE];
            let start = self.offset;
            let count = buf.len().min(BLOCK_SIZE - self.offset);

            self.dev.read_block(self.block_id, &mut data);
            data[start..start + count].copy_from_slice(&buf[..count]);
            self.dev.write_block(self.block_id, &data);

            self.offset += count;
            if self.offset >= BLOCK_SIZE {
                self.block_id += 1;
                self.offset -= BLOCK_SIZE;
            }
            count
        };
        write_size
    }

    /// Read a single block starting from the specified offset.
    #[allow(unused)]
    pub fn read_offset(&mut self, offset: usize) -> [u8; BLOCK_SIZE] {
        let block_id = offset / BLOCK_SIZE;
        let mut block_data = [0u8; BLOCK_SIZE];
        self.dev.read_block(block_id, &mut block_data);
        block_data
    }

    /// Write single block starting from the specified offset.
    #[allow(unused)]
    pub fn write_offset(&mut self, offset: usize, buf: &[u8]) -> usize {
        assert!(
            buf.len() == BLOCK_SIZE,
            "Buffer length must be equal to BLOCK_SIZE"
        );
        assert!(offset % BLOCK_SIZE == 0);
        let block_id = offset / BLOCK_SIZE;
        self.dev.write_block(block_id, buf);
        buf.len()
    }
}
