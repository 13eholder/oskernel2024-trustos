mod inode;
mod sb;

pub use inode::*;
pub use sb::*;

use alloc::sync::Arc;
use spin::Lazy;

use crate::{
    drivers::{BlockDeviceImpl, Disk},
    fs::SuperBlock,
};

use super::{Inode, Statfs};

static SUPER_BLOCK: Lazy<Arc<dyn SuperBlock>> = Lazy::new(|| {
    Arc::new(Ext4SuperBlock::new(
        Disk::new(BlockDeviceImpl::new_device()),
    ))
});

pub fn root_inode() -> Arc<dyn Inode> {
    SUPER_BLOCK.root_inode()
}

pub fn sync() {
    SUPER_BLOCK.sync()
}

pub fn fs_stat() -> Statfs {
    SUPER_BLOCK.fs_stat()
}

pub fn ls() {
    SUPER_BLOCK.ls()
}
