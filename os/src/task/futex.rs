use crate::{
    mm::PhysAddr,
    utils::{SysErrNo, SyscallRet},
};

use super::{block_current_and_run_next, current_task, wakeup_futex_task, TaskControlBlock};
use alloc::{
    collections::{BTreeMap, VecDeque},
    sync::{Arc, Weak},
};
use spin::{Lazy, Mutex};

type WaitQueue = VecDeque<Weak<TaskControlBlock>>;

/// 如果是PRIVATE_FUTEX,pid为进程的pid,否则pid为0(用户进程pid从1开始,0未被使用)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FutexKey {
    pa: PhysAddr,
    pid: usize,
}

impl FutexKey {
    pub fn new(pa: PhysAddr, pid: usize) -> Self {
        Self { pa, pid }
    }
}

pub static FUTEX_QUEUE: Lazy<Mutex<BTreeMap<FutexKey, WaitQueue>>> =
    Lazy::new(|| Mutex::new(BTreeMap::new()));

pub fn futex_wait(key: FutexKey) -> SyscallRet {
    let mut waitq = FUTEX_QUEUE.lock();
    let task = current_task().unwrap();
    if let Some(queue) = waitq.get_mut(&key) {
        queue.push_back(Arc::downgrade(&task));
    } else {
        waitq.insert(key, {
            let mut queue = VecDeque::new();
            queue.push_back(Arc::downgrade(&task));
            queue
        });
    }
    log::debug!("[futex_wait] blocked!");
    drop(task);
    drop(waitq);
    block_current_and_run_next();
    let task = current_task().unwrap();
    let task_inner = task.inner_lock();
    // woke by signal
    if !task_inner
        .sig_pending
        .difference(task_inner.sig_mask)
        .is_empty()
    {
        return Err(SysErrNo::EINTR);
    }
    Ok(0)
}

pub fn futex_wake_up(key: FutexKey, max_num: i32) -> usize {
    log::debug!(
        "[sys_futex] futex wakeup thread,max_num={},key={:?}",
        max_num,
        key
    );
    let mut futex_queue = FUTEX_QUEUE.lock();
    let mut num = 0;
    if let Some(queue) = futex_queue.get_mut(&key) {
        loop {
            if num >= max_num as usize {
                break;
            }
            if let Some(weak_task) = queue.pop_front() {
                if let Some(task) = weak_task.upgrade() {
                    //debug!("wake up task {}", task.pid());
                    wakeup_futex_task(task);
                    num += 1;
                }
            } else {
                break;
            }
        }
    }
    num
}

pub fn futex_requeue(old_key: FutexKey, max_num: i32, new_key: FutexKey, max_num2: i32) -> usize {
    log::debug!(
        "[futex_requeue],old_key={:?},max_num={},new_key={:?},max_num2={}",
        old_key,
        max_num,
        new_key,
        max_num2
    );
    let mut futex_queue = FUTEX_QUEUE.lock();
    let mut num = 0;
    let mut num2 = 0;
    let mut tmp = VecDeque::new();
    if let Some(queue) = futex_queue.get_mut(&old_key) {
        while let Some(weak_task) = queue.pop_front() {
            if let Some(task) = weak_task.upgrade() {
                if num < max_num {
                    wakeup_futex_task(task);
                    num += 1;
                } else if num2 < max_num2 {
                    tmp.push_back(Arc::downgrade(&task));
                    num2 += 1;
                }
            }
        }
    }
    if !tmp.is_empty() {
        futex_queue
            .entry(new_key)
            .or_insert_with(VecDeque::new)
            .extend(tmp);
    }
    num as usize
}
