// !Deprecated
use alloc::{
    string::{String, ToString},
    sync::Arc,
};
use hashbrown::HashMap;
use spin::{Lazy, RwLock};

use super::Inode;

pub static FSIDX: Lazy<RwLock<HashMap<String, Arc<dyn Inode>>>> =
    Lazy::new(|| RwLock::new(HashMap::new()));

pub fn has_inode(path: &str) -> bool {
    FSIDX.read().contains_key(path)
}

pub fn find_inode_idx(path: &str) -> Option<Arc<dyn Inode>> {
    FSIDX.read().get(path).map(|inode| Arc::clone(inode))
}

pub fn insert_inode_idx(path: &str, inode: Arc<dyn Inode>) {
    FSIDX.write().insert(path.to_string(), inode);
}

pub fn remove_inode_idx(path: &str) {
    FSIDX.write().remove(path);
}

pub fn print_inner() {
    println!("{:#?}", FSIDX.read().keys());
}
