use super::{File, Kstat, StMode};
use crate::utils::{SysErrNo, SyscallRet};
use crate::{
    mm::UserBuffer, sbi::console_getchar, syscall::PollEvents, task::suspend_current_and_run_next,
};
use alloc::vec::Vec;
/// # 标准输入输出接口
/// `os/src/fs/stdio.rs`
/// ```
/// pub struct Stdin
/// pub struct Stdout
/// ```
//

const LF: usize = 0x0a;
const CR: usize = 0x0d;

pub struct Stdin;

pub struct Stdout;

impl File for Stdin {
    fn readable(&self) -> bool {
        true
    }
    fn writable(&self) -> bool {
        false
    }
    fn read(&self, mut user_buf: UserBuffer) -> SyscallRet {
        /*
        //一次读取单个字符
        assert_eq!(user_buf.len(), 1);
        // busy loop
        let mut c: usize;
        loop {
            c = console_getchar();
            if c == 0 {
                suspend_current_and_run_next();
                continue;
            } else {
                break;
            }
        }
        let ch = c as u8;
        unsafe {
            user_buf.buffers[0].as_mut_ptr().write_volatile(ch);
        }
        1
        */
        //一次读取多个字符
        let mut c: usize;
        let mut count: usize = 0;
        let mut buf = Vec::new();
        while count < user_buf.len() {
            c = console_getchar();
            match c {
                // `c > 255`是为了兼容OPENSBI，OPENSBI未获取字符时会返回-1
                0 | 256.. => {
                    suspend_current_and_run_next();
                    continue;
                }
                CR => {
                    buf.push(LF as u8);
                    count += 1;
                    break;
                }
                LF => {
                    buf.push(LF as u8);
                    count += 1;
                    break;
                }
                _ => {
                    buf.push(c as u8);
                    count += 1;
                }
            }
        }
        user_buf.write(buf.as_slice());
        Ok(count)
    }
    fn write(&self, _user_buf: UserBuffer) -> SyscallRet {
        Err(SysErrNo::EINVAL)
        // panic!("Cannot write to stdin!");
    }
    fn poll(&self, events: PollEvents) -> PollEvents {
        let mut revents = PollEvents::empty();
        if events.contains(PollEvents::IN) {
            revents |= PollEvents::IN;
        }
        revents
    }
    fn fstat(&self) -> Kstat {
        Kstat {
            st_mode: StMode::FCHR.bits(),
            st_nlink: 1,
            ..Kstat::default()
        }
    }
}

impl File for Stdout {
    fn readable(&self) -> bool {
        false
    }
    fn writable(&self) -> bool {
        true
    }
    fn read(&self, _user_buf: UserBuffer) -> SyscallRet {
        panic!("Cannot read from stdout!");
    }
    fn write(&self, user_buf: UserBuffer) -> SyscallRet {
        for buffer in user_buf.buffers.iter() {
            print!("{}", core::str::from_utf8(*buffer).unwrap());
        }
        Ok(user_buf.len())
    }
    fn poll(&self, events: PollEvents) -> PollEvents {
        let mut revents = PollEvents::empty();
        if events.contains(PollEvents::OUT) {
            revents |= PollEvents::OUT;
        }
        revents
    }
    fn fstat(&self) -> Kstat {
        Kstat {
            st_mode: StMode::FCHR.bits(),
            st_nlink: 1,
            ..Kstat::default()
        }
    }
}
