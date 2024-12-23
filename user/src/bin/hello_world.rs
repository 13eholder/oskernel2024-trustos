#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;
use user_lib::mkdir;

#[no_mangle]
pub fn main() -> i32 {
    println!("Hello world from user mode program!");
    let p = mkdir(-100, "nihao\0", 0666);
    println!("{}", p);
    0
}
