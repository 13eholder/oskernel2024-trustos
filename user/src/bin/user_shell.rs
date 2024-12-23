#![no_std]
#![no_main]
#![allow(clippy::println_empty_string)]

extern crate alloc;

#[macro_use]
extern crate user_lib;

const LF: u8 = 0x0au8;
const CR: u8 = 0x0du8;
const DL: u8 = 0x7fu8;
const BS: u8 = 0x08u8;

use alloc::string::{String, ToString};
use alloc::vec::Vec;
use user_lib::console::getchar;
use user_lib::{execve, fork, waitpid};

#[no_mangle]
pub fn main() -> i32 {
    println!("Rust user shell");
    let mut line: String = String::new();
    let mut old = String::new();
    old += "\0";
    print!(">> ");
    loop {
        let c = getchar();
        match c {
            LF | CR => {
                println!("");
                if true {
                    if line.is_empty() {
                        line = old.clone();
                    }
                    let pid = fork();
                    if pid == 0 {
                        // child process
                        let args: Vec<String> = line
                            .split_whitespace()
                            .map(|s| (s.to_string() + "\0"))
                            .collect();
                        let argsstr: Vec<&str> = args.iter().map(|s| s.as_str()).collect();
                        if execve(&argsstr) == -1 {
                            println!("Error when executing!");
                            return -4;
                        }
                        unreachable!();
                    } else {
                        let mut exit_code: i32 = 0;
                        let exit_pid = waitpid(pid as usize, &mut exit_code);
                        assert_eq!(pid, exit_pid);
                        println!("Shell: Process {} exited with code {}", pid, exit_code);
                    }
                    old = line.clone();
                    line.clear();
                }
                print!(">> ");
            }
            BS | DL => {
                if !line.is_empty() {
                    print!("{}", BS as char);
                    print!(" ");
                    print!("{}", BS as char);
                    line.pop();
                }
            }
            _ => {
                print!("{}", c as char);
                line.push(c as char);
            }
        }
    }
}
