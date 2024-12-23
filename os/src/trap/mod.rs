//! Trap handling functionality
//!
//! For rCore, we have a single trap entry point, namely `__alltraps`. At
//! initialization in [`init()`], we set the `stvec` CSR to point to it.
//!
//! All traps go through `__alltraps`, which is defined in `trap.S`. The
//! assembly language code does just enough work restore the kernel space
//! context, ensuring that Rust code safely runs, and transfers control to
//! [`trap_handler()`].
//!
//! It then calls different functionality based on what exactly the exception
//! was. For example, timer interrupts trigger task preemption, and syscalls go
//! to [`syscall()`].
mod context;

use crate::{
    mm::{VirtAddr, VirtPageNum},
    signal::{check_if_any_sig_for_current_task, handle_signal, send_signal_to_thread, SigSet},
    syscall::{syscall, Syscall},
    task::{
        current_task, current_trap_cx, exit_current_and_run_next, suspend_current_and_run_next,
    },
    timer::{check_futex_timer, set_next_trigger},
    utils::{backtrace, hart_id},
};
use core::arch::global_asm;
use log::{debug, warn};
use riscv::register::{
    mtvec::TrapMode,
    scause::{self, Exception, Interrupt, Trap},
    sepc, sie,
    sstatus::{self, FS},
    stval, stvec,
};

pub use context::*;

global_asm!(include_str!("trap.S"));

extern "C" {
    fn __trap_from_user();
}
/// initialize CSR `stvec` as the entry of `__alltraps`
pub fn init() {
    set_kernel_trap_entry();
    //开启rustsbi的浮点指令
    unsafe {
        sstatus::set_fs(FS::Clean);
        // 允许内核态访问用户内存
        sstatus::set_sum();
    }
}

fn set_kernel_trap_entry() {
    unsafe {
        stvec::write(trap_from_kernel as usize, TrapMode::Direct);
    }
}

fn set_user_trap_entry() {
    unsafe {
        stvec::write(__trap_from_user as usize, TrapMode::Direct);
    }
}
/// enable timer interrupt in sie CSR
pub fn enable_timer_interrupt() {
    unsafe {
        sie::set_stimer();
    }
}

#[no_mangle]
/// handle an interrupt, exception, or system call from user space
pub fn trap_handler() {
    // unsafe {
    //     sstatus::set_sum();
    // }

    //记录用户空间花费CPU时间，同时准备内核空间花费CPU时间
    current_task()
        .unwrap()
        .inner_lock()
        .time_data
        .update_utime();

    let hartid = hart_id();

    set_kernel_trap_entry();
    let scause = scause::read();
    let stval = stval::read();
    match scause.cause() {
        Trap::Exception(Exception::UserEnvCall) => {
            // jump to next instruction anyway
            let mut cx = current_trap_cx();
            cx.sepc += 4;
            let syscall_id = Syscall::from(cx.gp.x[17]);
            // get system call return value
            let result = syscall(
                cx.gp.x[17],
                [
                    cx.gp.x[10],
                    cx.gp.x[11],
                    cx.gp.x[12],
                    cx.gp.x[13],
                    cx.gp.x[14],
                    cx.gp.x[15],
                ],
            );
            // cx is changed during sys_exec, so we have to call it again
            cx = current_trap_cx();
            cx.gp.x[10] = match result {
                Ok(res) => res,
                Err(errno) => -(errno as isize) as usize,
            };
            // handle error
            match result {
                Ok(ret) => debug!("[syscall ret] {:?} ret = {}", syscall_id, ret),
                Err(errno) => debug!("[syscall ret] {:?} ret = {}", syscall_id, errno.str()),
            }
        }
        Trap::Exception(Exception::StorePageFault)
        | Trap::Exception(Exception::LoadPageFault)
        | Trap::Exception(Exception::InstructionPageFault) => {
            // page fault
            let mut ok;
            {
                let task = current_task().unwrap();
                let task_inner = task.inner_lock();
                ok = task_inner
                    .memory_set
                    .lazy_page_fault(VirtAddr::from(stval).floor(), scause.cause());
                if !ok {
                    ok = task_inner
                        .memory_set
                        .cow_page_fault(VirtAddr::from(stval).floor(), scause.cause());
                }
                // drop task inner and task to avoid deadlock and exit exception
            }
            if !ok {
                warn!(
                    "[kernel] hart {} {:?} in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.",
                    hartid,
                    scause.cause(),
                    stval,
                    current_trap_cx().sepc,
                );
                //发送段错误信号
                debug!("send SIGSEGV signal!");
                send_signal_to_thread(current_task().unwrap().tid(), SigSet::SIGSEGV);
                // page fault exit code
                //exit_current_and_run_next(-2);
            }
        }
        Trap::Exception(Exception::StoreFault)
        | Trap::Exception(Exception::InstructionFault)
        | Trap::Exception(Exception::LoadFault) => {
            warn!(
                "[kernel] hart {} {:?} in application, bad addr = {:#x}, bad instruction = {:#x}, kernel killed it.",
                hartid,
                scause.cause(),
                stval,
                current_trap_cx().sepc,
            );
            // page fault exit code
            exit_current_and_run_next(-2);
        }
        Trap::Exception(Exception::IllegalInstruction) => {
            backtrace();
            warn!(
                "[kernel] [hart {}] IllegalInstruction at {:#x} in application, kernel killed it.",
                hartid,
                current_trap_cx().sepc,
            );
            // illegal instruction exit code
            exit_current_and_run_next(-3);
        }
        Trap::Interrupt(Interrupt::SupervisorTimer) => {
            // 检查futex操作是否超时
            check_futex_timer();
            // debug!("Timer Interupt!");
            suspend_current_and_run_next();
        }
        Trap::Exception(Exception::Breakpoint) => {
            warn!(
                "[kernel] Breakpoint from application, sepc = {:#x}",
                sepc::read(),
            );
            // jump to next instruction anyway
            let cx = current_trap_cx();
            cx.sepc += 2;
        }
        _ => {
            panic!(
                "Unsupported trap {:?}, stval = {:#x}!",
                scause.cause(),
                stval
            );
        }
    }
    //flush_tlb();
    //检查定时器
    current_task().unwrap().check_timer();

    //记录内核空间花费CPU时间，同时准备用户空间花费CPU时间
    current_task()
        .unwrap()
        .inner_lock()
        .time_data
        .update_stime();
}

#[no_mangle]
pub fn trap_return() {
    //检查信号
    if let Some(signo) = check_if_any_sig_for_current_task() {
        debug!("found signo in trap_return");
        handle_signal(signo);
    }

    if scause::read().cause() == Trap::Interrupt(scause::Interrupt::SupervisorTimer) {
        set_next_trigger();
    }

    // log::info!("return to user");

    set_user_trap_entry();
    extern "C" {
        #[allow(improper_ctypes)]
        fn __return_to_user(cx: *mut TrapContext);
    }
    unsafe {
        // 方便调试进入__return_to_user
        let trap_cx = current_trap_cx();
        // debug!("return to user,trap_ctx={:?}", trap_cx);
        __return_to_user(trap_cx);
    }
}

#[no_mangle]
pub fn trap_loop() {
    loop {
        trap_return();
        trap_handler();
    }
}

#[no_mangle]
pub fn trap_from_kernel() -> ! {
    backtrace();
    let stval = stval::read();
    let sepc = sepc::read();
    let stval_vpn = VirtPageNum::from(VirtAddr::from(stval));
    let sepc_vpn = VirtPageNum::from(VirtAddr::from(sepc));
    panic!(
        "stval = {:#x}(vpn {}), sepc = {:#x}(vpn{}),
        a trap {:?} from kernel!",
        stval,
        stval_vpn.0,
        sepc,
        sepc_vpn.0,
        scause::read().cause()
    );
}
