//! RISC-V timer-related functionality

use core::ops::Add;

use crate::sbi::set_timer;
use crate::signal::SigSet;
use crate::sync::SyncUnsafeCell;
use crate::task::TaskControlBlock;
use crate::{config::board::CLOCK_FREQ, task::wakeup_futex_task};
use alloc::{
    collections::BinaryHeap,
    sync::{Arc, Weak},
};
use core::cmp::Ordering;
use log::debug;
use riscv::register::time;
use spin::{Lazy, Mutex};

const TICKS_PER_SEC: usize = 100;
const MSEC_PER_SEC: usize = 1000;
#[allow(unused)]
const USEC_PER_SEC: usize = 1000000;
const NSEC_PER_SEC: usize = 1000000000;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Timespec {
    pub tv_sec: usize,  //秒
    pub tv_nsec: usize, //纳秒
}

impl Timespec {
    pub fn new(sec: usize, nsec: usize) -> Self {
        Self {
            tv_sec: sec,
            tv_nsec: nsec,
        }
    }
    pub fn to_tick(&self) -> usize {
        self.tv_sec * CLOCK_FREQ + (self.tv_nsec * CLOCK_FREQ / NSEC_PER_SEC)
    }
}

impl Add for Timespec {
    type Output = Timespec;
    fn add(self, rhs: Self) -> Self::Output {
        let mut tv_sec = self.tv_sec + rhs.tv_sec;
        let mut tv_nsec = self.tv_nsec + rhs.tv_nsec;
        tv_sec += tv_nsec / (1_000_000_000usize);
        tv_nsec %= 1_000_000_000usize;
        Self { tv_sec, tv_nsec }
    }
}

impl PartialOrd for Timespec {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.tv_sec > other.tv_sec {
            Some(Ordering::Greater)
        } else if self.tv_sec < other.tv_sec {
            Some(Ordering::Less)
        } else {
            if self.tv_nsec > other.tv_nsec {
                Some(Ordering::Greater)
            } else if self.tv_nsec < other.tv_nsec {
                Some(Ordering::Less)
            } else {
                Some(Ordering::Equal)
            }
        }
    }
}

pub fn calculate_left_timespec(endtime: Timespec) -> Timespec {
    let nowtime = get_time_spec();
    let mut endsec = endtime.tv_sec;
    let mut nsec: isize = endtime.tv_nsec as isize - nowtime.tv_nsec as isize;
    if nsec < 0 {
        endsec -= 1;
        nsec = 1_000_000_000isize + nsec;
    }
    Timespec {
        tv_sec: endsec - nowtime.tv_sec,
        tv_nsec: nsec as usize,
    }
}

pub struct Tms {
    pub tms_utime: isize,  //用户模式下花费的CPU时间
    pub tms_stime: isize,  //内核模式下花费的CPU时间
    pub tms_cutime: isize, //子进程在用户模式下花费的CPU时间
    pub tms_cstime: isize, //子进程在内核模式下花费的CPU时间
}

impl Tms {
    pub fn new(time_data: &TimeData) -> Self {
        Self {
            tms_utime: time_data.utime,
            tms_stime: time_data.stime,
            tms_cutime: time_data.cutime,
            tms_cstime: time_data.cstime,
        }
    }
}

#[derive(Clone)]
pub struct TimeData {
    pub utime: isize,
    pub stime: isize,
    pub cutime: isize,
    pub cstime: isize,
    pub lasttime: isize,
}

impl TimeData {
    pub fn new() -> Self {
        let now = (get_time_ms()) as isize;
        Self {
            utime: 0,
            stime: 0,
            cutime: 0,
            cstime: 0,
            lasttime: now,
        }
    }
    pub fn update_utime(&mut self) {
        let now = (get_time_ms()) as isize;
        let duration = now - self.lasttime;
        self.utime += duration;
        self.lasttime = now;
    }
    pub fn update_stime(&mut self) {
        let now = (get_time_ms()) as isize;
        let duration = now - self.lasttime;
        self.stime += duration;
        self.lasttime = now;
    }
    pub fn clear(&mut self) {
        let now = (get_time_ms()) as isize;
        self.utime = 0;
        self.stime = 0;
        self.cutime = 0;
        self.cstime = 0;
        self.lasttime = now;
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Itimerval {
    /// Interval for periodic timer
    pub it_interval: TimeVal,
    /// Time until next expiration
    pub it_value: TimeVal,
}

impl Itimerval {
    pub fn new() -> Self {
        Self {
            it_interval: TimeVal::new(0, 0),
            it_value: TimeVal::new(0, 0),
        }
    }
}
///以实际（即挂钟）时间倒计时。在每次到期时，都会生成一个 SIGALRM 信号
pub const ITIMER_REAL: usize = 0;
/// 此计时器根据进程消耗的用户模式 CPU 时间倒计时。（测量值包括进程中所有线程消耗的 CPU 时间。
/// 在每次到期时，都会生成一个 SIGVTALRM 信号
pub const ITIMER_VIRTUAL: usize = 1;
/// 此计时器根据进程消耗的总 CPU 时间（即用户和系统）进行倒计时。（测量值包括进程中所有线程消耗的 CPU 时间。
/// 在每次到期时，都会生成一个 SIGPROF 信号。
pub const ITIMER_PROF: usize = 2;

/// 三种 itimer,实际只会使用ITIMER_REAL
pub struct Timer {
    pub inner: SyncUnsafeCell<TimerInner>,
}

pub struct TimerInner {
    pub timer: Itimerval,
    pub last_time: TimeVal,
    pub once: bool,
    pub sig: SigSet,
}

impl TimerInner {
    pub fn new() -> Self {
        Self {
            timer: Itimerval::new(),
            last_time: TimeVal::new(0, 0),
            once: false,
            sig: SigSet::empty(),
        }
    }
}

impl Timer {
    pub fn new() -> Self {
        Self {
            inner: SyncUnsafeCell::new(TimerInner::new()),
        }
    }
    pub fn set_timer(&self, new: Itimerval, newsig: SigSet) {
        let inner = self.inner.get_unchecked_mut();
        inner.timer = new;
        inner.once = false;
        inner.last_time = TimeVal::new(0, 0);
        inner.sig = newsig;
    }
    pub fn set_last_time(&self, last_time: TimeVal) {
        self.inner.get_unchecked_mut().last_time = last_time;
    }
    pub fn set_trigger_once(&self, once: bool) {
        self.inner.get_unchecked_mut().once = once;
    }
    pub fn trigger_once(&self) -> bool {
        self.inner.get_unchecked_ref().once
    }
    pub fn last_time(&self) -> TimeVal {
        self.inner.get_unchecked_ref().last_time
    }
    pub fn timer(&self) -> Itimerval {
        self.inner.get_unchecked_ref().timer
    }
    pub fn sig(&self) -> SigSet {
        self.inner.get_unchecked_ref().sig
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TimeVal {
    pub tv_sec: usize,  //秒
    pub tv_usec: usize, //微秒
}

impl TimeVal {
    pub fn new(sec: usize, usec: usize) -> Self {
        Self {
            tv_sec: sec,
            tv_usec: usec,
        }
    }
    pub fn now() -> Self {
        let now_time = get_time_ms();
        Self {
            tv_sec: now_time / 1000,
            tv_usec: (now_time % 1000) * 1000,
        }
    }
    pub fn is_empty(&self) -> bool {
        self.tv_sec == 0 && self.tv_usec == 0
    }
}

impl Add for TimeVal {
    type Output = TimeVal;

    fn add(self, rhs: Self) -> Self::Output {
        let usec = self.tv_usec + rhs.tv_usec;
        Self {
            tv_sec: self.tv_sec + rhs.tv_sec + usec / 1_000_000,
            tv_usec: usec % 1_000_000,
        }
    }
}

impl PartialOrd for TimeVal {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        if self.tv_sec > other.tv_sec {
            Some(Ordering::Greater)
        } else if self.tv_sec < other.tv_sec {
            Some(Ordering::Less)
        } else {
            if self.tv_usec > other.tv_usec {
                Some(Ordering::Greater)
            } else if self.tv_usec < other.tv_usec {
                Some(Ordering::Less)
            } else {
                Some(Ordering::Equal)
            }
        }
    }
}
#[allow(unused)]
pub struct Rusage {
    pub ru_utime: TimeVal,
    pub ru_stime: TimeVal,
    //unused but needed
    ru_maxrss: isize,
    ru_ixrss: isize,
    ru_idrss: isize,
    ru_isrss: isize,
    ru_minflt: isize,
    ru_majflt: isize,
    ru_nswap: isize,
    ru_inblock: isize,
    ru_oublock: isize,
    ru_msgsnd: isize,
    ru_msgrcv: isize,
    ru_nsignals: isize,
    ru_nvcsw: isize,
    ru_nivcsw: isize,
}

impl Rusage {
    pub fn new_from_ms(utime: usize, stime: usize) -> Self {
        let utimeval = TimeVal::new(utime / 1000, (utime % 1000) * 1000);
        let stimeval = TimeVal::new(stime / 1000, (stime % 1000) * 1000);
        Self {
            ru_utime: utimeval,
            ru_stime: stimeval,
            ru_maxrss: 0,
            ru_ixrss: 0,
            ru_idrss: 0,
            ru_isrss: 0,
            ru_minflt: 0,
            ru_majflt: 0,
            ru_nswap: 0,
            ru_inblock: 0,
            ru_oublock: 0,
            ru_msgsnd: 0,
            ru_msgrcv: 0,
            ru_nsignals: 0,
            ru_nvcsw: 0,
            ru_nivcsw: 0,
        }
    }
}

///get current time
pub fn get_time() -> usize {
    time::read()
}
/// get current time in microseconds
pub fn get_time_ms() -> usize {
    time::read() / (CLOCK_FREQ / MSEC_PER_SEC)
}

pub fn get_time_spec() -> Timespec {
    let time = get_time_ms();
    Timespec::new(time / 1000, (time % 1000) * 1000000)
}
/// set the next timer interrupt
pub fn set_next_trigger() {
    const TIME_SLICE: usize = CLOCK_FREQ / TICKS_PER_SEC;
    set_timer(get_time() + TIME_SLICE);
}

#[derive(Debug, PartialEq, Eq)]
pub enum TimerType {
    Futex,
    StoppedTask,
}

/// 时钟计数器，与 itimer 间隔定时器不同，用于阻塞唤醒进程
pub struct TimerCondVar {
    pub expire: Timespec,
    pub task: Weak<TaskControlBlock>,
    pub kind: TimerType,
}
impl PartialEq for TimerCondVar {
    fn eq(&self, other: &Self) -> bool {
        self.expire == other.expire
    }
}
impl Eq for TimerCondVar {}
impl PartialOrd for TimerCondVar {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        let a = -(self.expire.to_tick() as isize);
        let b = -(other.expire.to_tick() as isize);
        Some(a.cmp(&b))
    }
}
impl Ord for TimerCondVar {
    fn cmp(&self, other: &Self) -> Ordering {
        self.partial_cmp(other).unwrap()
    }
}

pub static TIMERS: Lazy<Mutex<BinaryHeap<TimerCondVar>>> =
    Lazy::new(|| Mutex::new(BinaryHeap::<TimerCondVar>::new()));

pub fn add_futex_timer(expire: Timespec, task: Arc<TaskControlBlock>) {
    let mut timers = TIMERS.lock();
    debug!("add futex timer task {} {}", task.pid(), task.tid());
    timers.push(TimerCondVar {
        expire,
        task: Arc::downgrade(&task),
        kind: TimerType::Futex,
    });
}

pub fn add_stopped_task_timer(expire: Timespec, task: Arc<TaskControlBlock>) {
    let mut timers = TIMERS.lock();
    timers.push(TimerCondVar {
        expire,
        task: Arc::downgrade(&task),
        kind: TimerType::StoppedTask,
    });
}

pub fn check_futex_timer() {
    let mut timers = TIMERS.lock();
    let current = get_time_spec();
    while let Some(timer) = timers.peek() {
        // debug!("expire={:?}, current={:?}", timer.expire, current);
        if timer.expire <= current {
            if let Some(task) = timer.task.upgrade() {
                debug!("[check_timer] wake up task {} {}", task.pid(), task.tid());
                if timer.kind == TimerType::Futex {
                    // 调用 wakeup_task 唤醒超时线程
                    wakeup_futex_task(Arc::clone(&task));
                } else if timer.kind == TimerType::StoppedTask {
                    todo!()
                    // wakeup_stopped_task(Arc::clone(&task));
                }
            }
            timers.pop();
        } else {
            break;
        }
    }
}
