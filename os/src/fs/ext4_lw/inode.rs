use log::{debug, warn};
use lwext4_rust::{
    bindings::{O_CREAT, O_RDONLY, O_RDWR, O_TRUNC, SEEK_SET},
    Ext4File, InodeTypes,
};

use crate::{
    fs::{Dirent, Inode, InodeType, Kstat, OpenFlags, String},
    sync::SyncUnsafeCell,
    utils::{SysErrNo, SyscallRet},
};

use alloc::{format, vec};
use alloc::{sync::Arc, vec::Vec};

const MAX_LOOPTIMES: usize = 5;

pub struct Ext4Inode {
    inner: SyncUnsafeCell<Ext4InodeInner>,
}

pub struct Ext4InodeInner {
    f: Ext4File,
    delay: bool,
    if_dir: bool,
}

unsafe impl Send for Ext4Inode {}
unsafe impl Sync for Ext4Inode {}

impl Ext4Inode {
    pub fn new(path: &str, types: InodeTypes) -> Self {
        let is_dir = if types == InodeTypes::EXT4_DE_DIR {
            true
        } else {
            false
        };
        Ext4Inode {
            inner: SyncUnsafeCell::new(Ext4InodeInner {
                f: Ext4File::new(path, types),
                delay: false,
                if_dir: is_dir,
            }),
        }
    }
}

impl Inode for Ext4Inode {
    fn size(&self) -> usize {
        let file = &mut self.inner.get_unchecked_mut().f;
        let types = as_inode_type(file.file_type());
        if types == InodeType::File {
            let path = file.path();
            let path = path.to_str().unwrap();
            file.file_open(path, O_RDONLY);
            let fsize = file.file_size();
            fsize as usize
        } else {
            0
        }
    }
    /// Ext4Inode创建必须使用绝对路径
    fn create(&self, path: &str, ty: InodeType) -> Result<Arc<dyn Inode>, SysErrNo> {
        let types = as_ext4_de_type(ty);
        let file = &mut self.inner.get_unchecked_mut().f;
        let nf = Ext4Inode::new(path, types.clone());

        if !file.check_inode_exist(path, types.clone()) {
            let nfile = &mut nf.inner.get_unchecked_mut().f;
            if types == InodeTypes::EXT4_DE_DIR {
                if let Err(e) = nfile.dir_mk(path) {
                    return Err(SysErrNo::from(e));
                }
            } else if let Err(e) = nfile.file_open(path, O_RDWR | O_CREAT | O_TRUNC) {
                return Err(SysErrNo::from(e));
            } else {
                nfile.file_close()?;
            }
        }
        Ok(Arc::new(nf))
    }

    fn types(&self) -> InodeType {
        as_inode_type(self.inner.get_unchecked_mut().f.file_type())
    }

    fn is_dir(&self) -> bool {
        self.inner.get_unchecked_mut().if_dir
    }

    fn read_at(&self, off: usize, buf: &mut [u8]) -> SyscallRet {
        let file = &mut self.inner.get_unchecked_mut().f;
        let path = file.path();
        let path = path.to_str().unwrap();
        file.file_open(path, O_RDONLY)
            .map_err(|e| SysErrNo::from(e))?;
        file.file_seek(off as i64, SEEK_SET)
            .map_err(|e| SysErrNo::from(e))?;
        let r = file.file_read(buf);
        r.map_err(|e| SysErrNo::from(e))
    }

    fn write_at(&self, off: usize, buf: &[u8]) -> SyscallRet {
        let file = &mut self.inner.get_unchecked_mut().f;
        let path = file.path();
        let path = path.to_str().unwrap();
        file.file_open(path, O_RDWR)
            .map_err(|e| SysErrNo::from(e))?;
        file.file_seek(off as i64, SEEK_SET)
            .map_err(|e| SysErrNo::from(e))?;
        let r = file.file_write(buf);
        r.map_err(|e| SysErrNo::from(e))
    }

    fn truncate(&self, size: usize) -> SyscallRet {
        let file = &mut self.inner.get_unchecked_mut().f;
        let path = file.path();
        let path = path.to_str().unwrap();
        file.file_open(path, O_RDWR | O_CREAT | O_TRUNC)
            .map_err(|e| SysErrNo::from(e))?;

        let t = file.file_truncate(size as u64);
        t.map_err(|e| SysErrNo::from(e))
    }

    fn rename(&self, path: &str, new_path: &str) -> SyscallRet {
        let file = &mut self.inner.get_unchecked_mut().f;
        file.file_rename(path, new_path)
            .map_or(Err(SysErrNo::ENOENT), |_| Ok(0))
    }

    fn set_owner(&self, uid: u32, gid: u32) -> SyscallRet {
        let file = &mut self.inner.get_unchecked_mut().f;
        file.set_owner(uid, gid).map_err(|e| SysErrNo::from(e))
    }

    fn set_timestamps(
        &self,
        atime: Option<u64>,
        mtime: Option<u64>,
        ctime: Option<u64>,
    ) -> SyscallRet {
        let file = &mut self.inner.get_unchecked_mut().f;
        file.set_time(atime, mtime, ctime)
            .map_err(|e| SysErrNo::from(e))
    }
    fn sync(&self) {
        self.inner.get_unchecked_mut().f.file_cache_flush();
    }

    fn read_all(&self) -> Result<Vec<u8>, SysErrNo> {
        let file = &mut self.inner.get_unchecked_mut().f;
        let file_type = as_inode_type(file.types());

        if file_type == InodeType::File {
            let path = file.path();
            let path = path.to_str().unwrap();
            file.file_open(path, O_RDONLY)
                .map_err(|e| SysErrNo::from(e))?;
            let size = file.file_size() as usize;
            let mut buf: Vec<u8> = vec![0; size];
            file.file_seek(0, SEEK_SET).map_err(|e| SysErrNo::from(e))?;
            let r = file.file_read(buf.as_mut_slice());
            if let Err(e) = r {
                return Err(SysErrNo::from(e));
            } else {
                return Ok(buf);
            }
        } else {
            //unimplemented!("not support!");
            assert!(file_type == InodeType::SymLink);
            let mut real_path_buf = [0u8; 256];
            file.file_readlink(&mut real_path_buf, 256)?;
            let end = real_path_buf.iter().position(|v| *v == 0).unwrap();
            let file_path = core::str::from_utf8(&real_path_buf[..end]).unwrap();
            let path = self.path();
            let (prefix, _) = path.rsplit_once("/").unwrap();
            let abs_path = format!("{}/{}", prefix, file_path);
            let real_file = self.find(&abs_path, OpenFlags::O_RDWR, 0)?;
            real_file.read_all()
        }
    }

    fn find(
        &self,
        path: &str,
        flags: OpenFlags,
        loop_times: usize,
    ) -> Result<Arc<dyn Inode>, SysErrNo> {
        //log::info!("[Inode.find] origin path={}", path);
        let file = &mut self.inner.get_unchecked_mut().f;
        if file.check_inode_exist(path, InodeTypes::EXT4_DE_DIR) {
            Ok(Arc::new(Ext4Inode::new(path, InodeTypes::EXT4_DE_DIR)))
        } else if file.check_inode_exist(path, InodeTypes::EXT4_DE_REG_FILE) {
            if flags.contains(OpenFlags::O_DIRECTORY) {
                return Err(SysErrNo::ENOTDIR);
            }
            Ok(Arc::new(Ext4Inode::new(path, InodeTypes::EXT4_DE_REG_FILE)))
        } else if file.check_inode_exist(path, InodeTypes::EXT4_DE_SYMLINK) {
            if flags.contains(OpenFlags::O_ASK_SYMLINK) {
                return Ok(Arc::new(Ext4Inode::new(path, InodeTypes::EXT4_DE_SYMLINK)));
            }
            if loop_times >= MAX_LOOPTIMES {
                debug!("error ELOOP!");
                return Err(SysErrNo::ELOOP);
            }
            // 符号链接文件应该返回对应的真实的文件
            let mut file_name = [0u8; 256];
            let file = Ext4Inode::new(path, InodeTypes::EXT4_DE_SYMLINK);
            file.read_link(&mut file_name, 256)?;
            let end = file_name.iter().position(|v| *v == 0).unwrap();
            let file_path = core::str::from_utf8(&file_name[..end]).unwrap();
            //log::info!("[Inode.find] file_path={}", file_path);
            let (prefix, _) = path.rsplit_once("/").unwrap();
            //log::info!("[Inode.find] prefix={}", prefix);
            let abs_path = format!("{}/{}", prefix, file_path);
            //debug!("[Inode.find] symlink abs_path={}", &abs_path);
            return self.find(&abs_path, flags, loop_times + 1);

            // Ok(Arc::new(Ext4Inode::new(path, InodeTypes::EXT4_DE_SYMLINK)))
        } else {
            Err(SysErrNo::ENOENT)
        }
    }

    fn fstat(&self) -> Kstat {
        let file = &mut self.inner.get_unchecked_mut().f;
        let stat = file.fstat().unwrap();
        Kstat {
            st_dev: stat.st_dev,
            st_ino: stat.st_ino,
            st_mode: stat.st_mode,
            st_nlink: stat.st_nlink,
            st_uid: stat.st_uid,
            st_gid: stat.st_gid,
            st_size: stat.st_size,
            st_blksize: stat.st_blksize,
            st_blocks: stat.st_blocks,
            st_atime: stat.st_atime,
            st_ctime: stat.st_ctime,
            st_mtime: stat.st_mtime,
            ..Kstat::default()
        }
    }
    fn read_dentry(&self, off: usize, len: usize) -> Result<(Vec<u8>, isize), SysErrNo> {
        let file = &mut self.inner.get_unchecked_mut().f;
        let entries = file
            .read_dir_from(off as u64)
            .map_err(|e| SysErrNo::from(e))?;
        let mut de: Vec<u8> = Vec::new();
        let (mut res, mut f_off) = (0usize, usize::MAX);
        for entry in entries {
            let dirent = Dirent {
                d_ino: entry.d_ino,
                d_off: entry.d_off,
                d_reclen: entry.d_reclen,
                d_type: entry.d_type,
                d_name: entry.d_name,
            };
            if res + dirent.len() > len {
                break;
            }
            res += dirent.len();
            f_off = dirent.off();
            de.extend_from_slice(dirent.as_bytes());
        }
        // (res != 0).then(|| (de, f_off as isize))
        assert!(res != 0);
        Ok((de, f_off as isize))
    }

    fn read_link(&self, buf: &mut [u8], bufsize: usize) -> SyscallRet {
        let file = &mut self.inner.get_unchecked_mut().f;
        file.file_readlink(buf, bufsize)
            .map_err(|e| SysErrNo::from(e))
    }

    fn sym_link(&self, target: &str, path: &str) -> SyscallRet {
        let file = &mut self.inner.get_unchecked_mut().f;
        file.file_fsymlink(target, path)
            .map_err(|e| SysErrNo::from(e))
    }

    fn link_cnt(&self) -> SyscallRet {
        let file = &mut self.inner.get_unchecked_mut().f;
        let r = file.links_cnt();
        if let Err(e) = r {
            if e == 2 {
                return Ok(0);
            } else {
                return Err(SysErrNo::from(e));
            }
        }
        Ok(r.unwrap() as usize)
    }

    fn unlink(&self, path: &str) -> SyscallRet {
        let file = &mut self.inner.get_unchecked_mut().f;
        file.file_remove(path).map_err(|e| SysErrNo::from(e))
    }

    fn path(&self) -> String {
        self.inner
            .get_unchecked_ref()
            .f
            .path()
            .into_string()
            .unwrap()
    }
    fn delay(&self) {
        self.inner.get_unchecked_mut().delay = true;
    }

    fn if_delay(&self) -> bool {
        self.inner.get_unchecked_mut().delay
    }

    fn fmode(&self) -> Result<u32, SysErrNo> {
        let file = &mut self.inner.get_unchecked_mut().f;
        file.file_mode().map_err(|e| SysErrNo::from(e))
    }
    fn fmode_set(&self, mode: u32) -> SyscallRet {
        let file = &mut self.inner.get_unchecked_mut().f;
        file.file_mode_set(mode).map_err(|e| SysErrNo::from(e))
    }
    fn fowner_set(&self, owner: u32, group: u32) -> SyscallRet {
        let file = &mut self.inner.get_unchecked_mut().f;
        file.set_owner(owner, group).map_err(|e| SysErrNo::from(e))
    }
}

impl Drop for Ext4Inode {
    fn drop(&mut self) {
        let path = self.path();
        let inner = self.inner.get_unchecked_mut();
        if inner.delay {
            debug!("Ext4Inode delays unlink {:?}", path);
            inner.f.file_remove(&path);
        }
        inner.f.file_close().expect("failed to close fd");
    }
}

fn as_ext4_de_type(types: InodeType) -> InodeTypes {
    match types {
        InodeType::BlockDevice => InodeTypes::EXT4_DE_BLKDEV,
        InodeType::CharDevice => InodeTypes::EXT4_DE_CHRDEV,
        InodeType::Dir => InodeTypes::EXT4_DE_DIR,
        InodeType::Fifo => InodeTypes::EXT4_DE_FIFO,
        InodeType::File => InodeTypes::EXT4_DE_REG_FILE,
        InodeType::Socket => InodeTypes::EXT4_DE_SOCK,
        InodeType::SymLink => InodeTypes::EXT4_DE_SYMLINK,
        InodeType::Unknown => InodeTypes::EXT4_DE_UNKNOWN,
    }
}

fn as_inode_type(types: InodeTypes) -> InodeType {
    match types {
        InodeTypes::EXT4_INODE_MODE_FIFO | InodeTypes::EXT4_DE_FIFO => InodeType::Fifo,
        InodeTypes::EXT4_INODE_MODE_CHARDEV | InodeTypes::EXT4_DE_CHRDEV => InodeType::CharDevice,
        InodeTypes::EXT4_INODE_MODE_DIRECTORY | InodeTypes::EXT4_DE_DIR => InodeType::Dir,
        InodeTypes::EXT4_INODE_MODE_BLOCKDEV | InodeTypes::EXT4_DE_BLKDEV => InodeType::BlockDevice,
        InodeTypes::EXT4_INODE_MODE_FILE | InodeTypes::EXT4_DE_REG_FILE => InodeType::File,
        InodeTypes::EXT4_INODE_MODE_SOFTLINK | InodeTypes::EXT4_DE_SYMLINK => InodeType::SymLink,
        InodeTypes::EXT4_INODE_MODE_SOCKET | InodeTypes::EXT4_DE_SOCK => InodeType::Socket,
        _ => {
            warn!("unknown file type: {:?}", types);
            unreachable!()
        }
    }
}

#[cfg(feature = "ltp")]
use hashbrown::HashMap;
#[cfg(feature = "ltp")]
use spin::Lazy;

#[cfg(feature = "ltp")]
static LINK_CACHE: Lazy<HashMap<String, String>> = Lazy::new(|| HashMap::new());
