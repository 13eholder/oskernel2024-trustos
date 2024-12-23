#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;
use alloc::string::String;
use user_lib::{chdir, close, openat, run_busyboxsh, write, OpenFlags};

pub fn test_cat() {
    let fd = openat(
        -100,
        "nihao\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0666,
    );
    let inbuf: [u8; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    write(fd as usize, &inbuf, 10);
    close(fd as usize);
    println!("have create './nihao' and write 0~10 number, now should print 乱码");
    run_busyboxsh();
}

pub fn test_dirname() {
    let fd1 = openat(-100, "./mnt\0", OpenFlags::O_DIRECTROY, 0);
    let fd2 = openat(
        fd1,
        "test_dirname\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0,
    );
    close(fd1 as usize);
    close(fd2 as usize);
    println!("have create './mnt/test_dirname' now should print ./mnt");
    run_busyboxsh();
}

pub fn test_direct() {
    run_busyboxsh();
}

pub fn test_basename() {
    let fd1 = openat(-100, "./mnt\0", OpenFlags::O_DIRECTROY, 0);
    let fd2 = openat(
        fd1,
        "test_basename.txt\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0,
    );
    close(fd1 as usize);
    close(fd2 as usize);
    println!("have create './mnt/test_basename.txt' now should print test_basename");
    run_busyboxsh();
}

pub fn test_cp() {
    let fd = openat(
        -100,
        "test_cp.txt\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0666,
    );
    let fd2 = openat(
        -100,
        "test_copy.txt\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0666,
    );
    let inbuf: [u8; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    write(fd as usize, &inbuf, 10);
    close(fd as usize);
    close(fd2 as usize);
    println!("have create './test_cp.txt' and write 0~10 number, now cp to ./test_copy.txt");
    /*
    "busybox\0".as_ptr() as isize,
    "cp\0".as_ptr() as isize,
    "./test_cp.txt\0".as_ptr() as isize,
    "./test_copy.txt\0".as_ptr() as isize,
    0, */
    run_busyboxsh();
}

pub fn test_head() {
    let fd = openat(
        -100,
        "test_head\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0666,
    );
    let inbuf: [u8; 10] = [
        b'1', b'\n', b'3', b'4', b'\n', b'6', b'7', b'8', b'\n', b'6',
    ];
    write(fd as usize, &inbuf, 10);
    close(fd as usize);
    println!("have create './test_head' and write 4 lines, now head 2 lines");
    /*"busybox\0".as_ptr() as isize,
    "head\0".as_ptr() as isize,
    "-n\0".as_ptr() as isize,
    "2\0".as_ptr() as isize,
    "./test_head\0".as_ptr() as isize,
    0, */
    run_busyboxsh();
}

pub fn test_hexdump() {
    let fd = openat(
        -100,
        "test_hexdump\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0666,
    );
    let inbuf: [u8; 10] = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    write(fd as usize, &inbuf, 10);
    close(fd as usize);
    println!("have create './test_hexdump' and write 1~10 number, now hexdump show");
    /*"busybox\0".as_ptr() as isize,
    "hexdump\0".as_ptr() as isize,
    "./test_hexdump\0".as_ptr() as isize,
    0, */
    run_busyboxsh();
}

pub fn test_mv() {
    let fd1 = openat(-100, "./mnt\0", OpenFlags::O_DIRECTROY, 0);
    let fd2 = openat(
        -100,
        "test_mv.txt\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0,
    );
    close(fd1 as usize);
    close(fd2 as usize);
    //println!("have create './test_mv.txt' now mv to ./mnt/test_mvnew.txt");
    println!("have create './test_mv.txt' now mv to ./mnt/test_mvnew.txt");
    /*"busybox\0".as_ptr() as isize,
    "mv\0".as_ptr() as isize,
    "./test_mv.txt\0".as_ptr() as isize,
    "./mnt/test_mvnew.txt\0".as_ptr() as isize,
    0, */
    run_busyboxsh();
}

pub fn test_rmdir() {
    let fd1 = openat(
        -100,
        "./test_rmdir\0",
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        0,
    );
    close(fd1 as usize);
    println!("have create './test_rmdir' now rmdir it");
    /*"busybox\0".as_ptr() as isize,
    "rmdir\0".as_ptr() as isize,
    "./test_rmdir\0".as_ptr() as isize,
    0, */
    run_busyboxsh();
}

pub fn test_rm() {
    let fd1 = openat(-100, "./test_rm\0", OpenFlags::O_CREATE, 0);
    close(fd1 as usize);
    println!("have create './test_rm' now rm it");
    /*"busybox\0".as_ptr() as isize,
    "rm\0".as_ptr() as isize,
    "./test_rm\0".as_ptr() as isize,
    0, */
    run_busyboxsh();
}

pub fn test_sleep() {
    //make run SBI=opensbi
    /*"busybox\0".as_ptr() as isize,
    "sleep\0".as_ptr() as isize,
    "2\0".as_ptr() as isize,
    0, */
    run_busyboxsh();
}

pub fn test_sort() {
    //make run SBI=opensbi
    let fd = openat(
        -100,
        "test_sort\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0666,
    );
    let inbuf: [u8; 9] = [b'5', b'\n', b'7', b'\n', b'6', b'\n', b'1', b'\n', b'3'];
    write(fd as usize, &inbuf, 9);
    close(fd as usize);
    println!("have create './test_sort' and write unordered numbers, now sort it");
    /*"busybox\0".as_ptr() as isize,
    "sort\0".as_ptr() as isize,
    "-n\0".as_ptr() as isize,
    "./test_sort\0".as_ptr() as isize,
    0, */
    run_busyboxsh();
}

pub fn test_strings() {
    let fd = openat(
        -100,
        "test_strings\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0666,
    );
    let inbuf: [u8; 9] = [b't', b'r', b'u', b's', b't', b'o', b's', b'n', b'b'];
    write(fd as usize, &inbuf, 9);
    close(fd as usize);
    println!("have create './test_strings' and write trustosnb, now strings it");
    /*"busybox\0".as_ptr() as isize,
    "strings\0".as_ptr() as isize,
    "./test_strings\0".as_ptr() as isize,
    0, */
    run_busyboxsh();
}

pub fn test_tail() {
    let fd = openat(
        -100,
        "test_tail\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0666,
    );
    let inbuf: [u8; 10] = [
        b'1', b'\n', b'3', b'4', b'\n', b'6', b'7', b'8', b'\n', b'6',
    ];
    write(fd as usize, &inbuf, 10);
    close(fd as usize);
    println!("have create './test_tail' and write 4 lines, now tail 2 lines");
    /*"busybox\0".as_ptr() as isize,
    "tail\0".as_ptr() as isize,
    "-n\0".as_ptr() as isize,
    "2\0".as_ptr() as isize,
    "./test_tail\0".as_ptr() as isize,
    0, */
    run_busyboxsh();
}

pub fn test_touch() {
    let fd = openat(
        -100,
        "test_touch\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0666,
    );
    close(fd as usize);
    /*"busybox\0".as_ptr() as isize,
    "touch\0".as_ptr() as isize,
    "./test_touch\0".as_ptr() as isize,
    0, */
    run_busyboxsh();
}

pub fn test_uniq() {
    let fd = openat(
        -100,
        "test_uniq\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0666,
    );
    let inbuf: [u8; 9] = [b'1', b'\n', b'3', b'\n', b'3', b'\n', b'7', b'\n', b'3'];
    write(fd as usize, &inbuf, 9);
    close(fd as usize);
    println!("have create './test_uniq' and write 1/3/3/7/3, now uniq to 1/3/7/3");
    /*"busybox\0".as_ptr() as isize,
    "uniq\0".as_ptr() as isize,
    "./test_uniq\0".as_ptr() as isize,
    0, */
    run_busyboxsh();
}

pub fn test_clear() {
    run_busyboxsh();
}

pub fn test_cut() {
    let fd = openat(
        -100,
        "test_cut\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0666,
    );
    let inbufstring = String::from("apple:6\npear:10\nwatermalen:99");
    write(
        fd as usize,
        inbufstring.as_bytes(),
        inbufstring.as_bytes().len(),
    );
    close(fd as usize);
    println!("have create './test_cut' and write fruits and price, now cut for fruits");
    /*"busybox\0".as_ptr() as isize,
    "cut\0".as_ptr() as isize,
    "-d:\0".as_ptr() as isize,
    "-f1\0".as_ptr() as isize,
    "./test_cut\0".as_ptr() as isize,
    0, */
    run_busyboxsh();
}

pub fn test_find() {
    let fd1 = openat(-100, "./mnt\0", OpenFlags::O_DIRECTROY, 0);
    let fd2 = openat(
        fd1,
        "test_find\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0,
    );
    close(fd1 as usize);
    close(fd2 as usize);
    //println!("have create './test_mv.txt' now mv to ./mnt/test_mvnew.txt");
    println!("have create './test_find' in './mnt' now find it");
    /*"busybox\0".as_ptr() as isize,
    "find\0".as_ptr() as isize,
    ".\0".as_ptr() as isize,
    "-name\0".as_ptr() as isize,
    "test_find\0".as_ptr() as isize,
    0, */
    run_busyboxsh();
}

pub fn test_grep() {
    let fd = openat(
        -100,
        "test_grep\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0666,
    );
    let inbufstring = String::from("apple:6\npear:10\nwatermalen:99");
    write(
        fd as usize,
        inbufstring.as_bytes(),
        inbufstring.as_bytes().len(),
    );
    close(fd as usize);
    println!("have create './test_grep' and write fruits and price, now grep for pear");
    /*"busybox\0".as_ptr() as isize,
    "grep\0".as_ptr() as isize,
    "pear\0".as_ptr() as isize,
    "./test_grep\0".as_ptr() as isize,
    0, */
    run_busyboxsh();
}

pub fn test_md5sum() {
    let fd = openat(
        -100,
        "test_md5sum\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0666,
    );
    let inbufstring = String::from("apple:6\npear:10\nwatermalen:99");
    write(
        fd as usize,
        inbufstring.as_bytes(),
        inbufstring.as_bytes().len(),
    );
    close(fd as usize);
    println!("have create './test_md5sum' and write fruits and price, now calculate");
    /*"busybox\0".as_ptr() as isize,
    "md5sum\0".as_ptr() as isize,
    "./test_md5sum\0".as_ptr() as isize,
    0, */
    run_busyboxsh();
}

pub fn test_more() {
    let fd = openat(
        -100,
        "test_more\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0666,
    );
    let mut inbuf: [u8; 200] = [0; 200];
    for i in 0..inbuf.len() {
        if i % 2 == 1 {
            inbuf[i] = b'\n';
        } else {
            let num = (i % 10) as u8 + b'0';
            inbuf[i] = num;
        }
    }
    write(fd as usize, &inbuf, 200);
    close(fd as usize);
    println!("have create './test_more' and write 100个number+enter, now more to see");
    /*"busybox\0".as_ptr() as isize,
    "more\0".as_ptr() as isize,
    "./test_more\0".as_ptr() as isize,
    0, */
    run_busyboxsh();
}

pub fn test_od() {
    let fd = openat(
        -100,
        "test_od\0",
        OpenFlags::O_CREATE | OpenFlags::O_RDWR,
        0666,
    );
    let inbufstring = String::from("apple:6\npear:10\nwatermalen:99");
    write(
        fd as usize,
        inbufstring.as_bytes(),
        inbufstring.as_bytes().len(),
    );
    close(fd as usize);
    println!("have create './test_od' and write fruits and price, now od it in hex");
    /*"busybox\0".as_ptr() as isize,
    "od\0".as_ptr() as isize,
    "-t\0".as_ptr() as isize,
    "x1\0".as_ptr() as isize,
    "./test_od\0".as_ptr() as isize,
    0, */
    run_busyboxsh();
}

pub fn test_pwd() {
    let fd = openat(
        -100,
        "test_pwd\0",
        OpenFlags::O_DIRECTROY | OpenFlags::O_CREATE,
        0666,
    );
    chdir("./test_pwd\0");
    println!("have create './test_pwd' and chdir to it, now pwd");
    close(fd as usize);
    /*"busybox\0".as_ptr() as isize,
    "pwd\0".as_ptr() as isize,
    0, */
    run_busyboxsh();
}

#[no_mangle]
pub fn main() -> i32 {
    //test_cat();
    //test_dirname();
    test_direct();
    //test_basename();
    //test_cp();
    //test_head();
    //test_hexdump();
    //test_mv();
    //test_rmdir();
    //test_rm();
    //test_sleep();
    //test_sort();
    //test_strings();
    //test_tail();
    //test_touch();
    //test_uniq();
    //test_clear();
    //test_cut();
    //test_find();
    //test_grep();
    //test_md5sum();
    //test_more();
    //test_od();
    //test_pwd();
    0
}
