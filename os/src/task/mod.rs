//! Task management implementation
//!
//! Everything about task management, like starting and switching tasks is
//! implemented here.
//!
//! A single global instance of [`TaskManager`] called `TASK_MANAGER` controls
//! all the tasks in the whole operating system.
//!
//! A single global instance of [`Processor`] called `PROCESSOR` monitors running
//! task(s) for each core.
//!
//! A single global instance of [`PidAllocator`] called `PID_ALLOCATOR` allocates
//! pid for user apps.
//!
//! Be careful when you see `__switch` ASM function in `switch.S`. Control flow around this function
//! might not be what you expect.

#[allow(clippy::module_inception)]
#[allow(rustdoc::private_intra_doc_links)]
mod aux;
mod context;
mod futex;
mod manager;
mod processor;
mod switch;
mod sysinfo;
mod task;
mod tid;

use core::sync::atomic::AtomicU32;

use crate::{
    fs::{open, OpenFlags, NONE_MODE},
    mm::{put_data, VirtAddr},
    signal::{send_signal_to_thread_group, SigSet},
};
use alloc::{boxed::Box, sync::Arc};
pub use context::TaskContext;
pub use futex::*;
use log::debug;
pub use manager::*;
use spin::Lazy;
use switch::__switch;
pub use sysinfo::Sysinfo;
pub use task::{RobustList, TaskControlBlock, TaskStatus};

pub use aux::*;
pub use processor::{
    current_task, current_token, current_trap_cx, get_proc_by_hartid, run_tasks, schedule,
    take_current_task, Processor, PROCESSORS,
};
pub use tid::{tid_alloc, KernelStack, TidAllocator, TidHandle};

/// Suspend the current 'Running' task and run the next task in task list.
pub fn suspend_current_and_run_next() {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_lock();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    // Change status to Ready
    task_inner.task_status = TaskStatus::Ready;
    // ---- release current PCB
    drop(task_inner);
    drop(task);
    // jump to scheduling cycle
    schedule(task_cx_ptr);
}

pub fn block_current_and_run_next() {
    let task = take_current_task().unwrap();
    let mut task_inner = task.inner_lock();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    task_inner.task_status = TaskStatus::Blocked;
    drop(task_inner);
    drop(task);
    schedule(task_cx_ptr);
}

pub fn stop_current_and_run_next() {
    let task = take_current_task().unwrap();
    let mut task_inner = task.inner_lock();
    let task_cx_ptr = &mut task_inner.task_cx as *mut TaskContext;
    task_inner.task_status = TaskStatus::Stopped;
    drop(task_inner);
    // drop(task);
    stop_task(task);
    schedule(task_cx_ptr);
}

/// pid of usertests app in make run TEST=1
pub const IDLE_PID: usize = 0;

/// 杀死当前线程组的所有线程
pub fn exit_current_group_and_run_next(exit_code: i32) {
    let task = current_task().unwrap();
    let task_inner = task.inner_lock();
    let mut exit_code = exit_code;
    if task_inner.sig_table.not_exited() {
        //设置进程的SIGNAL_GROUP_EXIT标志并把终止代号放到current->signal->group_exit_code字段
        task_inner.sig_table.set_exit_code(exit_code);
        let pid = task.pid();
        drop(task_inner);
        drop(task);
        send_signal_to_thread_group(pid, SigSet::SIGKILL);
    } else {
        exit_code = task_inner.sig_table.exit_code();
        drop(task_inner);
        drop(task);
    }

    exit_current_and_run_next(exit_code);
}

pub fn exit_current_and_run_next(exit_code: i32) {
    let task = take_current_task().unwrap();
    let mut inner = task.inner_lock();
    debug!(
        "[sys_exit] thread {} exit, exit_code = {}",
        task.tid(),
        exit_code
    );

    // CLONE_CHILD_CLEARTID
    if inner.clear_child_tid != 0 {
        let token = inner.user_token();
        put_data(token, inner.clear_child_tid as *mut u32, 0);
        // 唤醒等待在 child_tid 的进程
        let pa = inner
            .memory_set
            .translate_va(VirtAddr::from(inner.clear_child_tid))
            .unwrap();
        let thread_shared_key = FutexKey::new(pa, task.pid());
        futex_wake_up(thread_shared_key, 1);
        let process_shared_key = FutexKey::new(pa, 0);
        futex_wake_up(process_shared_key, 1);
    }

    // 无论如何一个轻量级进程都会是一个线程
    // 释放线程相关资源
    remove_from_tid2task(task.tid());
    inner.dealloc_user_res();
    inner.task_status = TaskStatus::Zombie;

    drop(inner);

    // 一个进程的所有线程都退出了,此时回收资源
    {
        //debug!("work at exit for {} {}", task.pid(), task.tid());
        let thread_group = THREAD_GROUP.lock();
        if let Some(tasks) = thread_group.get(&task.pid()) {
            if tasks.iter().all(|task| task.inner_lock().is_zombie()) {
                drop(thread_group);
                send_signal_to_thread_group(task.ppid(), SigSet::SIGCHLD);
                let mut task_inner = task.inner_lock();
                task_inner.recycle();
                if task_inner.sig_table.not_exited() {
                    task_inner.sig_table.set_exit_code(exit_code);
                }
                #[cfg(feature = "ltp")]
                crate::fs::remove_proc_dir_and_file(task.pid());
                wakeup_parent(task.ppid());
            }
        }
    }
    drop(task);
    let mut _unused = TaskContext::zero_init();
    schedule(&mut _unused as *mut _);
}

///Globle process that init user shell
pub static INITPROC: Lazy<Arc<TaskControlBlock>> = Lazy::new(|| {
    Arc::new({
        let initproc = open("/initproc", OpenFlags::O_RDONLY, NONE_MODE)
            .expect("open initproc error!")
            .file()
            .expect("initproc can not be abs file!");
        let elf_data = initproc.inode.read_all().unwrap();
        let res = TaskControlBlock::new(&elf_data);
        res
    })
});
///Add init process to the manager
pub fn add_initproc() {
    add_task(INITPROC.clone());

    insert_into_tid2task(0, &INITPROC);
    insert_into_thread_group(0, &INITPROC);
}
///Init PROCESSORS
pub fn init() {
    unsafe {
        for p in PROCESSORS.iter_mut() {
            p.idle_task_cx = Some(Box::new(TaskContext::zero_init()));
        }
    }
}

// pub static CUR_UID: Lazy<Mutex<u32>> = Lazy::new(|| Mutex::new(0));
pub static CUR_UID: Lazy<AtomicU32> = Lazy::new(|| AtomicU32::new(0));

pub fn current_uid() -> u32 {
    CUR_UID.load(core::sync::atomic::Ordering::SeqCst)
}

pub fn change_current_uid(uid: u32) {
    CUR_UID.store(uid, core::sync::atomic::Ordering::SeqCst);
}
