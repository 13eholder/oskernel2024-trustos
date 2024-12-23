//! memory related syscall

use log::debug;

use super::{MmapFlags, MmapProt};
use crate::{
    config::mm::PAGE_SIZE,
    fs::{File, OpenFlags},
    mm::{
        is_bad_address, insert_bad_address, remove_bad_address, shm_attach, shm_create, shm_drop,
        shm_find, MapPermission, ShmFlags, VirtAddr,
    },
    task::current_task,
    utils::{page_round_up, SysErrNo, SyscallRet},
};

/// 参考 https://man7.org/linux/man-pages/man2/mmap.2.html
pub fn sys_mmap(
    addr: usize,
    len: usize,
    prot: u32,
    flags: u32,
    fd: usize,
    off: usize,
) -> SyscallRet {
    //需要有严格的判断返回错误的顺序！！！
    if flags == 0 {
        return Err(SysErrNo::EINVAL);
    }

    let flags = MmapFlags::from_bits(flags).unwrap();
    if fd == usize::MAX && !flags.contains(MmapFlags::MAP_ANONYMOUS) {
        return Err(SysErrNo::EBADF);
    }

    if len == 0 {
        return Err(SysErrNo::EINVAL);
    }
    let mmap_prot = MmapProt::from_bits(prot).unwrap();
    let map_perm: MapPermission = mmap_prot.into();
    // 地址合法性
    if flags.contains(MmapFlags::MAP_FIXED) && addr == 0 {
        return Err(SysErrNo::EPERM);
    }
    debug!(
        "[sys_mmap]: addr {:#x}, len {:#x}, fd {}, offset {:#x}, flags {:?}, prot is {:?}, map_perm {:?}",
        addr, len, fd as isize, off, flags,mmap_prot, map_perm
    );
    let task = current_task().unwrap();
    let task_inner = task.inner_lock();
    let len = page_round_up(len);
    if fd == usize::MAX {
        let rv = task_inner
            .memory_set
            .mmap(addr, len, map_perm, flags, None, usize::MAX);
        return Ok(rv);
    }
    if flags.contains(MmapFlags::MAP_ANONYMOUS) {
        //映射1字节没有任何权限的地址
        let rv = task_inner
            .memory_set
            .mmap(0, 1, MapPermission::empty(), flags, None, usize::MAX);
        insert_bad_address(rv);
        debug!("bad address is 0x{:x}", rv);
        return Ok(rv);
    }
    // check fd and map_permission
    let inode = task_inner.fd_table.get(fd);
    let file = inode.file()?;
    // 读写权限
    if (map_perm.contains(MapPermission::R) && !file.readable())
        || (map_perm.contains(MapPermission::W) && !file.writable())
        || (mmap_prot != MmapProt::PROT_NONE && inode.flags.contains(OpenFlags::O_WRONLY))
    {
        //如果需要读/写/执行方式映射，必须要求文件可读
        return Err(SysErrNo::EACCES);
    }
    let rv = task_inner
        .memory_set
        .mmap(addr, len, map_perm, flags, Some(file), off);
    // debug!("[sys_mmap] alloc addr={:#x}", rv);
    Ok(rv)
}

/// 参考 https://man7.org/linux/man-pages/man2/munmap.2.html
pub fn sys_munmap(addr: usize, len: usize) -> SyscallRet {
    debug!("[sys_munmap] addr={:#x}, len={:#x}", addr, len);
    let task = current_task().unwrap();
    let task_inner = task.inner_lock();
    let len = page_round_up(len);
    if is_bad_address(addr) {
        remove_bad_address(addr);
    }
    task_inner.memory_set.munmap(addr, len)
}

/// 参考 https://man7.org/linux/man-pages/man2/mprotect.2.html
pub fn sys_mprotect(addr: usize, len: usize, prot: u32) -> SyscallRet {
    if addr == 0 {
        return Err(SysErrNo::ENOMEM);
    }

    if (addr % PAGE_SIZE != 0) || (len % PAGE_SIZE != 0) {
        log::warn!("sys_mprotect: not align");
        return Err(SysErrNo::EINVAL);
    }
    let map_perm: MapPermission = MmapProt::from_bits(prot).unwrap().into();

    debug!(
        "[sys_mprotect] addr is {:x}, len is {:#x}, map_perm is {:?}",
        addr, len, map_perm
    );

    let task = current_task().unwrap();
    let mut inner = task.inner_lock();
    let memory_set = &mut inner.memory_set;
    let start_vpn = VirtAddr::from(addr).floor();
    let end_vpn = VirtAddr::from(addr + len).ceil();
    //修改各段的mappermission
    memory_set.mprotect(start_vpn, end_vpn, map_perm);
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/madvise.2.html
pub fn sys_madvise(_addr: usize, _len: usize, _advice: usize) -> SyscallRet {
    //伪实现，该系统调用用于给内存提建议
    // debug!(
    //     "[sys_madvise] addr is {}, len is {}, advice is {}",
    //     addr, len, advice
    // );
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/shmget.2.html
pub fn sys_shmget(key: i32, size: usize, shmflag: i32) -> SyscallRet {
    const IPC_PRIVATE: i32 = 0;
    // 忽略权限位
    let flags = ShmFlags::from_bits(shmflag & !0x1ff).unwrap();
    match key {
        IPC_PRIVATE => Ok(shm_create(size)),
        key if key > 0 => {
            if shm_find(key as usize) {
                if flags.contains(ShmFlags::IPC_CREAT | ShmFlags::IPC_EXCL) {
                    Err(SysErrNo::EEXIST)
                } else {
                    Ok(key as usize)
                }
            } else {
                if flags.contains(ShmFlags::IPC_CREAT) {
                    Ok(shm_create(size))
                } else {
                    Err(SysErrNo::ENOENT)
                }
            }
        }
        _ => Err(SysErrNo::ENOENT),
    }
}

/// 参考 https://man7.org/linux/man-pages/man2/shmat.2.html
pub fn sys_shmat(shmid: i32, shmaddr: usize, shmflag: i32) -> SyscallRet {
    let mut permission = MapPermission::U | MapPermission::R;
    if shmflag == 0 {
        permission |= MapPermission::W | MapPermission::X
    } else {
        let shmflg = ShmFlags::from_bits(shmflag).unwrap();
        if shmflg.contains(ShmFlags::SHM_EXEC) {
            permission |= MapPermission::X;
        }
        if !shmflg.contains(ShmFlags::SHM_RDONLY) {
            permission |= MapPermission::W;
        }
    }

    match shmid {
        key if key < 0 => Err(SysErrNo::EINVAL),
        _ => shm_attach(shmid as usize, shmaddr, permission),
    }
}

/// 参考 https://man7.org/linux/man-pages/man2/shmctl.2.html
pub fn sys_shmctl(shmid: i32, cmd: i32, _buf: usize) -> SyscallRet {
    const IPC_RMID: i32 = 0;
    match cmd {
        IPC_RMID => {
            shm_drop(shmid as usize);
            Ok(0)
        }
        _ => {
            panic!("[sys_shmctl] unsupport cmd");
        }
    }
}
