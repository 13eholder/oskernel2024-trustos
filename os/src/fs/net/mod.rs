use alloc::sync::Arc;

use super::{File, Kstat};

mod simple_net;
pub use simple_net::*;
pub struct Socket;

pub fn make_socket() -> Arc<dyn File> {
    Arc::new(Socket {})
}

impl File for Socket {
    fn readable(&self) -> bool {
        false
    }
    fn fstat(&self) -> Kstat {
        unimplemented!()
    }
}
