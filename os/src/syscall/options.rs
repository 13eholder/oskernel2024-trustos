/// 存放系统调用的各种Option
use crate::mm::MapPermission;
use num_enum::FromPrimitive;

bitflags! {
    pub struct WaitOption:u8{
        const DEFAULT = 0;
        const WNOHANG = 1;
        const WUNTRACED = 2;
        const WCONTINUED = 8;
    }
}

bitflags! {
    /// Open file flags
    pub struct CloneFlags: u32 {
        ///
        const SIGCHLD = (1 << 4) | (1 << 0);
        ///set if VM shared between processes
        const CLONE_VM = 1 << 8;
        ///set if fs info shared between processes
        const CLONE_FS = 1 << 9;
        ///set if open files shared between processes
        const CLONE_FILES = 1 << 10;
        ///set if signal handlers and blocked signals shared
        const CLONE_SIGHAND = 1 << 11;
        ///set if a pidfd should be placed in parent
        const CLONE_PIDFD = 1 << 12;
        ///set if we want to let tracing continue on the child too
        const CLONE_PTRACE = 1 << 13;
        ///set if the parent wants the child to wake it up on mm_release
        const CLONE_VFORK = 1 << 14;
        ///set if we want to have the same parent as the cloner
        const CLONE_PARENT = 1 << 15;
        ///Same thread group?
        const CLONE_THREAD = 1 << 16;
        ///New mount namespace group
        const CLONE_NEWNS = 1 << 17;
        ///share system V SEM_UNDO semantics
        const CLONE_SYSVSEM = 1 << 18;
        ///create a new TLS for the child
        const CLONE_SETTLS = 1 << 19;
        ///set the TID in the parent
        const CLONE_PARENT_SETTID = 1 << 20;
        ///clear the TID in the child
        const CLONE_CHILD_CLEARTID = 1 << 21;
        ///Unused, ignored
        const CLONE_DETACHED = 1 << 22;
        ///set if the tracing process can't force CLONE_PTRACE on this clone
        const CLONE_UNTRACED = 1 << 23;
        ///set the TID in the child
        const CLONE_CHILD_SETTID = 1 << 24;
        ///New cgroup namespace
        const CLONE_NEWCGROUP = 1 << 25;
        ///New utsname namespace
        const CLONE_NEWUTS = 1 << 26;
        ///New ipc namespace
        const CLONE_NEWIPC = 1 << 27;
        /// New user namespace
        const CLONE_NEWUSER = 1 << 28;
        ///New pid namespace
        const CLONE_NEWPID = 1 << 29;
        ///New network namespace
        const CLONE_NEWNET = 1 << 30;
        ///Clone io context
        const CLONE_IO = 1 << 31;
    }
}

impl CloneFlags {
    pub fn is_fork(&self) -> bool {
        self.contains(CloneFlags::SIGCHLD)
    }
}

// For Mmap
bitflags! {
    /// Mmap permissions
    pub struct MmapProt: u32 {
        /// None
        const PROT_NONE = 0;
        /// Readable
        const PROT_READ = 1 << 0;
        /// Writable
        const PROT_WRITE = 1 << 1;
        /// Executable
        const PROT_EXEC = 1 << 2;
    }
}

impl From<MmapProt> for MapPermission {
    fn from(prot: MmapProt) -> Self {
        let mut map_permission = MapPermission::U;
        if prot.contains(MmapProt::PROT_READ) {
            map_permission |= MapPermission::R;
        }
        if prot.contains(MmapProt::PROT_WRITE) {
            map_permission |= MapPermission::W;
        }
        if prot.contains(MmapProt::PROT_EXEC) {
            map_permission |= MapPermission::X;
        }
        map_permission
    }
}

bitflags! {
    /// Mmap flags
    pub struct MmapFlags: u32 {
        /// Shared
        const MAP_SHARED = 1;
        /// Private
        const MAP_PRIVATE = 1 << 1;
        /// Fixed
        const MAP_FIXED = 1 << 4;
        /// Anonymous
        const MAP_ANONYMOUS = 1 << 5;
        /// Compatity
        const MAP_DENYWRITE = 1 << 11;
        /// Stack
        const MAP_STACK = 1 << 17;
    }
}

#[repr(usize)]
#[allow(non_camel_case_types)]
#[derive(Debug, PartialEq, FromPrimitive)]
pub enum IoctlCommand {
    // For struct termios
    /// Gets the current serial port settings.
    TCGETS = 0x5401,
    /// Sets the serial port settings immediately.
    TCSETS = 0x5402,
    /// Sets the serial port settings after allowing the input and output buffers to drain/empty.
    TCSETSW = 0x5403,
    /// Sets the serial port settings after flushing the input and output buffers.
    TCSETSF = 0x5404,
    /// For struct termio
    /// Gets the current serial port settings.
    TCGETA = 0x5405,
    /// Sets the serial port settings immediately.
    TCSETA = 0x5406,
    /// Sets the serial port settings after allowing the input and output buffers to drain/empty.
    TCSETAW = 0x5407,
    /// Sets the serial port settings after flushing the input and output buffers.
    TCSETAF = 0x5408,
    /// Get the process group ID of the foreground process group on this terminal.
    TIOCGPGRP = 0x540F,
    /// Set the foreground process group ID of this terminal.
    TIOCSPGRP = 0x5410,
    /// Get window size.
    TIOCGWINSZ = 0x5413,
    /// Set window size.
    TIOCSWINSZ = 0x5414,
    /// Non-cloexec
    FIONCLEX = 0x5450,
    /// Cloexec
    FIOCLEX = 0x5451,
    /// rustc using pipe and ioctl pipe file with this request id
    /// for non-blocking/blocking IO control setting
    FIONBIO = 0x5421,
    /// Read time
    RTC_RD_TIME = 0x80247009,
    #[num_enum(default)]
    Default = 0,
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

pub struct Utsname {
    pub sysname: [u8; 65],
    pub nodename: [u8; 65],
    pub release: [u8; 65],
    pub version: [u8; 65],
    pub machine: [u8; 65],
    pub domainname: [u8; 65],
}

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct Iovec {
    /// user space buf starting address
    pub iov_base: usize,
    /// number of bytes to transfer
    pub iov_len: usize,
}
unsafe impl Send for Iovec {}
unsafe impl Sync for Iovec {}

// rlimit
#[allow(unused)]
#[repr(C)]
#[derive(Clone, Copy, Debug)]
pub struct RLimit {
    pub rlim_cur: usize, /* Soft limit */
    pub rlim_max: usize, /* Hard limit (ceiling for rlim_cur) */
}

bitflags! {
    pub struct SignalMaskFlag: u32 {
        const SIG_BLOCK = 0;
        const SIG_UNBLOCK = 1;
        const SIG_SETMASK = 2;
    }
}

bitflags! {
    pub struct FcntlCmd:usize{
        const F_DUPFD = 0;
        const F_GETFD = 1;
        const F_SETFD = 2;
        const F_GETFL = 3;
        const F_SETFL= 4;
        const F_DUPFD_CLOEXEC= 1030;
    }
}

pub const FD_SET_SIZE: usize = 1024;
pub const FD_SET_LEN: usize = FD_SET_SIZE / (8 * core::mem::size_of::<usize>());

/// FdSet为大小为1024的位图，分装在16个大小为usize的fds_bits子位图中，每一个位都代表一个文件描述符
#[derive(Debug, Copy, Clone)]
#[repr(C)]
pub struct FdSet {
    pub fds_bits: [usize; FD_SET_LEN],
}

impl FdSet {
    pub fn clear_all(&mut self) {
        self.fds_bits.fill(0);
    }
    pub fn got_fd(&mut self, fd: usize) -> bool {
        assert!(fd < FD_SET_SIZE);
        let offset = fd % FD_SET_LEN;
        (self.fds_bits[fd / FD_SET_LEN] & (1 << offset)) != 0
    }
    pub fn mark_fd(&mut self, fd: usize, value: bool) {
        if fd >= FD_SET_SIZE {
            return;
        }
        let offset = fd % FD_SET_LEN;
        if value {
            self.fds_bits[fd / FD_SET_LEN] |= 1 << offset;
        } else {
            self.fds_bits[fd / FD_SET_LEN] &= !(1 << offset);
        }
    }
}

bitflags! {
    pub struct FutexCmd:u32{
        const FUTEX_WAIT = 0;
        const FUTEX_WAKE = 1;
        const FUTEX_REQUEUE = 3;
    }
}

bitflags! {
pub struct FutexOpt: u32 {
    const FUTEX_PRIVATE_FLAG = 128;
    const FUTEX_CLOCK_REALTIME = 256;
}
}

bitflags! {
    pub struct FaccessatFileMode : u32 {
        const S_ISUID = 0o04000;
        const S_ISGID = 0o02000;
        const S_ISVTX = 0o01000;

        const S_IRUSR = 0o0400;
        const S_IWUSR = 0o0200;
        const S_IXUSR = 0o0100;
        const S_IRWXU = 0o0700;
        const S_IRGRP = 0o0040;
        const S_IWGRP = 0o0020;
        const S_IXGRP = 0o0010;
        const S_IRWXG = 0o0070;
        const S_IROTH = 0o0004;
        const S_IWOTH = 0o0002;
        const S_IXOTH = 0o0001;
        const S_IRWXO = 0o0007;
    }
}

bitflags! {
    pub struct FaccessatMode: u32 {
        const F_OK = 0;
        const X_OK = 1;
        const W_OK = 2;
        const R_OK = 4;
    }
}
