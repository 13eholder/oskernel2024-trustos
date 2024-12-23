//! Implementation of [`PageTableEntry`] and [`PageTable`].
use crate::{config::mm::KERNEL_PGNUM_OFFSET, mm::KernelAddr, timer::get_time};

use super::{
    frame_alloc, FrameTracker, MemorySet, PhysAddr, PhysPageNum, StepByOne, VirtAddr, VirtPageNum,
    KERNEL_SPACE,
};
use alloc::{sync::Arc, vec, vec::Vec};
use bitflags::*;
use riscv::register::scause::{Exception, Trap};

use core::arch::asm;
pub fn flush_tlb() {
    unsafe {
        asm!("sfence.vma");
    }
}
bitflags! {
    pub struct PTEFlags: u8 {
        const V = 1 << 0;
        const R = 1 << 1;
        const W = 1 << 2;
        const X = 1 << 3;
        const U = 1 << 4;
        const G = 1 << 5;
        const A = 1 << 6;
        const D = 1 << 7;
    }
}

#[derive(Copy, Clone)]
#[repr(C)]
/// page table entry structure
pub struct PageTableEntry {
    ///PTE
    pub bits: usize,
}
/// visionFive2没有设置A标志位和D标志位的硬件,需要手动设置
#[cfg(feature = "board_vf2")]
pub static PTE_FLAGS_MASK: PTEFlags =
    PTEFlags::from_bits_truncate(PTEFlags::V.bits | PTEFlags::A.bits | PTEFlags::D.bits);

#[cfg(feature = "board_ramdisk")]
pub static PTE_FLAGS_MASK: PTEFlags =
    PTEFlags::from_bits_truncate(PTEFlags::V.bits | PTEFlags::A.bits | PTEFlags::D.bits);

#[cfg(feature = "board_qemu")]
pub static PTE_FLAGS_MASK: PTEFlags = PTEFlags::V;

impl PageTableEntry {
    ///Create a PTE from ppn
    pub fn new(ppn: PhysPageNum, flags: PTEFlags) -> Self {
        PageTableEntry {
            bits: ppn.0 << 10 | flags.bits as usize,
        }
    }
    ///Return an empty PTE
    pub fn empty() -> Self {
        PageTableEntry { bits: 0 }
    }
    ///Return 44bit ppn
    pub fn ppn(&self) -> PhysPageNum {
        (self.bits >> 10 & ((1usize << 44) - 1)).into()
    }
    ///Return 10bit flag
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.bits as u8).unwrap()
    }
    ///Check PTE valid
    pub fn is_valid(&self) -> bool {
        (self.flags() & PTEFlags::V) != PTEFlags::empty()
    }
    ///Check PTE readable
    pub fn readable(&self) -> bool {
        (self.flags() & PTEFlags::R) != PTEFlags::empty()
    }
    ///Check PTE writable
    pub fn writable(&self) -> bool {
        (self.flags() & PTEFlags::W) != PTEFlags::empty()
    }
    ///Check PTE executable
    pub fn executable(&self) -> bool {
        (self.flags() & PTEFlags::X) != PTEFlags::empty()
    }
    ///PTE cow
    /// 因为flags只给了8位，改成16位的话又要改一堆接口，于是就不用flags了
    pub fn is_cow(&self) -> bool {
        self.bits & (1 << 9) != 0
    }
    pub fn set_cow(&mut self) {
        (*self).bits = self.bits | (1 << 9);
    }
    pub fn reset_cow(&mut self) {
        (*self).bits = self.bits & !(1 << 9);
    }
    pub fn reset_w(&mut self) {
        (*self).bits = self.bits & !(1 << 2);
    }
    pub fn set_w(&mut self) {
        (*self).bits = self.bits | (1 << 2);
    }
    pub fn set_bits(&mut self, ppn: PhysPageNum, flags: PTEFlags) {
        self.bits = ppn.0 << 10 | flags.bits as usize;
    }
    // only X+W+R can be set
    // pub fn set_pte_flags(&mut self, flags: usize) {
    //     self.bits = (self.bits & !(0b1110 as usize)) | (flags & (0b1110 as usize));
    // }
    pub fn set_flags(&mut self, flags: PTEFlags) {
        let new_flags: u8 = flags.bits().clone();
        self.bits = (self.bits & 0xFFFF_FFFF_FFFF_FF00) | (new_flags as usize);
    }
    pub fn set_map_flags(&mut self, flags: PTEFlags) {
        let new_flags: u8 = (self.bits & 0xFF) as u8 | flags.bits().clone();
        self.bits = (self.bits & 0xFFFF_FFFF_FFFF_FF00) | (new_flags as usize);
    }
}
///Record root ppn and has the same lifetime as 1 and 2 level `PageTableEntry`
pub struct PageTable {
    root_ppn: PhysPageNum,
    frames: Vec<Arc<FrameTracker>>,
}

/// Assume that it won't oom when creating/mapping.
impl PageTable {
    /// Create an empty `PageTable`
    pub fn new() -> Self {
        let frame = frame_alloc().unwrap();
        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }
    pub fn new_from_kernel() -> Self {
        let frame = frame_alloc().unwrap();
        let locked_kernel = KERNEL_SPACE.lock();
        let kernel_root_ppn = locked_kernel.page_table.root_ppn;
        // 第一级页表
        let index = VirtPageNum::from(KERNEL_PGNUM_OFFSET).indexes()[0];
        frame.ppn.pte_array()[index..].copy_from_slice(&kernel_root_ppn.pte_array()[index..]);
        PageTable {
            root_ppn: frame.ppn,
            frames: vec![frame],
        }
    }

    /// Temporarily used to get arguments from user space.
    pub fn from_token(satp: usize) -> Self {
        Self {
            root_ppn: PhysPageNum::from(satp & ((1usize << 44) - 1)),
            frames: Vec::new(),
        }
    }
    /// Find phsical address by virtual address, create a frame if not exist
    fn find_pte_create(&mut self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                let frame = frame_alloc().unwrap();
                *pte = PageTableEntry::new(frame.ppn, PTE_FLAGS_MASK);
                self.frames.push(frame);
            }
            ppn = pte.ppn();
        }
        result
    }
    /// Find phsical address by virtual address
    fn find_pte(&self, vpn: VirtPageNum) -> Option<&mut PageTableEntry> {
        let idxs = vpn.indexes();
        let mut ppn = self.root_ppn;
        let mut result: Option<&mut PageTableEntry> = None;
        for (i, idx) in idxs.iter().enumerate() {
            let pte = &mut ppn.pte_array()[*idx];
            if i == 2 {
                result = Some(pte);
                break;
            }
            if !pte.is_valid() {
                return None;
            }
            ppn = pte.ppn();
        }
        result
    }
    /// Create a mapping form `vpn` to `ppn`
    pub fn map(&mut self, vpn: VirtPageNum, ppn: PhysPageNum, flags: PTEFlags) {
        let pte = self.find_pte_create(vpn).unwrap();
        assert!(
            !pte.is_valid(),
            "vpn {:?}, va {:x} is mapped before mapping",
            vpn,
            vpn.0 << 12
        );
        *pte = PageTableEntry::new(ppn, flags | PTE_FLAGS_MASK);
    }
    /// Delete a mapping form `vpn`
    pub fn unmap(&mut self, vpn: VirtPageNum) {
        // 如果不存在,即lazy allocation,跳过即可
        if let Some(pte) = self.find_pte(vpn) {
            if pte.is_valid() {
                *pte = PageTableEntry::empty();
            }
        }
    }
    /// Translate `VirtPageNum` to `PageTableEntry`
    pub fn translate(&self, vpn: VirtPageNum) -> Option<PageTableEntry> {
        self.find_pte(vpn).map(|pte| *pte)
    }
    /// Translate `VirtAddr` to `PhysAddr`
    pub fn translate_va(&self, va: VirtAddr) -> Option<PhysAddr> {
        self.find_pte(va.floor()).map(|pte| {
            let aligned_pa: PhysAddr = pte.ppn().into();
            let offset = va.page_offset();
            let aligned_pa_usize: usize = aligned_pa.into();
            (aligned_pa_usize + offset).into()
        })
    }
    /// Get root ppn
    pub fn token(&self) -> usize {
        8usize << 60 | self.root_ppn.0
    }
    pub fn set_cow(&mut self, vpn: VirtPageNum) {
        self.find_pte_create(vpn).unwrap().set_cow();
    }
    pub fn reset_cow(&mut self, vpn: VirtPageNum) {
        self.find_pte_create(vpn).unwrap().reset_cow();
    }
    pub fn set_w(&mut self, vpn: VirtPageNum) {
        self.find_pte_create(vpn).unwrap().set_w();
    }
    pub fn reset_w(&mut self, vpn: VirtPageNum) {
        self.find_pte_create(vpn).unwrap().reset_w();
    }
    pub fn set_flags(&mut self, vpn: VirtPageNum, flags: PTEFlags) {
        self.find_pte_create(vpn)
            .unwrap()
            .set_flags(flags | PTE_FLAGS_MASK);
    }
    pub fn set_map_flags(&mut self, vpn: VirtPageNum, flags: PTEFlags) {
        self.find_pte_create(vpn)
            .unwrap()
            .set_map_flags(flags | PTE_FLAGS_MASK);
    }
    pub fn clear(&mut self) {
        self.frames.clear();
    }
}
/// Translate a pointer to a mutable u8 Vec through page table
pub fn translated_byte_buffer(
    token: usize,
    ptr: *const u8,
    len: usize,
) -> Option<Vec<&'static mut [u8]>> {
    let page_table = PageTable::from_token(token);
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        match page_table.translate(vpn) {
            None => {
                panic!("[kernel] mm: 0x{:x} not mapped", start);
                log::debug!("vpn {:#x} not found", vpn.0);
                return None;
            }
            Some(ref pte) => {
                assert!(pte.is_valid(), "vpn {} to ppn {}", vpn.0, pte.ppn().0);
                if !pte.is_valid() {
                    log::debug!("vpn {:#x} invalid", vpn.0);
                    return None;
                }
            }
        }
        let ppn = page_table.translate(vpn).unwrap().ppn();
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.bytes_array_mut()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.bytes_array_mut()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    Some(v)
}
/// Safely Translate a pointer to a mutable u8 Vec through page table
pub fn safe_translated_byte_buffer(
    memory_set: Arc<MemorySet>,
    ptr: *const u8,
    len: usize,
) -> Option<Vec<&'static mut [u8]>> {
    let page_table = PageTable::from_token(memory_set.token());
    let mut start = ptr as usize;
    let end = start + len;
    let mut v = Vec::new();
    while start < end {
        let start_va = VirtAddr::from(start);
        let mut vpn = start_va.floor();
        match page_table.translate(vpn) {
            None => {
                memory_set.lazy_page_fault(vpn, Trap::Exception(Exception::LoadPageFault));
            }
            Some(ref pte) => {
                if !pte.is_valid() {
                    memory_set.lazy_page_fault(vpn, Trap::Exception(Exception::LoadPageFault));
                }
            }
        }
        let ppn = match page_table.translate(vpn) {
            None => {
                return None;
            }
            Some(ref pte) => {
                if !pte.is_valid() {
                    return None;
                }
                pte.ppn()
            }
        };
        vpn.step();
        let mut end_va: VirtAddr = vpn.into();
        end_va = end_va.min(VirtAddr::from(end));
        if end_va.page_offset() == 0 {
            v.push(&mut ppn.bytes_array_mut()[start_va.page_offset()..]);
        } else {
            v.push(&mut ppn.bytes_array_mut()[start_va.page_offset()..end_va.page_offset()]);
        }
        start = end_va.into();
    }
    Some(v)
}

///Translate a generic through page table and return a mutable reference
fn translated_refmut<T>(token: usize, ptr: *mut T) -> &'static mut T {
    let page_table = PageTable::from_token(token);
    let va = ptr as usize;
    KernelAddr::from(page_table.translate_va(VirtAddr::from(va)).unwrap()).as_mut()
}

/// 将数据 `data` 写入 `token` 地址空间 `ptr` 处，
/// 其中虚拟地址 `ptr` 解析得到的物理地址可以跨页
pub fn put_data<T: 'static>(token: usize, ptr: *mut T, data: T) {
    let page_table = PageTable::from_token(token);
    let mut va = VirtAddr::from(ptr as usize);
    let pa = page_table.translate_va(va).unwrap();
    let size = core::mem::size_of::<T>();
    // 若数据跨页，则转换成字节数据写入
    if (pa + size - 1).floor() != pa.floor() {
        let bytes =
            unsafe { core::slice::from_raw_parts(&data as *const _ as usize as *const u8, size) };
        for i in 0..size {
            *(page_table.translate_va(va).unwrap().as_mut()) = bytes[i];
            va = va + 1;
        }
    } else {
        *translated_refmut(token, ptr) = data;
    }
}

///Array of u8 slice that user communicate with os
pub struct UserBuffer {
    ///U8 vec
    pub buffers: Vec<&'static mut [u8]>,
}

impl UserBuffer {
    ///Create a `UserBuffer` by parameter
    pub fn new(buffers: Vec<&'static mut [u8]>) -> Self {
        Self { buffers }
    }
    pub fn new_single(buffer: &'static mut [u8]) -> Self {
        Self {
            buffers: vec![buffer],
        }
    }
    ///Length of `UserBuffer`
    pub fn len(&self) -> usize {
        let mut total: usize = 0;
        for b in self.buffers.iter() {
            total += b.len();
        }
        total
    }
    /// 将内容数组返回
    pub fn read(&mut self, len: usize) -> Vec<u8> {
        let mut bytes = vec![0; len];
        let mut current = 0;
        for sub_buff in self.buffers.iter_mut() {
            let mut sblen = (*sub_buff).len();
            if current + sblen > len {
                sblen = len - current;
            }
            bytes[current..current + sblen].copy_from_slice(&(*sub_buff)[..sblen]);
            current += sblen;
            if current == len {
                return bytes;
            }
        }
        bytes
    }
    /// 将一个Buffer的数据写入UserBuffer，返回写入长度
    pub fn write(&mut self, buff: &[u8]) -> usize {
        let len = self.len().min(buff.len());
        if len == 0 {
            return len;
        }
        let mut current = 0;
        for sub_buff in self.buffers.iter_mut() {
            let mut sblen = (*sub_buff).len();
            if buff.len() > 10 {
                if current + sblen > len {
                    sblen = len - current;
                }
                (*sub_buff)[..sblen].copy_from_slice(&buff[current..current + sblen]);
                current += sblen;
                if current == len {
                    return len;
                }
            } else {
                for j in 0..sblen {
                    (*sub_buff)[j] = buff[current];
                    current += 1;
                    if current == len {
                        return len;
                    }
                }
            }
        }
        return len;
    }
    //在指定位置写入数据
    pub fn write_at(&mut self, offset: usize, buff: &[u8]) -> isize {
        //未被使用，暂不做优化
        let len = buff.len();
        if offset + len > self.len() {
            return -1;
        }
        let mut head = 0; // offset of slice in UBuffer
        let mut current = 0; // current offset of buff

        for sub_buff in self.buffers.iter_mut() {
            let sblen = (*sub_buff).len();
            if head + sblen < offset {
                continue;
            } else if head < offset {
                for j in (offset - head)..sblen {
                    (*sub_buff)[j] = buff[current];
                    current += 1;
                    if current == len {
                        return len as isize;
                    }
                }
            } else {
                //head + sblen > offset and head > offset
                for j in 0..sblen {
                    (*sub_buff)[j] = buff[current];
                    current += 1;
                    if current == len {
                        return len as isize;
                    }
                }
            }
            head += sblen;
        }
        0
    }

    pub fn fill0(&mut self) -> usize {
        for sub_buff in self.buffers.iter_mut() {
            let sblen = (*sub_buff).len();
            for j in 0..sblen {
                (*sub_buff)[j] = 0;
            }
        }
        self.len()
    }

    pub fn fillrandom(&mut self) -> usize {
        //随机数生成方法： 线性计算+噪声+零特殊处理
        let mut random: u8 = (get_time() % 256) as u8;
        for sub_buff in self.buffers.iter_mut() {
            let sblen = (*sub_buff).len();
            for j in 0..sblen {
                if random == 0 {
                    random = (get_time() % 256) as u8;
                }
                random = (((random as usize) * (get_time() / 3 % 256) + 37) % 256) as u8; //生成一个字节大小的随机数
                (*sub_buff)[j] = random;
            }
        }
        self.len()
    }

    pub fn printbuf(&mut self, size: usize) {
        if size == 0 {
            return;
        }
        let mut count: usize = 0;
        for sub_buff in self.buffers.iter_mut() {
            let sblen = (*sub_buff).len();
            for j in 0..sblen {
                print!("{} ", (*sub_buff)[j]);
                count += 1;
                if count == size {
                    println!("");
                    return;
                }
            }
        }
    }

    pub fn clear(&mut self) -> usize {
        self.buffers.clear();
        self.len()
    }
}

impl IntoIterator for UserBuffer {
    type Item = *mut u8;
    type IntoIter = UserBufferIterator;
    fn into_iter(self) -> Self::IntoIter {
        UserBufferIterator {
            buffers: self.buffers,
            current_buffer: 0,
            current_idx: 0,
        }
    }
}
/// Iterator of `UserBuffer`
pub struct UserBufferIterator {
    buffers: Vec<&'static mut [u8]>,
    current_buffer: usize,
    current_idx: usize,
}

impl Iterator for UserBufferIterator {
    type Item = *mut u8;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current_buffer >= self.buffers.len() {
            None
        } else {
            let r = &mut self.buffers[self.current_buffer][self.current_idx] as *mut _;
            if self.current_idx + 1 == self.buffers[self.current_buffer].len() {
                self.current_idx = 0;
                self.current_buffer += 1;
            } else {
                self.current_idx += 1;
            }
            Some(r)
        }
    }
}
