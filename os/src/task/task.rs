//!Implementation of [`TaskControlBlock`]
use super::{
    aux::{Aux, AuxType},
    insert_into_process_group, insert_into_tid2task, tid_alloc, KernelStack, TaskContext,
    TidHandle,
};
use crate::{
    config::mm::{
        PAGE_SIZE, PRE_ALLOC_PAGES, USER_HEAP_SIZE, USER_STACK_SIZE, USER_STACK_TOP,
        USER_TRAP_CONTEXT_TOP,
    },
    data_flow,
    fs::{FdTable, FsInfo},
    mm::{
        flush_tlb, put_data, MapAreaType, MapPermission, MemorySet, MemorySetInner, PageTable,
        PageTableEntry, PhysPageNum, VPNRange, VirtAddr, VirtPageNum,
    },
    signal::{SigSet, SigTable},
    syscall::CloneFlags,
    task::insert_into_thread_group,
    timer::{TimeData, TimeVal, Timer},
    trap::TrapContext,
    utils::{get_abs_path, is_abs_path, SysErrNo},
};
use alloc::{string::String, sync::Arc, vec::Vec};
use core::mem::size_of;
use spin::{Mutex, MutexGuard};

#[derive(Clone, Copy, Debug)]
pub struct RobustList {
    pub head: usize,
    pub len: usize,
}

impl RobustList {
    // from strace
    pub const HEAD_SIZE: usize = 24;
    pub fn default() -> Self {
        RobustList { head: 0, len: 24 }
    }
}

pub struct TaskControlBlock {
    // immutable
    tid: TidHandle,
    /// 父进程退出后,子进程会被挂到initproc下,此时需要改变.
    /// 但只有wakeup_parent会用到改变后的值,手动特殊处理了
    /// 其他地方若有需求再修改
    ppid: usize,
    pid: usize,
    pub kernel_stack: KernelStack,
    // mutable
    inner: Mutex<TaskControlBlockInner>,
}

pub struct TaskControlBlockInner {
    pub trap_cx_ppn: PhysPageNum,
    pub trap_cx_bottom: usize,
    pub user_stack_top: usize, // exclusive
    pub task_cx: TaskContext,
    pub task_status: TaskStatus,
    pub memory_set: Arc<MemorySet>,
    pub fd_table: Arc<FdTable>,
    pub fs_info: Arc<FsInfo>,
    pub time_data: TimeData,
    pub user_heappoint: usize,  //堆顶指针,小于等于user_heaptop
    pub user_heapbottom: usize, //堆底指针
    pub set_child_tid: usize,
    pub clear_child_tid: usize,
    pub sig_table: Arc<SigTable>,
    pub sig_mask: SigSet,
    pub sig_pending: SigSet,
    pub timer: Arc<Timer>,
    pub robust_list: RobustList,
    pub user_id: u32,
}

impl TaskControlBlockInner {
    pub fn trap_cx(&self) -> &'static mut TrapContext {
        self.trap_cx_ppn.as_mut()
    }
    pub fn user_token(&self) -> usize {
        self.memory_set.token()
    }
    fn status(&self) -> TaskStatus {
        self.task_status
    }
    pub fn is_zombie(&self) -> bool {
        self.status() == TaskStatus::Zombie
    }
    pub fn is_group_exit(&self) -> bool {
        self.sig_table.is_exited()
    }
    pub fn alloc_user_res(&mut self) {
        let (ustack_bottom, ustack_top) = self.memory_set.lazy_insert_framed_area_with_hint(
            USER_STACK_TOP,
            USER_STACK_SIZE,
            MapPermission::R | MapPermission::W | MapPermission::U,
            MapAreaType::Stack,
        );
        let (trap_cx_bottom, _) = self.memory_set.insert_framed_area_with_hint(
            USER_TRAP_CONTEXT_TOP,
            PAGE_SIZE,
            MapPermission::R | MapPermission::W,
            MapAreaType::Trap,
        );
        let trap_cx_ppn = self
            .memory_set
            .translate(VirtAddr::from(trap_cx_bottom).floor())
            .unwrap()
            .ppn();
        self.user_stack_top = ustack_top;
        self.trap_cx_ppn = trap_cx_ppn;
        self.trap_cx_bottom = trap_cx_bottom;

        let user_stack_range = VPNRange::new(
            VirtAddr::from(ustack_bottom).floor(),
            VirtAddr::from(ustack_top).floor(),
        );
        //预先为栈顶分配几页，用于环境变量等初始数据
        let page_table = PageTable::from_token(self.memory_set.token());
        let area = self
            .memory_set
            .get_mut()
            .areas
            .iter_mut()
            .find(|area| area.vpn_range.range() == user_stack_range.range())
            .unwrap();
        for i in 1..=PRE_ALLOC_PAGES {
            let vpn = (area.vpn_range.end().0 - i).into();
            let pte: Option<PageTableEntry> = page_table.translate(vpn);
            if pte.is_none() || !pte.unwrap().is_valid() {
                area.map_one(&mut self.memory_set.get_mut().page_table, vpn);
            }
        }
    }
    pub fn clone_user_res(&mut self, another: &TaskControlBlockInner) {
        self.alloc_user_res();
        self.memory_set.lazy_clone_area(
            VirtAddr::from(self.user_stack_top - USER_STACK_SIZE).floor(),
            another.memory_set.get_ref(),
        );
        self.memory_set.clone_area(
            VirtAddr::from(self.trap_cx_bottom).floor(),
            another.memory_set.get_ref(),
        );
    }
    pub fn dealloc_user_res(&mut self) {
        if self.user_stack_top != 0 {
            self.memory_set.remove_area_with_start_vpn(
                VirtAddr::from(self.user_stack_top - USER_STACK_SIZE).floor(),
            );
        }
        self.memory_set
            .remove_area_with_start_vpn(VirtAddr::from(self.trap_cx_bottom).floor());
        flush_tlb();
    }

    pub fn recycle(&mut self) {
        self.memory_set.recycle_data_pages();
        self.fd_table.clear();
        self.fs_info.clear();
    }

    pub fn get_abs_path(&self, dirfd: isize, path: &str) -> Result<String, SysErrNo> {
        if is_abs_path(path) {
            Ok(get_abs_path("/", path))
        } else if dirfd != -100 {
            // AT_FDCWD=-100
            let dirfd = dirfd as usize;
            if let Some(file) = self.fd_table.try_get(dirfd) {
                let base_path = file.file()?.inode.path();
                if path.is_empty() {
                    Ok(base_path)
                } else {
                    Ok(get_abs_path(&base_path, path))
                }
            } else {
                Err(SysErrNo::EINVAL)
            }
        } else {
            Ok(get_abs_path(self.fs_info.cwd(), path))
        }
    }
}

impl TaskControlBlock {
    pub fn inner_lock(&self) -> MutexGuard<TaskControlBlockInner> {
        self.inner.lock()
    }
    pub fn tid(&self) -> usize {
        self.tid.0
    }
    pub fn pid(&self) -> usize {
        self.pid
    }
    pub fn ppid(&self) -> usize {
        self.ppid
    }
    /// 只有initproc会调用
    pub fn new(elf_data: &[u8]) -> Self {
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (mut memory_set, user_heapbottom, entry_point, _) = MemorySetInner::from_elf(elf_data);
        // alloc a pid and a kernel stack in kernel space
        let tid_handle = tid_alloc();
        let kernel_stack = KernelStack::new(&tid_handle);
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack.pos();
        memory_set.insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
            MapAreaType::Stack,
        );
        let task = Self {
            tid: tid_handle,
            pid: 1,
            ppid: 1,
            kernel_stack,
            inner: Mutex::new(TaskControlBlockInner {
                trap_cx_ppn: 0.into(),
                trap_cx_bottom: 0,
                user_stack_top: 0,
                task_cx: TaskContext::goto_trap_loop(kernel_stack_top),
                task_status: TaskStatus::Ready,
                memory_set: Arc::new(MemorySet::new(memory_set)),
                fd_table: Arc::new(FdTable::new_with_stdio()),
                fs_info: Arc::new(FsInfo::new(String::from("/"))),
                time_data: TimeData::new(),
                user_heappoint: user_heapbottom,
                user_heapbottom,
                set_child_tid: 0,
                clear_child_tid: 0,
                sig_table: Arc::new(SigTable::new()),
                sig_mask: SigSet::empty(),
                sig_pending: SigSet::empty(),
                timer: Arc::new(Timer::new()),
                robust_list: RobustList::default(),
                user_id: 0,
            }),
        };

        let mut task_inner = task.inner_lock();
        task_inner.alloc_user_res();
        // prepare TrapContext in user space
        let trap_cx = task_inner.trap_cx();
        *trap_cx =
            TrapContext::app_init_context(entry_point, task_inner.user_stack_top, kernel_stack_top);
        drop(task_inner);
        task
    }
    pub fn exec(&self, elf_data: &[u8], argv: &Vec<String>, env: &mut Vec<String>) {
        let mut task_inner = self.inner_lock();
        //用户栈高地址到低地址：环境变量字符串/参数字符串/aux辅助向量/环境变量地址数组/参数地址数组/参数数量
        // memory_set with elf program headers/trampoline/trap context/user stack
        let (memory_set, user_hp, entry_point, mut auxv) = MemorySetInner::from_elf(elf_data);

        // 替换 memory_set
        task_inner.memory_set = Arc::new(MemorySet::new(memory_set));
        task_inner.memory_set.activate();

        if task_inner.clear_child_tid != 0 {
            data_flow!({ *(task_inner.clear_child_tid as *mut u32) = 0 });
        }

        // 重新分配用户资源
        task_inner.alloc_user_res();
        task_inner.sig_table = Arc::new(SigTable::new());
        let fd_table = Arc::new(FdTable::from_another(&task_inner.fd_table));
        task_inner.fd_table = fd_table;
        task_inner.fd_table.close_on_exec();
        task_inner.sig_mask = SigSet::empty();
        task_inner.sig_pending = SigSet::empty();
        task_inner.time_data.clear();

        let mut user_sp = task_inner.user_stack_top;

        //环境变量内容入栈
        let mut envp = Vec::new();
        for env in env.iter() {
            user_sp -= env.len() + 1;
            envp.push(user_sp);
            for (j, c) in env.as_bytes().iter().enumerate() {
                unsafe {
                    *((user_sp + j) as *mut u8) = *c;
                }
            }
            unsafe {
                *((user_sp + env.len()) as *mut u8) = 0;
            }
        }
        envp.push(0);
        user_sp -= user_sp % size_of::<usize>();

        //存放字符串首址的数组
        let mut argvp = Vec::new();
        for arg in argv.iter() {
            // 计算字符串在栈上的地址
            user_sp -= arg.len() + 1;
            argvp.push(user_sp);
            for (j, c) in arg.as_bytes().iter().enumerate() {
                unsafe {
                    *((user_sp + j) as *mut u8) = *c;
                }
            }
            // 添加字符串末尾的 null 字符
            unsafe {
                *((user_sp + arg.len()) as *mut u8) = 0;
            }
        }
        user_sp -= user_sp % size_of::<usize>(); //以8字节对齐
        argvp.push(0);

        //需要随便放16个字节，不知道干嘛用的。
        user_sp -= 16;
        auxv.push(Aux::new(AuxType::RANDOM, user_sp));
        for i in 0..0xf {
            unsafe {
                *((user_sp + i) as *mut u8) = i as u8;
            }
        }
        user_sp -= user_sp % 16;

        //将auxv放入栈中
        auxv.push(Aux::new(AuxType::EXECFN, argvp[0]));
        auxv.push(Aux::new(AuxType::NULL, 0));
        for aux in auxv.iter().rev() {
            user_sp -= size_of::<Aux>();
            unsafe {
                *(user_sp as *mut usize) = aux.aux_type as usize;
                *((user_sp + size_of::<usize>()) as *mut usize) = aux.value;
            }
        }

        //将环境变量指针数组放入栈中
        user_sp -= envp.len() * size_of::<usize>();
        let envp_base = user_sp;
        for i in 0..envp.len() {
            unsafe {
                *((user_sp + i * size_of::<usize>()) as *mut usize) = envp[i];
            }
        }

        // println!("arg pointers:");
        user_sp -= argvp.len() * size_of::<usize>();
        let argv_base = user_sp;
        //将参数指针数组放入栈中
        for i in 0..argvp.len() {
            unsafe {
                *((user_sp + i * size_of::<usize>()) as *mut usize) = argvp[i];
            }
        }

        //将argc放入栈中
        user_sp -= size_of::<usize>();
        unsafe {
            *(user_sp as *mut usize) = argv.len();
        }

        //以8字节对齐
        user_sp -= user_sp % size_of::<usize>();

        //将设置了O_CLOEXEC位的文件描述符关闭
        task_inner.fd_table.close_on_exec();

        let mut trap_cx =
            TrapContext::app_init_context(entry_point, user_sp, self.kernel_stack.top());
        trap_cx.gp.x[10] = argv.len();
        trap_cx.gp.x[11] = argv_base;
        trap_cx.gp.x[12] = envp_base;
        *task_inner.trap_cx() = trap_cx;
        task_inner.user_heappoint = user_hp;
        task_inner.user_heapbottom = user_hp;
    }
    ///
    pub fn clone_process(
        self: &Arc<TaskControlBlock>,
        flags: CloneFlags,
        stack: usize,
        parent_tid: *mut u32,
        tls: usize,
        child_tid: *mut u32,
    ) -> Result<Arc<TaskControlBlock>, SysErrNo> {
        let parent_inner = self.inner.lock();

        let tid_handle = tid_alloc();
        let kernel_stack = KernelStack::new(&tid_handle);
        let kernel_stack_top = kernel_stack.top();

        // 检查是否共享虚拟内存
        let memory_set = if flags.contains(CloneFlags::CLONE_VM) {
            Arc::clone(&parent_inner.memory_set)
        } else {
            Arc::new(MemorySet::new(MemorySetInner::from_existed_user(
                &parent_inner.memory_set,
            )))
        };
        // 检查是否共享文件系统信息
        //filesystem information.  This includes the root
        //of the filesystem, the current working directory, and the umask
        let fs_info = if flags.contains(CloneFlags::CLONE_FS) {
            Arc::clone(&parent_inner.fs_info)
        } else {
            Arc::new(FsInfo::from_another(&parent_inner.fs_info))
        };
        // 检查是否共享打开文件表
        let fd_table = if flags.contains(CloneFlags::CLONE_FILES) {
            Arc::clone(&parent_inner.fd_table)
        } else {
            Arc::new(FdTable::from_another(&parent_inner.fd_table))
        };
        // 检查是否共享信号处理程序表
        let sig_table = if flags.contains(CloneFlags::CLONE_SIGHAND) {
            Arc::clone(&parent_inner.sig_table)
        } else {
            Arc::new(SigTable::from_another(&parent_inner.sig_table))
        };
        // 检查是否需要设置 parent_tid
        if flags.contains(CloneFlags::CLONE_PARENT_SETTID) {
            put_data(parent_inner.user_token(), parent_tid, tid_handle.0 as u32);
        }
        // 检查是否需要设置子进程的 set_child_tid,clear_child_tid
        let set_child_tid = if flags.contains(CloneFlags::CLONE_CHILD_SETTID) {
            child_tid as usize
        } else {
            0
        };
        let clear_child_tid = if flags.contains(CloneFlags::CLONE_CHILD_CLEARTID) {
            child_tid as usize
        } else {
            0
        };
        let (pid, mut ppid, timer, sig_pending, sig_mask);
        sig_pending = SigSet::empty();
        // 检查是否创建线程
        if flags.contains(CloneFlags::CLONE_THREAD) {
            pid = self.pid;
            ppid = self.ppid;
            timer = Arc::clone(&parent_inner.timer);
            sig_mask = SigSet::empty();
        } else {
            pid = tid_handle.0;
            ppid = self.pid;
            timer = Arc::new(Timer::new());
            sig_mask = parent_inner.sig_mask.clone();
        }
        if flags.contains(CloneFlags::CLONE_PARENT) {
            ppid = self.ppid;
        }
        let child = Arc::new(TaskControlBlock {
            tid: tid_handle,
            ppid,
            pid,
            kernel_stack,
            inner: Mutex::new(TaskControlBlockInner {
                trap_cx_ppn: 0.into(),
                trap_cx_bottom: 0,
                user_stack_top: 0,
                task_cx: TaskContext::goto_trap_loop(kernel_stack_top),
                task_status: TaskStatus::Ready,
                memory_set,
                fd_table,
                fs_info,
                time_data: TimeData::new(),
                user_heappoint: parent_inner.user_heappoint,
                user_heapbottom: parent_inner.user_heapbottom,
                set_child_tid,
                clear_child_tid,
                sig_table,
                sig_mask,
                sig_pending,
                timer,
                robust_list: RobustList::default(),
                user_id: parent_inner.user_id,
            }),
        });

        let mut child_inner = child.inner_lock();
        // debug!(
        //     "to {:?}, parent is {:?}, child is {:?}",
        //     SigSet::from_sig(3),
        //     parent_inner.sig_pending.get_mut().actions[3].act,
        //     child_inner.sig_pending.get_mut().actions[3].act
        // );
        if flags.contains(CloneFlags::CLONE_THREAD) {
            // 线程
            child_inner.alloc_user_res();
            *child_inner.trap_cx() = *parent_inner.trap_cx();
        } else {
            // fork
            child_inner.clone_user_res(&parent_inner);
            // for child process, fork returns 0
            child_inner.trap_cx().gp.x[10] = 0;
        }
        let trap_cx = child_inner.trap_cx();
        trap_cx.kernel_sp = kernel_stack_top;

        if stack != 0 {
            // 移除分配的stack
            let ustack = child_inner.user_stack_top;
            child_inner
                .memory_set
                .remove_area_with_start_vpn(VirtAddr::from(ustack - USER_STACK_SIZE).floor());
            child_inner.user_stack_top = 0;
            // 设置运行的起始地址和参数以及stack
            let (entry_point, arg) =
                data_flow!({ (*(stack as *const usize), *((stack + 8) as *const usize)) });
            // sepc/entry
            // debug!("[new thread] entry_point:{:#x}", entry_point);
            trap_cx.sepc = entry_point;
            //a0
            trap_cx.gp.x[10] = arg;
            //sp
            trap_cx.set_sp(stack);
        }
        if flags.contains(CloneFlags::CLONE_SETTLS) {
            // tp
            trap_cx.gp.x[4] = tls;
        }
        // CLONE_CHILD_SETTID
        if flags.contains(CloneFlags::CLONE_CHILD_SETTID) {
            let child_token = child_inner.user_token();
            put_data(child_token, child_tid, child.tid() as u32);
        }

        #[cfg(feature = "ltp")]
        if !flags.contains(CloneFlags::CLONE_THREAD) {
            crate::fs::create_proc_dir_and_file(pid, ppid);
        }

        drop(child_inner);
        drop(parent_inner);
        insert_into_tid2task(child.tid(), &child);
        insert_into_thread_group(child.pid, &child);
        if !flags.contains(CloneFlags::CLONE_THREAD) {
            insert_into_process_group(child.ppid, &child);
        }
        Ok(child)
    }

    ///修改数据段大小，懒分配
    pub fn growproc(&self, grow_size: isize) -> usize {
        let mut inner = self.inner_lock();
        if grow_size == 0 {
            return inner.user_heappoint;
        }
        //因为Brk一定连续，所以就只需要有一个Brk段
        let area = inner
            .memory_set
            .get_mut()
            .areas
            .iter_mut()
            .find(|area| area.area_type == MapAreaType::Brk)
            .unwrap();
        let growed_addr: usize = inner.user_heappoint + grow_size as usize;
        let shrinked_addr: usize = (inner.user_heappoint as isize + grow_size) as usize;
        let user_vpn_top: VirtPageNum =
            ((inner.user_heapbottom + USER_HEAP_SIZE) / PAGE_SIZE).into();
        let growed_vpn: VirtPageNum = (growed_addr / PAGE_SIZE + 1).into();
        let shrinked_vpn: VirtPageNum = (shrinked_addr / PAGE_SIZE + 1).into();
        if grow_size > 0 {
            if growed_vpn >= user_vpn_top {
                panic!("USER_HEAP overflow as {:#X}!", growed_addr);
            }
            //因为是懒分配，只要改范围就行了
            area.vpn_range = VPNRange::new((inner.user_heapbottom / PAGE_SIZE).into(), growed_vpn);
            inner.user_heappoint = growed_addr;
        } else {
            if shrinked_addr < inner.user_heapbottom {
                panic!("USER_HEAP downflow at {:#X}!", shrinked_addr);
            }
            area.vpn_range =
                VPNRange::new((inner.user_heapbottom / PAGE_SIZE).into(), shrinked_vpn);
            while !area.data_frames.is_empty() {
                let page = area.data_frames.pop_last().unwrap();
                if page.0 < growed_vpn {
                    area.data_frames.insert(page.0, page.1);
                    break;
                }
                inner.memory_set.get_mut().page_table.unmap(page.0);
            }
            inner.user_heappoint = shrinked_addr;
        }
        flush_tlb();
        inner.user_heappoint
    }

    pub fn check_timer(&self) {
        let mut task_inner = self.inner_lock();
        let timer = task_inner.timer.clone();
        let now = TimeVal::now();
        if timer.trigger_once() {
            // 只触发一次,单次计时器
            if now > timer.last_time() + timer.timer().it_value {
                // log::info!("Timer Alarm Once");
                task_inner.sig_pending |= timer.sig();
                timer.set_trigger_once(false);
                timer.set_last_time(now);
            }
        } else if !timer.timer().it_interval.is_empty() {
            //间隔触发
            if now > timer.last_time() + timer.timer().it_interval {
                // log::info!("Timer Alarm!");
                task_inner.sig_pending |= timer.sig();
                timer.set_last_time(now);
            }
        }
    }
}

#[derive(Copy, Clone, PartialEq)]
pub enum TaskStatus {
    Ready,
    Running,
    Zombie,
    Blocked,
    Stopped,
}
