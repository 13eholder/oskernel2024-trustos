use crate::{
    fs::{Kstat, SEEK_CUR, SEEK_END, SEEK_SET},
    mm::UserBuffer,
    syscall::PollEvents,
    utils::{SysErrNo, SyscallRet},
};

use super::{File, Inode};
use alloc::sync::Arc;
use spin::Mutex;

pub struct OSInode {
    readable: bool, // 该文件是否允许通过 sys_read 进行读
    writable: bool, // 该文件是否允许通过 sys_write 进行写
    pub inode: Arc<dyn Inode>,
    pub(crate) inner: Mutex<OSInodeInner>,
}
pub struct OSInodeInner {
    pub(crate) offset: usize, // 偏移量
}

impl OSInode {
    pub fn new(readable: bool, writable: bool, inode: Arc<dyn Inode>) -> Self {
        Self {
            readable,
            writable,
            inode,
            inner: Mutex::new(OSInodeInner { offset: 0 }),
        }
    }
}

// 为 OSInode 实现 File Trait
impl File for OSInode {
    fn readable(&self) -> bool {
        self.readable
    }

    fn writable(&self) -> bool {
        self.writable
    }

    fn read(&self, mut buf: UserBuffer) -> SyscallRet {
        let mut inner = self.inner.lock();
        let mut total_read_size = 0usize;

        if self.inode.size() <= inner.offset {
            //读取位置超过文件大小，返回结果为EOF
            return Ok(0);
        }

        // 这边要使用 iter_mut()，因为要将数据写入
        for slice in buf.buffers.iter_mut() {
            let read_size = self.inode.read_at(inner.offset, *slice)?;
            if read_size == 0 {
                break;
            }
            inner.offset += read_size;
            total_read_size += read_size;
        }
        Ok(total_read_size)
    }

    fn write(&self, buf: UserBuffer) -> SyscallRet {
        let mut inner = self.inner.lock();
        let mut total_write_size = 0usize;
        for slice in buf.buffers.iter() {
            let write_size = self.inode.write_at(inner.offset, *slice)?;
            assert_eq!(write_size, slice.len());
            inner.offset += write_size;
            total_write_size += write_size;
        }
        Ok(total_write_size)
    }

    fn fstat(&self) -> Kstat {
        self.inode.fstat()
    }

    fn poll(&self, events: PollEvents) -> PollEvents {
        let mut revents = PollEvents::empty();
        if events.contains(PollEvents::IN) && self.readable {
            revents |= PollEvents::IN;
        }
        if events.contains(PollEvents::OUT) && self.writable {
            revents |= PollEvents::OUT;
        }
        revents
    }
    fn lseek(&self, offset: isize, whence: usize) -> SyscallRet {
        if whence > 2 {
            return Err(SysErrNo::EINVAL);
        }
        let mut inner = self.inner.lock();
        if whence == SEEK_SET {
            inner.offset = offset as usize;
        } else if whence == SEEK_CUR {
            let newoff = inner.offset as isize + offset;
            if newoff < 0 {
                return Err(SysErrNo::EINVAL);
            }
            inner.offset = newoff as usize;
        } else if whence == SEEK_END {
            let newoff = self.inode.size() as isize + offset;
            if newoff < 0 {
                return Err(SysErrNo::EINVAL);
            }
            inner.offset = newoff as usize;
        }
        Ok(inner.offset)
    }
}
