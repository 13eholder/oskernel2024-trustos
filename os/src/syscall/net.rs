use alloc::{format, string::ToString};

use crate::{
    data_flow,
    fs::{make_socket, make_socketpair, FileClass, FileDescriptor, OpenFlags},
    task::current_task,
    utils::{SysErrNo, SyscallRet},
};
use log::debug;

/// 参考 https://man7.org/linux/man-pages/man2/socket.2.html
pub fn sys_socket(_domain: u32, _type: u32, _protocol: u32) -> SyscallRet {
    let task = current_task().unwrap();
    let task_inner = task.inner_lock();
    let new_fd = task_inner.fd_table.alloc_fd()?;
    let close_on_exec = (_type & 0o2000000) == 0o2000000;
    let non_block = (_type & 0o4000) == 0o4000;
    let mut flags = OpenFlags::empty();
    if close_on_exec {
        flags |= OpenFlags::O_CLOEXEC;
    }
    if non_block {
        flags |= OpenFlags::O_NONBLOCK;
    }
    task_inner.fd_table.set(
        new_fd,
        FileDescriptor::new(flags, FileClass::Abs(make_socket())),
    );
    task_inner
        .fs_info
        .insert(format!("socket{}", new_fd).to_string(), new_fd);
    Ok(new_fd)
}

/// 参考 https://man7.org/linux/man-pages/man2/bind.2.html
pub fn sys_bind(_sockfd: usize, _addr: *const u8, _addrlen: u32) -> SyscallRet {
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/getsockname.2.html
pub fn sys_getsockname(_sockfd: usize, _addr: *const u8, _addrlen: u32) -> SyscallRet {
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/getpeername.2.html
pub fn sys_getpeername(_sockfd: usize, _addr: *const u8, _addrlen: u32) -> SyscallRet {
    Err(SysErrNo::Default)
}

/// 参考 https://man7.org/linux/man-pages/man2/setsockopt.2.html
pub fn sys_setsockopt(
    _sockfd: usize,
    _level: u32,
    _optname: u32,
    _optcal: *const u8,
    _optlen: u32,
) -> SyscallRet {
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/sendto.2.html
pub fn sys_sendto(
    _sockfd: usize,
    _buf: *const u8,
    _len: usize,
    _flags: u32,
    _dest_addr: *const u8,
    _addrlen: u32,
) -> SyscallRet {
    Ok(1)
}

/// 参考 https://man7.org/linux/man-pages/man2/recvfrom.2.html
pub fn sys_recvfrom(
    _sockfd: usize,
    buf: *mut u8,
    _len: usize,
    _flags: u32,
    _src_addr: *const u8,
    _addrlen: u32,
) -> SyscallRet {
    data_flow!({
        *buf = b'x';
        *buf.add(1) = b'0';
    });
    Ok(1)
}

/// 参考 https://man7.org/linux/man-pages/man2/listen.2.html
pub fn sys_listen(_sockfd: usize, _backlog: u32) -> SyscallRet {
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/connect.2.html
pub fn sys_connect(_sockfd: usize, _addr: *const u8, _addrlen: u32) -> SyscallRet {
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/accept.2.html
pub fn sys_accept(_sockfd: usize, _addr: *const u8, _addrlen: u32) -> SyscallRet {
    Ok(0)
}

pub fn sys_accept4(_sockfd: usize, _addr: *const u8, _addrlen: u32, _flags: u32) -> SyscallRet {
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/sendmsg.2.html
pub fn sys_sendmsg(_sockfd: usize, _addr: *const u8, _flags: u32) -> SyscallRet {
    Ok(0)
}

pub fn sys_socketpair(domain: u32, stype: u32, protocol: u32, sv: *mut u32) -> SyscallRet {
    debug!(
        "[sys_socketpair] domain is {}, type is {}, protocol is {}, sv is {}",
        domain, stype, protocol, sv as usize
    );

    let task = current_task().unwrap();
    let inner = task.inner_lock();

    let (socket1, socket2) = make_socketpair();
    let close_on_exec = (stype & 0o2000000) == 0o2000000;
    let non_block = (stype & 0o4000) == 0o4000;
    let mut flags = OpenFlags::empty();
    if close_on_exec {
        flags |= OpenFlags::O_CLOEXEC;
    }
    if non_block {
        flags |= OpenFlags::O_NONBLOCK;
    }

    let new_fd1 = inner.fd_table.alloc_fd()?;
    inner
        .fd_table
        .set(new_fd1, FileDescriptor::new(flags, FileClass::Abs(socket1)));
    inner
        .fs_info
        .insert(format!("socket{}", new_fd1).to_string(), new_fd1);

    let new_fd2 = inner.fd_table.alloc_fd()?;
    inner
        .fd_table
        .set(new_fd2, FileDescriptor::new(flags, FileClass::Abs(socket2)));
    inner
        .fs_info
        .insert(format!("socket{}", new_fd2).to_string(), new_fd2);

    data_flow!({
        *sv = new_fd1 as u32;
        *sv.add(1) = new_fd2 as u32;
    });

    Ok(0)
}
