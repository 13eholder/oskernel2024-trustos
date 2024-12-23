//!Implementation of [`PidAllocator`]
use crate::{
    config::mm::{KERNEL_STACK_SIZE, KSTACK_TOP, PAGE_SIZE},
    mm::{MapAreaType, MapPermission, VirtAddr, KERNEL_SPACE},
};
use alloc::vec::Vec;
use spin::{Lazy, Mutex};
///Pid Allocator struct
pub struct TidAllocator {
    current: usize,
    recycled: Vec<usize>,
}

impl TidAllocator {
    ///Create an empty `TidAllocator`
    pub fn new() -> Self {
        TidAllocator {
            current: 1,
            recycled: Vec::new(),
        }
    }
    ///Allocate a tid
    pub fn alloc(&mut self) -> TidHandle {
        if let Some(tid) = self.recycled.pop() {
            TidHandle(tid)
        } else {
            self.current += 1;
            TidHandle(self.current - 1)
        }
    }
    ///Recycle a tid
    pub fn dealloc(&mut self, tid: usize) {
        assert!(tid < self.current);
        assert!(
            !self.recycled.iter().any(|ttid| *ttid == tid),
            "pid {} has been deallocated!",
            tid
        );
        self.recycled.push(tid);
    }
}

pub static TID_ALLOCATOR: Lazy<Mutex<TidAllocator>> = Lazy::new(|| Mutex::new(TidAllocator::new()));

///Bind pid lifetime to `TidHandle`
pub struct TidHandle(pub usize);

impl Drop for TidHandle {
    fn drop(&mut self) {
        TID_ALLOCATOR.lock().dealloc(self.0);
    }
}
///Allocate a pid from PID_ALLOCATOR
pub fn tid_alloc() -> TidHandle {
    TID_ALLOCATOR.lock().alloc()
}

/// Return (bottom, top) of a kernel stack in kernel space.
pub fn kernel_stack_position(app_id: usize) -> (usize, usize) {
    let top = KSTACK_TOP - app_id * (KERNEL_STACK_SIZE + PAGE_SIZE);
    let bottom = top - KERNEL_STACK_SIZE;
    (bottom, top)
}
///Kernelstack for app
pub struct KernelStack {
    tid: usize,
}

impl KernelStack {
    ///Create a kernelstack from pid
    pub fn new(tid_handle: &TidHandle) -> Self {
        let (kernel_stack_bottom, kernel_stack_top) = kernel_stack_position(tid_handle.0);
        KERNEL_SPACE.lock().insert_framed_area(
            kernel_stack_bottom.into(),
            kernel_stack_top.into(),
            MapPermission::R | MapPermission::W,
            MapAreaType::Stack,
        );
        KernelStack { tid: tid_handle.0 }
    }
    ///Get the value on the top of kernelstack
    pub fn top(&self) -> usize {
        let (_, kernel_stack_top) = kernel_stack_position(self.tid);
        kernel_stack_top
    }
    /// Return (bottom, top) of a kernel stack in kernel space.)
    pub fn pos(&self) -> (usize, usize) {
        kernel_stack_position(self.tid)
    }
    pub fn bottom(&self) -> usize {
        let (kernel_stack_bottom, _) = kernel_stack_position(self.tid);
        kernel_stack_bottom
    }
}

impl Drop for KernelStack {
    fn drop(&mut self) {
        KERNEL_SPACE
            .lock()
            .remove_area_with_start_vpn(VirtAddr::from(self.bottom()).floor());
    }
}
