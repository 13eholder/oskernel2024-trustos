#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{fork, wait};

#[no_mangle]
pub fn main() -> i32 {
    assert_eq!(wait(&mut 0i32), -1);
    let pid = fork();
    if pid == 0 {
        // child process
        println!("hello child process!");
        100
    } else {
        // parent process
        let mut exit_code: i32 = 0;
        assert_eq!(pid, wait(&mut exit_code));
        assert_eq!(exit_code >> 8, 100);
        println!("child process pid = {}, exit code = {}", pid, exit_code);
        0
    }
}
