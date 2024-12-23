#![no_std]
#![feature(linkage)]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![allow(dead_code)]
#[macro_use]
pub mod console;
mod lang_items;
mod syscall;

extern crate alloc;
#[macro_use]
extern crate bitflags;

use buddy_system_allocator::LockedHeap;
use syscall::*;

const USER_HEAP_SIZE: usize = 32768;

static mut HEAP_SPACE: [u8; USER_HEAP_SIZE] = [0; USER_HEAP_SIZE];

#[global_allocator]
static HEAP: LockedHeap = LockedHeap::empty();

#[alloc_error_handler]
pub fn handle_alloc_error(layout: core::alloc::Layout) -> ! {
    panic!("Heap allocation error, layout = {:?}", layout);
}

#[no_mangle]
#[link_section = ".text.entry"]
pub extern "C" fn _start() -> ! {
    unsafe {
        HEAP.lock()
            .init(HEAP_SPACE.as_ptr() as usize, USER_HEAP_SIZE);
    }
    exit(main());
}

#[linkage = "weak"]
#[no_mangle]
fn main() -> i32 {
    panic!("Cannot find main!");
}

// 定义一份打开文件的标志
bitflags! {
    pub struct OpenFlags: u32 {
        // reserve 3 bits for the access mode
        const O_RDONLY      = 0;
        const O_WRONLY      = 1;
        const O_RDWR        = 2;
        const O_ACCMODE     = 3;
        const O_CREATE      = 0o100;
        const O_EXCL        = 0o200;
        const O_NOCTTY      = 0o400;
        const O_TRUNC       = 0o1000;
        const O_APPEND      = 0o2000;
        const O_NONBLOCK    = 0o4000;
        const O_DSYNC       = 0o10000;
        const O_SYNC        = 0o4010000;
        const O_RSYNC       = 0o4010000;
        const O_DIRECTROY   = 0o200000;
        const O_NOFOLLOW    = 0o400000;
        const O_CLOEXEC     = 0o2000000;    //描述符标志

        const O_ASYNC       = 0o20000;
        const O_DIRECT      = 0o40000;
        const O_LARGEFILE   = 0o100000;
        const O_NOATIME     = 0o1000000;
        const O_PATH        = 0o10000000;
        const O_TMPFILE     = 0o20200000;
    }
}

pub fn openat(fd: isize, path: &str, flags: OpenFlags, mode: u32) -> isize {
    sys_openat(fd, path, flags.bits, mode)
}
pub fn close(fd: usize) -> isize {
    sys_close(fd)
}
pub fn read(fd: usize, buf: &mut [u8], count: usize) -> isize {
    sys_read(fd, buf, count)
}
pub fn write(fd: usize, buf: &[u8], count: usize) -> isize {
    sys_write(fd, buf, count)
}
pub fn exit(exit_code: i32) -> ! {
    sys_exit(exit_code);
}
pub fn yield_() -> isize {
    sys_yield()
}
pub fn shutdown() -> isize {
    println!("shutdown!");
    sys_shutdown()
}

/// 返回ms
pub fn get_time() -> usize {
    let mut time_sepc: [usize; 2] = [0, 0];
    unsafe {
        sys_gettimeofday(core::slice::from_raw_parts_mut(
            &mut time_sepc as *mut usize as *mut u8,
            2 * core::mem::size_of::<usize>(),
        ));
    }
    return time_sepc[0] * 1000 + time_sepc[1] / 1000_000;
}
pub fn getpid() -> isize {
    sys_getpid()
}
pub fn fork() -> isize {
    sys_fork()
}
pub fn exec(path: &str) -> isize {
    sys_exec(path)
}
pub fn execve(args: &[&str]) -> isize {
    sys_execve(args)
}
pub fn run_busyboxsh() -> isize {
    sys_busyboxsh()
}
pub fn run_finaltest() -> isize {
    sys_finaltest()
}
pub fn wait(exit_code: &mut i32) -> isize {
    sys_waitpid(-1, exit_code as *mut _, 0)
}

pub fn run_libc_bench() -> isize {
    sys_run_libc_bench()
}

pub fn run_splice() -> isize {
    sys_run_splice()
}

pub fn run_lmbench_test() -> isize {
    sys_run_lmbench_test()
}

pub fn waitpid(pid: usize, exit_code: &mut i32) -> isize {
    sys_waitpid(pid as isize, exit_code as *mut _, 0)
}

#[derive(Debug)]
pub struct Timespec {
    pub tv_sec: usize,  //秒
    pub tv_nsec: usize, //纳秒
}

impl Timespec {
    pub fn new(sec: usize, nsec: usize) -> Self {
        Self {
            tv_sec: sec,
            tv_nsec: nsec,
        }
    }
    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, size) }
    }
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts_mut(self as *mut _ as *mut u8, size) }
    }
}

pub fn sleep(period_ms: usize) {
    let mut req: [usize; 2] = [0, period_ms * 1_000_000];
    let mut rem: [usize; 2] = [0, 0];
    unsafe {
        use ::core::slice::from_raw_parts_mut;
        use core::mem::size_of;
        sys_nanosleep(
            from_raw_parts_mut(&mut req as *mut usize as *mut _, size_of::<Timespec>()),
            from_raw_parts_mut(&mut rem as *mut usize as *mut _, size_of::<Timespec>()),
        );
    }
}

pub fn getcwd(buf: &mut [u8], size: usize) -> isize {
    sys_getcwd(buf, size)
}

pub fn dup(fd: usize) -> isize {
    sys_dup(fd)
}

pub fn dup2(old: usize, new: usize, _zero: usize) -> isize {
    sys_dup3(old, new)
}

pub fn chdir(path: &str) -> isize {
    sys_chdir(path)
}

pub fn mkdir(dirfd: isize, path: &str, mode: u32) -> isize {
    sys_mkdirat(dirfd, path, mode)
}

pub fn getdents(fd: usize, buf: &mut [u8], len: usize) -> isize {
    sys_getdents64(fd, buf, len)
}

pub fn link(oldfd: isize, oldpath: &str, newfd: isize, newpath: &str, flags: u32) -> isize {
    sys_linkat(oldfd, oldpath, newfd, newpath, flags)
}

pub fn unlink(dirfd: isize, path: &str, flags: OpenFlags) -> isize {
    sys_unlinkat(dirfd, path, flags.bits)
}

pub fn umount(special: &str, flags: u32) -> isize {
    sys_umount2(special, flags)
}

pub fn mount(special: &str, dir: &str, ftype: &str, flags: u32, data: &mut [u8]) -> isize {
    sys_mount(special, dir, ftype, flags, data)
}

pub fn fstat(fd: usize, kst: &mut [u8]) -> isize {
    sys_fstat(fd, kst)
}

pub fn pipe(fd: &mut [u32], zero: isize) -> isize {
    sys_pipe2(fd, zero)
}

pub fn getppid() -> isize {
    sys_getppid()
}

pub fn times(tms: &mut [u8]) -> isize {
    sys_times(tms)
}

bitflags! {
    pub struct Clockid: u32 {
        const CLOCK_REALTIME = 0;
        const CLOCK_MONOTONIC = 1 << 0;
        const CLOCK_PROCESS_CPUTIME_ID = 1 << 1;
    }
}

pub fn clock_gettime(clockid: Clockid, tp: &mut Timespec) -> isize {
    sys_clock_gettime(clockid.bits() as usize, tp.as_bytes_mut())
}

pub fn sched_getaffinity(pid: usize, mask: *mut usize) -> isize {
    sys_sched_getaffinity(pid, mask)
}

pub fn sched_setaffinity(pid: usize, mask: *const usize) -> isize {
    sys_sched_setaffinity(pid, mask)
}

#[derive(Debug)]
pub struct Sysinfo {
    pub uptime: usize,
    pub totalram: usize,
    pub procs: usize,
}

impl Sysinfo {
    pub fn new(newuptime: usize, newtotalram: usize, newprocs: usize) -> Self {
        Self {
            uptime: newuptime,
            totalram: newtotalram,
            procs: newprocs,
        }
    }
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts_mut(self as *mut _ as *mut u8, size) }
    }
}

pub fn sysinfo(info: &mut Sysinfo) -> isize {
    sys_sysinfo(info.as_bytes_mut())
}

const SA_NOCLDSTOP: usize = 1; /* Don't send SIGCHLD when children stop.  */
const SA_NOCLDWAIT: usize = 2; /* Don't create zombie on child death.  */
const SA_SIGINFO: usize = 4; /* Invoke signal-catching function with
                             :usize             three arguments instead of one.  */
const SA_ONSTACK: usize = 0x08000000; /* Use signal stack by using `sa_restorer'. */
const SA_RESTART: usize = 0x10000000; /* Restart syscall on signal return.  */
const SA_NODEFER: usize = 0x40000000; /* Don't automatically block the signal when
                                      :usize                 its handler is being executed.  */
const SA_RESETHAND: usize = 0x80000000; /* Reset to SIG_DFL on entry to handler.  */

pub struct SigAction {
    pub sa_handler: usize,
    pub sa_flags: usize,
    pub sa_mask: usize,
    pub sa_restore: usize,
}

impl SigAction {
    pub fn new(sa_handler: usize, sa_flags: usize, sa_mask: usize, sa_restore: usize) -> Self {
        Self {
            sa_handler,
            sa_flags,
            sa_mask,
            sa_restore,
        }
    }
}

pub fn sigaction(signum: usize, act: &SigAction, oldact: &mut SigAction) -> isize {
    sys_sigaction(signum, act as *const _ as usize, oldact as *mut _ as usize)
}

pub fn sigreturn() -> isize {
    sys_sigreturn()
}

pub fn kill(pid: usize, signum: usize) -> isize {
    sys_kill(pid, signum)
}

#[derive(Debug)]
pub struct Kstat {
    st_dev: u32,   // 包含文件的设备 ID
    st_ino: u64,   // 索引节点号
    st_mode: u32,  // 文件类型和模式
    st_nlink: u32, // 硬链接数
    st_uid: u32,   // 所有者的用户 ID
    st_gid: u32,   // 所有者的组 ID
    st_rdev: u32,  // 设备 ID（如果是特殊文件）
    __pad: u64,
    st_size: i64,    // 总大小，以字节为单位
    st_blksize: i64, // 文件系统 I/O 的块大小
    __pad2: i32,
    st_blocks: u64,     // 分配的 512B 块数
    st_atime_sec: i64,  // 上次访问时间
    st_atime_nsec: i64, // 上次访问时间（纳秒精度）
    st_mtime_sec: i64,  // 上次修改时间
    st_mtime_nsec: i64, // 上次修改时间（纳秒精度）
    st_ctime_sec: i64,  // 上次状态变化的时间
    st_ctime_nsec: i64, // 上次状态变化的时间（纳秒精度）
    __unused: [u32; 2],
}

impl Kstat {
    pub fn new() -> Self {
        Self {
            st_dev: 0,
            st_ino: 0,
            st_mode: 0,
            st_nlink: 1,
            st_uid: 0,
            st_gid: 0,
            st_rdev: 0,
            __pad: 0,
            st_size: 0,
            st_blksize: 0,
            __pad2: 0,
            st_blocks: 0,
            st_atime_sec: 0,
            st_atime_nsec: 0,
            st_mtime_sec: 0,
            st_mtime_nsec: 0,
            st_ctime_sec: 0,
            st_ctime_nsec: 0,
            __unused: [0; 2],
        }
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts_mut(self as *mut _ as *mut u8, size) }
    }
}

pub fn fstatat(dirfd: isize, path: &str, kst: &mut Kstat, flags: usize) -> isize {
    sys_fstatat(dirfd, path, kst.as_bytes_mut(), flags)
}

const FAT_SUPER_MAGIC: i64 = 0x4006;

#[derive(Debug)]
pub struct Statfs {
    f_type: i64,       // Type of filesystem
    f_bsize: i64,      // Optimal transfer block size
    f_blocks: i64,     // Total data blocks in filesystem
    f_bfree: i64,      // Free blocks in filesystem
    f_bavail: i64,     // Free blocks available to unprivileged user
    f_files: i64,      // Total inodes in filesystem
    f_ffree: i64,      // Free inodes in filesystem
    f_fsid: i64,       // Filesystem ID
    f_name_len: i64,   // Maximum length of filenames
    f_frsize: i64,     // Fragment size
    f_flags: i64,      // Mount flags of filesystem
    f_spare: [i64; 4], // Padding bytes
}

impl Statfs {
    pub fn new() -> Self {
        Self {
            f_type: FAT_SUPER_MAGIC,
            f_bsize: 512,
            f_blocks: 1048576,
            f_bfree: 1048576,
            f_bavail: 0,
            f_files: 131072,
            f_ffree: 131072,
            f_fsid: 0,
            f_name_len: 255,
            f_frsize: 0,
            f_flags: 0,
            f_spare: [0; 4],
        }
    }

    pub fn empty() -> Self {
        Self {
            f_type: 0,
            f_bsize: 0,
            f_blocks: 0,
            f_bfree: 0,
            f_bavail: 0,
            f_files: 0,
            f_ffree: 0,
            f_fsid: 0,
            f_name_len: 0,
            f_frsize: 0,
            f_flags: 0,
            f_spare: [0; 4],
        }
    }

    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const _ as *const u8, size) }
    }

    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts_mut(self as *mut _ as *mut u8, size) }
    }
}

pub fn statfs(statfs: &mut Statfs) -> isize {
    sys_statfs(statfs.as_bytes_mut())
}

bitflags! {
    pub struct FaccessatMode: u32 {
        const F_OK = 0;
        const X_OK = 1<<0;
        const W_OK = 1<<1;
        const R_OK = 1<<2;
    }
}

pub fn faccessat(dirfd: isize, path: &str, mode: FaccessatMode, flags: usize) -> isize {
    sys_faccessat(dirfd, path, mode.bits, flags)
}
pub fn lseek(fd: usize, offset: isize, whence: usize) -> isize {
    sys_lseek(fd, offset, whence)
}

pub fn fcntl(fd: usize, cmd: usize, arg: usize) -> isize {
    sys_fcntl(fd, cmd, arg)
}

pub fn sendfile(outfd: usize, infd: usize, offset_ptr: *const usize, count: usize) -> isize {
    sys_sendfile(outfd, infd, offset_ptr, count)
}

pub fn pread64(fd: usize, buf: &mut [u8], count: usize, offset: usize) -> isize {
    sys_pread64(fd, buf, count, offset)
}
pub fn pwrite64(fd: usize, buf: &[u8], count: usize, offset: usize) -> isize {
    sys_pwrite64(fd, buf, count, offset)
}

pub fn ftruncate(fd: usize, length: i32) -> isize {
    sys_ftruncate(fd, length)
}

pub fn fsync(fd: usize) -> isize {
    sys_fsync(fd)
}

pub fn sync() -> isize {
    sys_sync()
}

pub fn symlinkat(target: &str, newfd: isize, linkpath: &str) -> isize {
    sys_symlinkat(target, newfd, linkpath)
}

pub fn readlinkat(dirfd: isize, pathname: &str, buf: &mut [u8], bufsiz: usize) -> isize {
    sys_readlinkat(dirfd, pathname, buf, bufsiz)
}

#[derive(Debug)]
pub struct TimeVal {
    pub tv_sec: usize,  //秒
    pub tv_usec: usize, //微秒
}

impl TimeVal {
    pub fn new(sec: usize, usec: usize) -> Self {
        Self {
            tv_sec: sec,
            tv_usec: usec,
        }
    }
    pub fn as_bytes(&self) -> &[u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts(self as *const _ as usize as *const u8, size) }
    }
}

#[derive(Debug)]
pub struct Rusage {
    pub ru_utime: TimeVal,
    pub ru_stime: TimeVal,
    //unused but needed
    ru_maxrss: isize,
    ru_ixrss: isize,
    ru_idrss: isize,
    ru_isrss: isize,
    ru_minflt: isize,
    ru_majflt: isize,
    ru_nswap: isize,
    ru_inblock: isize,
    ru_oublock: isize,
    ru_msgsnd: isize,
    ru_msgrcv: isize,
    ru_nsignals: isize,
    ru_nvcsw: isize,
    ru_nivcsw: isize,
}

impl Rusage {
    pub fn new() -> Self {
        Self {
            ru_utime: TimeVal::new(0, 0),
            ru_stime: TimeVal::new(0, 0),
            ru_maxrss: 0,
            ru_ixrss: 0,
            ru_idrss: 0,
            ru_isrss: 0,
            ru_minflt: 0,
            ru_majflt: 0,
            ru_nswap: 0,
            ru_inblock: 0,
            ru_oublock: 0,
            ru_msgsnd: 0,
            ru_msgrcv: 0,
            ru_nsignals: 0,
            ru_nvcsw: 0,
            ru_nivcsw: 0,
        }
    }
    pub fn as_bytes_mut(&mut self) -> &mut [u8] {
        let size = core::mem::size_of::<Self>();
        unsafe { core::slice::from_raw_parts_mut(self as *mut _ as *mut u8, size) }
    }
}

pub fn getrusage(who: isize, usage: &mut Rusage) -> isize {
    sys_getrusage(who, usage.as_bytes_mut())
}

pub fn renameat2(
    olddirfd: isize,
    oldpath: &str,
    newdirfd: isize,
    newpath: &str,
    flags: u32,
) -> isize {
    sys_renameat2(olddirfd, oldpath, newdirfd, newpath, flags)
}

pub fn copy_file_range(
    infd: usize,
    off_in: *const usize,
    outfd: usize,
    off_out: *const usize,
    count: usize,
    flags: u32,
) -> isize {
    sys_copy_file_range(infd, off_in, outfd, off_out, count, flags)
}

pub fn getrandom(buf: &mut [u8], buflen: usize, flags: u32) -> isize {
    sys_getrandom(buf, buflen, flags)
}

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct PollFd {
    /// 等待的 fd
    pub fd: i32,
    /// 等待的事件
    pub events: PollEvents,
    /// 返回的事件
    pub revents: PollEvents,
}

impl PollFd {
    pub fn new() -> Self {
        Self {
            fd: 0,
            events: PollEvents::empty(),
            revents: PollEvents::empty(),
        }
    }
}

bitflags! {
    //表示对应在文件上等待或者发生过的事件
    pub struct PollEvents: u16 {
        /// 可读
        const IN = 0x0001;
        /// 可写
        const OUT = 0x0004;
        /// 报错
        const ERR = 0x0008;
        /// 已终止，如 pipe 的另一端已关闭连接的情况
        const HUP = 0x0010;
        /// 无效的 fd
        const INVAL = 0x0020;
    }
}

pub fn ppoll(fds_ptr: &mut [PollFd], nfds: usize, tmo_p: &Timespec, mask: usize) -> isize {
    sys_ppoll(fds_ptr.as_mut_ptr() as usize, nfds, tmo_p.as_bytes(), mask)
}
