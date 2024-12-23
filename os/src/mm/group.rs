use alloc::{collections::btree_map::BTreeMap, sync::Arc, vec::Vec};
use spin::{Lazy, Mutex};

use super::{FrameTracker, VirtPageNum};
pub const GROUP_SIZE: usize = 0x1000;

//共享空间管理器,mmap专用，因为只有mmap会在有固定内容但没加载时fork
pub static GROUP_SHARE: Lazy<Mutex<GroupManager>> = Lazy::new(|| Mutex::new(GroupManager::new()));
//以MapArea为单元分组，每个MapArea一个groupid,在同一个group内的MapArea共享内存
struct GroupInner {
    //该组内共享的帧
    pub shared_frames: BTreeMap<VirtPageNum, Arc<FrameTracker>>,
    //该组内剩余的maparea数量
    pub maparea_num: usize,
}
impl GroupInner {
    pub fn new() -> GroupInner {
        Self {
            shared_frames: BTreeMap::new(),
            maparea_num: 0,
        }
    }
}
pub struct GroupManager {
    unused_id: Vec<usize>,
    groups: BTreeMap<usize, GroupInner>,
}
impl GroupManager {
    pub fn new() -> GroupManager {
        Self {
            unused_id: (1..GROUP_SIZE).collect(),
            groups: BTreeMap::new(),
        }
    }
    //分配一个groupID
    pub fn alloc_id(&mut self) -> usize {
        if let Some(id) = self.unused_id.pop() {
            id
        } else {
            0
        }
    }
    //添加一个maparea
    pub fn add_area(&mut self, id: usize) {
        if id == 0 {
            return;
        }
        //println!("add area {}", id);
        if !self.groups.contains_key(&id) {
            self.groups.insert(id, GroupInner::new());
        }
        self.groups.get_mut(&id).unwrap().maparea_num += 1;
    }
    //释放一个maparea
    pub fn del_area(&mut self, id: usize) {
        if id == 0 {
            return;
        }
        //println!("del area {}", id);
        let num = self.groups.get_mut(&id).unwrap();
        num.maparea_num -= 1;
        if num.maparea_num == 0 {
            self.groups.remove(&id);
            self.unused_id.push(id);
        }
    }
    //添加共享帧
    //同一组只有第一次触发懒分配时调用，所有权直接转移给shared_frames
    pub fn add_frame(&mut self, id: usize, vpn: VirtPageNum, frame: Arc<FrameTracker>) {
        self.groups
            .get_mut(&id)
            .unwrap()
            .shared_frames
            .insert(vpn, frame);
    }
    //查找共享帧
    pub fn find(&mut self, id: usize, vpn: VirtPageNum) -> Option<Arc<FrameTracker>> {
        if id == 0 {
            return None;
        }
        //get(&id)必定成功
        if let Some(frame) = self.groups.get(&id).unwrap().shared_frames.get(&vpn) {
            Some(frame.clone())
        } else {
            None
        }
    }
}
