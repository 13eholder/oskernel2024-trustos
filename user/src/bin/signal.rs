#![no_std]
#![no_main]

use user_lib::{getpid, kill, sigaction, SigAction};

#[macro_use]
extern crate user_lib;

pub fn func1() {
    println!("I am func1 and I am cool!");
}

pub fn func2() {
    println!("I am func2 and I was true!")
}

#[no_mangle]
pub fn main() -> i32 {
    println!("test signal");
    let sig = SigAction::new(func1 as usize, 0, 0, 0);
    let mut old_sig = SigAction::new(0, 0, 0, 0);
    let result = sigaction(2, &sig, &mut old_sig);
    println!("sigaction result is {}", result);
    let pid = getpid();
    println!("pid is {}", pid);
    let result2 = kill(pid as usize, 2);
    println!("kill result is {}", result2);
    // 9 Kill -> exit
    kill(pid as usize, 9);
    0
}
