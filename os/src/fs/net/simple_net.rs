use alloc::sync::Arc;

use super::{File, Kstat};
use crate::{
    fs::{make_pipe, Pipe, StMode},
    mm::UserBuffer,
    syscall::PollEvents,
    utils::SyscallRet,
};

pub struct SimpleSocket {
    read_end: Arc<Pipe>,
    write_end: Arc<Pipe>,
}

impl SimpleSocket {
    pub fn new(r_end: Arc<Pipe>, w_end: Arc<Pipe>) -> Self {
        Self {
            read_end: r_end,
            write_end: w_end,
        }
    }
}

pub fn make_socketpair() -> (Arc<SimpleSocket>, Arc<SimpleSocket>) {
    let (r1, w1) = make_pipe();
    let (r2, w2) = make_pipe();
    let socket1 = Arc::new(SimpleSocket::new(r1, w2));
    let socket2 = Arc::new(SimpleSocket::new(r2, w1));
    (socket1, socket2)
}

impl File for SimpleSocket {
    fn readable(&self) -> bool {
        true
    }

    fn writable(&self) -> bool {
        true
    }

    fn read(&self, buf: UserBuffer) -> SyscallRet {
        self.read_end.read(buf)
    }

    fn write(&self, buf: UserBuffer) -> SyscallRet {
        self.write_end.write(buf)
    }

    fn fstat(&self) -> Kstat {
        Kstat {
            st_mode: StMode::FSOCK.bits(),
            st_nlink: 1,
            ..Kstat::default()
        }
    }

    fn poll(&self, events: PollEvents) -> PollEvents {
        let mut revents = PollEvents::empty();
        let read_buffer = self.read_end.inner_lock();
        let write_buffer = self.write_end.inner_lock();
        if events.contains(PollEvents::IN) && read_buffer.available_read() > 0 {
            revents |= PollEvents::IN;
        }
        if events.contains(PollEvents::OUT) && write_buffer.available_write() > 0 {
            revents |= PollEvents::OUT;
        }
        revents
    }
}
