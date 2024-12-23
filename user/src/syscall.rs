#![allow(dead_code)]
use core::arch::asm;

use alloc::vec::Vec;

const SYSCALL_GETCWD: usize = 17;
const SYSCALL_DUP: usize = 23;
const SYSCALL_DUP3: usize = 24;
const SYSCALL_FCNTL: usize = 25;
const SYSCALL_MKDIRAT: usize = 34;
const SYSCALL_UNLINKAT: usize = 35;
const SYSCALL_SYMLINKAT: usize = 36;
const SYSCALL_LINKAT: usize = 37;
const SYSCALL_UMOUNT2: usize = 39;
const SYSCALL_MOUNT: usize = 40;
const SYSCALL_STATFS: usize = 43;
const SYSCALL_FTRUNCATE: usize = 46;
const SYSCALL_FACCESSAT: usize = 48;
const SYSCALL_CHDIR: usize = 49;
const SYSCALL_OPENAT: usize = 56;
const SYSCALL_CLOSE: usize = 57;
const SYSCALL_PIPE2: usize = 59;
const SYSCALL_GETDENTS64: usize = 61;
const SYSCALL_LSEEK: usize = 62;
const SYSCALL_READ: usize = 63;
const SYSCALL_WRITE: usize = 64;
const SYSCALL_PREAD64: usize = 67;
const SYSCALL_PWRITE64: usize = 68;
const SYSCALL_SENDFILE: usize = 71;
const SYSCALL_PPOLL: usize = 73;
const SYSCALL_READLINKAT: usize = 78;
const SYSCALL_FSTATAT: usize = 79;
const SYSCALL_FSTAT: usize = 80;
const SYSCALL_SYNC: usize = 81;
const SYSCALL_FSYNC: usize = 82;
const SYSCALL_UTIMENSAT: usize = 88;
const SYSCALL_EXIT: usize = 93;
const SYSCALL_NANOSLEEP: usize = 101;
const SYSCALL_CLOCK_GETTIME: usize = 113;
const SYSCALL_SCHED_SETAFFINITY: usize = 122;
const SYSCALL_SCHED_GETAFFINITY: usize = 123;
const SYSCALL_SCHED_YIELD: usize = 124;
const SYSCALL_KILL: usize = 129;
const SYSCALL_SIGACTION: usize = 134;
const SYSCALL_SIGRETURN: usize = 139;
const SYSCALL_TIMES: usize = 153;
const SYSCALL_GETRUSAGE: usize = 165;
const SYSCALL_GETTIMEOFDAY: usize = 169;
const SYSCALL_GETPID: usize = 172;
const SYSCALL_GETPPID: usize = 173;
const SYSCALL_SYSINFO: usize = 179;
const SYSCALL_CLONE: usize = 220;
const SYSCALL_EXECVE: usize = 221;
const SYSCALL_WAIT4: usize = 260;
const SYSCALL_RENAMEAT2: usize = 276;
const SYSCALL_GETRANDOM: usize = 278;
const SYSCALL_COPYFILERANGE: usize = 285;

const SYSCALL_SHUTDOWN: usize = 1000;
const SYSCALL_STRACE: usize = 2000;

fn syscall(id: usize, args: [isize; 6]) -> isize {
    let mut ret: isize;
    unsafe {
        asm!(
            "ecall",
            inlateout("x10") args[0] => ret,
            in("x11") args[1],
            in("x12") args[2],
            in("x13") args[3],
            in("x14") args[4],
            in("x15") args[5],
            in("x17") id
        );
    }
    ret
}

pub fn sys_openat(fd: isize, path: &str, flags: u32, mode: u32) -> isize {
    syscall(
        SYSCALL_OPENAT,
        [
            fd,
            path.as_ptr() as isize,
            flags as isize,
            mode as isize,
            0,
            0,
        ],
    )
}

pub fn sys_close(fd: usize) -> isize {
    syscall(SYSCALL_CLOSE, [fd as isize, 0, 0, 0, 0, 0])
}

pub fn sys_read(fd: usize, buffer: &mut [u8], count: usize) -> isize {
    syscall(
        SYSCALL_READ,
        [
            fd as isize,
            buffer.as_mut_ptr() as isize,
            count as isize,
            0,
            0,
            0,
        ],
    )
}

pub fn sys_write(fd: usize, buffer: &[u8], count: usize) -> isize {
    syscall(
        SYSCALL_WRITE,
        [
            fd as isize,
            buffer.as_ptr() as isize,
            count as isize,
            0,
            0,
            0,
        ],
    )
}

pub fn sys_exit(exit_code: i32) -> ! {
    syscall(SYSCALL_EXIT, [exit_code as isize, 0, 0, 0, 0, 0]);
    panic!("sys_exit never returns!");
}

pub fn sys_yield() -> isize {
    syscall(SYSCALL_SCHED_YIELD, [0, 0, 0, 0, 0, 0])
}

pub fn sys_shutdown() -> isize {
    syscall(SYSCALL_SHUTDOWN, [0, 0, 0, 0, 0, 0])
}

pub fn sys_gettimeofday(ts: &mut [u8]) -> isize {
    syscall(
        SYSCALL_GETTIMEOFDAY,
        [ts.as_mut_ptr() as isize, 0, 0, 0, 0, 0],
    )
}

pub fn sys_getpid() -> isize {
    syscall(SYSCALL_GETPID, [0, 0, 0, 0, 0, 0])
}

pub fn sys_fork() -> isize {
    const SIGCHLD: usize = 17;
    syscall(SYSCALL_CLONE, [SIGCHLD as isize, 0, 0, 0, 0, 0])
}

pub fn sys_exec(path: &str) -> isize {
    syscall(SYSCALL_EXECVE, [path.as_ptr() as isize, 0, 0, 0, 0, 0])
}

pub fn sys_execve(args: &[&str]) -> isize {
    let mut v: Vec<isize> = args.iter().map(|s| s.as_ptr() as isize).collect();
    v.push(0);
    syscall(
        SYSCALL_EXECVE,
        [
            args[0].as_ptr() as isize,
            v.as_mut_slice().as_ptr() as isize,
            0,
            0,
            0,
            0,
        ],
    )
}
// "busybox_testcode.sh\0".as_ptr() as isize,

//已通过测例：abort01，abs01,access01,access03,access04,brk01,brk02,chdir04,clock_getres01
//          clock_gettime01,clock_gettime02,clock_nanosleep01,clone01,clone04,close01
//          close02,copy_file_range03,dup01,dup02,dup03,dup04,dup07,dup201,dup202,dup203,
//          dup204,dup205,dup206,dup207,dup3_01,dup3_02,exit02,faccessat01,faccessat02,fchmodat01
//          fchmodat02,fcntl03,fcntl04,fcntl08,fcntl12,fstat03,ftruncate01,getcwd01,getpagesize01
//          getppid02,getrandom01,getrandom02,getrandom03,getrandom04,getrandom05,getrusage01
//          getrusage02,gettid02,gettimeofday01,gettimeofday02,getuid01
//          fchmod01,fchmod04,lseek01,lseek07,madvise01,memcpy01,memset01,mkdir05,mmap02,mmap05,mmap06，
//          mmap08,mmap09，mmap19,mprotect05,nanosleep02,nanosleep04,open01,open03,open04,open09,pipe01
//          pipe03,pipe06,pipe08,pipe10,pipe11,pipe13,pipe14,pipe2_01,poll01,pread01,pread02,pselect03
//          pwrite01,pwrite02,pwrite03,read01,read02,read04,readdir01,readv01,readv02,select03
//          sendfile02,sendfile03,sendfile05,sendfile06,sendfile08,set_tid_address01,setitimer01
//          setitimer02,sigaction01,stat01,stat02,string01,symlink02,symlink04,syscall01,time01
//          times01,uname01,uname02,wait01,wait02,wait401,waitpid01,waitpid03,waitpid04，write02
//          write03，write05，writev01
//内存不够的测例：write01
//单独运行方法：修改 ltp/testcases/bin/{} 括号内的内容为目标文件
//全部运行方法：运行ltp_testcode.sh

pub fn sys_busyboxsh() -> isize {
    syscall(
        SYSCALL_EXECVE,
        [
            "/busybox\0".as_ptr() as isize,
            [
                "busybox\0".as_ptr() as isize,
                "sh\0".as_ptr() as isize,
                "splice-test-1\0".as_ptr() as isize,
                //"./test-ltp.sh\0".as_ptr() as isize,
                //"ltp/testcases/bin/abort01\0".as_ptr() as isize,
                // "libctest_testcode.sh\0".as_ptr() as isize,
                0,
            ]
            .as_ptr() as isize,
            0,
            0,
            0,
            0,
        ],
    )
}

pub fn sys_run_libc_bench() -> isize {
    syscall(
        SYSCALL_EXECVE,
        ["/libc-bench\0".as_ptr() as isize, 0, 0, 0, 0, 0],
    )
}

pub fn sys_run_splice() -> isize {
    syscall(
        SYSCALL_EXECVE,
        ["/splice-test-4\0".as_ptr() as isize, 0, 0, 0, 0, 0],
    )
}

//lmbench_all lat_pagefault -P 1 /var/tmp/XXX
pub fn sys_run_lmbench_test() -> isize {
    syscall(
        SYSCALL_EXECVE,
        [
            "/lmbench_all\0".as_ptr() as isize,
            [
                "lmbench_all\0".as_ptr() as isize,
                "lat_pagefault\0".as_ptr() as isize,
                "-P\0".as_ptr() as isize,
                "1\0".as_ptr() as isize,
                "/var/tmp/XXX\0".as_ptr() as isize,
                0,
            ]
            .as_ptr() as isize,
            0,
            0,
            0,
            0,
        ],
    )
}

pub fn sys_finaltest() -> isize {
    syscall(
        SYSCALL_EXECVE,
        [
            "/busybox\0".as_ptr() as isize,
            [
                "busybox\0".as_ptr() as isize,
                "sh\0".as_ptr() as isize,
                "test_all_1stage.sh\0".as_ptr() as isize,
                0,
            ]
            .as_ptr() as isize,
            0,
            0,
            0,
            0,
        ],
    )
}

pub fn sys_waitpid(pid: isize, exit_code: *mut i32, options: i32) -> isize {
    let mut rv = syscall(
        SYSCALL_WAIT4,
        [pid as isize, exit_code as isize, options as isize, 0, 0, 0],
    );
    if rv < 0 {
        rv = -1;
    }
    rv
}
pub fn sys_getcwd(buf: &mut [u8], size: usize) -> isize {
    syscall(
        SYSCALL_GETCWD,
        [buf.as_mut_ptr() as isize, size as isize, 0, 0, 0, 0],
    )
}

pub fn sys_dup(fd: usize) -> isize {
    syscall(SYSCALL_DUP, [fd as isize, 0, 0, 0, 0, 0])
}

pub fn sys_dup3(old: usize, new: usize) -> isize {
    syscall(SYSCALL_DUP3, [old as isize, new as isize, 0, 0, 0, 0])
}

pub fn sys_chdir(path: &str) -> isize {
    syscall(SYSCALL_CHDIR, [path.as_ptr() as isize, 0, 0, 0, 0, 0])
}

pub fn sys_mkdirat(dirfd: isize, path: &str, mode: u32) -> isize {
    syscall(
        SYSCALL_MKDIRAT,
        [dirfd, path.as_ptr() as isize, mode as isize, 0, 0, 0],
    )
}

pub fn sys_getdents64(fd: usize, buf: &mut [u8], len: usize) -> isize {
    syscall(
        SYSCALL_GETDENTS64,
        [
            fd as isize,
            buf.as_mut_ptr() as isize,
            len as isize,
            0,
            0,
            0,
        ],
    )
}

pub fn sys_linkat(oldfd: isize, oldpath: &str, newfd: isize, newpath: &str, flags: u32) -> isize {
    syscall(
        SYSCALL_LINKAT,
        [
            oldfd,
            oldpath.as_ptr() as isize,
            newfd,
            newpath.as_ptr() as isize,
            flags as isize,
            0,
        ],
    )
}

pub fn sys_unlinkat(dirfd: isize, path: &str, flags: u32) -> isize {
    syscall(
        SYSCALL_UNLINKAT,
        [dirfd, path.as_ptr() as isize, flags as isize, 0, 0, 0],
    )
}

pub fn sys_umount2(special: &str, flags: u32) -> isize {
    syscall(
        SYSCALL_UMOUNT2,
        [special.as_ptr() as isize, flags as isize, 0, 0, 0, 0],
    )
}

pub fn sys_mount(special: &str, dir: &str, ftype: &str, flags: u32, data: &mut [u8]) -> isize {
    syscall(
        SYSCALL_MOUNT,
        [
            special.as_ptr() as isize,
            dir.as_ptr() as isize,
            ftype.as_ptr() as isize,
            flags as isize,
            data.as_ptr() as isize,
            0,
        ],
    )
}

pub fn sys_fstat(fd: usize, kst: &mut [u8]) -> isize {
    syscall(
        SYSCALL_FSTAT,
        [fd as isize, kst.as_mut_ptr() as isize, 0, 0, 0, 0],
    )
}

pub fn sys_pipe2(fd: &mut [u32], _zero: isize) -> isize {
    syscall(SYSCALL_PIPE2, [fd.as_mut_ptr() as isize, 0, 0, 0, 0, 0])
}

pub fn sys_getppid() -> isize {
    syscall(SYSCALL_GETPPID, [0, 0, 0, 0, 0, 0])
}

pub fn sys_times(tms: &mut [u8]) -> isize {
    syscall(SYSCALL_TIMES, [tms.as_mut_ptr() as isize, 0, 0, 0, 0, 0])
}

pub fn sys_nanosleep(req: &mut [u8], rem: &mut [u8]) -> isize {
    syscall(
        SYSCALL_NANOSLEEP,
        [
            req.as_mut_ptr() as isize,
            rem.as_mut_ptr() as isize,
            0,
            0,
            0,
            0,
        ],
    )
}

pub fn sys_clock_gettime(clockid: usize, tp: &mut [u8]) -> isize {
    syscall(
        SYSCALL_CLOCK_GETTIME,
        [clockid as isize, tp.as_mut_ptr() as isize, 0, 0, 0, 0],
    )
}

pub fn sys_sched_getaffinity(pid: usize, mask: *mut usize) -> isize {
    syscall(
        SYSCALL_SCHED_GETAFFINITY,
        [pid as isize, 0, mask as isize, 0, 0, 0],
    )
}

pub fn sys_sched_setaffinity(pid: usize, mask: *const usize) -> isize {
    syscall(
        SYSCALL_SCHED_SETAFFINITY,
        [pid as isize, 0, mask as isize, 0, 0, 0],
    )
}

pub fn sys_sysinfo(info: &mut [u8]) -> isize {
    syscall(SYSCALL_SYSINFO, [info.as_mut_ptr() as isize, 0, 0, 0, 0, 0])
}

pub fn sys_sigaction(signum: usize, act: usize, oldact: usize) -> isize {
    syscall(
        SYSCALL_SIGACTION,
        [signum as isize, act as isize, oldact as isize, 0, 0, 0],
    )
}

pub fn sys_sigreturn() -> isize {
    syscall(SYSCALL_SIGRETURN, [0, 0, 0, 0, 0, 0])
}

pub fn sys_kill(pid: usize, signum: usize) -> isize {
    syscall(SYSCALL_KILL, [pid as isize, signum as isize, 0, 0, 0, 0])
}

pub fn sys_fstatat(dirfd: isize, path: &str, kst: &mut [u8], flags: usize) -> isize {
    syscall(
        SYSCALL_FSTATAT,
        [
            dirfd,
            path.as_ptr() as isize,
            kst.as_mut_ptr() as isize,
            flags as isize,
            0,
            0,
        ],
    )
}

pub fn sys_statfs(statfs: &mut [u8]) -> isize {
    syscall(
        SYSCALL_STATFS,
        [0, statfs.as_mut_ptr() as isize, 0, 0, 0, 0],
    )
}

pub fn sys_faccessat(dirfd: isize, path: &str, mode: u32, flags: usize) -> isize {
    syscall(
        SYSCALL_FACCESSAT,
        [
            dirfd,
            path.as_ptr() as isize,
            mode as isize,
            flags as isize,
            0,
            0,
        ],
    )
}

pub fn sys_utimensat(dirfd: isize, path: &str, times: &[u8], flags: usize) -> isize {
    syscall(
        SYSCALL_UTIMENSAT,
        [
            dirfd,
            path.as_ptr() as isize,
            times.as_ptr() as isize,
            flags as isize,
            0,
            0,
        ],
    )
}

pub fn sys_lseek(fd: usize, offset: isize, whence: usize) -> isize {
    syscall(
        SYSCALL_LSEEK,
        [fd as isize, offset as isize, whence as isize, 0, 0, 0],
    )
}

pub fn sys_fcntl(fd: usize, cmd: usize, arg: usize) -> isize {
    syscall(
        SYSCALL_FCNTL,
        [fd as isize, cmd as isize, arg as isize, 0, 0, 0],
    )
}

pub fn sys_sendfile(outfd: usize, infd: usize, offset_ptr: *const usize, count: usize) -> isize {
    syscall(
        SYSCALL_SENDFILE,
        [
            outfd as isize,
            infd as isize,
            offset_ptr as isize,
            count as isize,
            0,
            0,
        ],
    )
}

pub fn sys_pwrite64(fd: usize, buffer: &[u8], count: usize, offset: usize) -> isize {
    syscall(
        SYSCALL_PWRITE64,
        [
            fd as isize,
            buffer.as_ptr() as isize,
            count as isize,
            offset as isize,
            0,
            0,
        ],
    )
}

pub fn sys_pread64(fd: usize, buffer: &mut [u8], count: usize, offset: usize) -> isize {
    syscall(
        SYSCALL_PREAD64,
        [
            fd as isize,
            buffer.as_mut_ptr() as isize,
            count as isize,
            offset as isize,
            0,
            0,
        ],
    )
}

pub fn sys_ftruncate(fd: usize, length: i32) -> isize {
    syscall(
        SYSCALL_FTRUNCATE,
        [fd as isize, length as isize, 0, 0, 0, 0],
    )
}

pub fn sys_fsync(fd: usize) -> isize {
    syscall(SYSCALL_FSYNC, [fd as isize, 0, 0, 0, 0, 0])
}

pub fn sys_sync() -> isize {
    syscall(SYSCALL_SYNC, [0, 0, 0, 0, 0, 0])
}

pub fn sys_symlinkat(target: &str, newfd: isize, linkpath: &str) -> isize {
    syscall(
        SYSCALL_SYMLINKAT,
        [
            target.as_ptr() as isize,
            newfd,
            linkpath.as_ptr() as isize,
            0,
            0,
            0,
        ],
    )
}

pub fn sys_readlinkat(dirfd: isize, pathname: &str, buf: &mut [u8], bufsiz: usize) -> isize {
    syscall(
        SYSCALL_READLINKAT,
        [
            dirfd,
            pathname.as_ptr() as isize,
            buf.as_mut_ptr() as isize,
            bufsiz as isize,
            0,
            0,
        ],
    )
}

pub fn sys_getrusage(who: isize, usage: &mut [u8]) -> isize {
    syscall(
        SYSCALL_GETRUSAGE,
        [who as isize, usage.as_mut_ptr() as isize, 0, 0, 0, 0],
    )
}

pub fn sys_renameat2(
    olddirfd: isize,
    oldpath: &str,
    newdirfd: isize,
    newpath: &str,
    flags: u32,
) -> isize {
    syscall(
        SYSCALL_RENAMEAT2,
        [
            olddirfd,
            oldpath.as_ptr() as isize,
            newdirfd,
            newpath.as_ptr() as isize,
            flags as isize,
            0,
        ],
    )
}

pub fn sys_copy_file_range(
    infd: usize,
    off_in: *const usize,
    outfd: usize,
    off_out: *const usize,
    count: usize,
    flags: u32,
) -> isize {
    syscall(
        SYSCALL_COPYFILERANGE,
        [
            infd as isize,
            off_in as isize,
            outfd as isize,
            off_out as isize,
            count as isize,
            flags as isize,
        ],
    )
}

pub fn sys_getrandom(buf: &mut [u8], buflen: usize, flags: u32) -> isize {
    syscall(
        SYSCALL_GETRANDOM,
        [
            buf.as_mut_ptr() as isize,
            buflen as isize,
            flags as isize,
            0,
            0,
            0,
        ],
    )
}

pub fn sys_ppoll(fds_ptr: usize, nfds: usize, tmo_p: &[u8], mask: usize) -> isize {
    syscall(
        SYSCALL_PPOLL,
        [
            fds_ptr as isize,
            nfds as isize,
            tmo_p.as_ptr() as isize,
            mask as isize,
            0,
            0,
        ],
    )
}
