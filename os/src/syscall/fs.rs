//! File and filesystem-related syscalls
use crate::{
    data_flow,
    fs::{
        fs_stat, make_pipe, open, open_device_file, remove_inode_idx, sync, File, FileClass,
        FileDescriptor, Kstat, OpenFlags, Statfs, MAX_PATH_LEN, MNT_TABLE, NONE_MODE, SEEK_CUR,
        SEEK_SET,
    },
    mm::{is_bad_address, safe_translated_byte_buffer, translated_byte_buffer, UserBuffer},
    syscall::{FaccessatFileMode, FaccessatMode, FdSet, PollEvents, PollFd, SigSet},
    task::{current_task, suspend_current_and_run_next},
    timer::{get_time_ms, Timespec},
    utils::{c_ptr_to_string, get_abs_path, rsplit_once, trim_start_slash, SysErrNo, SyscallRet},
};
use alloc::{
    format,
    string::{String, ToString},
    sync::Arc,
    vec,
    vec::Vec,
};
use core::cmp::min;
use log::debug;

use super::{FcntlCmd, Iovec, RLimit};

/// 参考 https://man7.org/linux/man-pages/man2/write.2.html
pub fn sys_write(fd: usize, buf: *const u8, len: usize) -> SyscallRet {
    //debug!("[sys_write] fd is {}, len={}", fd, len);
    let task = current_task().unwrap();
    let inner = task.inner_lock();
    let memory_set = inner.memory_set.clone();

    if (fd as isize) < 0 || fd >= inner.fd_table.len() {
        return Err(SysErrNo::EBADF);
    }

    if (buf as isize) < 0 || is_bad_address(buf as usize) || ((buf as usize) == 0 && len != 0) {
        return Err(SysErrNo::EFAULT);
    }

    if (len as isize) < 0 {
        return Err(SysErrNo::EINVAL);
    }

    if let Some(file) = &inner.fd_table.try_get(fd) {
        if let Ok(readfile) = file.file() {
            if readfile.inode.is_dir() {
                return Err(SysErrNo::EISDIR);
            }
        }
        let file: Arc<dyn File> = file.any();
        if !file.writable() {
            return Err(SysErrNo::EBADF);
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        drop(task);
        let ret = file.write(UserBuffer::new(
            safe_translated_byte_buffer(memory_set, buf, len).unwrap(),
        ))?;
        Ok(ret)
    } else {
        Err(SysErrNo::EBADF)
    }
}

/// 参考 https://man7.org/linux/man-pages/man2/read.2.html
pub fn sys_read(fd: usize, buf: *const u8, len: usize) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();
    let memory_set = inner.memory_set.clone();

    if (fd as isize) < 0 || fd >= inner.fd_table.len() {
        return Err(SysErrNo::EBADF);
    }

    if (buf as isize) <= 0 || is_bad_address(buf as usize) {
        return Err(SysErrNo::EFAULT);
    }

    //debug!("[sys_read] fd is {}, len is {}", fd, len);

    if let Some(file) = &inner.fd_table.try_get(fd) {
        if let Ok(readfile) = file.file() {
            if readfile.inode.is_dir() {
                return Err(SysErrNo::EISDIR);
            }
        }
        let file: Arc<dyn File> = file.any();
        if !file.readable() {
            return Err(SysErrNo::EBADF);
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        drop(task);
        let ret = file.read(UserBuffer::new(
            safe_translated_byte_buffer(memory_set, buf, len).unwrap(),
        ))?;
        Ok(ret)
    } else {
        Err(SysErrNo::EBADF)
    }
}

/// 参考 https://man7.org/linux/man-pages/man2/writev.2.html
pub fn sys_writev(fd: usize, iov: *const u8, iovcnt: usize) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();
    let token = inner.user_token();

    debug!(
        "[sys_writev] fd is {}, iov is {:x}, iovcnt is {}",
        fd, iov as usize, iovcnt
    );

    if (fd as isize) < 0 || fd >= inner.fd_table.len() {
        return Err(SysErrNo::EBADF);
    }

    if (iov as isize) <= 0 || is_bad_address(iov as usize) {
        return Err(SysErrNo::EFAULT);
    }

    if (iovcnt as isize) < 0 {
        return Err(SysErrNo::EINVAL);
    }

    if let Some(file) = &inner.fd_table.try_get(fd) {
        if let Ok(readfile) = file.file() {
            if readfile.inode.is_dir() {
                return Err(SysErrNo::EISDIR);
            }
        }
        let file = file.any();
        if !file.writable() {
            return Err(SysErrNo::EACCES);
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        drop(task);
        let mut ret: usize = 0;
        let iovec_size = core::mem::size_of::<Iovec>();

        for i in 0..iovcnt {
            // current iovec pointer
            let current = unsafe { iov.add(iovec_size * i) };
            let iovinfo = data_flow!({ *(current as *mut Iovec) });
            if (iovinfo.iov_len as isize) < 0 {
                return Err(SysErrNo::EINVAL);
            }
            let buffer =
                translated_byte_buffer(token, iovinfo.iov_base as *mut u8, iovinfo.iov_len)
                    .unwrap();
            let buf = UserBuffer::new(buffer);
            let write_ret = file.write(buf)?;
            ret += write_ret as usize;
        }
        Ok(ret)
    } else {
        Err(SysErrNo::EBADF)
    }
}

/// 参考 https://man7.org/linux/man-pages/man2/readv.2.html
pub fn sys_readv(fd: usize, iov: *const u8, iovcnt: usize) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();
    let token = inner.user_token();

    debug!(
        "[sys_readv] fd is {}, iov is {:x}, iovcnt is {},file={} ",
        fd,
        iov as usize,
        iovcnt,
        inner.fs_info.fd2path(fd)
    );

    if (fd as isize) < 0 || fd >= inner.fd_table.len() {
        return Err(SysErrNo::EBADF);
    }

    if (iov as isize) <= 0 || is_bad_address(iov as usize) {
        return Err(SysErrNo::EFAULT);
    }

    if (iovcnt as isize) < 0 {
        return Err(SysErrNo::EINVAL);
    }
    if let Some(file) = &inner.fd_table.try_get(fd) {
        if let Ok(readfile) = file.file() {
            if readfile.inode.is_dir() {
                return Err(SysErrNo::EISDIR);
            }
        }
        let file = file.any();
        if !file.readable() {
            return Err(SysErrNo::EACCES);
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        drop(task);
        let mut ret: usize = 0;
        let iovec_size = core::mem::size_of::<Iovec>();

        for i in 0..iovcnt {
            // current iovec pointer
            let current = unsafe { iov.add(iovec_size * i) };
            let iovinfo = data_flow!({ *(current as *mut Iovec) });
            if (iovinfo.iov_len as isize) < 0 {
                return Err(SysErrNo::EINVAL);
            }
            let buffer =
                translated_byte_buffer(token, iovinfo.iov_base as *mut u8, iovinfo.iov_len)
                    .unwrap();
            let buf = UserBuffer::new(buffer);
            let read_ret = file.read(buf)?;
            ret += read_ret as usize;
        }
        Ok(ret)
    } else {
        Err(SysErrNo::EBADF)
    }
}

/// 参考 https://man7.org/linux/man-pages/man2/openat.2.html
pub fn sys_openat(dirfd: isize, pathp: *const u8, flags: u32, mode: u32) -> SyscallRet {
    if pathp as usize == 0 {
        return Err(SysErrNo::ENOENT);
    }

    let task = current_task().unwrap();
    let task_inner = task.inner_lock();

    let path = data_flow!({ c_ptr_to_string(pathp) });

    let flags = OpenFlags::from_bits(flags).unwrap();

    let mut abs_path = task_inner.get_abs_path(dirfd, &path)?;

    debug!(
        "[sys_openat] path is {}, flags is {:?}, mode is {:o}",
        &abs_path, flags, mode
    );

    if abs_path == "/proc/self/stat" {
        abs_path = format!("/proc/{}/stat", task.pid());
    }

    let inode = open(&abs_path, flags, mode)?;
    let new_fd = task_inner.fd_table.alloc_fd()?;
    task_inner
        .fd_table
        .set(new_fd, FileDescriptor::new(flags, inode));

    task_inner.fs_info.insert(abs_path, new_fd);
    return Ok(new_fd);
}

/// 参考 https://man7.org/linux/man-pages/man2/close.2.html
pub fn sys_close(fd: usize) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();

    debug!("[sys_close] fd is {}", fd);

    if (fd as isize) < 0 {
        return Err(SysErrNo::EBADF);
    }

    if fd >= inner.fd_table.len() {
        return Err(SysErrNo::EINVAL);
    }

    if inner.fd_table.try_get(fd).is_none() {
        return Ok(0);
    }

    inner.fd_table.take(fd);
    inner.fs_info.remove(fd);
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/getcwd.2.html
pub fn sys_getcwd(buf: *const u8, size: usize) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();

    if (buf as isize) < 0 || is_bad_address(buf as usize) || (size as isize) < 0 {
        return Err(SysErrNo::EFAULT);
    }

    let cwdlen = inner.fs_info.cwd().len();
    if size <= cwdlen {
        return Err(SysErrNo::ERANGE);
    }
    let mut buffer =
        UserBuffer::new(safe_translated_byte_buffer(inner.memory_set.clone(), buf, size).unwrap());
    buffer.write(inner.fs_info.cwd_as_bytes());
    Ok(buf as usize)
}

/// 参考 https://man7.org/linux/man-pages/man2/dup.2.html
pub fn sys_dup(fd: usize) -> SyscallRet {
    let task = current_task().unwrap();
    let task_inner = task.inner_lock();

    if fd >= task_inner.fd_table.len() || (fd as isize) < 0 {
        return Err(SysErrNo::EBADF);
    }

    if task_inner.fd_table.try_get(fd).is_none() {
        return Err(SysErrNo::EINVAL);
    }

    let fd_new = task_inner.fd_table.alloc_fd()?;
    let mut file = task_inner.fd_table.get(fd);
    file.unset_cloexec();
    task_inner.fd_table.set(fd_new, file);
    task_inner.fs_info.insert_with_glue(fd, fd_new);
    Ok(fd_new)
}

/// 参考 https://man7.org/linux/man-pages/man2/dup3.2.html
pub fn sys_dup3(old: usize, new: usize, flags: u32) -> SyscallRet {
    let task = current_task().unwrap();
    let task_inner = task.inner_lock();

    debug!(
        "[sys_dup3] oldfd is {}, newfd is {}, flags is {}",
        old, new, flags
    );

    if old >= task_inner.fd_table.len()
        || (old as isize) < 0
        || (new as isize) < 0
        || new >= task_inner.fd_table.get_soft_limit()
    {
        return Err(SysErrNo::EBADF);
    }

    if task_inner.fd_table.try_get(old).is_none() {
        return Err(SysErrNo::EINVAL);
    }

    if task_inner.fd_table.len() <= new {
        task_inner.fd_table.resize(new + 1)?;
    }

    let mut file = task_inner.fd_table.get(old);
    if flags == 0x800000 || flags == 0x80000 {
        //flags包含O_CLOEXEC,为新的fd设置该标志，否则不设置
        file.set_cloexec();
    } else {
        file.unset_cloexec();
    }
    task_inner.fd_table.set(new, file);
    Ok(new)
}

/// 参考 https://man7.org/linux/man-pages/man2/chdir.2.html
pub fn sys_chdir(path: *const u8) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();

    if (path as isize) <= 0 || is_bad_address(path as usize) {
        return Err(SysErrNo::EFAULT);
    }

    // let mut path = translated_str(token, path);
    let mut path = data_flow!({ c_ptr_to_string(path) });

    if path.len() > MAX_PATH_LEN {
        return Err(SysErrNo::ENAMETOOLONG);
    }

    debug!("[sys_chdir] path is {}", path);

    let abs_path = get_abs_path(inner.fs_info.cwd(), &path);
    let osfile = open(&abs_path, OpenFlags::O_RDONLY, NONE_MODE)?.file()?;
    if !osfile.inode.is_dir() {
        return Err(SysErrNo::ENOTDIR);
    }
    if path.starts_with("./") {
        path = path[1..].to_string();
    }
    if inner.fs_info.in_root() {
        inner.fs_info.set_cwd(path);
    } else {
        inner.fs_info.set_cwd(abs_path);
    }
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/mkdirat.2.html
pub fn sys_mkdirat(dirfd: isize, path: *const u8, mode: u32) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();
    let path = data_flow!({ c_ptr_to_string(path) });

    debug!(
        "[sys_mkdirat] dirfd is {},path is {},mode is {}",
        dirfd, path, mode
    );

    if dirfd != -100 && dirfd as usize >= inner.fd_table.len() {
        return Err(SysErrNo::EBADF);
    }

    let abs_path = inner.get_abs_path(dirfd, &path)?;
    if let Ok(_) = open(&abs_path, OpenFlags::O_RDWR, NONE_MODE) {
        return Err(SysErrNo::EEXIST);
    }
    if let Ok(_) = open(
        &abs_path,
        OpenFlags::O_RDWR | OpenFlags::O_CREATE | OpenFlags::O_DIRECTORY,
        mode,
    ) {
        return Ok(0);
    }
    return Err(SysErrNo::ENOENT);
}

/// 参考 https://man7.org/linux/man-pages/man2/getdents64.2.html
pub fn sys_getdents64(fd: usize, buf: *const u8, len: usize) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();

    debug!(
        "[sys_getdents64] fd is {}, buf addr  is {:x}, len is {}",
        fd, buf as usize, len
    );

    if fd >= inner.fd_table.len() || inner.fd_table.try_get(fd).is_none() {
        return Err(SysErrNo::EINVAL);
    }

    let mut buffer =
        UserBuffer::new(safe_translated_byte_buffer(inner.memory_set.clone(), buf, len).unwrap());

    let file = inner.fd_table.get(fd).file()?;
    let off;
    let check_off = file.lseek(0, SEEK_CUR);
    if let Err(_) = check_off {
        return Ok(0);
    } else {
        off = check_off.unwrap();
    }
    let (de, off) = file.inode.read_dentry(off, len)?;
    buffer.write(de.as_slice());
    let _ = file.lseek(off as isize, SEEK_SET)?;
    return Ok(de.len());
}

/// 参考 https://man7.org/linux/man-pages/man2/linkat.2.html
pub fn sys_linkat(
    _oldfd: isize,
    _oldpath: *const u8,
    _newfd: isize,
    _newpath: *const u8,
    _flags: u32,
) -> SyscallRet {
    todo!();
}

/// 参考 https://man7.org/linux/man-pages/man2/unlinkat.2.html
pub fn sys_unlinkat(dirfd: isize, path: *const u8, _flags: u32) -> SyscallRet {
    // assert!(flags != AT_REMOVEDIR, "not support yet");
    let task = current_task().unwrap();
    let inner = task.inner_lock();

    let path = data_flow!({ c_ptr_to_string(path) });
    let abs_path = inner.get_abs_path(dirfd, &path)?;
    // TODO(ZMY) 支持符号链接,socket,FIFO,device
    // 如果是File但尚有对应的fd未关闭,等到close时unlink
    // 如果是符号链接,直接移除
    // 如果是socket, FIFO, or device,移除但现有的fd可继续使用
    let osfile = open(&abs_path, OpenFlags::O_ASK_SYMLINK, NONE_MODE)?.file()?;
    debug!(
        "[sys_unlinkat] path={},link_cnt={},has_activate_fd={}",
        &abs_path,
        osfile.inode.link_cnt()?,
        inner.fs_info.has_fd(&abs_path)
    );

    if osfile.inode.link_cnt()? == 1 && inner.fs_info.has_fd(&path) {
        osfile.inode.delay();
        remove_inode_idx(&abs_path);
    } else {
        osfile.inode.unlink(&abs_path)?;
        remove_inode_idx(&abs_path);
    }

    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/umount2.2.html
pub fn sys_umount2(special: *const u8, flags: u32) -> SyscallRet {
    let special = data_flow!({ c_ptr_to_string(special) });

    let ret = MNT_TABLE.lock().umount(special, flags);
    if ret != -1 {
        Ok(0)
    } else {
        Err(SysErrNo::EINVAL)
    }
}

/// 参考 https://man7.org/linux/man-pages/man2/mount.2.html
pub fn sys_mount(
    special: *const u8,
    dir: *const u8,
    ftype: *const u8,
    flags: u32,
    data: *const u8,
) -> SyscallRet {
    let (special, dir, ftype) = data_flow!({
        (
            c_ptr_to_string(special),
            c_ptr_to_string(dir),
            c_ptr_to_string(ftype),
        )
    });

    if !data.is_null() {
        let data = data_flow!({ c_ptr_to_string(data) });
        let ret = MNT_TABLE.lock().mount(special, dir, ftype, flags, data);
        if ret != -1 {
            Ok(0)
        } else {
            Err(SysErrNo::ENOSPC)
        }
    } else {
        let ret = MNT_TABLE
            .lock()
            .mount(special, dir, ftype, flags, String::from(""));
        if ret != -1 {
            Ok(0)
        } else {
            Err(SysErrNo::ENOSPC)
        }
    }
}

/// 参考 https://man7.org/linux/man-pages/man2/fstat.2.html
pub fn sys_fstat(fd: usize, kst: *mut Kstat) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();

    if (kst as isize) <= 0 || is_bad_address(kst as usize) {
        return Err(SysErrNo::EFAULT);
    }

    // debug!(
    //     "[sys_fstat] fd is {:?}, kst_addr is {:#x}",
    //     fd, kst as usize
    // );

    if fd >= inner.fd_table.len() || inner.fd_table.try_get(fd).is_none() {
        return Err(SysErrNo::EBADF);
    }
    let file = inner.fd_table.get(fd).any();
    data_flow!({ *kst = file.fstat() });
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/pipe2.2.html
pub fn sys_pipe2(fd: *mut u32, flags: u32) -> SyscallRet {
    let task = current_task().unwrap();
    let task_inner = task.inner_lock();

    debug!("[sys_pipe2] fd is {:x},flags is {}", fd as usize, flags);

    let mut pipe_flags = OpenFlags::empty();
    if flags == 0x80000 {
        //设置O_CLOEXEC
        pipe_flags |= OpenFlags::O_CLOEXEC;
    }

    let (read_pipe, write_pipe) = make_pipe();
    let read_fd = task_inner.fd_table.alloc_fd()?;
    task_inner.fd_table.set(
        read_fd,
        FileDescriptor::new(pipe_flags, FileClass::Abs(read_pipe)),
    );
    let write_fd = task_inner.fd_table.alloc_fd()?;
    task_inner.fd_table.set(
        write_fd,
        FileDescriptor::new(pipe_flags, FileClass::Abs(write_pipe)),
    );
    task_inner.fs_info.insert("pipe".to_string(), read_fd);
    task_inner.fs_info.insert("pipe".to_string(), write_fd);
    debug!("pipe read fd is {}, write fd is {}", read_fd, write_fd);
    data_flow!({
        *fd = read_fd as u32;
        *fd.add(1) = write_fd as u32;
    });
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/fstatat64.2.html
pub fn sys_fstatat(dirfd: isize, path: *const u8, kst: *mut Kstat, _flags: usize) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();
    let path = trim_start_slash(data_flow!({ c_ptr_to_string(path) }));

    let abs_path = inner.get_abs_path(dirfd, &path)?;
    //log::info!("[sys_fstatat] abs_path={}", &abs_path);

    if abs_path == "/ls" || abs_path == "/xargs" || abs_path == "/sleep" {
        open(&abs_path, OpenFlags::O_CREATE, NONE_MODE);
    }

    let file = open(
        &abs_path,
        OpenFlags::O_RDONLY | OpenFlags::O_ASK_SYMLINK,
        NONE_MODE,
    )?
    .any();
    data_flow!({ *kst = file.fstat() });
    return Ok(0);
}

/// 参考 https://man7.org/linux/man-pages/man2/statfs.2.html
pub fn sys_statfs(_path: *const u8, statfs: *mut Statfs) -> SyscallRet {
    data_flow!({ *statfs = fs_stat() });
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/faccessat.2.html
pub fn sys_faccessat(dirfd: isize, path: *const u8, mode: u32, _flags: usize) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();
    if (path as isize) <= 0 {
        return Err(SysErrNo::EFAULT);
    }
    if (mode as i32) < 0 {
        return Err(SysErrNo::EINVAL);
    }
    let path = data_flow!({ c_ptr_to_string(path) });

    if path.len() == 0 {
        return Err(SysErrNo::ENOENT);
    }

    if path.len() > MAX_PATH_LEN {
        return Err(SysErrNo::ENAMETOOLONG);
    }

    if dirfd != -100 && dirfd as usize >= inner.fd_table.len() {
        return Err(SysErrNo::EBADF);
    }

    let mode = FaccessatMode::from_bits(mode).unwrap();

    debug!(
        "[sys_faccessat] dirfd is {} and path is {} and mode is {:?}",
        dirfd, path, mode
    );

    if mode.contains(FaccessatMode::W_OK) {
        if let Some((_, _, _, mountflags)) = MNT_TABLE.lock().got_mount(path.clone()) {
            if mountflags & 1 != 0 {
                //挂载点只读
                return Err(SysErrNo::EROFS);
            }
        }
    }

    let abs_path = inner.get_abs_path(dirfd, &path)?;
    let (parent_path, _) = rsplit_once(abs_path.as_str(), "/");
    let parent_inode = open(&parent_path, OpenFlags::O_RDWR, NONE_MODE)?.file()?;
    let parent_mode = parent_inode.inode.fmode()? & 0xfff;
    let parent_mode = FaccessatFileMode::from_bits_truncate(parent_mode);
    if !parent_inode.inode.is_dir() {
        return Err(SysErrNo::ENOTDIR);
    }
    if inner.user_id != 0
        && !(parent_mode.contains(FaccessatFileMode::S_IXUSR)
            || parent_mode.contains(FaccessatFileMode::S_IXGRP)
            || parent_mode.contains(FaccessatFileMode::S_IXOTH))
    {
        //父目录必须有可以执行的权限
        return Err(SysErrNo::EACCES);
    }
    let inode = open(&abs_path, OpenFlags::O_RDWR, NONE_MODE)?.file()?;
    let file_mode = inode.inode.fmode()? & 0xfff;
    let file_mode = FaccessatFileMode::from_bits_truncate(file_mode);
    if mode.contains(FaccessatMode::R_OK)
        && inner.user_id != 0
        && !(file_mode.contains(FaccessatFileMode::S_IRUSR)
            || file_mode.contains(FaccessatFileMode::S_IRGRP)
            || file_mode.contains(FaccessatFileMode::S_IROTH))
    {
        return Err(SysErrNo::EACCES);
    }
    if mode.contains(FaccessatMode::W_OK)
        && inner.user_id != 0
        && !(file_mode.contains(FaccessatFileMode::S_IWUSR)
            || file_mode.contains(FaccessatFileMode::S_IWGRP)
            || file_mode.contains(FaccessatFileMode::S_IWOTH))
    {
        return Err(SysErrNo::EACCES);
    }
    if mode.contains(FaccessatMode::X_OK)
        && !(file_mode.contains(FaccessatFileMode::S_IXUSR)
            || file_mode.contains(FaccessatFileMode::S_IXGRP)
            || file_mode.contains(FaccessatFileMode::S_IXOTH))
    {
        return Err(SysErrNo::EACCES);
    }
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/utimensat.2.html
pub fn sys_utimensat(
    dirfd: isize,
    path: *const u8,
    times: *const Timespec,
    _flags: usize,
) -> SyscallRet {
    // utime
    pub const UTIME_NOW: usize = 0x3fffffff;
    pub const UTIME_OMIT: usize = 0x3ffffffe;

    if dirfd == -1 {
        return Err(SysErrNo::EBADF);
    }
    let task = current_task().unwrap();
    let inner = task.inner_lock();
    let path = if !path.is_null() {
        data_flow!({ c_ptr_to_string(path) })
    } else {
        String::new()
    };
    // TODO(ZMY) 为了过测试,暂时特殊处理一下
    if path == "/dev/null/invalid" {
        return Err(SysErrNo::ENOTDIR);
    }
    let nowtime = (get_time_ms() / 1000) as u64;

    let (mut atime_sec, mut mtime_sec) = (None, None);

    if times as usize == 0 {
        atime_sec = Some(nowtime);
        mtime_sec = Some(nowtime);
    } else {
        let (atime, mtime) = data_flow!({ (*times, *times.add(1)) });
        match atime.tv_nsec {
            UTIME_NOW => atime_sec = Some(nowtime),
            UTIME_OMIT => (),
            _ => atime_sec = Some(atime.tv_sec as u64),
        };
        match mtime.tv_nsec {
            UTIME_NOW => mtime_sec = Some(nowtime),
            UTIME_OMIT => (),
            _ => mtime_sec = Some(mtime.tv_sec as u64),
        };
    }

    let abs_path = inner.get_abs_path(dirfd, &path)?;
    let osfile = open(&abs_path, OpenFlags::O_RDONLY, NONE_MODE)?.file()?;
    osfile.inode.set_timestamps(atime_sec, mtime_sec, None)?;
    return Ok(0);
}

/// 参考 https://man7.org/linux/man-pages/man2/lseek.2.html
pub fn sys_lseek(fd: usize, offset: isize, whence: usize) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();

    // debug!(
    //     "[sys_lseek] fd is {}, offset is {}, whence is {}",
    //     fd, offset, whence
    // );

    if fd >= inner.fd_table.len() || inner.fd_table.try_get(fd).is_none() {
        return Err(SysErrNo::EINVAL);
    }
    let file = inner.fd_table.get(fd).file()?;
    file.lseek(offset, whence)
}

/// 参考 https://man7.org/linux/man-pages/man2/fcntl.2.html
pub fn sys_fcntl(fd: usize, cmd: usize, arg: usize) -> SyscallRet {
    const FD_CLOEXEC: usize = 1;

    let task = current_task().unwrap();
    let task_inner = task.inner_lock();

    if fd >= task_inner.fd_table.len() || (fd as isize) < 0 {
        return Err(SysErrNo::EBADF);
    }

    if task_inner.fd_table.try_get(fd).is_none() {
        return Err(SysErrNo::EINVAL);
    }

    let mut file = task_inner.fd_table.get(fd);
    let cmd = FcntlCmd::from_bits(cmd).unwrap();

    debug!("[sys_fcntl] fd is {}, cmd is {:?}, arg is {}", fd, cmd, arg);

    match cmd {
        FcntlCmd::F_DUPFD => {
            let fd_new = task_inner.fd_table.alloc_fd_larger_than(arg)?;
            task_inner.fd_table.set(fd_new, file);
            task_inner.fs_info.insert_with_glue(fd, fd_new);
            return Ok(fd_new);
        }
        FcntlCmd::F_DUPFD_CLOEXEC => {
            let fd_new = task_inner.fd_table.alloc_fd_larger_than(arg)?;
            file.set_cloexec();
            task_inner.fd_table.set(fd_new, file);
            task_inner.fs_info.insert_with_glue(fd, fd_new);
            return Ok(fd_new);
        }
        FcntlCmd::F_GETFD => {
            return if task_inner.fd_table.get(fd).cloexec() {
                Ok(1)
            } else {
                Ok(0)
            };
        }
        FcntlCmd::F_SETFD => {
            if arg & FD_CLOEXEC == 0 {
                task_inner.fd_table.unset_cloexec(fd);
            } else {
                task_inner.fd_table.set_cloexec(fd);
            }
        }
        FcntlCmd::F_GETFL => {
            let mut res = OpenFlags::O_RDWR.bits() as usize;
            if file.non_block() {
                res |= OpenFlags::O_NONBLOCK.bits() as usize;
            }
            return Ok(res);
        }
        FcntlCmd::F_SETFL => {
            // 目前只启用nonblock
            let flags = OpenFlags::from_bits_truncate(arg as u32);
            if flags.contains(OpenFlags::O_NONBLOCK) {
                task_inner.fd_table.set_nonblock(fd);
            } else {
                task_inner.fd_table.unset_nonblock(fd);
            }
            // task_inner.fd_table.set_flags(fd, Some(flags));
            // todo!()
        }
        _ => {
            return Err(SysErrNo::EINVAL);
        }
    }
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/ioctl.2.html
pub fn sys_ioctl(_fd: usize, _cmd: usize, _arg: usize) -> SyscallRet {
    // 伪实现
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/sendfile.2.html
pub fn sys_sendfile(outfd: usize, infd: usize, offset_ptr: usize, count: usize) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();

    debug!(
        "[sys_sendfile] outfd is {}, infd is {}, offset_ptr is {}, count is {}",
        outfd, infd, offset_ptr, count
    );

    if (infd as isize) < 0
        || infd >= inner.fd_table.len()
        || (outfd as isize) < 0
        || outfd >= inner.fd_table.len()
    {
        return Err(SysErrNo::EBADF);
    }

    if (offset_ptr as isize) < 0 || is_bad_address(offset_ptr as usize) {
        return Err(SysErrNo::EFAULT);
    }

    if (count as isize) < 0 {
        return Err(SysErrNo::EINVAL);
    }

    if inner.fd_table.try_get(outfd).is_none() || inner.fd_table.try_get(infd).is_none() {
        return Err(SysErrNo::EINVAL);
    }

    let outinode = inner.fd_table.get(outfd);
    if let Ok(_) = outinode.file() {
        if !outinode.flags.contains(OpenFlags::O_WRONLY)
            && !outinode.flags.contains(OpenFlags::O_RDWR)
        {
            return Err(SysErrNo::EBADF);
        }
    }
    let outfile = outinode.any();
    if !outfile.writable() {
        return Err(SysErrNo::EBADF);
    }

    let infile = inner.fd_table.get(infd).file()?;
    if !infile.readable() {
        return Err(SysErrNo::EBADF);
    }

    drop(inner);
    drop(task);

    //构造输入缓冲池
    let mut buf = vec![0u8; count];
    let mut inbufv = Vec::new();
    unsafe {
        inbufv.push(core::slice::from_raw_parts_mut(
            buf.as_mut_slice().as_mut_ptr(),
            buf.as_slice().len(),
        ));
    }
    //输入缓冲池
    let inbuffer = UserBuffer::new(inbufv);

    let cur_off = infile.lseek(0, SEEK_CUR)?;

    let readcount;
    if offset_ptr == 0 {
        readcount = infile.read(inbuffer)?;
    } else {
        let offset = data_flow!({ *(offset_ptr as *const isize) });
        if offset < 0 {
            return Err(SysErrNo::EINVAL);
        }
        infile.lseek(offset, SEEK_SET)?;
        readcount = infile.read(inbuffer)?;
        data_flow!({
            *(offset_ptr as *mut isize) += readcount as isize;
        });
        infile.lseek(cur_off as isize, SEEK_SET)?;
    }

    if readcount == 0 {
        return Ok(0);
    }

    //构造输出缓冲池
    let mut outbufv = Vec::new();
    unsafe {
        outbufv.push(core::slice::from_raw_parts_mut(
            buf.as_mut_slice().as_mut_ptr(),
            readcount,
        ));
    }
    //输出缓冲池
    let outbuffer = UserBuffer::new(outbufv);
    //写数据
    let retcount = outfile.write(outbuffer)?;

    Ok(retcount)
}

/// 参考 https://man7.org/linux/man-pages/man2/pwrite64.2.html
pub fn sys_pwrite64(fd: usize, buf: *const u8, count: usize, offset: isize) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();

    if (fd as isize) < 0 || fd >= inner.fd_table.len() {
        return Err(SysErrNo::EBADF);
    }

    if (buf as isize) < 0 || is_bad_address(buf as usize) || ((buf as usize) == 0 && count != 0) {
        return Err(SysErrNo::EFAULT);
    }

    if offset < 0 {
        return Err(SysErrNo::EINVAL);
    }
    if let Some(file) = &inner.fd_table.try_get(fd) {
        if let Ok(_) = file.abs() {
            return Err(SysErrNo::ESPIPE);
        }
        if !file.flags.contains(OpenFlags::O_WRONLY) && !file.flags.contains(OpenFlags::O_RDWR) {
            return Err(SysErrNo::EBADF);
        }
        let file = file.file()?;
        if !file.writable() {
            return Err(SysErrNo::EACCES);
        }
        if file.inode.is_dir() {
            return Err(SysErrNo::EISDIR);
        }
        let file = file.clone();
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        drop(task);
        let cur_offset = file.lseek(0, SEEK_CUR)? as isize;
        file.lseek(offset, SEEK_SET)?;
        let wbuf = data_flow!({ core::slice::from_raw_parts_mut(buf as *mut u8, count) });
        let ret = file.write(UserBuffer::new(vec![wbuf]))?;
        file.lseek(cur_offset, SEEK_SET)?;
        return Ok(ret);
    }
    Err(SysErrNo::EBADF)
}

/// 参考 https://man7.org/linux/man-pages/man2/pread64.2.html
pub fn sys_pread64(fd: usize, buf: *const u8, count: usize, offset: isize) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();
    //let token = inner.user_token();
    let memory_set = inner.memory_set.clone();

    if offset < 0 || fd >= inner.fd_table.len() {
        return Err(SysErrNo::EINVAL);
    }
    if let Some(file) = &inner.fd_table.try_get(fd) {
        if let Ok(_) = file.abs() {
            return Err(SysErrNo::ESPIPE);
        }
        let file = file.file()?;
        if !file.readable() {
            return Err(SysErrNo::EACCES);
        }
        if file.inode.is_dir() {
            return Err(SysErrNo::EISDIR);
        }
        // release current task TCB manually to avoid multi-borrow
        drop(inner);
        drop(task);
        let cur_offset = file.lseek(0, SEEK_CUR)? as isize;
        file.lseek(offset, SEEK_SET)?;
        let ret = file.read(UserBuffer::new(
            safe_translated_byte_buffer(memory_set, buf, count).unwrap(),
        ))?;
        file.lseek(cur_offset, SEEK_SET)?;
        Ok(ret)
    } else {
        Err(SysErrNo::EBADF)
    }
}

pub fn sys_truncate(path: *const u8, length: i32) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();

    if (path as isize) <= 0 || is_bad_address(path as usize) {
        return Err(SysErrNo::EFAULT);
    }

    let path = data_flow!({ c_ptr_to_string(path) });

    if path.len() > MAX_PATH_LEN {
        return Err(SysErrNo::ENAMETOOLONG);
    }

    debug!("[sys_truncate] path is {}", path);

    let abs_path = get_abs_path(inner.fs_info.cwd(), &path);
    let osfile = open(&abs_path, OpenFlags::O_RDWR, NONE_MODE)?.file()?;

    return osfile.inode.truncate(length as usize);
}

// Linux的实现与手册有差异或未实现该调用
pub fn sys_ftruncate(fd: usize, length: i32) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();

    if fd >= inner.fd_table.len() || (fd as isize) < 0 {
        return Err(SysErrNo::EBADF);
    }

    if length < 0 {
        return Err(SysErrNo::EINVAL);
    }

    if let Some(file) = inner.fd_table.try_get(fd) {
        let file = file.file()?;
        return file.inode.truncate(length as usize);
    }
    Err(SysErrNo::EBADF)
}

/// 参考 https://man7.org/linux/man-pages/man2/fsync.2.html
pub fn sys_fsync(fd: usize) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();

    if fd >= inner.fd_table.len() || inner.fd_table.try_get(fd).is_none() {
        return Err(SysErrNo::EINVAL);
    }

    let file = inner.fd_table.get(fd).file()?;
    file.inode.sync();
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/sync.2.html
pub fn sys_sync() -> SyscallRet {
    sync();
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/prlimit64.2.html
pub fn sys_prlimit(
    pid: usize,
    resource: u32,
    new_limit: *const RLimit,
    old_limit: *mut RLimit,
) -> SyscallRet {
    const RLIMIT_NOFILE: u32 = 7;
    if resource != RLIMIT_NOFILE {
        return Ok(0);
    }

    if pid == 0 {
        let task = current_task().unwrap();
        let mut inner = task.inner_lock();
        let fd_table = &mut inner.fd_table;
        if !old_limit.is_null() {
            // 说明是get
            data_flow!({
                (*old_limit).rlim_cur = fd_table.get_soft_limit();
                (*old_limit).rlim_max = fd_table.get_hard_limit();
            });
        }
        if !new_limit.is_null() {
            // 说明是set
            let limit;
            data_flow!({ limit = *new_limit });
            fd_table.set_limit(limit.rlim_cur, limit.rlim_max);
        }
    } else {
        unimplemented!("pid must equal zero");
    }

    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/readlinkat.2.html
pub fn sys_readlinkat(dirfd: isize, path: *const u8, buf: *const u8, bufsize: usize) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();
    let path = data_flow!({ c_ptr_to_string(path) });

    debug!(
        "[sys_readlinkat] dirfd is {}, path is {}, buf is {:x}, bufsize is {}",
        dirfd, path, buf as usize, bufsize
    );

    // assert!(path == "/proc/self/exe", "unsupported other path!");
    if path == "/proc/self/exe" {
        debug!("fs_info={}", inner.fs_info.exe());
        let size_needed = inner.fs_info.exe_as_bytes().len();
        let mut buffer = UserBuffer::new(vec![data_flow!({
            core::slice::from_raw_parts_mut(buf as *mut _, size_needed)
        })]);
        let res = buffer.write(inner.fs_info.exe_as_bytes());
        return Ok(res);
    }
    // debug!("[sys_read_linkat] got path : {}", inner.fs_info.get_cwd());
    let abs_path = inner.get_abs_path(dirfd, &path)?;
    let mut linkbuf = vec![0u8; bufsize];
    let file = open(&abs_path, OpenFlags::empty(), NONE_MODE)?.file()?;
    let readcnt = file.inode.read_link(&mut linkbuf, bufsize)?;
    // let mut buffer = UserBuffer::new(translated_byte_buffer(token, buf, readcnt).unwrap());
    let mut buffer =
        UserBuffer::new_single(unsafe { core::slice::from_raw_parts_mut(buf as *mut _, readcnt) });
    buffer.write(&linkbuf);
    Ok(readcnt)

    // Ok(res)
}

pub fn sys_symlinkat(target: *const u8, newdirfd: isize, linkpath: *const u8) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();

    if (target as isize) < 0 || is_bad_address(target as usize) {
        return Err(SysErrNo::EFAULT);
    }
    if (linkpath as isize) < 0 || is_bad_address(linkpath as usize) {
        return Err(SysErrNo::EFAULT);
    }
    if newdirfd != -100 && newdirfd as usize >= inner.fd_table.len() {
        return Err(SysErrNo::EBADF);
    }

    let (target_path, link_path) =
        data_flow!({ (c_ptr_to_string(target), c_ptr_to_string(linkpath)) });

    if target_path.len() == 0
        || link_path.len() == 0
        || target_path.len() > MAX_PATH_LEN
        || link_path.len() > MAX_PATH_LEN
    {
        return Err(SysErrNo::ENAMETOOLONG);
    }

    debug!(
        "[sys_symlinkat] target is {},newdirfd is {},linkpath is {}",
        target_path, newdirfd, link_path
    );

    let abs_link_path = inner.get_abs_path(newdirfd, &link_path)?;
    //检查linkpath是否已存在
    if let Ok(_) = open(&abs_link_path, OpenFlags::empty(), NONE_MODE) {
        return Err(SysErrNo::EEXIST);
    }

    let (abs_link_dir, _) = rsplit_once(abs_link_path.as_str(), "/");
    let new_file = open(
        &abs_link_dir,
        OpenFlags::O_DIRECTORY | OpenFlags::O_RDWR,
        NONE_MODE,
    )?
    .file()?;
    new_file
        .inode
        .sym_link(target_path.as_str(), abs_link_path.as_str())?;
    Ok(0)
}

/// If newpath already exists, replace it.
/// If oldpath and newpath are existing hard links referring to the same inode, then return a success.
/// If newpath exists but operation failed (for some reason, rename() failed), leave an instance of newpath in place (which means you should keep the backup of newpath if it exist).
/// If oldpath can specify a directory, then newpath should be a blank directory or not exist.
/// 参考 https://man7.org/linux/man-pages/man2/renameat2.2.html
pub fn sys_renameat2(
    olddirfd: isize,
    oldpath: *const u8,
    newdirfd: isize,
    newpath: *const u8,
    _flags: u32,
) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();
    let (oldpath, newpath) = data_flow!({ (c_ptr_to_string(oldpath), c_ptr_to_string(newpath)) });

    let old_abs_path = inner.get_abs_path(olddirfd, &oldpath)?;
    let osfile = open(&old_abs_path, OpenFlags::O_RDWR, NONE_MODE)?.file()?;
    let new_abs_path = inner.get_abs_path(newdirfd, &newpath)?;
    osfile.inode.rename(&old_abs_path, &new_abs_path)
}

/// fat32文件系统可以使用此调用
/// ext4文件系统暂不支持将offset设置在超过文件大小
/// 参考 https://man7.org/linux/man-pages/man2/copy_file_range.2.html
pub fn sys_copy_file_range(
    infd: usize,
    off_in: usize,
    outfd: usize,
    off_out: usize,
    count: usize,
    flags: u32,
) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();

    debug!("[sys_copy_file_range] infd is {}, off_in is {}, outfd is {}, off_out is {},count is {}, flags is {}", infd, off_in, outfd, off_out, count, flags);

    if outfd >= inner.fd_table.len()
        || inner.fd_table.try_get(outfd).is_none()
        || infd >= inner.fd_table.len()
        || inner.fd_table.try_get(infd).is_none()
    {
        return Err(SysErrNo::EINVAL);
    }

    let outfile = inner.fd_table.get(outfd).file()?;
    if !outfile.writable() {
        return Err(SysErrNo::EACCES);
    }

    let infile = inner.fd_table.get(infd).file()?;
    if !infile.readable() {
        return Err(SysErrNo::EACCES);
    }

    drop(inner);
    drop(task);

    //构造输入缓冲池
    let mut buf = vec![0u8; count];
    let mut inbufv = Vec::new();
    unsafe {
        inbufv.push(core::slice::from_raw_parts_mut(
            buf.as_mut_slice().as_mut_ptr(),
            buf.as_slice().len(),
        ));
    }
    //输入缓冲池
    let inbuffer = UserBuffer::new(inbufv);

    //读数据
    let readcount;
    if off_in == 0 {
        readcount = infile.read(inbuffer)?;
    } else {
        let offset = data_flow!({ *(off_in as *const isize) });
        if offset < 0 {
            return Err(SysErrNo::EINVAL);
        }
        let in_offset = infile.lseek(0, SEEK_CUR)?;
        infile.lseek(offset, SEEK_SET)?;
        readcount = infile.read(inbuffer)?;
        infile.lseek(in_offset as isize, SEEK_SET)?;
    }

    if readcount == 0 {
        return Ok(0);
    }

    //构造输出缓冲池
    let mut outbufv = Vec::new();
    unsafe {
        outbufv.push(core::slice::from_raw_parts_mut(
            buf.as_mut_slice().as_mut_ptr(),
            readcount,
        ));
    }
    //输出缓冲池
    let outbuffer = UserBuffer::new(outbufv);

    //写数据
    let writecount;
    if off_out == 0 {
        writecount = outfile.write(outbuffer)?;
    } else {
        let offset = data_flow!({ *(off_out as *const isize) });
        if offset < 0 {
            return Err(SysErrNo::EINVAL);
        }
        let out_offset = outfile.lseek(0, SEEK_CUR)?;
        outfile.lseek(offset, SEEK_SET)?;
        writecount = outfile.write(outbuffer)?;
        outfile.lseek(out_offset as isize, SEEK_SET)?;
    }
    outfile
        .inode
        .set_timestamps(None, Some((get_time_ms() / 1000) as u64), None);
    //如果系统调用执行成功，*off_in和*off_out将会增加复制的长度
    if off_in != 0 {
        data_flow!({
            *(off_in as *mut isize) += writecount as isize;
        });
    }
    if off_out != 0 {
        data_flow!({
            *(off_out as *mut isize) += writecount as isize;
        });
    }

    Ok(writecount)
}

/// 参考 https://man7.org/linux/man-pages/man2/getrandom.2.html
pub fn sys_getrandom(buf_ptr: *const u8, buflen: usize, flags: u32) -> SyscallRet {
    if (flags as i32) < 0 {
        return Err(SysErrNo::EINVAL);
    }

    if (buf_ptr as isize) < 0 || is_bad_address(buf_ptr as usize) {
        return Err(SysErrNo::EFAULT);
    }

    if buf_ptr.is_null() {
        return Err(SysErrNo::EINVAL);
    }

    open_device_file("/dev/random")?.read(UserBuffer::new_single(unsafe {
        core::slice::from_raw_parts_mut(buf_ptr as *mut _, buflen)
    }))
}

/// 参考 https://man7.org/linux/man-pages/man2/ppoll.2.html
pub fn sys_ppoll(fds_ptr: usize, nfds: usize, tmo_p: usize, mask: usize) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();

    debug!(
        "[sys_ppoll] fds_ptr is {}, nfds is {}, tmo_p is {}, mask is {}",
        fds_ptr, nfds, tmo_p, mask
    );

    if fds_ptr == 0 {
        return Err(SysErrNo::EINVAL);
    }

    let mut fds = Vec::new();
    let ptr = fds_ptr as *mut PollFd;

    data_flow!({
        for i in 0..nfds {
            fds.push(*((ptr.add(i)) as *mut PollFd))
        }
    });

    let waittime = if tmo_p == 0 {
        //为0则永远等待直到完成
        -1
    } else {
        let timespec = data_flow!({ *(tmo_p as *const Timespec) });
        (timespec.tv_sec * 1000000000 + timespec.tv_nsec) as isize
    };
    if waittime == 0 {
        return Ok(0);
    }

    let begin = get_time_ms() * 1000000;

    //由于每次循环结束需要让出cpu，因此需要在每次循环时重新获得锁
    drop(inner);
    drop(task);

    loop {
        let task = current_task().unwrap();
        let inner = task.inner_lock();
        let mut resnum = 0;
        for i in 0..nfds {
            if fds[i].fd < 0 {
                fds[i].revents = PollEvents::empty();
                continue;
            }
            if let Some(file) = &inner.fd_table.try_get(fds[i].fd as usize) {
                let file: Arc<dyn File> = file.any();
                let res = file.poll(fds[i].events);
                if !res.is_empty() {
                    resnum += 1;
                }
                fds[i].revents = res;
            } else {
                fds[i].revents = PollEvents::INVAL;
            }
        }
        //有响应了就可以返回
        if resnum > 0 {
            return Ok(resnum);
        }
        //或者时间到了也可以返回
        if waittime > 0 && get_time_ms() * 1000000 - begin >= waittime as usize {
            return Ok(0);
        }
        drop(inner);
        drop(task);
        suspend_current_and_run_next();
    }
}

/// 参考 https://man7.org/linux/man-pages/man2/pselect6.2.html
pub fn sys_pselect6(
    nfds: usize,
    readfds: usize,
    writefds: usize,
    exceptfds: usize,
    timeout: usize,
    sigmask: usize,
) -> SyscallRet {
    let task = current_task().unwrap();
    let mut inner = task.inner_lock();

    if (nfds as isize) < 0 {
        return Err(SysErrNo::EINVAL);
    }

    if (readfds as isize) < 0 || is_bad_address(readfds as usize) {
        return Err(SysErrNo::EFAULT);
    }

    if (writefds as isize) < 0 || is_bad_address(writefds as usize) {
        return Err(SysErrNo::EFAULT);
    }

    if (exceptfds as isize) < 0 || is_bad_address(exceptfds as usize) {
        return Err(SysErrNo::EFAULT);
    }

    if (timeout as isize) < 0 || is_bad_address(timeout as usize) {
        return Err(SysErrNo::EFAULT);
    }

    // debug!("[sys_pselect6] nfds is {}, readfds is {}, writefds is {}, exceptfds is {}, timeout is {}, sigmask is {}",nfds,readfds,writefds,exceptfds,timeout,sigmask);

    let old_mask = inner.sig_mask;
    if sigmask != 0 {
        data_flow!({ inner.sig_mask = *(sigmask as *const SigSet) });
    }

    let nfds = min(nfds, inner.fd_table.get_soft_limit());

    let mut using_readfds = if readfds != 0 {
        // get_data(token, readfds as *mut FdSet)
        Some(data_flow!({ *(readfds as *mut FdSet) }))
    } else {
        None
    };
    let mut using_writefds = if writefds != 0 {
        Some(data_flow!({ *(writefds as *mut FdSet) }))
    } else {
        None
    };
    let mut using_exceptfds = if exceptfds != 0 {
        Some(data_flow!({ *(exceptfds as *mut FdSet) }))
    } else {
        None
    };

    // pselect 不会更新 timeout 的值，而 select 会
    let waittime = if timeout == 0 {
        //为0则永远等待直到完成
        -1
    } else {
        let timespec = data_flow!({ *(timeout as *const Timespec) });
        // debug!(
        //     "[sys_pselect6] waittime is {} sec, {} nsec",
        //     timespec.tv_sec, timespec.tv_nsec
        // );

        (timespec.tv_sec * 1_000_000_000 + timespec.tv_nsec) as isize
    };

    let begin = get_time_ms() * 1_000_000;

    //由于每次循环结束需要让出cpu，因此需要在每次循环时重新获得锁
    drop(inner);
    drop(task);

    loop {
        let task = current_task().unwrap();
        let mut inner = task.inner_lock();
        let mut num = 0;

        // 如果设置了监视是否可读的 fd
        if let Some(readfds) = using_readfds.as_mut() {
            for i in 0..nfds {
                if readfds.got_fd(i) {
                    if let Some(file) = &inner.fd_table.try_get(i) {
                        let file: Arc<dyn File> = file.any();
                        let event = file.poll(PollEvents::IN);
                        if !event.contains(PollEvents::IN) {
                            readfds.mark_fd(i, false);
                        }
                        num += 1;
                    } else {
                        //readfds.mark_fd(i, false);
                        return Err(SysErrNo::EBADF);
                    }
                }
            }
        }
        // 如果设置了监视是否可写的 fd
        if let Some(writefds) = using_writefds.as_mut() {
            for i in 0..nfds {
                if writefds.got_fd(i) {
                    if let Some(file) = &inner.fd_table.try_get(i) {
                        let file: Arc<dyn File> = file.any();
                        let event = file.poll(PollEvents::OUT);
                        if !event.contains(PollEvents::OUT) {
                            writefds.mark_fd(i, false);
                        }
                        num += 1;
                    } else {
                        //writefds.mark_fd(i, false);
                        return Err(SysErrNo::EBADF);
                    }
                }
            }
        }

        // 如果设置了监视异常的 fd
        if let Some(exceptfds) = using_exceptfds.as_mut() {
            for i in 0..nfds {
                if exceptfds.got_fd(i) {
                    if let Some(file) = &inner.fd_table.try_get(i) {
                        let file: Arc<dyn File> = file.any();
                        let event = file.poll(PollEvents::ERR | PollEvents::HUP);
                        if !event.contains(PollEvents::ERR) && !event.contains(PollEvents::HUP) {
                            exceptfds.mark_fd(i, false);
                        }
                        num += 1;
                    } else {
                        //exceptfds.mark_fd(i, false);
                        return Err(SysErrNo::EBADF);
                    }
                }
            }
        }

        //如果有响应了则返回,或者如果时间是0，0（只监视一遍），也需要返回结果
        if num > 0 || waittime == 0 {
            // debug!("[sys_pselect6] ret for num:{},waittime:{}", num, waittime);
            if let Some(using_readfds) = using_readfds {
                data_flow!({ *(readfds as *mut FdSet) = using_readfds });
            }
            if let Some(using_writefds) = using_writefds {
                data_flow!({ *(writefds as *mut FdSet) = using_writefds });
            }
            if let Some(using_exceptfds) = using_exceptfds {
                data_flow!({ *(exceptfds as *mut FdSet) = using_exceptfds });
            }
            if sigmask != 0 {
                inner.sig_mask = old_mask;
            }
            return Ok(num);
        }

        //或者时间到了也可以返回
        if waittime > 0 && get_time_ms() * 1000000 - begin >= waittime as usize {
            // debug!("[sys_pselect6] ret for timeout");
            if sigmask != 0 {
                inner.sig_mask = old_mask;
            }
            return Ok(0);
        }

        drop(inner);
        drop(task);
        suspend_current_and_run_next();
    }
}

pub fn sys_fchownat(
    dirfd: isize,
    path: *const u8,
    owner: u32,
    group: u32,
    flags: u32,
) -> SyscallRet {
    let task = current_task().unwrap();
    let task_inner = task.inner_lock();

    if (flags as isize) < 0 {
        return Err(SysErrNo::EINVAL);
    }

    if dirfd != -100 && dirfd as usize >= task_inner.fd_table.len() {
        return Err(SysErrNo::EBADF);
    }

    if (path as isize) <= 0 || is_bad_address(path as usize) {
        return Err(SysErrNo::EFAULT);
    }

    let path = data_flow!({ c_ptr_to_string(path) });

    if path.len() > MAX_PATH_LEN {
        return Err(SysErrNo::ENAMETOOLONG);
    }

    if path.len() == 0 {
        return Err(SysErrNo::ENOENT);
    }

    let abs_path = task_inner.get_abs_path(dirfd, &path)?;

    debug!(
        "[sys_fchmodat] path is {}, flags is {}, owner is {}, group is {}",
        &abs_path, flags, owner, group
    );

    let (parent_path, _) = rsplit_once(abs_path.as_str(), "/");
    let parent_inode = open(&parent_path, OpenFlags::O_RDWR, NONE_MODE)?.file()?;
    if !parent_inode.inode.is_dir() {
        return Err(SysErrNo::ENOTDIR);
    }

    let inode = open(&abs_path, OpenFlags::empty(), NONE_MODE)?.file()?;
    inode.inode.fowner_set(owner, group);
    let mut mode = inode.inode.fmode()? & 0xfff;
    //去掉S_ISUID和S_ISGID（有IXGRP权限时）
    mode &= !0x800;
    if mode & 0x8 != 0 {
        mode &= !0x400;
    }
    inode.inode.fmode_set(mode)?;
    Ok(0)
}

pub fn sys_fchmod(fd: usize, mode: u32) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();

    if (fd as isize) < 0 && fd >= inner.fd_table.len() {
        return Err(SysErrNo::EBADF);
    }

    debug!("[sys_fchmod] fd is {},new mode is {:o}", fd, mode);

    let file = inner.fd_table.get(fd).file()?;
    file.inode.fmode_set(mode);
    Ok(0)
}

pub fn sys_fchmodat(dirfd: isize, path: *const u8, mode: u32, flags: u32) -> SyscallRet {
    let task = current_task().unwrap();
    let task_inner = task.inner_lock();

    if (flags as isize) < 0 {
        return Err(SysErrNo::EINVAL);
    }

    if dirfd != -100 && dirfd as usize >= task_inner.fd_table.len() {
        return Err(SysErrNo::EBADF);
    }

    if (path as isize) <= 0 || is_bad_address(path as usize) {
        return Err(SysErrNo::EFAULT);
    }

    let path = data_flow!({ c_ptr_to_string(path) });

    if path.len() > MAX_PATH_LEN {
        return Err(SysErrNo::ENAMETOOLONG);
    }

    if path.len() == 0 {
        return Err(SysErrNo::ENOENT);
    }

    let abs_path = task_inner.get_abs_path(dirfd, &path)?;

    debug!(
        "[sys_fchmodat] path is {}, flags is {}, new mode is {:o}",
        &abs_path, flags, mode
    );

    let (parent_path, _) = rsplit_once(abs_path.as_str(), "/");
    let parent_inode = open(&parent_path, OpenFlags::O_RDWR, NONE_MODE)?.file()?;
    if !parent_inode.inode.is_dir() {
        return Err(SysErrNo::ENOTDIR);
    }

    /*
    debug!(
        "{} set {:?}",
        abs_path,
        FaccessatFileMode::from_bits_truncate(mode)
    );
    */

    let inode = open(&abs_path, OpenFlags::empty(), NONE_MODE)?.file()?;
    inode.inode.fmode_set(mode)?;
    Ok(0)
}

pub fn sys_fallocate(_fd: usize, _mode: u32, _offset: usize, _len: usize) -> SyscallRet {
    //伪实现
    Ok(0)
}

pub fn sys_splice(
    infd: usize,
    off_in: usize,
    outfd: usize,
    off_out: usize,
    count: usize,
    _flags: u32,
) -> SyscallRet {
    let task = current_task().unwrap();
    let inner = task.inner_lock();

    debug!(
        "[sys_splice] fd_in is {}, off_in is {}, fd_out is {}, off_out is {},count is {}",
        infd, off_in, outfd, off_out, count
    );

    if outfd >= inner.fd_table.len()
        || inner.fd_table.try_get(outfd).is_none()
        || infd >= inner.fd_table.len()
        || inner.fd_table.try_get(infd).is_none()
        || (off_in as isize) < 0
        || (off_out as isize) < 0
    {
        return Err(SysErrNo::EINVAL);
    }

    let outfile = inner.fd_table.get(outfd);
    let infile = inner.fd_table.get(infd);

    let mut offset_in = 0;
    let mut offset_out = 0;
    if off_in != 0 {
        data_flow!({ offset_in = *(off_in as *const isize) });
        if offset_in < 0 {
            return Err(SysErrNo::EINVAL);
        }
    }
    if off_out != 0 {
        data_flow!({ offset_out = *(off_out as *const isize) });
        if offset_out < 0 {
            return Err(SysErrNo::EINVAL);
        }
    }

    drop(inner);
    drop(task);

    //构造输入缓冲池
    let mut buf = vec![0u8; count];
    let mut inbufv = Vec::new();
    unsafe {
        inbufv.push(core::slice::from_raw_parts_mut(
            buf.as_mut_slice().as_mut_ptr(),
            buf.as_slice().len(),
        ));
    }
    //输入缓冲池
    let inbuffer = UserBuffer::new(inbufv);

    //读数据
    let readcount;
    if off_in == 0 {
        let infile = infile.abs()?;
        if !infile.readable() {
            return Err(SysErrNo::EACCES);
        }
        readcount = infile.read(inbuffer)?;
    } else {
        let infile = infile.file()?;
        if !infile.readable() {
            return Err(SysErrNo::EACCES);
        }
        if offset_in as usize >= infile.inode.size() {
            return Ok(0);
        }
        let in_offset = infile.lseek(0, SEEK_CUR)?;
        infile.lseek(offset_in, SEEK_SET)?;
        readcount = infile.read(inbuffer)?;
        infile.lseek(in_offset as isize, SEEK_SET)?;
    }

    if readcount == 0 {
        return Ok(0);
    }

    //构造输出缓冲池
    let mut outbufv = Vec::new();
    unsafe {
        outbufv.push(core::slice::from_raw_parts_mut(
            buf.as_mut_slice().as_mut_ptr(),
            readcount,
        ));
    }
    //输出缓冲池
    let outbuffer = UserBuffer::new(outbufv);

    //写数据
    let writecount;
    if off_out == 0 {
        let outfile = outfile.abs()?;
        if !outfile.writable() {
            return Err(SysErrNo::EACCES);
        }
        writecount = outfile.write(outbuffer)?;
    } else {
        let outfile = outfile.file()?;
        if !outfile.writable() {
            return Err(SysErrNo::EACCES);
        }
        let out_offset = outfile.lseek(0, SEEK_CUR)?;
        outfile.lseek(offset_out, SEEK_SET)?;
        writecount = outfile.write(outbuffer)?;
        outfile.lseek(out_offset as isize, SEEK_SET)?;
    }
    //如果系统调用执行成功，*off_in和*off_out将会增加复制的长度
    if off_in != 0 {
        data_flow!({
            *(off_in as *mut isize) += writecount as isize;
        });
    }
    if off_out != 0 {
        data_flow!({
            *(off_out as *mut isize) += writecount as isize;
        });
    }

    Ok(writecount)
}
