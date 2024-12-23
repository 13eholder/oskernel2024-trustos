//! Implementation of physical and virtual address and page number.
use super::PageTableEntry;
use crate::config::mm::{KERNEL_ADDR_OFFSET, PAGE_SIZE, PAGE_SIZE_BITS};
use core::{
    fmt::{self, Debug, Formatter},
    ops::{Add, Sub},
};

const PA_WIDTH_SV39: usize = 56;
const VA_WIDTH_SV39: usize = 39;
const PPN_WIDTH_SV39: usize = PA_WIDTH_SV39 - PAGE_SIZE_BITS;
const VPN_WIDTH_SV39: usize = VA_WIDTH_SV39 - PAGE_SIZE_BITS;

/// Definitions
#[repr(C)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
pub struct PhysAddr(pub usize);

#[repr(C)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
///virtual address
pub struct VirtAddr(pub usize);

/// Kernel Address
#[repr(C)]
#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct KernelAddr(pub usize);

#[repr(C)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
///phiscal page number
pub struct PhysPageNum(pub usize);

#[repr(C)]
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
///virtual page number
pub struct VirtPageNum(pub usize);

/// Debugging

impl Debug for VirtAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("VA:{:#x}", self.0))
    }
}
impl Debug for VirtPageNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("VPN:{:#x}", self.0))
    }
}
impl Debug for PhysAddr {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("PA:{:#x}", self.0))
    }
}
impl Debug for PhysPageNum {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.write_fmt(format_args!("PPN:{:#x}", self.0))
    }
}

/// T: {PhysAddr, VirtAddr, PhysPageNum, VirtPageNum}
/// T -> usize: T.0
/// usize -> T: usize.into()

impl From<usize> for PhysAddr {
    fn from(v: usize) -> Self {
        // Self(v & ((1 << PA_WIDTH_SV39) - 1))
        let tmp = (v as isize >> PA_WIDTH_SV39) as isize;
        assert!(tmp == 0 || tmp == -1);
        Self(v)
    }
}
impl From<usize> for PhysPageNum {
    fn from(v: usize) -> Self {
        // Self(v & ((1 << PPN_WIDTH_SV39) - 1))
        let tmp = (v as isize >> PPN_WIDTH_SV39) as isize;
        assert!(tmp == 0 || tmp == -1);
        Self(v)
    }
}
/// 拓展虚拟地址到512GB
impl From<usize> for VirtAddr {
    fn from(v: usize) -> Self {
        // Self(v & ((1 << VA_WIDTH_SV39) - 1))
        let tmp = (v as isize >> VA_WIDTH_SV39) as isize;
        assert!(tmp == 0 || tmp == -1, "invalid va: {:#x}", v);
        Self(v)
    }
}
impl From<usize> for VirtPageNum {
    fn from(v: usize) -> Self {
        // Self(v & ((1 << VPN_WIDTH_SV39) - 1))
        let tmp = v >> (VPN_WIDTH_SV39 - 1);
        assert!(tmp == 0 || tmp == (1 << (52 - VPN_WIDTH_SV39 + 1)) - 1);
        Self(v)
    }
}
impl From<PhysAddr> for usize {
    fn from(v: PhysAddr) -> Self {
        v.0
    }
}
impl From<PhysPageNum> for usize {
    fn from(v: PhysPageNum) -> Self {
        v.0
    }
}
impl From<VirtAddr> for usize {
    fn from(v: VirtAddr) -> Self {
        if v.0 >= (1 << (VA_WIDTH_SV39 - 1)) {
            v.0 | (!((1 << VA_WIDTH_SV39) - 1))
        } else {
            v.0
        }
    }
}
impl From<VirtPageNum> for usize {
    fn from(v: VirtPageNum) -> Self {
        v.0
    }
}

/// impl From for KernelAddr
impl From<usize> for KernelAddr {
    fn from(v: usize) -> Self {
        Self(v)
    }
}

impl From<PhysAddr> for KernelAddr {
    fn from(v: PhysAddr) -> Self {
        Self(v.0 + KERNEL_ADDR_OFFSET)
    }
}

impl From<KernelAddr> for PhysAddr {
    fn from(v: KernelAddr) -> Self {
        Self(v.0 - KERNEL_ADDR_OFFSET)
    }
}

impl From<KernelAddr> for VirtAddr {
    fn from(v: KernelAddr) -> Self {
        Self(v.0)
    }
}

impl From<KernelAddr> for PhysPageNum {
    fn from(v: KernelAddr) -> Self {
        PhysAddr::from(v).floor()
    }
}

impl Add for VirtAddr {
    type Output = VirtAddr;
    fn add(self, rhs: Self) -> Self::Output {
        VirtAddr(self.0 + rhs.0)
    }
}
impl Add<usize> for VirtAddr {
    type Output = VirtAddr;
    fn add(self, rhs: usize) -> Self::Output {
        VirtAddr(self.0 + rhs)
    }
}
impl Sub for VirtAddr {
    type Output = VirtAddr;
    fn sub(self, rhs: Self) -> Self::Output {
        VirtAddr(self.0 - rhs.0)
    }
}
impl Sub<usize> for VirtAddr {
    type Output = VirtAddr;
    fn sub(self, rhs: usize) -> Self::Output {
        VirtAddr(self.0 - rhs)
    }
}

impl Add for PhysAddr {
    type Output = PhysAddr;
    fn add(self, rhs: Self) -> Self::Output {
        PhysAddr(self.0 + rhs.0)
    }
}
impl Add<usize> for PhysAddr {
    type Output = PhysAddr;
    fn add(self, rhs: usize) -> Self::Output {
        PhysAddr(self.0 + rhs)
    }
}
impl Sub for PhysAddr {
    type Output = PhysAddr;
    fn sub(self, rhs: Self) -> Self::Output {
        PhysAddr(self.0 - rhs.0)
    }
}
impl Sub<usize> for PhysAddr {
    type Output = PhysAddr;
    fn sub(self, rhs: usize) -> Self::Output {
        PhysAddr(self.0 - rhs)
    }
}

/// impl KernelAddr
impl KernelAddr {
    pub fn as_ref<T>(&self) -> &'static T {
        unsafe { (self.0 as *const T).as_ref().unwrap() }
    }
    ///Get mutable reference to `PhysAddr` value
    pub fn as_mut<T>(&self) -> &'static mut T {
        unsafe { (self.0 as *mut T).as_mut().unwrap() }
    }
}

///
impl VirtAddr {
    ///`VirtAddr`->`VirtPageNum`
    pub fn floor(&self) -> VirtPageNum {
        VirtPageNum(self.0 / PAGE_SIZE)
    }
    ///`VirtAddr`->`VirtPageNum`
    pub fn ceil(&self) -> VirtPageNum {
        if self.0 == 0 {
            VirtPageNum(0)
        } else {
            VirtPageNum((self.0 - 1 + PAGE_SIZE) / PAGE_SIZE)
        }
    }
    ///Get page offset
    pub fn page_offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }
    ///Check page aligned
    pub fn aligned(&self) -> bool {
        self.page_offset() == 0
    }
}
impl From<VirtAddr> for VirtPageNum {
    fn from(v: VirtAddr) -> Self {
        assert_eq!(v.page_offset(), 0);
        v.floor()
    }
}
impl From<VirtPageNum> for VirtAddr {
    fn from(v: VirtPageNum) -> Self {
        Self(v.0 << PAGE_SIZE_BITS)
    }
}
impl PhysAddr {
    ///`PhysAddr`->`PhysPageNum`
    pub fn floor(&self) -> PhysPageNum {
        PhysPageNum(self.0 / PAGE_SIZE)
    }
    ///`PhysAddr`->`PhysPageNum`
    pub fn ceil(&self) -> PhysPageNum {
        if self.0 == 0 {
            PhysPageNum(0)
        } else {
            PhysPageNum((self.0 - 1 + PAGE_SIZE) / PAGE_SIZE)
        }
    }
    ///Get page offset
    pub fn page_offset(&self) -> usize {
        self.0 & (PAGE_SIZE - 1)
    }
    ///Check page aligned
    pub fn aligned(&self) -> bool {
        self.page_offset() == 0
    }
}
impl From<PhysAddr> for PhysPageNum {
    fn from(v: PhysAddr) -> Self {
        assert_eq!(v.page_offset(), 0);
        v.floor()
    }
}
impl From<PhysPageNum> for PhysAddr {
    fn from(v: PhysPageNum) -> Self {
        Self(v.0 << PAGE_SIZE_BITS)
    }
}

impl VirtPageNum {
    ///Return VPN 3 level index
    pub fn indexes(&self) -> [usize; 3] {
        let mut vpn = self.0;
        let mut idx = [0usize; 3];
        for i in (0..3).rev() {
            idx[i] = vpn & 511;
            vpn >>= 9;
        }
        idx
    }
    ///快速转换成usize，方便输出使用
    pub fn as_usize(&self) -> usize {
        self.0
    }
}

impl PhysAddr {
    ///Get reference to `PhysAddr` value
    pub fn as_ref<T>(&self) -> &'static T {
        // unsafe { (self.0 as *const T).as_ref().unwrap() }
        KernelAddr::from(*self).as_ref()
    }
    ///Get mutable reference to `PhysAddr` value
    pub fn as_mut<T>(&self) -> &'static mut T {
        // unsafe { (self.0 as *mut T).as_mut().unwrap() }
        KernelAddr::from(*self).as_mut()
    }
}
impl PhysPageNum {
    ///Get `PageTableEntry` on `PhysPageNum`
    pub fn pte_array(&self) -> &'static mut [PageTableEntry] {
        let pa: PhysAddr = (*self).into();
        let kernel_va = KernelAddr::from(pa).0;
        unsafe { core::slice::from_raw_parts_mut(kernel_va as *mut PageTableEntry, 512) }
    }
    ///Get u8 array on `PhysPageNum`
    pub fn bytes_array_mut(&self) -> &'static mut [u8] {
        let pa: PhysAddr = (*self).into();
        let kernel_va = KernelAddr::from(pa).0;
        unsafe { core::slice::from_raw_parts_mut(kernel_va as *mut u8, 4096) }
    }
    pub fn bytes_array(&self) -> &'static [u8] {
        let pa: PhysAddr = (*self).into();
        let kernel_va = KernelAddr::from(pa).0;
        unsafe { core::slice::from_raw_parts(kernel_va as *mut u8, 4096) }
    }
    ///Get u8 array on `PhysPageNum` with given length
    pub fn bytes_array_from_offset(&self, offset: usize, len: usize) -> &'static mut [u8] {
        let pa: PhysAddr = (*self).into();
        let kernel_va = KernelAddr::from(pa).0 + offset;
        unsafe { core::slice::from_raw_parts_mut(kernel_va as *mut u8, len) }
    }
    ///Get Get mutable reference to `PhysAddr` value on `PhysPageNum`
    pub fn as_mut<T>(&self) -> &'static mut T {
        let pa: PhysAddr = (*self).into();
        let kernel_va = KernelAddr::from(pa);
        kernel_va.as_mut()
    }
}
///Add value by one
pub trait StepByOne {
    ///Add value by one
    fn step(&mut self);
}
impl StepByOne for VirtPageNum {
    fn step(&mut self) {
        self.0 += 1;
    }
}
impl StepByOne for PhysPageNum {
    fn step(&mut self) {
        self.0 += 1;
    }
}

#[derive(Copy, Clone)]
/// a simple [l,r) range structure for type T
pub struct SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    l: T,
    r: T,
}
impl<T> SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(start: T, end: T) -> Self {
        assert!(start <= end, "start {:?} > end {:?}!", start, end);
        Self { l: start, r: end }
    }
    pub fn start(&self) -> T {
        self.l
    }
    pub fn end(&self) -> T {
        self.r
    }
    pub fn range(&self) -> (T, T) {
        (self.l, self.r)
    }
}
impl<T> IntoIterator for SimpleRange<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;
    type IntoIter = SimpleRangeIterator<T>;
    fn into_iter(self) -> Self::IntoIter {
        SimpleRangeIterator::new(self.l, self.r)
    }
}
/// iterator for the simple range structure
pub struct SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    current: T,
    end: T,
}
impl<T> SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    pub fn new(l: T, r: T) -> Self {
        Self { current: l, end: r }
    }
}
impl<T> Iterator for SimpleRangeIterator<T>
where
    T: StepByOne + Copy + PartialEq + PartialOrd + Debug,
{
    type Item = T;
    fn next(&mut self) -> Option<Self::Item> {
        if self.current == self.end {
            None
        } else {
            let t = self.current;
            self.current.step();
            Some(t)
        }
    }
}
/// a simple range structure for virtual page number
pub type VPNRange = SimpleRange<VirtPageNum>;

use hashbrown::HashSet;
use spin::{lazy::Lazy, Mutex};

//坏地址表，mmap映射坏地址时加入此表
static BAD_ADDRESS: Lazy<Mutex<HashSet<usize>>> = Lazy::new(|| Mutex::new(HashSet::new()));

pub fn insert_bad_address(va: usize) {
    BAD_ADDRESS.lock().insert(va);
}

pub fn is_bad_address(va: usize) -> bool {
    BAD_ADDRESS.lock().contains(&va)
}

pub fn remove_bad_address(va: usize) {
    BAD_ADDRESS.lock().remove(&va);
}
