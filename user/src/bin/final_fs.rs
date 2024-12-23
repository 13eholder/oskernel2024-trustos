#![no_std]
#![no_main]
use user_lib::{
    chdir, close, copy_file_range, faccessat, fcntl, fstatat, fsync, ftruncate, getrandom, lseek,
    mkdir, openat, ppoll, pread64, pwrite64, read, readlinkat, renameat2, statfs, symlinkat, sync,
    unlink, write, FaccessatMode, Kstat, OpenFlags, PollEvents, PollFd, Statfs, Timespec,
};

#[macro_use]
extern crate user_lib;

fn test_fstatat() {
    println!("-----------------test fstatat-----------------");
    let mut kst = Kstat::new();
    mkdir(-100, "nihao\0", 0666);
    let result = fstatat(-100, "nihao\0", &mut kst, 0);
    println!("result is {} and kst is {:?}", result, kst);
    println!("-----------------end fstatat-----------------");
    println!("");
}

fn test_statfs() {
    println!("-----------------test statfs-----------------");
    let mut stat = Statfs::empty();
    let result = statfs(&mut stat);
    println!("result is {} and ourstatfs is {:?}", result, stat);
    println!("-----------------end statfs-----------------");
    println!("");
}

fn test_faccessat() {
    println!("-----------------test faccessat-----------------");
    let mode1 = FaccessatMode::F_OK | FaccessatMode::X_OK | FaccessatMode::W_OK;
    //let mode2 = FaccessatMode::R_OK;
    mkdir(-100, "test_faccessat\0", 0666);
    println!("now test OK:");
    let result = faccessat(-100, "test_faccessat\0", mode1, 0);
    //println!("now test NOT OK:");
    //let result2 = faccessat(-100, "test_faccessat\0", mode2, 0);
    println!("result is {} which should be 0", result);
    //println!("result2 is {} which should be -1", result2);
    println!("-----------------end faccessat-----------------");
    println!("");
}

fn test_lseek() {
    println!("-----------------test lseek-----------------");
    mkdir(-100, "test_lseek\0", 0666);
    let fd = openat(-100, "test_lseek\0", OpenFlags::O_RDWR, 0);
    let result1 = lseek(fd as usize, -100, 2);
    let result2 = lseek(fd as usize, 100, 1);
    close(fd as usize);
    println!("result1 is {} which should be -22(EINVAL)", result1);
    println!("result2 is {} which should be 100", result2);
    println!("-----------------end lseek-----------------");
    println!("");
}

fn test_fcntl() {
    println!("-----------------test fcntl-----------------");
    mkdir(-100, "test_fcntl\0", 0666);
    let fd = openat(-100, "test_fcntl\0", OpenFlags::O_RDWR, 0);
    println!("open gets fd as {}", fd);
    let result1 = fcntl(fd as usize, 0, 0); //dup
    let result2 = fcntl(fd as usize, 2, 1); //设置CLOEXEC
    let result3 = fcntl(fd as usize, 1, 0); //获得CLOEXEC
    let flags = OpenFlags::all();
    let result4 = fcntl(fd as usize, 4, flags.bits() as usize); //设置openflags
    let result5 = fcntl(fd as usize, 3, 0); //获得openflags
    close(fd as usize);
    println!("result1 is {} which should fd+1", result1);
    println!("result2 is {} which should be 0", result2);
    println!("result3 is {} which should be 1", result3);
    println!("result4 is {} which should be 0", result4);
    println!("result5 is {} which is Openflags::all()", result5);
    println!("-----------------end fcntl-----------------");
    println!("");
}
/*
fn test_sendfile() {
    println!("-----------------test sendfile-----------------");
    let infd = openat(
        -100,
        "test_infd\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0,
    );
    let outfd = openat(
        -100,
        "test_outfd\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0,
    );
    let inbuf: [u8; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let writeresult = write(infd as usize, &inbuf, 10);
    println!(
        "have written {} bytes [1,2,3,4,5,6,7,8,9,10]\nnow lseek to the beginning of the infile",
        writeresult
    );
    let offset: usize = 5;
    println!("now we need the last 5 number");
    let result = sendfile(outfd as usize, infd as usize, &offset as *const usize, 5);
    println!("sendfile over, now lseek to the beginning of the outfile");
    lseek(outfd as usize, 0, 0);
    println!("lseek over, now let's read");
    let mut outbuf: [u8; 5] = [0; 5];
    let readresult = read(outfd as usize, &mut outbuf, 5);
    println!(
        "read {} bytes , now print send bytes which should be: [6,7,8,9,10]",
        readresult
    );
    for &byte in &outbuf {
        print!("{} ", byte);
    }
    close(infd as usize);
    close(outfd as usize);
    println!("");
    println!("result is {} which should be 5", result);
    println!("-----------------end sendfile-----------------");
    println!("");
}
*/

fn _test_pwr64() {
    println!("-----------------test pwrite64 & pread64-----------------");
    let fd = openat(
        -100,
        "test_pwr64\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0,
    );
    let inbuf: [u8; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let writeresult = pwrite64(fd as usize, &inbuf, 10, 50);
    println!(
        "have written {} bytes [1,2,3,4,5,6,7,8,9,10] at offset 50",
        writeresult
    );
    let mut outbuf: [u8; 5] = [0; 5];
    let readresult = pread64(fd as usize, &mut outbuf, 5, 55);
    println!(
        "read {} bytes at offset 55 , now print send bytes which should be: [6,7,8,9,10]",
        readresult
    );
    for &byte in &outbuf {
        print!("{} ", byte);
    }
    println!("");
    close(fd as usize);
    println!("-----------------end pwrite64 & pread64-----------------");
    println!("");
}

fn test_ftruncate() {
    println!("-----------------test ftruncate-----------------");
    let fd = openat(
        -100,
        "test_ftruncate\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0,
    );
    //有修改导致无法指定长度
    //let result1 = ftruncate(fd as usize, -5);
    //let result2 = ftruncate(fd as usize, 100);
    let result = ftruncate(fd as usize, 0);
    close(fd as usize);
    //println!("result1 is {} which should be -1", result1);
    //println!("result2 is {} which should be 0", result2);
    println!("result is {} which should be 0", result);
    println!("-----------------end ftruncate-----------------");
    println!("");
}

fn test_fsync() {
    println!("-----------------test fsync-----------------");
    let fd = openat(
        -100,
        "test_fsync\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0,
    );
    let result = fsync(fd as usize);
    close(fd as usize);
    println!("result is {} which should be 0", result);
    println!("-----------------end fsync-----------------");
    println!("");
}

fn test_sync() {
    println!("-----------------test sync-----------------");
    let result = sync();
    println!("result is {} which should be 0", result);
    println!("-----------------end sync-----------------");
    println!("");
}

fn _test_symlinkat_readlinkat() {
    println!("-----------------test symlinkat & readlinkat-----------------");
    let fd = openat(
        -100,
        "test_readlinkat\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0,
    );
    let result1 = symlinkat("test_readlinkat\0", -100, "test_symlinkat");
    let mut buf: [u8; 100] = [0; 100];
    let result2 = readlinkat(-100, "test_symlinkat", &mut buf, 100);
    let buf_ascii = buf.to_ascii_lowercase();
    println!("now print got string which should be test_readlinkat");
    for i in 0..buf_ascii.len() {
        print!("{}", buf_ascii[i] as char);
    }
    println!("");
    close(fd as usize);
    println!("result1 is {} which should be 0", result1);
    println!(
        "result2 is {} which should be 15 which is the length",
        result2
    );
    println!("-----------------end symlinkat & readlinkat-----------------");
    println!("");
}

fn test_renameat2() {
    println!("-----------------test renameat2-----------------");
    let fd = openat(
        -100,
        "test_renameat2\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0,
    );
    let result1 = renameat2(-100, "test_renameat2\0", -100, "test_renameat2_ok\0", 1);
    let result2 = openat(-100, "test_renameat2_ok\0", OpenFlags::O_RDWR, 0);
    println!("result1 is {} which should be 0", result1);
    println!("result2 is {} which should not be -1", result2);
    close(fd as usize);
    close(result2 as usize);
    println!("-----------------end renameat2-----------------");
    println!("");
}

fn test_openat() {
    println!("-----------------test openat-----------------");
    let fd = openat(
        -100,
        "./mnt\0",
        OpenFlags::O_CREATE | OpenFlags::O_DIRECTROY | OpenFlags::O_RDWR,
        0,
    );
    println!("fd {} open ./mnt ok ", fd);
    let result1 = openat(
        fd,
        "test_openat\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0,
    );
    chdir("./mnt\0");
    let result2 = openat(-100, "test_openat\0", OpenFlags::O_RDWR, 0);
    println!("result1 is {} which should not be -1", result1);
    println!("result2 is {} which should not be -1", result2);
    close(fd as usize);
    close(result1 as usize);
    close(result2 as usize);
    println!("-----------------end openat-----------------");
    println!("");
}

fn _test_copy_file_range() {
    println!("-----------------test copy_file_range-----------------");
    let infd = openat(
        -100,
        "test_infd2\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0,
    );
    let outfd = openat(
        -100,
        "test_outfd2\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0,
    );
    let inbuf: [u8; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let writeresult = write(infd as usize, &inbuf, 10);
    println!(
        "have written {} bytes [1,2,3,4,5,6,7,8,9,10]\nnow lseek to the beginning of the infile",
        writeresult
    );
    let inoffset: usize = 2;
    let outoffset: usize = 4;
    println!("now we need the middle 5 number");
    let result = copy_file_range(
        infd as usize,
        &inoffset as *const usize,
        outfd as usize,
        &outoffset as *const usize,
        5,
        0,
    );
    println!("copy_file_range over, now lseek to the beginning of the outfile");
    lseek(outfd as usize, 4, 0);
    println!("lseek over, now let's read");
    let mut outbuf: [u8; 5] = [0; 5];
    let readresult = read(outfd as usize, &mut outbuf, 5);
    println!(
        "read {} bytes , now print send bytes which should be: [3,4,5,6,7]",
        readresult
    );
    for &byte in &outbuf {
        print!("{} ", byte);
    }
    close(infd as usize);
    close(outfd as usize);
    println!("");
    println!("result is {} which should be 5", result);
    println!("-----------------end copy_file_range-----------------");
    println!("");
}

pub fn test_getrandom() {
    println!("-----------------test getrandom-----------------");
    let mut buf: [u8; 10] = [0; 10];
    let result = getrandom(&mut buf, 10, 0);
    println!("have get 10 random number");
    for &byte in &buf {
        print!("{} ", byte);
    }
    println!("");
    println!("result is {} which should be 10", result);
    println!("-----------------end getrandom-----------------");
    println!("");
}

pub fn test_ppoll() {
    println!("-----------------test ppoll-----------------");
    let mut fds = [PollFd::new(); 2];
    fds[0].fd = 1;
    fds[0].events = PollEvents::all();
    let fd = openat(
        -100,
        "test_ppoll\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0,
    );
    fds[1].fd = fd as i32;
    fds[1].events = PollEvents::all();
    let tmo_p = Timespec::new(10, 0);
    let result = ppoll(&mut fds, 2, &tmo_p, 0);
    println!("fds[0] revents is {} which is not 0", fds[0].revents.bits());
    println!("fds[1] revents is {} which is not 0", fds[1].revents.bits());
    println!("");
    println!("result is {} which should be 2", result);
    println!("-----------------end ppoll-----------------");
    println!("");
}

pub fn test_write_after_unlink() {
    println!("-----------------write_after_unlink-----------------");
    let fd0 = openat(
        -100,
        "./waudir\0",
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0,
    );
    println!("fd0 {} create dir ./waudir ok ", fd0);
    let fd = openat(
        -100,
        "/waudir/wau\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0,
    );
    println!("fd {} create ./wau ok ", fd);
    close(fd as usize);
    close(fd0 as usize);
    let result1 = unlink(-100, "./waudir/wau\0", OpenFlags::empty());
    println!("result1 is {} which should be 0", result1);
    let result2 = unlink(-100, "./waudir\0", OpenFlags::empty());
    println!("result2 is {} which should be 0", result2);
    let fd1 = openat(
        -100,
        "./waudir\0",
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0,
    );
    let fd2 = openat(
        -100,
        "/waudir/wau\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0,
    );
    let buf: [u8; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let writeresult = write(fd as usize, &buf, 10);
    println!("have written {} bytes [1,2,3,4,5,6,7,8,9,10]", writeresult);
    close(fd1 as usize);
    close(fd2 as usize);
    println!("-----------------end write_after_unlink-----------------");
    println!("");
}

#[no_mangle]
pub fn main() -> i32 {
    test_fstatat();
    test_statfs();
    test_faccessat();
    test_lseek();
    test_fcntl();
    //test_sendfile();  //无法使用，因为sendfile是抽象文件传给普通文件
    //test_pwr64();
    test_ftruncate();
    test_fsync();
    test_sync();
    //test_symlinkat_readlinkat();
    test_renameat2();
    test_openat();
    //test_copy_file_range();
    test_getrandom();
    test_ppoll();
    test_write_after_unlink();
    0
}
