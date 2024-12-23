#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
extern crate alloc;

use user_lib::{close, openat, read, OpenFlags};

#[no_mangle]
pub fn main() -> i32 {
    let fd = openat(-100,"filea\0", OpenFlags::O_RDONLY,0);
    if fd == -1 {
        panic!("Error occured when opening file");
    }
    let fd = fd as usize;
    let mut buf = [0u8; 100];
    loop {
        let size = read(fd, &mut buf,100) as usize;
        if size == 0 {
            break;
        }
        println!("{}", core::str::from_utf8(&buf[..size]).unwrap());
    }
    close(fd);
    0
}
