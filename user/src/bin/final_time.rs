#![no_std]
#![no_main]

#[macro_use]
extern crate user_lib;

use user_lib::{clock_gettime, exit, fork, getrusage, wait, Clockid, Rusage, Timespec};

fn test_clock_gettime() {
    println!("-----------------test clock_gettime-----------------");
    let clockid = Clockid::CLOCK_REALTIME;
    let mut tp = Timespec::new(0, 0);
    let result = clock_gettime(clockid, &mut tp);
    println!("tp is {:?} and result is {}", tp, result);
    println!("-----------------end clock_gettime-----------------");
    println!("");
}

fn test_getrusage() {
    println!("-----------------test getrusage-----------------");
    let mut usage = Rusage::new();
    let result1 = getrusage(0, &mut usage);
    println!("got rusage_self as {:?} ", usage);
    let pid = fork();
    if pid == 0 {
        println!("fork one child and exit it");
        exit(0);
    }
    let mut exit_code: i32 = 0;
    println!("now wait");
    wait(&mut exit_code);
    let result2 = getrusage(-1, &mut usage);
    println!("got rusage_children as {:?} ", usage);
    println!("result1 is {} which should be 0", result1);
    println!("result2 is {} which should be 0", result2);
    println!("-----------------end getrusage-----------------");
    println!("");
}

#[no_mangle]
pub fn main() -> i32 {
    test_clock_gettime();
    test_getrusage();
    0
}
