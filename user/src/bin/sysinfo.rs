#![no_std]
#![no_main]

use user_lib::{sysinfo, Sysinfo};

#[macro_use]
extern crate user_lib;

#[no_mangle]
pub fn main() -> i32 {
    let mut info = Sysinfo::new(0, 0, 0);
    sysinfo(&mut info);
    println!("ourinfo is {:?}", info);
    0
}
