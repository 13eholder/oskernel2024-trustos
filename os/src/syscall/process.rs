use crate::{
    data_flow,
    fs::{open, OpenFlags, NONE_MODE},
    mm::{is_bad_address, VirtAddr},
    signal::check_if_any_sig_for_current_task,
    syscall::{CloneFlags, FutexCmd, FutexOpt, Utsname},
    task::{
        add_task, change_current_uid, current_task, current_uid, exit_current_and_run_next,
        exit_current_group_and_run_next, find_task_by_tid, futex_requeue, futex_wait,
        futex_wake_up, move_child_process_to_init, remove_all_from_thread_group,
        suspend_current_and_run_next, task_num, FutexKey, Sysinfo, PROCESS_GROUP,
    },
    timer::{add_futex_timer, calculate_left_timespec, get_time_ms, get_time_spec, Timespec},
    utils::{c_ptr_to_string, get_abs_path, trim_start_slash, SysErrNo, SyscallRet},
};
use alloc::{
    string::{String, ToString},
    sync::Arc,
    vec::Vec,
};

use log::debug;

/// 参考 https://man7.org/linux/man-pages/man2/exit.2.html
pub fn sys_exit(exit_code: i32) -> ! {
    exit_current_and_run_next(exit_code);
    unreachable!();
}

/// 参考 https://man7.org/linux/man-pages/man2/exit_group.2.html
pub fn sys_exit_group(exit_code: i32) -> SyscallRet {
    exit_current_group_and_run_next(exit_code);
    unreachable!();
}

/// 参考 https://man7.org/linux/man-pages/man2/sched_yield.2.html
pub fn sys_sched_yield() -> SyscallRet {
    suspend_current_and_run_next();
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/getpid.2.html
pub fn sys_getpid() -> SyscallRet {
    Ok(current_task().unwrap().pid())
}

/// 参考 https://man7.org/linux/man-pages/man2/getppid.2.html
pub fn sys_getppid() -> SyscallRet {
    Ok(current_task().unwrap().ppid())
}

/// 参考 https://man7.org/linux/man-pages/man2/getuid.2.html
pub fn sys_getuid() -> SyscallRet {
    Ok(current_uid() as usize)
}

pub fn sys_setuid(uid: u32) -> SyscallRet {
    let task = current_task().unwrap();
    let mut task_inner = task.inner_lock();
    task_inner.user_id = uid;
    change_current_uid(uid);
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/geteuid.2.html
pub fn sys_geteuid() -> SyscallRet {
    Ok(current_uid() as usize)
}

/// 参考 https://man7.org/linux/man-pages/man2/getgid.2.html
pub fn sys_getgid() -> SyscallRet {
    Ok(0) // root group
}

/// 参考 https://man7.org/linux/man-pages/man2/getegid.2.html
pub fn sys_getegid() -> SyscallRet {
    Ok(0) // root group
}

/// 参考 https://man7.org/linux/man-pages/man2/gettid.2.html
pub fn sys_gettid() -> SyscallRet {
    Ok(current_task().unwrap().tid())
}

/// 参考 https://man7.org/linux/man-pages/man2/setsid.2.html
pub fn sys_setsid() -> SyscallRet {
    //涉及到会话和进程组，暂时伪实现
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/set_tid_address.2.html
pub fn sys_settidaddress(tidptr: usize) -> SyscallRet {
    current_task().unwrap().inner_lock().clear_child_tid = tidptr;
    sys_gettid()
}

/// 参考 https://man7.org/linux/man-pages/man2/clone.2.html
/// void (*fn)(void* arg) 参数通过栈传递,如果stack_ptr!=0, fn=0(stack),arg=8(stack)
pub fn sys_clone(
    flags: usize,
    stack_ptr: usize,
    parent_tid_ptr: usize,
    tls_ptr: usize,
    child_tid_ptr: usize,
) -> SyscallRet {
    let flags = CloneFlags::from_bits(flags as u32).unwrap();
    debug!(
        "[sys_clone] flags {:?},stack:{:#x},parent_tid_ptr:{:#x},child_tid_ptr:{:#x},tls_ptr:{:#x}",
        flags, stack_ptr, parent_tid_ptr, child_tid_ptr, tls_ptr
    );

    let task = current_task().unwrap();
    let new_task = task.clone_process(
        flags,
        stack_ptr,
        parent_tid_ptr as *mut u32,
        tls_ptr,
        child_tid_ptr as *mut u32,
    )?;
    let new_tid = new_task.tid();
    // we do not have to move to next instruction since we have done it before
    // add new task to scheduler
    add_task(new_task);
    Ok(new_tid)
}

/// 参考 https://man7.org/linux/man-pages/man2/execve.2.html
pub fn sys_execve(pathp: *const u8, mut argvp: *const usize, mut envp: *const usize) -> SyscallRet {
    let task = current_task().unwrap();
    let task_inner = task.inner_lock();

    let mut argv = Vec::<String>::new();
    let mut env = Vec::<String>::new();
    let mut path;
    unsafe {
        // 处理Path
        path = trim_start_slash(c_ptr_to_string(pathp));
        if path.ends_with(".sh") {
            //.sh文件不是可执行文件，需要用busybox的sh来启动
            argv.push(String::from("busybox"));
            argv.push(String::from("sh"));
            path = String::from("/busybox");
        }

        //处理argv参数
        loop {
            let argv_ptr = *argvp;
            if argv_ptr == 0 {
                break;
            }
            argv.push(c_ptr_to_string(argv_ptr as *const u8));
            argvp = argvp.add(1);
        }

        debug!("[sys_execve] path is {},arg is {:?}", path, argv);

        // 处理envp参数
        if !envp.is_null() {
            loop {
                let envp_ptr = *envp;
                if envp_ptr == 0 {
                    break;
                }
                env.push(c_ptr_to_string(envp_ptr as *const u8));
                envp = envp.add(1);
            }
        }
    }

    let env_path = "PATH=/:/bin:".to_string();
    if !env.contains(&env_path) {
        env.push(env_path);
    }

    let env_ld_library_path = "LD_LIBRARY_PATH=/lib:/lib/glibc:/lib/musl:".to_string();
    if !env.contains(&env_ld_library_path) {
        env.push(env_ld_library_path);
    }

    let env_enough = "ENOUGH=100000".to_string();
    if !env.contains(&env_enough) {
        //设置系统最大负载
        env.push(env_enough);
    }

    debug!("[sys_execve] env is {:?}", env);

    let cwd = if !path.starts_with('/') {
        task_inner.fs_info.cwd()
    } else {
        "/"
    };
    let abs_path = get_abs_path(&cwd, &path);
    let app_inode = open(&abs_path, OpenFlags::O_RDONLY, NONE_MODE)?.file()?;
    task_inner.fs_info.set_exe(abs_path);
    let elf_data = app_inode.inode.read_all()?;
    drop(task_inner);
    task.exec(&elf_data, &argv, &mut env);
    // task.inner_lock().memory_set.activate();
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/futex.2.html
pub fn sys_futex(
    uaddr: *mut i32,
    futex_op: u32,
    val: i32,
    timeoutp: *const Timespec,
    uaddr2: *mut u32,
    _val3: i32,
) -> SyscallRet {
    let cmd = FutexCmd::from_bits(futex_op & 0x7f).unwrap();
    let opt = FutexOpt::from_bits_truncate(futex_op);
    if uaddr.align_offset(4) != 0 {
        return Err(SysErrNo::EINVAL);
    }

    let task = current_task().unwrap();
    let task_inner = task.inner_lock();
    let pa = task_inner
        .memory_set
        .translate_va(VirtAddr::from(uaddr as usize))
        .unwrap();

    let private = opt.contains(FutexOpt::FUTEX_PRIVATE_FLAG);

    let key = if private {
        FutexKey::new(pa, task.pid())
    } else {
        FutexKey::new(pa, 0)
    };

    log::debug!(
        "[sys_futex] uaddr = {:x}, key = {:?}, cmd = {:?}, val = {},opt={:?}",
        uaddr as usize,
        key,
        cmd,
        val,
        opt
    );

    match cmd {
        FutexCmd::FUTEX_WAIT => {
            let futex_word = data_flow!({ *uaddr });
            log::debug!("[sys_futex] futex_word = {}", futex_word,);
            if futex_word != val {
                return Err(SysErrNo::EAGAIN);
            }
            if !timeoutp.is_null() {
                let timeout = data_flow!({ *timeoutp });
                log::debug!("[sys_futex] timeout={:?}", timeout);
                add_futex_timer(get_time_spec() + timeout, current_task().unwrap());
            }
            drop(task_inner);
            drop(task);
            futex_wait(key)
        }
        FutexCmd::FUTEX_WAKE => {
            drop(task_inner);
            drop(task);
            Ok(futex_wake_up(key, val))
        }
        FutexCmd::FUTEX_REQUEUE => {
            let pa2 = task_inner
                .memory_set
                .translate_va(VirtAddr::from(uaddr2 as usize))
                .ok_or(SysErrNo::EINVAL)?;
            let new_key = if private {
                FutexKey::new(pa2, task.pid())
            } else {
                FutexKey::new(pa2, 0)
            };
            drop(task_inner);
            drop(task);
            Ok(futex_requeue(key, val, new_key, timeoutp as i32))
        }
        _ => unimplemented!(),
    }
}

/// 等待子进程状态发生变化,即子进程终止或被信号停止或被信号挂起
/// < -1   meaning wait for any child process whose process group ID
///         is equal to the absolute value of pid.
///-1     meaning wait for any child process.
///0      meaning wait for any child process whose process group ID
///       is equal to that of the calling process at the time of the call to waitpid().
///> 0    meaning wait for the child whose process ID is equal to the value of pid.
/// 参考 https://man7.org/linux/man-pages/man2/wait4.2.html
pub fn sys_wait4(pid: isize, wstatus: *mut i32, options: i32) -> SyscallRet {
    //assert!(options == 0, "not support options yet");
    //默认所有进程都在同一个组
    // debug!(
    //     "[sys_wait4] pid is {},wstatus is {:#x}, options is {}",
    //     pid, wstatus as usize, options
    // );
    if (pid as i32) == i32::MIN {
        return Err(SysErrNo::ESRCH);
    }
    if options < 0 || options > 100 {
        return Err(SysErrNo::EINVAL);
    }
    loop {
        let mut process_group = PROCESS_GROUP.lock();
        let task = current_task().unwrap();
        let task_inner = task.inner_lock();
        // 暂时没有子进程,返回即可
        if !process_group.contains_key(&task.pid()) {
            // debug!("[sys_wait4] no child process");
            return Err(SysErrNo::ECHILD);
        }
        // 只有fork出来的子进程会被放入
        let children = process_group.get_mut(&task.pid()).unwrap();
        if !children
            .iter()
            .any(|p| pid == -1 || pid as usize == p.pid())
        {
            // debug!("[sys_wait4] no child process");
            return Err(SysErrNo::ECHILD);
        }
        // 寻找符合条件的进程组
        // 使用clone避免多次借用或借用冲突
        let pair = children
            .iter()
            .enumerate()
            .find(|(_, p)| {
                // ++++ temporarily access child PCB exclusively
                p.inner_lock().is_group_exit() && (pid == -1 || pid as usize == p.pid())
                // ++++ release child PCB
            })
            .map(|(idx, t)| (idx, Arc::clone(t)));

        if let Some((idx, child)) = pair {
            let found_pid = child.pid();
            let child_inner = child.inner_lock();
            let exit_code = child_inner.sig_table.exit_code();
            if wstatus as usize != 0x0 {
                debug!(
                    "[sys_wait4] wait pid {}: child {} exit with code {}, wstatus= {:#x}",
                    pid, found_pid, exit_code, wstatus as usize
                );
                if exit_code >= 128 && exit_code <= 255 {
                    //表示由于信号而退出的
                    data_flow!({
                        *wstatus = exit_code;
                    });
                } else {
                    data_flow!({
                        *wstatus = exit_code << 8;
                    });
                }
            }
            drop(child_inner);
            // 从父进程的子进程组移除
            children.remove(idx);
            drop(task_inner);
            drop(task);
            drop(process_group);
            // 从线程组移除
            remove_all_from_thread_group(found_pid);
            // 转移子进程
            move_child_process_to_init(found_pid);

            assert_eq!(
                Arc::strong_count(&child),
                1,
                "process{} can't recycled",
                child.pid()
            );
            return Ok(found_pid);
        } else {
            drop(task_inner);
            drop(task);
            drop(process_group);
            suspend_current_and_run_next();
        }
    }
}

/// 参考 https://man7.org/linux/man-pages/man2/nanosleep.2.html
pub fn sys_nanosleep(req_ptr: *const Timespec, rem: *mut Timespec) -> SyscallRet {
    if (req_ptr as isize) <= 0 || is_bad_address(req_ptr as usize) {
        return Err(SysErrNo::EFAULT);
    }

    if (rem as isize) < 0 || is_bad_address(rem as usize) {
        return Err(SysErrNo::EFAULT);
    }

    // debug!(
    //     "[sys_nanosleep] req is {:x}, rem is {:x}",
    //     req as usize, rem as usize
    // );
    let req = data_flow!({ *req_ptr });
    let waittime = req.tv_sec * 1_000_000_000usize + req.tv_nsec;
    let begin = get_time_ms() * 1_000_000usize;
    let endtime = get_time_spec() + req;

    if (req.tv_sec as isize) < 0 || (req.tv_nsec as isize) < 0 || req.tv_nsec >= 1000_000_000usize {
        return Err(SysErrNo::EINVAL);
    }

    // debug!(
    //     "[sys_nanosleep] ready to sleep for {} sec, {} nsec",
    //     req.tv_sec, req.tv_nsec
    // );

    while get_time_ms() * 1_000_000usize - begin < waittime {
        if let Some(_) = check_if_any_sig_for_current_task() {
            //被信号唤醒
            if rem as usize != 0 {
                data_flow!({
                    *rem = calculate_left_timespec(endtime);
                });
            }
            return Err(SysErrNo::EINTR);
        }
        suspend_current_and_run_next();
    }
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/uname.2.html
pub fn sys_uname(uname_addr: *mut Utsname) -> SyscallRet {
    if (uname_addr as isize) < 0 || is_bad_address(uname_addr as usize) {
        return Err(SysErrNo::EFAULT);
    }

    fn str2u8(s: &str) -> [u8; 65] {
        let mut b = [0; 65];
        b[0..s.len()].copy_from_slice(s.as_bytes());
        b
    }
    let uname = Utsname {
        sysname: str2u8("Linux"),
        nodename: str2u8("TrustOS"),
        release: str2u8("5.0.0"),
        version: str2u8("5.0.0"),
        machine: str2u8("RISC-V64"),
        domainname: str2u8("TrustOS"),
    };
    data_flow!({ *uname_addr = uname });
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/brk.2.html
pub fn sys_brk(brk_addr: usize) -> SyscallRet {
    let former_addr = current_task().unwrap().growproc(0);
    if brk_addr == 0 {
        return Ok(former_addr);
    }
    let grow_size: isize = (brk_addr - former_addr) as isize;
    Ok(current_task().unwrap().growproc(grow_size))
}

/// 参考 https://man7.org/linux/man-pages/man2/sysinfo.2.html
pub fn sys_sysinfo(info: *mut Sysinfo) -> SyscallRet {
    data_flow!({ *info = Sysinfo::new(get_time_ms() / 1000, 1 << 56, task_num()) });
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/umask.2.html
pub fn sys_umask(_mask: u32) -> SyscallRet {
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/syslog.2.html
pub fn sys_syslog(_logtype: isize, _bufp: *const u8, _len: usize) -> SyscallRet {
    // 伪实现
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/sched_setaffinity.2.html
pub fn sys_sched_setaffinity(_pid: usize, _cpusetsize: usize, _mask: usize) -> SyscallRet {
    // debug!(
    //     "[sys_sched_setaffinity] pid is {}, cpusetsize is {}, mask is {}",
    //     pid, cpusetsize, mask
    // );
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/sched_getaffinity.2.html
pub fn sys_sched_getaffinity(_pid: usize, _cpusetsize: usize, _mask: usize) -> SyscallRet {
    // debug!(
    //     "[sys_sched_getaffinity] pid is {}, cpusetsize is {}, mask is {}",
    //     pid, cpusetsize, mask
    // );
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/sched_setscheduler.2.html
pub fn sys_sched_setscheduler(_pid: usize, _policy: usize, _param: *const u8) -> SyscallRet {
    // debug!(
    //     "[sys_sched_setscheduler] pid is {}, policy is {}, param is {:x}",
    //     pid, policy, param as usize
    // );
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/sched_getscheduler.2.html
pub fn sys_sched_getscheduler(_pid: usize) -> SyscallRet {
    // debug!("[sys_sched_getscheduler] pid is {}", pid);
    //由于使用的是标准的时间片调度算法，直接返回SCHED_OHTER = 0
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/sched_getparam.2.html
pub fn sys_sched_getparam(_pid: usize, _param: *const u8) -> SyscallRet {
    // debug!(
    //     "[sys_sched_getparam] pid is {}, param is {:x}",
    //     pid, param as usize
    // );
    //由于使用的是标准的时间片调度算法，param参数需要被忽略
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/clock_nanosleep.2.html
pub fn sys_clock_nanosleep(
    clockid: usize,
    flags: u32,
    time_ptr: *const Timespec,
    remain: *mut Timespec,
) -> SyscallRet {
    const TIME_ABSTIME: u32 = 1;
    let task = current_task().unwrap();
    let task_inner = task.inner_lock();
    drop(task_inner);
    drop(task);

    if clockid != 0 && clockid != 1 {
        return Err(SysErrNo::EOPNOTSUPP);
    }

    if (time_ptr as isize) <= 0 || is_bad_address(time_ptr as usize) {
        return Err(SysErrNo::EFAULT);
    }

    if (remain as isize) < 0 || is_bad_address(remain as usize) {
        return Err(SysErrNo::EFAULT);
    }

    debug!(
        "[sys_clock_nanosleep] clockid is {}, flags is {}, t is {:x}, remain is {:x}",
        clockid, flags, time_ptr as usize, remain as usize
    );

    // let t = get_data(memory_set.token(), time_ptr);
    let t = data_flow!({ *time_ptr });

    if (t.tv_sec as isize) < 0 || (t.tv_nsec as isize) < 0 || t.tv_nsec >= 1_000_000_000usize {
        return Err(SysErrNo::EINVAL);
    }

    let waittime = t.tv_sec * 1_000_000_000usize + t.tv_nsec;

    let begin = get_time_ms() * 1_000_000usize;
    let endtime = if flags == TIME_ABSTIME {
        //绝对时间
        t
    } else {
        //相对时间
        get_time_spec() + t
    };

    debug!(
        "[sys_clock_nanosleep] ready to sleep for {} sec, {} nsec",
        t.tv_sec, t.tv_nsec
    );

    while get_time_ms() * 1_000_000usize - begin < waittime {
        if let Some(_) = check_if_any_sig_for_current_task() {
            //被信号唤醒
            debug!("interupt by signal");
            if remain as usize != 0 {
                // put_data(memory_set, remain, calculate_left_timespec(endtime));
                data_flow!({ *remain = calculate_left_timespec(endtime) })
            }
            //handle_signal(signo);
            return Err(SysErrNo::EINTR);
        }
        suspend_current_and_run_next();
    }
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/set_robust_list.2.html
pub fn sys_set_robust_list(head: usize, len: usize) -> SyscallRet {
    if len != crate::task::RobustList::HEAD_SIZE {
        return Err(SysErrNo::EINVAL);
    }
    let task = current_task().unwrap();
    let mut task_inner = task.inner_lock();
    task_inner.robust_list.head = head;
    //inner.robust_list.len = len;
    Ok(0)
}

/// 参考 https://man7.org/linux/man-pages/man2/get_robust_list.2.html
pub fn sys_get_robust_list(pid: usize, head_ptr: *mut usize, len_ptr: *mut usize) -> SyscallRet {
    let mut task = find_task_by_tid(pid);
    if task.is_none() && pid == 0 {
        task = current_task();
    }
    if let Some(task) = task {
        let task_inner = task.inner_lock();
        data_flow!({
            *head_ptr = task_inner.robust_list.head;
            *len_ptr = task_inner.robust_list.len;
        });
        Ok(0)
    } else {
        Err(SysErrNo::ESRCH)
    }
}
