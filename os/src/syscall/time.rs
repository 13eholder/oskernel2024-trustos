use crate::data_flow;
use crate::mm::is_bad_address;
use crate::signal::SigSet;
use crate::task::current_task;
use crate::timer::{
    get_time_spec, Itimerval, Rusage, TimeVal, Timespec, Tms, ITIMER_PROF, ITIMER_REAL,
    ITIMER_VIRTUAL,
};
use crate::utils::{SysErrNo, SyscallRet};
use log::debug;

const MAX_CLOCKS: usize = 12;

/// 参考 https://man7.org/linux/man-pages/man2/gettimeofday.2.html
pub fn sys_gettimeofday(ts: *mut Timespec, tz: usize) -> SyscallRet {
    if (ts as isize) < 0 || is_bad_address(ts as usize) {
        return Err(SysErrNo::EFAULT);
    }

    if (tz as isize) < 0 || is_bad_address(tz as usize) {
        return Err(SysErrNo::EFAULT);
    }
    data_flow!({ *ts = get_time_spec() });
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/times.2.html
pub fn sys_times(tms: *mut Tms) -> SyscallRet {
    let task = current_task().unwrap();
    let task_inner = task.inner_lock();
    data_flow!({ *tms = Tms::new(&task_inner.time_data) });
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/setitimer.2.html
pub fn sys_settimer(
    which: usize,
    new_value: *const Itimerval,
    old_value: *mut Itimerval,
) -> SyscallRet {
    if (which as isize) < 0 {
        return Err(SysErrNo::EINVAL);
    }
    if (new_value as isize) < 0 || is_bad_address(new_value as usize) {
        return Err(SysErrNo::EFAULT);
    }
    if (old_value as isize) < 0 || is_bad_address(old_value as usize) {
        return Err(SysErrNo::EFAULT);
    }

    let task = current_task().unwrap();
    let task_inner = task.inner_lock();

    let sig = match which {
        ITIMER_REAL => SigSet::SIGALRM,
        ITIMER_VIRTUAL => SigSet::SIGVTALRM,
        ITIMER_PROF => SigSet::SIGPROF,
        _ => return Err(SysErrNo::EINVAL),
    };

    if old_value as usize != 0 {
        data_flow!({
            *old_value = task_inner.timer.timer();
        });
    }
    if new_value as usize != 0 {
        let new_timer = data_flow!({ *new_value });
        debug!("[sys_settimer] new_timer={:?}", new_timer);
        task_inner.timer.set_timer(new_timer, sig);
        task_inner.timer.set_last_time(TimeVal::now());
        if new_timer.it_interval.is_empty() {
            if !new_timer.it_value.is_empty() {
                task_inner.timer.set_trigger_once(true);
            }
        } else {
            task_inner.timer.set_trigger_once(false);
        }
    }
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/clock_gettime.2.html
pub fn sys_clock_gettime(clockid: usize, tp: *mut Timespec) -> SyscallRet {
    let time = get_time_spec();

    if (tp as isize) <= 0 || is_bad_address(tp as usize) {
        return Err(SysErrNo::EFAULT);
    }

    if clockid >= MAX_CLOCKS {
        return Err(SysErrNo::EINVAL);
    }

    data_flow!({
        *tp = time;
    });
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/getrusage.2.html
pub fn sys_getrusage(who: isize, usage: *mut Rusage) -> SyscallRet {
    // TrustOS目前只支持 RUSAGESELF 和 RUSAGECHILDEN
    // debug!(
    //     "[sys_getrusage] who is {}, usage is {:x}",
    //     who, usage as usize
    // );

    if who < -1 {
        return Err(SysErrNo::EINVAL);
    }

    if (usage as isize) < 0 || is_bad_address(usage as usize) {
        return Err(SysErrNo::EFAULT);
    }

    const RUSAGESELF: isize = 0;
    const RUSAGECHILDEN: isize = -1;

    let task = current_task().unwrap();
    let inner = task.inner_lock();

    match who {
        RUSAGESELF => {
            let gotusage = Rusage::new_from_ms(
                inner.time_data.utime as usize,
                inner.time_data.stime as usize,
            );
            data_flow!({ *usage = gotusage });
            Ok(0)
        }
        RUSAGECHILDEN => {
            let gotusage = Rusage::new_from_ms(
                inner.time_data.cutime as usize,
                inner.time_data.cstime as usize,
            );
            data_flow!({ *usage = gotusage });
            Ok(0)
        }
        _ => return Err(SysErrNo::EINVAL),
    }
}

/// 参考 https://man7.org/linux/man-pages/man2/clock_getres.2.html
pub fn sys_clock_getres(clockid: usize, res: *mut Timespec) -> SyscallRet {
    debug!(
        "[sys_clock_getres] clockid is {}, res is {:x}",
        clockid, res as usize
    );

    if (clockid as isize) < 0 {
        return Err(SysErrNo::EINVAL);
    }

    data_flow!({
        *res = Timespec::new(0, 1);
    });
    Ok(0)
}
