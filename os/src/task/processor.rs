//!Implementation of [`Processor`] and Intersection of control flow
use super::{
    __switch, add_task, change_current_uid, fetch_task, TaskContext, TaskControlBlock, TaskStatus,
};
use crate::{config::sync::HART_NUM, timer::check_futex_timer, trap::TrapContext, utils::hart_id};
use alloc::{boxed::Box, sync::Arc};
///Processor management structure
pub struct Processor {
    ///The task currently executing on the current processor
    pub current: Option<Arc<TaskControlBlock>>,
    ///The basic control flow of each core, helping to select and switch process
    pub idle_task_cx: Option<Box<TaskContext>>,
}

impl Processor {
    ///Create an empty Processor
    pub const fn new() -> Self {
        Self {
            current: None,
            idle_task_cx: None,
        }
    }
    ///Get mutable reference to `idle_task_cx`
    fn get_idle_task_cx_ptr(&mut self) -> *mut TaskContext {
        self.idle_task_cx.as_mut().unwrap().as_mut() as *mut _
    }
    ///Get current task in moving semanteme
    pub fn take_current(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.current.take()
    }
    ///Get current task in cloning semanteme
    pub fn current(&self) -> Option<Arc<TaskControlBlock>> {
        self.current.as_ref().map(Arc::clone)
    }
}

const EMPTY_PROCESSOR: Processor = Processor::new();
/// 不需要加锁,每个核只会访问固定的Processor
pub static mut PROCESSORS: [Processor; HART_NUM] = [EMPTY_PROCESSOR; HART_NUM];

///attach to processors
pub fn get_proc_by_hartid(hartid: usize) -> &'static mut Processor {
    unsafe { &mut PROCESSORS[hartid] }
}

///The main part of process execution and scheduling
///Loop `fetch_task` to get the process that needs to run, and switch the process through `__switch`
pub fn run_tasks() {
    loop {
        let processor = get_proc_by_hartid(hart_id());
        let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
        if let Some(cur_task) = take_current_task() {
            let mut cur_task_inner = cur_task.inner_lock();
            check_futex_timer();
            if let Some(next_task) = fetch_task() {
                /*
                debug!(
                    "drop task {},fetch task {}",
                    cur_task.pid(),
                    next_task.pid()
                );
                */
                let mut next_task_inner = next_task.inner_lock();
                let next_task_cx_ptr = &next_task_inner.task_cx as *const TaskContext;
                next_task_inner.task_status = TaskStatus::Running;
                next_task_inner.memory_set.activate();
                change_current_uid(next_task_inner.user_id);
                drop(next_task_inner);
                drop(cur_task_inner);
                processor.current = Some(next_task);
                add_task(cur_task);
                unsafe {
                    __switch(idle_task_cx_ptr, next_task_cx_ptr);
                }
            } else {
                cur_task_inner.task_status = TaskStatus::Running;
                let cur_task_cx_ptr = &cur_task_inner.task_cx as *const TaskContext;
                drop(cur_task_inner);
                processor.current = Some(cur_task);
                unsafe {
                    __switch(idle_task_cx_ptr, cur_task_cx_ptr);
                }
            }
        } else {
            // 第一次调度，抢占
            if let Some(task) = fetch_task() {
                // info!("first fetch task {}", task.pid());
                let mut task_inner = task.inner_lock();
                let next_task_cx_ptr = &task_inner.task_cx as *const TaskContext;
                task_inner.task_status = TaskStatus::Running;
                task_inner.memory_set.activate();
                drop(task_inner);
                processor.current = Some(task);
                unsafe {
                    __switch(idle_task_cx_ptr, next_task_cx_ptr);
                }
            }
            //不切换到内核的地址空间，可能继续运行或转到别的任务
            check_futex_timer();
        }
    }
}
///Take the current task,leaving a None in its place
pub fn take_current_task() -> Option<Arc<TaskControlBlock>> {
    get_proc_by_hartid(hart_id()).take_current()
}
///Get running task
pub fn current_task() -> Option<Arc<TaskControlBlock>> {
    get_proc_by_hartid(hart_id()).current()
}
///Get token of the address space of current task
pub fn current_token() -> usize {
    // get_proc_by_hartid(hart_id()).token()
    riscv::register::satp::read().bits()
}

///Get the mutable reference to trap context of current task
pub fn current_trap_cx() -> &'static mut TrapContext {
    current_task().unwrap().inner_lock().trap_cx()
}
///Return to idle control flow for new scheduling
pub fn schedule(switched_task_cx_ptr: *mut TaskContext) {
    let processor = get_proc_by_hartid(hart_id());
    let idle_task_cx_ptr = processor.get_idle_task_cx_ptr();
    unsafe {
        __switch(switched_task_cx_ptr, idle_task_cx_ptr);
    }
}
