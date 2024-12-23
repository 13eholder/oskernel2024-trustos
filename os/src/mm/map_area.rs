use super::{
    frame_alloc, FrameTracker, PTEFlags, PageTable, PhysPageNum, StepByOne, VPNRange, VirtAddr,
    VirtPageNum, GROUP_SHARE,
};
use crate::{
    config::mm::{KERNEL_PGNUM_OFFSET, PAGE_SIZE},
    fs::OSInode,
    syscall::MmapFlags,
};
use alloc::{collections::BTreeMap, sync::Arc, vec::Vec};

/// map area structure, controls a contiguous piece of virtual memory
/// 逻辑段
pub struct MapArea {
    pub vpn_range: VPNRange,
    pub data_frames: BTreeMap<VirtPageNum, Arc<FrameTracker>>,
    pub map_type: MapType,
    pub map_perm: MapPermission,
    pub area_type: MapAreaType,
    pub mmap_file: MmapFile,
    pub mmap_flags: MmapFlags,
    pub groupid: usize,
}
impl Drop for MapArea {
    fn drop(&mut self) {
        GROUP_SHARE.lock().del_area(self.groupid);
    }
}
impl MapArea {
    pub fn new(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission,
        area_type: MapAreaType,
    ) -> Self {
        let start_vpn: VirtPageNum = start_va.floor();
        let end_vpn: VirtPageNum = end_va.ceil();
        Self {
            vpn_range: VPNRange::new(start_vpn, end_vpn),
            data_frames: BTreeMap::new(),
            map_type,
            map_perm,
            area_type,
            mmap_file: MmapFile::empty(),
            mmap_flags: MmapFlags::empty(),
            groupid: 0,
        }
    }
    pub fn new_mmap(
        start_va: VirtAddr,
        end_va: VirtAddr,
        map_type: MapType,
        map_perm: MapPermission,
        area_type: MapAreaType,
        file: Option<Arc<OSInode>>,
        offset: usize,
        mmap_flags: MmapFlags,
    ) -> Self {
        let start_vpn: VirtPageNum = start_va.floor();
        let end_vpn: VirtPageNum = end_va.ceil();
        let groupid;
        if mmap_flags.contains(MmapFlags::MAP_SHARED) {
            groupid = 0;
        } else {
            groupid = GROUP_SHARE.lock().alloc_id();
            GROUP_SHARE.lock().add_area(groupid);
        }
        Self {
            vpn_range: VPNRange::new(start_vpn, end_vpn),
            data_frames: BTreeMap::new(),
            map_type,
            map_perm,
            area_type,
            mmap_file: MmapFile::new(file, offset),
            mmap_flags,
            groupid,
        }
    }
    pub fn from_another(another: &MapArea) -> Self {
        Self {
            vpn_range: VPNRange::new(another.vpn_range.start(), another.vpn_range.end()),
            data_frames: BTreeMap::new(),
            map_type: another.map_type,
            map_perm: another.map_perm,
            area_type: another.area_type,
            mmap_file: another.mmap_file.clone(),
            mmap_flags: another.mmap_flags,
            groupid: another.groupid,
        }
    }
    pub fn map_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        let ppn: PhysPageNum;
        match self.map_type {
            MapType::Direct => {
                ppn = PhysPageNum(vpn.0 - KERNEL_PGNUM_OFFSET);
            }
            MapType::Framed => {
                let frame = frame_alloc().unwrap();
                ppn = frame.ppn;
                self.data_frames.insert(vpn, frame);
            }
        }
        let pte_flags = PTEFlags::from_bits(self.map_perm.bits).unwrap();
        page_table.map(vpn, ppn, pte_flags);
    }
    pub fn unmap_one(&mut self, page_table: &mut PageTable, vpn: VirtPageNum) {
        if self.map_type == MapType::Framed {
            self.data_frames.remove(&vpn);
        }
        page_table.unmap(vpn);
    }
    pub fn map(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.map_one(page_table, vpn);
        }
    }
    pub fn map_given_frames(&mut self, page_table: &mut PageTable, frames: Vec<Arc<FrameTracker>>) {
        for (vpn, frame) in self.vpn_range.clone().into_iter().zip(frames.into_iter()) {
            let pte_flags = PTEFlags::from_bits(self.map_perm.bits).unwrap();
            page_table.map(vpn, frame.ppn, pte_flags);
            self.data_frames.insert(vpn, frame);
        }
    }
    pub fn unmap(&mut self, page_table: &mut PageTable) {
        for vpn in self.vpn_range {
            self.unmap_one(page_table, vpn);
        }
    }
    /// data: start-aligned but maybe with shorter length
    /// assume that all frames were cleared before
    pub fn copy_data(&mut self, page_table: &mut PageTable, data: &[u8], offset: usize) {
        assert_eq!(self.map_type, MapType::Framed);
        let mut start: usize = 0;
        let mut page_offset: usize = offset;
        let mut current_vpn = self.vpn_range.start();
        let len = data.len();
        loop {
            let src = &data[start..len.min(start + PAGE_SIZE - page_offset)];
            let dst = &mut page_table
                .translate(current_vpn)
                .unwrap()
                .ppn()
                .bytes_array_mut()[page_offset..(page_offset + src.len())];
            dst.copy_from_slice(src);

            start += PAGE_SIZE - page_offset;

            page_offset = 0;
            if start >= len {
                break;
            }
            current_vpn.step();
        }
    }
    pub fn kernel_stack_frame(&self) -> Vec<Arc<FrameTracker>> {
        let mut v = Vec::new();
        for vpn in self.vpn_range {
            v.push(self.data_frames.get(&vpn).unwrap().clone())
        }
        v
    }
    pub fn flags(&self) -> PTEFlags {
        PTEFlags::from_bits(self.map_perm.bits).unwrap()
    }
}

#[derive(Copy, Clone, PartialEq, Debug)]
/// map type for memory set: identical or framed
pub enum MapType {
    Direct,
    Framed,
}

bitflags! {
    /// map permission corresponding to that in pte: `R W X U`
    pub struct MapPermission: u8 {
        ///Readable
        const R = 1 << 1;
        ///Writable
        const W = 1 << 2;
        ///Excutable
        const X = 1 << 3;
        ///Accessible in U mode
        const U = 1 << 4;
    }
}

/// Map area type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MapAreaType {
    /// Segments from elf file, e.g. text, rodata, data, bss
    Elf,
    /// Stack
    Stack,
    /// Brk
    Brk,
    /// Mmap
    Mmap,
    /// For Trap Context
    Trap,
    /// Shared memory
    Shm,
    /// Physical frames(for kernel)
    Physical,
    /// MMIO(for kernel)
    MMIO,
}

#[derive(Clone)]
pub struct MmapFile {
    pub file: Option<Arc<OSInode>>,
    pub offset: usize,
}

impl MmapFile {
    pub fn empty() -> Self {
        Self {
            file: None,
            offset: 0,
        }
    }

    pub fn new(file: Option<Arc<OSInode>>, offset: usize) -> Self {
        Self { file, offset }
    }
}
