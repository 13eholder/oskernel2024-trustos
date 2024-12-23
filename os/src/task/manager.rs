//!Implementation of [`TaskManager`]
use super::{current_task, TaskControlBlock, TaskStatus, INITPROC};
use alloc::collections::{BTreeMap, VecDeque};
use alloc::sync::Arc;
use alloc::vec::Vec;
use hashbrown::HashSet;
use log::debug;
use spin::{Lazy, Mutex};
///A array of `TaskControlBlock` that is thread-safe
pub struct TaskManager {
    ready_queue: VecDeque<Arc<TaskControlBlock>>,
    stopped_queue: VecDeque<Arc<TaskControlBlock>>,
}

/// A simple FIFO scheduler.
impl TaskManager {
    ///Creat an empty TaskManager
    pub fn new() -> Self {
        Self {
            ready_queue: VecDeque::new(),
            stopped_queue: VecDeque::new(),
        }
    }
    ///Remove the first task and return it,or `None` if `TaskManager` is empty
    pub fn fetch(&mut self) -> Option<Arc<TaskControlBlock>> {
        self.ready_queue.pop_front()
    }
    pub fn ready_procs_num(&self) -> usize {
        self.ready_queue.len()
    }
    pub fn wakeup_parent(&mut self, pid: usize) {
        let idx = self
            .ready_queue
            .iter()
            .enumerate()
            .find(|(_, task)| {
                task.pid() == pid && task.inner_lock().task_status == TaskStatus::Ready
            })
            .map(|(idx, _)| idx);
        if let Some(idx) = idx {
            // log::info!("wake up parent {}", pid);
            let p = self.ready_queue.remove(idx).unwrap();
            self.ready_queue.push_front(p);
        } else {
            // info!("no parent pid=={}", pid);
            // 父进程已被回收,被添加到了initproc下
            if pid != 1 {
                // log::info!("wake up parent {}", 0);
                self.wakeup_parent(1);
            }
        }
    }
}

pub static TASK_MANAGER: Lazy<Mutex<TaskManager>> = Lazy::new(|| Mutex::new(TaskManager::new()));
///Interface offered to add task
pub fn add_task(task: Arc<TaskControlBlock>) {
    let tid = task.tid();
    if if_in_manager(tid) {
        return;
    }
    //debug!("add task {} {}", task.pid(), task.tid());
    TASK_MANAGER.lock().ready_queue.push_back(task);
    insert_in_manager(tid);
}
///Interface offered to pop the first task
pub fn fetch_task() -> Option<Arc<TaskControlBlock>> {
    if let Some(task) = TASK_MANAGER.lock().fetch() {
        remove_in_manager(task.tid());
        return Some(task);
    } else {
        None
    }
}
pub fn wakeup_parent(pid: usize) {
    TASK_MANAGER.lock().wakeup_parent(pid);
}

pub fn ready_procs_num() -> usize {
    TASK_MANAGER.lock().ready_procs_num()
}

pub fn find_task_by_tid(tid: usize) -> Option<Arc<TaskControlBlock>> {
    if current_task().unwrap().tid() == tid {
        current_task()
    } else {
        TASK_MANAGER
            .lock()
            .ready_queue
            .iter()
            .find(|t| t.tid() == tid)
            .cloned()
    }
}

pub fn stop_task(task: Arc<TaskControlBlock>) {
    TASK_MANAGER.lock().stopped_queue.push_back(task);
}

pub fn wakeup_stopped_task(task: Arc<TaskControlBlock>) {
    let mut manager = TASK_MANAGER.lock();
    manager
        .stopped_queue
        .retain(|t| Arc::as_ptr(t) != Arc::as_ptr(&task));
    if manager
        .ready_queue
        .iter()
        .find(|t| t.tid() == task.tid())
        .is_none()
    {
        manager.ready_queue.push_back(task);
    }
}

pub static TID_TO_TASK: Lazy<Mutex<BTreeMap<usize, Arc<TaskControlBlock>>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));

pub fn tid2task(tid: usize) -> Option<Arc<TaskControlBlock>> {
    match TID_TO_TASK.lock().get(&tid) {
        Some(task) => Some(task.clone()),
        None => None,
    }
}

pub fn insert_into_tid2task(tid: usize, task: &Arc<TaskControlBlock>) {
    TID_TO_TASK.lock().insert(tid, task.clone());
}

pub fn remove_from_tid2task(tid: usize) {
    TID_TO_TASK.lock().remove(&tid);
}

pub fn task_num() -> usize {
    TID_TO_TASK.lock().len()
}

/// 线程组
pub static THREAD_GROUP: Lazy<Mutex<BTreeMap<usize, Vec<Arc<TaskControlBlock>>>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));

pub fn insert_into_thread_group(pid: usize, task: &Arc<TaskControlBlock>) {
    THREAD_GROUP
        .lock()
        .entry(pid)
        .or_insert_with(Vec::new)
        .push(task.clone());
}
/// 删除整个线程组,同时将线程从tid2task移除
pub fn remove_all_from_thread_group(pid: usize) {
    THREAD_GROUP.lock().remove(&pid);
}

/// 需要持有Arc,避免进程在exit时被释放,等待到wait释放进程
/// 一个进程组实际上是一个进程的所有子进程
pub static PROCESS_GROUP: Lazy<Mutex<BTreeMap<usize, Vec<Arc<TaskControlBlock>>>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));

pub fn insert_into_process_group(ppid: usize, task: &Arc<TaskControlBlock>) {
    PROCESS_GROUP
        .lock()
        .entry(ppid)
        .or_insert_with(Vec::new)
        .push(Arc::clone(task));
}

pub fn remove_from_process_group(ppid: usize, pid: usize) {
    let mut inner = PROCESS_GROUP.lock();
    if let Some(tasks) = inner.get_mut(&ppid) {
        tasks.retain(|x| x.pid() != pid);
        if tasks.is_empty() {
            inner.remove(&ppid);
        }
    }
}

pub fn move_child_process_to_init(ppid: usize) {
    let mut inner = PROCESS_GROUP.lock();
    if let Some(tasks) = inner.remove(&ppid) {
        let init_childer = inner.get_mut(&INITPROC.pid()).unwrap();
        for child in tasks {
            //debug!("give child {} {}", child.pid(), child.tid());
            init_childer.push(child);
        }
    }
}

pub fn wakeup_futex_task(task: Arc<TaskControlBlock>) {
    let mut task_inner = task.inner_lock();
    task_inner.task_status = TaskStatus::Ready;
    debug!("[futex wakeup task] thread={}", task.tid());
    drop(task_inner);
    add_task(task);
}

pub static TID_IN: Lazy<Mutex<HashSet<usize>>> = Lazy::new(|| Mutex::new(HashSet::new()));

pub fn if_in_manager(tid: usize) -> bool {
    TID_IN.lock().contains(&tid)
}

pub fn insert_in_manager(tid: usize) {
    TID_IN.lock().insert(tid);
}

pub fn remove_in_manager(tid: usize) {
    TID_IN.lock().remove(&tid);
}
