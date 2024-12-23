use alloc::{string::String, sync::Arc, vec::Vec};
use spin::{Lazy, Mutex};

const MNT_MAXLEN: usize = 16;

pub struct MountTable {
    mnt_list: Vec<(String, String, String, u32)>, // special, dir, fstype
}

impl MountTable {
    pub fn mount(
        &mut self,
        special: String,
        dir: String,
        fstype: String,
        flags: u32,
        data: String,
    ) -> isize {
        if self.mnt_list.len() == MNT_MAXLEN {
            return -1;
        }
        // 已存在
        if let Some((mountspecial, _, mountfstype, mountflags)) =
            self.mnt_list.iter_mut().find(|(_, d, _, _)| *d == dir)
        {
            if flags & 32 != 0 {
                //包含MS_REMOUNT标志
                *mountspecial = special;
                *mountfstype = fstype;
                *mountflags = flags;
            }
            return 0;
        }

        // todo
        _ = data;

        //log::info!("push mount dir {} with flags={}", dir, flags);

        self.mnt_list.push((special, dir, fstype, flags));
        0
    }

    pub fn got_mount(&mut self, dir: String) -> Option<(String, String, String, u32)> {
        if let Some(mount) = self.mnt_list.iter().find(|&(_, d, _, _)| *d == dir) {
            return Some((*mount).clone());
        }
        return None;
    }

    pub fn umount(&mut self, special: String, flags: u32) -> isize {
        let len = self.mnt_list.len();

        // todo
        _ = flags;

        for i in 0..len {
            // 根据系统调用规范应该是 self.mnt_list[i].0 == special
            // 然而测试程序传的是 dir，因此这里加了一个或运算
            if self.mnt_list[i].0 == special || self.mnt_list[i].1 == special {
                self.mnt_list.remove(i);
                return 0;
            }
        }
        -1
    }
}

pub static MNT_TABLE: Lazy<Arc<Mutex<MountTable>>> = Lazy::new(|| {
    let mnt_table = MountTable {
        mnt_list: Vec::new(),
    };
    Arc::new(Mutex::new(mnt_table))
});
