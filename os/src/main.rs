//! The main module and entrypoint
//!
//! Various facilities of the kernels are implemented as submodules. The most
//! important ones are:
//!
//! - [`trap`]: Handles all cases of switching from userspace to the kernel
//! - [`task`]: Task management
//! - [`syscall`]: System call handling and implementation
//! - [`mm`]: Address map using SV39
//! - [`sync`]: Wrap a static data structure inside it so that we are able to access it without any `unsafe`.
//! - [`fs`]: Separate user from file system with some structures
//!
//! The operating system also starts in this module. Kernel code starts
//! executing from `entry.asm`, after which [`rust_main()`] is called to
//! initialize various pieces of functionality. (See its source code for
//! details.)
//!
//! We then call [`task::run_tasks()`] and for the first time go to
//! userspace.

// #![deny(warnings)]
//#![deny(missing_docs)]
#![allow(unused_must_use)]
#![allow(unreachable_code)]
// #![allow(dead_code)]
// #![allow(unused_imports)]
#![allow(unused_unsafe)]
#![no_std]
#![no_main]
#![feature(panic_info_message)]
#![feature(alloc_error_handler)]
#![feature(sync_unsafe_cell)]
extern crate alloc;

#[macro_use]
extern crate bitflags;

mod boards;

#[macro_use]
mod console;
mod config;
mod drivers;
pub mod fs;
pub mod lang_items;
pub mod logger;
pub mod mm;
pub mod sbi;
pub mod signal;
pub mod sync;
pub mod syscall;
pub mod task;
pub mod timer;
pub mod trap;
pub mod utils;

use crate::mm::activate_kernel_space;
use config::{
    mm::{HART_START_ADDR, KERNEL_ADDR_OFFSET},
    sync::HART_NUM,
};
use core::{
    arch::{asm, global_asm},
    sync::atomic::{AtomicBool, Ordering},
    usize,
};

global_asm!(include_str!("entry.asm"));
/// clear BSS segment
fn clear_bss() {
    extern "C" {
        fn sbss();
        fn ebss();
    }
    unsafe {
        core::slice::from_raw_parts_mut(sbss as usize as *mut u8, ebss as usize - sbss as usize)
            .fill(0);
    }
}

/// ADD KERNEL_ADDR_OFFSET and jump to rust_main
#[no_mangle]
pub fn trampoline(hartid: usize) {
    unsafe {
        asm!("add sp, sp, {}", in(reg) KERNEL_ADDR_OFFSET);
        asm!("la t0, rust_main");
        asm!("add t0, t0, {}", in(reg) KERNEL_ADDR_OFFSET);
        asm!("mv a0, {}", in(reg) hartid);
        asm!("jalr zero, 0(t0)");
    }
}

static FIRST_HART: AtomicBool = AtomicBool::new(true);
static INIT_FINISHED: AtomicBool = AtomicBool::new(false);
/// boot start_hart之外的所有 hart
pub fn boot_all_harts(hartid: usize) {
    for i in (0..HART_NUM).filter(|id| *id != hartid) {
        sbi::hart_start(i, HART_START_ADDR).unwrap();
    }
}

#[no_mangle]
/// the rust entry-point of os
pub fn rust_main(hartid: usize) -> ! {
    if FIRST_HART.load(Ordering::SeqCst) == true {
        FIRST_HART.store(false, Ordering::SeqCst);
        clear_bss();
        println!("[kernel] Hello, world!");
        println!(
            r#" ,--.--------.                                 ,-,--.   ,--.--------.  
        /==/,  -   , -\   .-.,.---.    .--.-. .-.-.  ,-.'-  _\ /==/,  -   , -\ 
        \==\.-.  - ,-./  /==/  `   \  /==/ -|/=/  | /==/_ ,_.' \==\.-.  - ,-./ 
         `--`\==\- \    |==|-, .=., | |==| ,||=| -| \==\  \     `--`\==\- \    
              \==\_ \   |==|   '='  / |==|- | =/  |  \==\ -\         \==\_ \   
              |==|- |   |==|- ,   .'  |==|,  \/ - |  _\==\ ,\        |==|- |   
              |==|, |   |==|_  . ,'.  |==|-   ,   / /==/\/ _ |       |==|, |   
              /==/ -/   /==/  /\ ,  ) /==/ , _  .'  \==\ - , /       /==/ -/   
              `--`--`   `--`-`--`--'  `--`..---'     `--`---'        `--`--`   "#
        );
        mm::init();
        logger::init();
        trap::init();
        task::init();
        fs::init();
        task::add_initproc();
        INIT_FINISHED.store(true, Ordering::SeqCst);
        fs::list_apps();
        boot_all_harts(hartid);
        trap::enable_timer_interrupt();
        timer::set_next_trigger();
    } else {
        loop {}
        // barrier
        while !INIT_FINISHED.load(Ordering::SeqCst) {}

        println!(
            "[kernel] ---------- hart {} is starting... ----------",
            hartid
        );
        trap::init();
        activate_kernel_space();
        trap::enable_timer_interrupt();
        timer::set_next_trigger();
    }
    task::run_tasks();
    panic!("Unreachable in rust_main!");
}
