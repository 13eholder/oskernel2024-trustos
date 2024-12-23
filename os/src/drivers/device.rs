/// All supported device types.
#[allow(unused)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum DeviceType {
    /// Block storage device (e.g., disk).
    Block,
    /// Character device (e.g., serial port).
    Char,
    /// Network device (e.g., ethernet card).
    Net,
    /// Graphic display device (e.g., GPU)
    Display,
}

pub trait BaseDriver: Send + Sync {
    /// The name of the device.
    fn device_name(&self) -> &str;

    /// The type of the device.
    fn device_type(&self) -> DeviceType;
}

/// Operations that require a block storage device driver to implement.
pub trait BlockDriver: BaseDriver {
    /// The number of blocks in this storage device.
    ///
    /// The total size of the device is `num_blocks() * block_size()`.
    fn num_blocks(&self) -> usize;
    /// The size of each block in bytes.
    fn block_size(&self) -> usize;

    /// Reads blocked data from the given block.
    ///
    /// The size of the buffer may exceed the block size, in which case multiple
    /// contiguous blocks will be read.
    fn read_block(&mut self, block_id: usize, buf: &mut [u8]);

    /// Writes blocked data to the given block.
    ///
    /// The size of the buffer may exceed the block size, in which case multiple
    /// contiguous blocks will be written.
    fn write_block(&mut self, block_id: usize, buf: &[u8]);

    /// Flushes the device to write all pending data to the storage.
    fn flush(&mut self);
}
