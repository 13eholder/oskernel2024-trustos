#![no_std]
#![no_main]
#![allow(unused_imports)]
#![allow(unused_variables)]

extern crate user_lib;

use user_lib::{
    fork, run_busyboxsh, run_finaltest, run_libc_bench, run_lmbench_test, run_splice, shutdown,
    wait,
};

#[no_mangle]
fn main() -> i32 {
    // println!("initproc running...");
    if fork() == 0 {
        // exec("user_shell\0");
        //run_busyboxsh();
        run_finaltest();
        // run_libc_bench();
        // run_lmbench_test();
        // run_splice();
    } else {
        loop {
            let mut exit_code: i32 = 0;
            let _ = wait(&mut exit_code);
            // shutdown();
            // sleep(5000);
            // println!(
            //     "[initproc] Released a zombie process, pid={}, exit_code={}",
            //     pid, exit_code,
            // );
        }
    }
    0
}
