#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{close, openat, read, write, OpenFlags};

#[no_mangle]
pub fn main() -> i32 {
    let test_str = "Hello, world!";
    let filea = "filea\0";
    let fd = openat(-100, filea, OpenFlags::O_CREATE | OpenFlags::O_WRONLY, 0);
    assert!(fd > 0);
    let fd = fd as usize;
    write(fd, test_str.as_bytes(), test_str.as_bytes().len());
    close(fd);
    let fd = openat(-100, filea, OpenFlags::O_RDONLY, 0);
    assert!(fd > 0);
    let fd = fd as usize;
    let mut buffer = [0u8; 100];
    let _ = read(fd, &mut buffer, 100) as usize;
    close(fd);
    println!("file_test passed!");
    0
}
