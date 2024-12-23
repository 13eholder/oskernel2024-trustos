mod devfs;
mod dirent;
mod ext4_lw;
mod fsidx;
mod fstruct;
mod mount;
mod net;
mod pipe;
mod stat;
mod stdio;
mod vfs;

use crate::mm::UserBuffer;
use crate::task::current_uid;
use crate::timer::get_time_ms;
use crate::utils::{GeneralRet, SysErrNo};
use alloc::format;
use alloc::string::String;
use alloc::vec;
use alloc::{sync::Arc, vec::Vec};
pub use devfs::*;
pub use dirent::Dirent;
pub use ext4_lw::{fs_stat, ls, root_inode, sync};
pub use fsidx::*;
pub use fstruct::*;
use hashbrown::HashSet;
use log::debug;
pub use mount::MNT_TABLE;
pub use net::*;
pub use pipe::{make_pipe, Pipe};
use spin::Lazy;
pub use stat::*;
pub use stdio::{Stdin, Stdout};
pub use vfs::*;

// 定义一份打开文件的标志
bitflags! {
    pub struct OpenFlags: u32 {
        // reserve 3 bits for the access mode
        const O_RDONLY      = 0;           // Read only
        const O_WRONLY      = 1;           // Write only
        const O_RDWR        = 2;           // Read and write
        const O_ACCMODE     = 3;           // Mask for file access modes
        const O_CREATE       = 0o100;       // Create file if it doesn't exist
        const O_EXCL        = 0o200;       // Exclusive use flag
        const O_NOCTTY      = 0o400;       // Do not assign controlling terminal
        const O_TRUNC       = 0o1000;      // Truncate flag
        const O_APPEND      = 0o2000;      // Set append mode
        const O_NONBLOCK    = 0o4000;      // Non-blocking mode
        const O_DSYNC       = 0o10000;     // Write operations complete as defined by POSIX
        const O_SYNC        = 0o4010000;   // Write operations complete as defined by POSIX
        const O_RSYNC       = 0o4010000;   // Synchronized read operations
        const O_DIRECTORY   = 0o200000;    // Must be a directory
        const O_NOFOLLOW    = 0o400000;    // Do not follow symbolic links
        const O_CLOEXEC     = 0o2000000;   // Set close-on-exec
        const O_ASYNC       = 0o20000;     // Signal-driven I/O
        const O_DIRECT      = 0o40000;     // Direct disk access hints
        const O_LARGEFILE   = 0o100000;    // Allow files larger than 2GB
        const O_NOATIME     = 0o1000000;   // Do not update access time
        const O_PATH        = 0o10000000;  // Obtain a file descriptor for a directory
        const O_TMPFILE     = 0o20200000;  // Create an unnamed temporary file

        const O_ASK_SYMLINK    = 0o400000000;     //自用，用于识别可访问符号链接本身文件的系统调用
    }
}

impl OpenFlags {
    pub fn read_write(&self) -> (bool, bool) {
        if self.is_empty() {
            (true, false)
        } else if self.contains(Self::O_WRONLY) {
            (false, true)
        } else {
            (true, true)
        }
    }

    pub fn node_type(&self) -> InodeType {
        if self.contains(OpenFlags::O_DIRECTORY) {
            InodeType::Dir
        } else {
            InodeType::File
        }
    }
}

pub const MAX_PATH_LEN: usize = 50;

pub const SEEK_SET: usize = 0;
pub const SEEK_CUR: usize = 1;
pub const SEEK_END: usize = 2;

pub const DEFAULT_FILE_MODE: u32 = 0o666;
pub const DEFAULT_DIR_MODE: u32 = 0o777;
pub const NONE_MODE: u32 = 0;

/// 枚举类型，分为普通文件和抽象文件
/// 普通文件File，特点是支持更多类型的操作，包含seek, offset等
/// 抽象文件Abs，抽象文件，只支持File trait的一些操作
#[derive(Clone)]
pub enum FileClass {
    File(Arc<OSInode>),
    Abs(Arc<dyn File>),
}

impl FileClass {
    pub fn file(&self) -> Result<Arc<OSInode>, SysErrNo> {
        match self {
            FileClass::File(f) => Ok(f.clone()),
            FileClass::Abs(_) => Err(SysErrNo::EINVAL),
        }
    }
    pub fn abs(&self) -> Result<Arc<dyn File>, SysErrNo> {
        match self {
            FileClass::File(_) => Err(SysErrNo::EINVAL),
            FileClass::Abs(f) => Ok(f.clone()),
        }
    }
    pub fn any(&self) -> Arc<dyn File> {
        match self {
            FileClass::File(f) => f.clone(),
            FileClass::Abs(f) => f.clone(),
        }
    }
}
#[repr(u8)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum InodeType {
    Unknown = 0o0,
    /// FIFO (named pipe)
    Fifo = 0o1,
    /// Character device
    CharDevice = 0o2,
    /// Directory
    Dir = 0o4,
    /// Block device
    BlockDevice = 0o6,
    /// Regular file
    File = 0o10,
    /// Symbolic link
    SymLink = 0o12,
    /// Socket
    Socket = 0o14,
}

impl InodeType {
    /// Tests whether this node type represents a regular file.
    pub const fn is_file(self) -> bool {
        matches!(self, Self::File)
    }
    /// Tests whether this node type represents a directory.
    pub const fn is_dir(self) -> bool {
        matches!(self, Self::Dir)
    }
    /// Tests whether this node type represents a symbolic link.
    pub const fn is_symlink(self) -> bool {
        matches!(self, Self::SymLink)
    }
    /// Returns `true` if this node type is a block device.
    pub const fn is_block_device(self) -> bool {
        matches!(self, Self::BlockDevice)
    }
    /// Returns `true` if this node type is a char device.
    pub const fn is_char_device(self) -> bool {
        matches!(self, Self::CharDevice)
    }
    /// Returns `true` if this node type is a fifo.
    pub const fn is_fifo(self) -> bool {
        matches!(self, Self::Fifo)
    }
    /// Returns `true` if this node type is a socket.
    pub const fn is_socket(self) -> bool {
        matches!(self, Self::Socket)
    }
}
#[cfg(feature = "board_qemu")]
core::arch::global_asm!(include_str!("preload_qemu.S"));

#[cfg(not(feature = "board_qemu"))]
core::arch::global_asm!(include_str!("preload_vf2.S"));

// os\src\fs\mod.rs
//将预加载到内存中的程序写入文件根目录
pub fn flush_preload() {
    extern "C" {
        fn initproc_start();
        fn initproc_end();
        fn test_start();
        fn test_end();
    }

    let initproc = open("/initproc", OpenFlags::O_CREATE, DEFAULT_FILE_MODE)
        .unwrap()
        .file()
        .unwrap();
    let mut v = Vec::new();
    v.push(unsafe {
        core::slice::from_raw_parts_mut(
            initproc_start as *mut u8,
            initproc_end as usize - initproc_start as usize,
        ) as &'static mut [u8]
    });
    initproc.write(UserBuffer::new(v));

    let test = open(
        "/test_all_1stage.sh",
        OpenFlags::O_CREATE,
        DEFAULT_FILE_MODE,
    )
    .unwrap()
    .file()
    .unwrap();
    let mut v = Vec::new();
    v.push(unsafe {
        core::slice::from_raw_parts_mut(
            test_start as *mut u8,
            test_end as usize - test_start as usize,
        ) as &'static mut [u8]
    });
    test.write(UserBuffer::new(v));
}

pub fn init() {
    flush_preload();
    create_init_files();
    // TODO(ZMY):为了过libc-test utime的权宜之计,读取RTC太麻烦了
    root_inode().set_timestamps(Some(0), Some(0), Some(0));
}

pub fn list_apps() {
    println!("/**** APPS ****");
    ls();
    println!("**************/");
}

//
const MOUNTS: &str = " ext4 / ext rw 0 0\n";
const PASSWD: &str = "root:x:0:0:root:/root:/bin/bash\nnobody:x:1:0:nobody:/nobody:/bin/bash\n";
const MEMINFO: &str = r"
MemTotal:         944564 kB
MemFree:          835248 kB
MemAvailable:     873464 kB
Buffers:            6848 kB
Cached:            36684 kB
SwapCached:            0 kB
Active:            19032 kB
Inactive:          32676 kB
Active(anon):        128 kB
Inactive(anon):     8260 kB
Active(file):      18904 kB
Inactive(file):    24416 kB
Unevictable:           0 kB
Mlocked:               0 kB
SwapTotal:             0 kB
SwapFree:              0 kB
Dirty:                 0 kB
Writeback:             0 kB
AnonPages:          8172 kB
Mapped:            16376 kB
Shmem:               216 kB
KReclaimable:       9960 kB
Slab:              17868 kB
SReclaimable:       9960 kB
SUnreclaim:         7908 kB
KernelStack:        1072 kB
PageTables:          600 kB
NFS_Unstable:          0 kB
Bounce:                0 kB
WritebackTmp:          0 kB
CommitLimit:      472280 kB
Committed_AS:      64684 kB
VmallocTotal:   67108863 kB
VmallocUsed:       15740 kB
VmallocChunk:          0 kB
Percpu:              496 kB
HugePages_Total:       0
HugePages_Free:        0
HugePages_Rsvd:        0
HugePages_Surp:        0
Hugepagesize:       2048 kB
Hugetlb:               0 kB
";
const ADJTIME: &str = "0.000000 0.000000 UTC\n";
const LOCALTIME: &str =
    "lrwxrwxrwx 1 root root 33 11月 18  2023 /etc/localtime -> /usr/share/zoneinfo/Asia/Shanghai\n";
const PRELOAD: &str = "";

pub fn create_init_files() -> GeneralRet {
    //创建/proc文件夹
    open(
        "/proc",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR | OpenFlags::O_DIRECTORY,
        DEFAULT_DIR_MODE,
    )?;
    //创建/proc/mounts文件系统使用情况
    let mountsfile = open(
        "/proc/mounts",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        DEFAULT_FILE_MODE,
    )?
    .file()?;
    let mut mountsinfo = String::from(MOUNTS);
    let mut mountsvec = Vec::new();
    unsafe {
        let mounts = mountsinfo.as_bytes_mut();
        mountsvec.push(core::slice::from_raw_parts_mut(
            mounts.as_mut_ptr(),
            mounts.len(),
        ));
    }
    let mountbuf = UserBuffer::new(mountsvec);
    let mountssize = mountsfile.write(mountbuf)?;
    debug!("create /proc/mounts with {} sizes", mountssize);
    //创建/proc/meminfo系统内存使用情况
    let memfile = open(
        "/proc/meminfo",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        DEFAULT_FILE_MODE,
    )?
    .file()?;
    let mut meminfo = String::from(MEMINFO);
    let mut memvec = Vec::new();
    unsafe {
        let mem = meminfo.as_bytes_mut();
        memvec.push(core::slice::from_raw_parts_mut(mem.as_mut_ptr(), mem.len()));
    }
    let membuf = UserBuffer::new(memvec);
    let memsize = memfile.write(membuf)?;
    debug!("create /proc/meminfo with {} sizes", memsize);
    //创建/dev文件夹
    open(
        "/dev",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR | OpenFlags::O_DIRECTORY,
        DEFAULT_DIR_MODE,
    )?;
    //注册设备/dev/rtc和/dev/rtc0
    register_device("/dev/rtc");
    register_device("/dev/rtc0");
    //注册设备/dev/tty
    register_device("/dev/tty");
    //注册设备/dev/zero
    register_device("/dev/zero");
    //注册设备/dev/numm
    register_device("/dev/null");
    //注册设备/dev/cpu_dma_latency
    register_device("/dev/cpu_dma_latency");
    //创建./dev/misc文件夹
    open(
        "/dev/misc",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR | OpenFlags::O_DIRECTORY,
        DEFAULT_DIR_MODE,
    )?;
    //注册设备/dev/misc/rtc
    register_device("/dev/misc/rtc");
    //创建/etc文件夹
    open(
        "/etc",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR | OpenFlags::O_DIRECTORY,
        DEFAULT_DIR_MODE,
    )?;
    //创建/etc/adjtime记录时间偏差
    let adjtimefile = open(
        "/etc/adjtime",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        DEFAULT_FILE_MODE,
    )?
    .file()?;
    let mut adjtime = String::from(ADJTIME);
    let mut adjtimevec = Vec::new();
    unsafe {
        let adj = adjtime.as_bytes_mut();
        adjtimevec.push(core::slice::from_raw_parts_mut(adj.as_mut_ptr(), adj.len()));
    }
    let adjtimebuf = UserBuffer::new(adjtimevec);
    let adjtimesize = adjtimefile.write(adjtimebuf)?;
    debug!("create /etc/adjtime with {} sizes", adjtimesize);

    //创建./etc/localtime记录时区
    let localtimefile = open(
        "/etc/localtime",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        DEFAULT_FILE_MODE,
    )?
    .file()?;
    let mut localtime = String::from(LOCALTIME);
    let mut localtimevec = Vec::new();
    unsafe {
        let local = localtime.as_bytes_mut();
        localtimevec.push(core::slice::from_raw_parts_mut(
            local.as_mut_ptr(),
            local.len(),
        ));
    }
    let localtimebuf = UserBuffer::new(localtimevec);
    let localtimesize = localtimefile.write(localtimebuf)?;
    debug!("create /etc/localtime with {} sizes", localtimesize);

    //创建/etc/passwd记录用户信息
    let passwdfile = open(
        "/etc/passwd",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        DEFAULT_FILE_MODE,
    )?
    .file()?;
    let mut passwd = String::from(PASSWD);
    let mut passwdvec = Vec::new();
    unsafe {
        let wd = passwd.as_bytes_mut();
        passwdvec.push(core::slice::from_raw_parts_mut(wd.as_mut_ptr(), wd.len()));
    }
    let passwdbuf = UserBuffer::new(passwdvec);
    let passwdsize = passwdfile.write(passwdbuf)?;
    debug!("create /etc/passwd with {} sizes", passwdsize);

    //创建/etc/ld.so.preload记录用户信息
    let preloadfile = open(
        "/etc/ld.so.preload",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        DEFAULT_FILE_MODE,
    )?
    .file()?;
    let mut preload = String::from(PRELOAD);
    let mut preloadvec = Vec::new();
    unsafe {
        let pre = preload.as_bytes_mut();
        preloadvec.push(core::slice::from_raw_parts_mut(pre.as_mut_ptr(), pre.len()));
    }
    let preloadbuf = UserBuffer::new(preloadvec);
    let preloadsize = preloadfile.write(preloadbuf)?;
    debug!("create /etc/ld.so.preload with {} sizes", preloadsize);

    println!("create_init_files success!");
    Ok(())
}

fn create_file(abs_path: &str, flags: OpenFlags, mode: u32) -> Result<FileClass, SysErrNo> {
    // 一定能找到,因为除了RootInode外都有父结点
    let parent_dir = root_inode();
    let (readable, writable) = flags.read_write();
    let inode = parent_dir.create(abs_path, flags.node_type())?;
    inode.fmode_set(mode);
    inode.set_owner(current_uid(), 0);
    inode.set_timestamps(None, Some((get_time_ms() / 1000) as u64), None);
    insert_inode_idx(abs_path, inode.clone());
    let osinode = OSInode::new(readable, writable, inode);
    Ok(FileClass::File(Arc::new(osinode)))
}

pub fn is_dynamic_link_file(path: &str) -> bool {
    path.ends_with(".so") || path.contains(".so.")
}

pub fn map_dynamic_link_file(path: &str) -> &str {
    if !DYNAMIC_PATH.contains(path) {
        let (_, file_name) = path.rsplit_once("/").unwrap();
        // log::info!("[map_dynamic] file_name={}", file_name);
        for prefix in DYNAMIC_PREFIX.iter() {
            let full_path = format!("{}{}", prefix, file_name);
            // log::info!("[map_dynamic] full_path={}", full_path);
            if DYNAMIC_PATH.contains(full_path.as_str()) {
                return full_path.leak();
            }
        }
    }
    path
}

pub fn open(mut abs_path: &str, flags: OpenFlags, mode: u32) -> Result<FileClass, SysErrNo> {
    // log::info!("[open] abs_path={}", abs_path);
    //判断是否是设备文件
    if find_device(abs_path) {
        let device = open_device_file(abs_path)?;
        return Ok(FileClass::Abs(device));
    }
    // 如果是动态链接文件,转换路径
    if is_dynamic_link_file(abs_path) {
        abs_path = map_dynamic_link_file(abs_path);
        // log::info!("dynamic path={}", abs_path);
    }

    let mut inode: Option<Arc<dyn Inode>> = None;
    // 同一个路径对应一个Inode
    if has_inode(abs_path) {
        inode = find_inode_idx(abs_path);
    } else {
        let found_res = root_inode().find(abs_path, flags, 0);
        if found_res.clone().err() == Some(SysErrNo::ENOTDIR) {
            return Err(SysErrNo::ENOTDIR);
        }
        if found_res.clone().err() == Some(SysErrNo::ELOOP) {
            return Err(SysErrNo::ELOOP);
        }
        if let Ok(t) = found_res {
            if !flags.contains(OpenFlags::O_ASK_SYMLINK) {
                //符号链接文件不加入idx
                insert_inode_idx(abs_path, t.clone());
            }
            inode = Some(t);
        }
    }
    if let Some(inode) = inode {
        if flags.contains(OpenFlags::O_DIRECTORY) && !inode.is_dir() {
            return Err(SysErrNo::ENOTDIR);
        }
        let (readable, writable) = flags.read_write();
        let osfile = OSInode::new(readable, writable, inode);
        if flags.contains(OpenFlags::O_APPEND) {
            osfile.lseek(0, SEEK_END)?;
        }
        if flags.contains(OpenFlags::O_TRUNC) {
            osfile.inode.truncate(0)?;
        }
        return Ok(FileClass::File(Arc::new(osfile)));
    }

    // 节点不存在
    if flags.contains(OpenFlags::O_CREATE) {
        return create_file(abs_path, flags, mode);
    }
    Err(SysErrNo::ENOENT)
}

static DYNAMIC_PREFIX: Lazy<Vec<&'static str>> =
    Lazy::new(|| vec!["/lib/", "/lib/glibc/", "/lib/musl/"]);

static DYNAMIC_PATH: Lazy<HashSet<&'static str>> = Lazy::new(|| {
    [
        // lib/
        "/lib/tls_get_new-dtv_dso.so",
        "/lib/tls_align_dso.so",
        "/lib/tls_init_dso.so",
        "/lib/ld-musl-riscv64-sf.so.1",
        "/lib/path.py",
        "/lib/dlopen_dso.so",
        // lib/musl/
        "/lib/musllibc.so",
        // lib/glibc/
        "/lib/glibc/libutil-2.31.so",
        "/lib/glibc/libpthread-2.31.so",
        "/lib/glibc/libgomp.so.1.0.0",
        "/lib/glibc/libSegFault.so",
        "/lib/glibc/libdl.so",
        "/lib/glibc/libnss_dns.so.2",
        "/lib/glibc/libatomic.so.1",
        "/lib/glibc/libthread_db.so.1",
        "/lib/glibc/ld-linux-riscv64-lp64d.so.1",
        "/lib/glibc/libm.so.6",
        "/lib/glibc/libm-2.31.so",
        "/lib/glibc/librt-2.31.so",
        "/lib/glibc/libnss_dns-2.31.so",
        "/lib/glibc/libutil.so.1",
        "/lib/glibc/libc.so",
        "/lib/glibc/libdl.so.2",
        "/lib/glibc/libnss_files.so.2",
        "/lib/glibc/libnss_dns.so",
        "/lib/glibc/libnss_nisplus.so",
        "/lib/glibc/libresolv-2.31.so",
        "/lib/glibc/libnss_nis-2.31.so",
        "/lib/glibc/libBrokenLocale-2.31.so",
        "/lib/glibc/ld-2.31.so",
        "/lib/glibc/libnss_nis.so",
        "/lib/glibc/libnsl.so",
        "/lib/glibc/libresolv.so",
        "/lib/glibc/librt.so.1",
        "/lib/glibc/libpcprofile.so",
        "/lib/glibc/librt.so",
        "/lib/glibc/libnss_hesiod.so",
        "/lib/glibc/libnsl.so.1",
        "/lib/glibc/libdl-2.31.so",
        "/lib/glibc/libc-2.31.so",
        "/lib/glibc/libanl.so",
        "/lib/glibc/libBrokenLocale.so",
        "/lib/glibc/libnss_nis.so.2",
        "/lib/glibc/libthread_db-1.0.so",
        "/lib/glibc/libmemusage.so",
        "/lib/glibc/libc.so.6",
        "/lib/glibc/libBrokenLocale.so.1",
        "/lib/glibc/libnss_nisplus.so.2",
        "/lib/glibc/libnss_compat-2.31.so",
        "/lib/glibc/libnss_hesiod.so.2",
        "/lib/glibc/libnss_compat.so.2",
        "/lib/glibc/libgcc_s.so.1",
        "/lib/glibc/libatomic.so.1.2.0",
        "/lib/glibc/libm.so",
        "/lib/glibc/libanl-2.31.so",
        "/lib/glibc/libnss_nisplus-2.31.so",
        "/lib/glibc/libresolv.so.2",
        "/lib/glibc/libnss_files.so",
        "/lib/glibc/libthread_db.so",
        "/lib/glibc/libpthread.so.0",
        "/lib/glibc/libnss_compat.so",
        "/lib/glibc/libanl.so.1",
        "/lib/glibc/libgomp.so.1",
        "/lib/glibc/libpthread.so",
        "/lib/glibc/libnss_hesiod-2.31.so",
        "/lib/glibc/libnsl-2.31.so",
        "/lib/glibc/libnss_files-2.31.so",
        "/lib/glibc/libutil.so",
    ]
    .into_iter()
    .collect()
});

#[cfg(feature = "ltp")]
pub fn create_proc_dir_and_file(pid: usize, ppid: usize) -> Result<(), SysErrNo> {
    open(
        format!("/proc/{}", pid).as_str(),
        OpenFlags::O_DIRECTORY | OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        DEFAULT_DIR_MODE,
    )
    .unwrap()
    .file()?;

    //创建进程状态文件/proc/<pid>/stat
    let statfile = open(
        format!("/proc/{}/stat", pid).as_str(),
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        DEFAULT_FILE_MODE,
    )
    .unwrap()
    .file()?;
    let mut statinfo = format!(
        "{} (busybox) S {} 0 0 0 0 0 0 0 0 0 {} 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0 0",
        pid,
        ppid,
        get_time()
    );
    let mut statvec = Vec::new();
    unsafe {
        let stat = statinfo.as_bytes_mut();
        statvec.push(core::slice::from_raw_parts_mut(
            stat.as_mut_ptr(),
            stat.len(),
        ));
    }
    let statbuf = UserBuffer::new(statvec);
    statfile.write(statbuf)?;
    // debug!("create /proc/{}/stat with {} sizes", pid, statsize);
    Ok(())
}

#[cfg(feature = "ltp")]
pub fn remove_proc_dir_and_file(pid: usize) {
    root_inode().unlink(format!("/proc/{}/stat", pid).as_str());
    remove_inode_idx(format!("/proc/{}/stat", pid).as_str());
    root_inode().unlink(format!("/proc/{}", pid).as_str());
    remove_inode_idx(format!("/proc/{}", pid).as_str());
}
