use core::fmt::{self, Write};

const STDIN: usize = 0;
const STDOUT: usize = 1;

// use spin::Mutex;

use super::{read, write};

struct Stdout;

impl Write for Stdout {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        write(STDOUT, s.as_bytes(), s.as_bytes().len());
        Ok(())
    }
}
// static LOCKED_STDOUT: Mutex<Stdout> = Mutex::new(Stdout {});
pub fn print(args: fmt::Arguments) {
    Stdout.write_fmt(args).unwrap();
    // LOCKED_STDOUT.lock().write_fmt(args).unwrap();
}

#[macro_export]
macro_rules! print {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!($fmt $(, $($arg)+)?));
    }
}

#[macro_export]
macro_rules! println {
    ($fmt: literal $(, $($arg: tt)+)?) => {
        $crate::console::print(format_args!(concat!($fmt, "\n") $(, $($arg)+)?));
    }
}

pub fn getchar() -> u8 {
    let mut c = [0u8; 1];
    read(STDIN, &mut c, 1);
    c[0]
}
